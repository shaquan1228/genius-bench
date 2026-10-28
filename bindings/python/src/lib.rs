use pyo3::IntoPyObjectExt;
use pyo3::exceptions::{PyAttributeError, PyRuntimeError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PyFloat, PyInt, PyList, PyString, PyTuple};
use semantic_core as core;
use std::collections::HashSet;

pyo3::create_exception!(kernel._native, ValidationError, PyValueError);
pyo3::create_exception!(kernel._native, GradingError, PyRuntimeError);

fn tuple<'py, T: IntoPyObject<'py>>(py: Python<'py>, items: Vec<T>) -> PyResult<Py<PyTuple>> {
    Ok(PyTuple::new(py, items)?.unbind())
}

fn check_depth(depth: usize, kind: &str) -> PyResult<()> {
    if depth > core::NESTING_LIMIT {
        Err(PyValueError::new_err(format!(
            "{kind} nesting exceeds {} levels",
            core::NESTING_LIMIT
        )))
    } else {
        Ok(())
    }
}

fn check_value_depth(value: &core::Value, depth: usize) -> PyResult<()> {
    check_depth(depth + value.nesting_depth(), "Kernel value")
}

fn check_predicate_depth(predicate: &core::Predicate, depth: usize) -> PyResult<()> {
    check_depth(depth + predicate.nesting_depth(), "Predicate")
}

fn sequence_snapshot<'py>(value: &Bound<'py, PyAny>) -> PyResult<Vec<Bound<'py, PyAny>>> {
    if let Ok(items) = value.cast::<PyList>() {
        Ok(items.to_tuple().iter().collect())
    } else if let Ok(items) = value.cast::<PyTuple>() {
        Ok(items.iter().collect())
    } else {
        value.try_iter()?.collect()
    }
}

fn convert(
    value: &Bound<'_, PyAny>,
    active: &mut HashSet<usize>,
    depth: usize,
) -> PyResult<core::Value> {
    if let Ok(value) = value.extract::<PyRef<'_, Value>>() {
        let value = core::Value::from(value.clone());
        check_value_depth(&value, depth)?;
        return Ok(value);
    }
    if let Ok(record) = value.extract::<PyRef<'_, Record>>() {
        check_depth(depth + 1, "Kernel value")?;
        for (_, item) in &record.0.0 {
            check_value_depth(item, depth + 1)?;
        }
        return Ok(core::Value::Record(record.0.0.clone()));
    }
    if value.is_none() {
        return Ok(core::Value::Null);
    }
    if value.is_instance_of::<PyBool>() {
        return Ok(core::Value::Bool(value.extract()?));
    }
    if value.is_instance_of::<PyInt>() {
        return Ok(core::Value::Int(value.extract()?));
    }
    if value.is_instance_of::<PyFloat>() {
        return Ok(core::Value::Float(value.extract()?));
    }
    if value.is_instance_of::<PyString>() {
        return Ok(core::Value::String(value.extract()?));
    }
    let address = value.as_ptr() as usize;
    if !active.insert(address) {
        return Err(PyValueError::new_err("Cyclic kernel value"));
    }
    check_depth(depth + 1, "Kernel value")?;
    let result = if let Ok(mapping) = value.cast::<PyDict>() {
        mapping
            .iter()
            .collect::<Vec<_>>()
            .into_iter()
            .map(|(name, item)| {
                Ok((
                    name.extract::<String>()?,
                    convert(&item, active, depth + 1)?,
                ))
            })
            .collect::<PyResult<Vec<_>>>()
            .map(core::Value::Record)
    } else if value.is_instance_of::<PyList>() || value.is_instance_of::<PyTuple>() {
        sequence_snapshot(value)?
            .into_iter()
            .map(|item| convert(&item, active, depth + 1))
            .collect::<PyResult<Vec<_>>>()
            .map(core::Value::List)
    } else {
        match value.getattr("__kernel_value__") {
            Ok(method) => convert(&method.call0()?, active, depth + 1),
            Err(error) if error.is_instance_of::<PyAttributeError>(value.py()) => {
                Err(PyTypeError::new_err(format!(
                    "Unsupported kernel value: {}",
                    value.get_type().name()?
                )))
            }
            Err(error) => Err(error),
        }
    };
    active.remove(&address);
    result
}

fn host_value(value: &Bound<'_, PyAny>) -> PyResult<core::Value> {
    convert(value, &mut HashSet::new(), 0)
}

fn host_fields(fields: Vec<(String, Bound<'_, PyAny>)>) -> PyResult<Vec<(String, core::Value)>> {
    fields
        .into_iter()
        .map(|(name, value)| Ok((name, host_value(&value)?)))
        .collect()
}

fn host_values(items: &Bound<'_, PyAny>, depth: usize) -> PyResult<Vec<core::Value>> {
    let mut active = HashSet::new();
    items
        .extract::<Vec<Bound<'_, PyAny>>>()?
        .iter()
        .map(|item| convert(item, &mut active, depth))
        .collect()
}

fn host_record(fields: &Bound<'_, PyAny>) -> PyResult<Vec<(String, core::Value)>> {
    let mut active = HashSet::new();
    sequence_snapshot(fields)?
        .into_iter()
        .map(|pair| pair.extract::<(String, Bound<'_, PyAny>)>())
        .collect::<PyResult<Vec<_>>>()?
        .into_iter()
        .map(|(name, item)| Ok((name, convert(&item, &mut active, 1)?)))
        .collect()
}

fn nested(predicate: core::Predicate) -> PyResult<core::Predicate> {
    check_predicate_depth(&predicate, 1)?;
    Ok(predicate)
}

fn named<'py, C: Clone, B: IntoPyObject<'py>>(
    py: Python<'py>,
    items: &[(String, C)],
    wrap: impl Fn(C) -> B,
) -> PyResult<Bound<'py, PyAny>> {
    Ok(PyTuple::new(
        py,
        items
            .iter()
            .map(|(name, item)| (name.clone(), wrap(item.clone()))),
    )?
    .into_any())
}

fn values<'py>(py: Python<'py>, items: &[core::Value]) -> PyResult<Bound<'py, PyAny>> {
    Ok(PyTuple::new(py, items.iter().cloned().map(Value::from))?.into_any())
}

/* Variant fields hold core data, accept native Python input, and read back as tuples
of typed variants. PyO3 builds each variant's constructor and getters from these types.
https://pyo3.rs/v0.29.3/class#complex-enums */
macro_rules! field {
    ($name:ident($inner:ty), |$input:ident| $extract:expr, |$data:ident, $py:ident| $output:expr) => {
        #[derive(Clone)]
        struct $name($inner);

        impl<'py> FromPyObject<'_, 'py> for $name {
            type Error = PyErr;
            fn extract($input: Borrowed<'_, 'py, PyAny>) -> PyResult<Self> {
                Ok(Self($extract))
            }
        }

        impl<'py> IntoPyObject<'py> for &$name {
            type Target = PyAny;
            type Output = Bound<'py, PyAny>;
            type Error = PyErr;
            fn into_pyobject(self, $py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
                let $data = &self.0;
                $output
            }
        }
    };
}

field!(Path(Vec<String>), |input| input.extract()?, |path, py| Ok(
    PyTuple::new(py, path)?.into_any()
));
field!(
    Data(core::Value),
    |input| host_value(&input)?,
    |value, py| Value::from(value.clone()).into_bound_py_any(py)
);
field!(
    Allowed(Vec<core::Value>),
    |input| host_values(&input, 0)?,
    |items, py| values(py, items)
);
field!(
    Items(Vec<core::Value>),
    |input| host_values(&input, 1)?,
    |items, py| values(py, items)
);
field!(
    Fields(Vec<(String, core::Value)>),
    |input| host_record(&input)?,
    |fields, py| named(py, fields, Value::from)
);
field!(
    Pred(core::Predicate),
    |input| input.extract::<Predicate>()?.into(),
    |predicate, py| Predicate::from(predicate.clone()).into_bound_py_any(py)
);
field!(
    Operand(core::Predicate),
    |input| nested(input.extract::<Predicate>()?.into())?,
    |predicate, py| Predicate::from(predicate.clone()).into_bound_py_any(py)
);
field!(
    Checks(Vec<(String, core::Predicate)>),
    |input| input
        .extract::<Vec<(String, Predicate)>>()?
        .into_iter()
        .map(|(name, check)| Ok((name, nested(check.into())?)))
        .collect::<PyResult<_>>()?,
    |checks, py| named(py, checks, Predicate::from)
);
field!(
    Assessments(Vec<(String, core::Assessment)>),
    |input| input
        .extract::<Vec<(String, Assessment)>>()?
        .into_iter()
        .map(|(name, check)| (name, check.into()))
        .collect(),
    |checks, py| named(py, checks, Assessment::from)
);

#[pyclass(module = "kernel._native", frozen, from_py_object)]
#[derive(Clone)]
enum Value {
    Null {},
    #[pyo3(constructor = (*, value))]
    Bool {
        value: bool,
    },
    #[pyo3(constructor = (*, value))]
    Int {
        value: i64,
    },
    #[pyo3(constructor = (*, value))]
    Float {
        value: f64,
    },
    #[pyo3(constructor = (*, value))]
    String {
        value: String,
    },
    #[pyo3(constructor = (*, items))]
    List {
        items: Items,
    },
    #[pyo3(constructor = (*, fields))]
    Record {
        fields: Fields,
    },
}

impl From<core::Value> for Value {
    fn from(value: core::Value) -> Self {
        match value {
            core::Value::Null => Self::Null {},
            core::Value::Bool(value) => Self::Bool { value },
            core::Value::Int(value) => Self::Int { value },
            core::Value::Float(value) => Self::Float { value },
            core::Value::String(value) => Self::String { value },
            core::Value::List(items) => Self::List {
                items: Items(items),
            },
            core::Value::Record(fields) => Self::Record {
                fields: Fields(fields),
            },
        }
    }
}

impl From<Value> for core::Value {
    fn from(value: Value) -> Self {
        match value {
            Value::Null {} => Self::Null,
            Value::Bool { value } => Self::Bool(value),
            Value::Int { value } => Self::Int(value),
            Value::Float { value } => Self::Float(value),
            Value::String { value } => Self::String(value),
            Value::List { items } => Self::List(items.0),
            Value::Record { fields } => Self::Record(fields.0),
        }
    }
}

#[pymethods]
impl Value {
    #[staticmethod]
    #[pyo3(signature = (*, value))]
    fn from_python(value: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(host_value(value)?.into())
    }
    #[pyo3(signature = (*, path))]
    fn get(&self, path: Vec<String>) -> Option<Self> {
        core::Value::from(self.clone())
            .get(&path)
            .cloned()
            .map(Self::from)
    }
    #[pyo3(signature = (*, other))]
    fn equal(&self, other: &Bound<'_, PyAny>) -> PyResult<bool> {
        Ok(core::Value::from(self.clone()).equal(&host_value(other)?))
    }
}

#[pyclass(module = "kernel._native", frozen, from_py_object)]
#[derive(Clone)]
struct Record(Fields);

#[pymethods]
impl Record {
    #[new]
    #[pyo3(signature = (*, fields))]
    fn new(fields: Fields) -> Self {
        Self(fields)
    }
    #[getter]
    fn fields<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        (&self.0).into_pyobject(py)
    }
    fn __kernel_value__(&self) -> Value {
        Value::Record {
            fields: self.0.clone(),
        }
    }
}

#[pyclass(module = "kernel._native", frozen, from_py_object)]
#[derive(Clone)]
enum Predicate {
    #[pyo3(constructor = (*, path))]
    Present { path: Path },
    #[pyo3(constructor = (*, path, expected))]
    Equals { path: Path, expected: Data },
    #[pyo3(constructor = (*, path, allowed))]
    Within { path: Path, allowed: Allowed },
    #[pyo3(constructor = (*, path, item))]
    Contains { path: Path, item: Data },
    #[pyo3(constructor = (*, path, min_words, max_words))]
    Words {
        path: Path,
        min_words: i64,
        max_words: i64,
    },
    #[pyo3(constructor = (*, checks))]
    All { checks: Checks },
    #[pyo3(constructor = (*, checks))]
    Any { checks: Checks },
    #[pyo3(constructor = (*, check))]
    Not { check: Operand },
}

impl From<core::Predicate> for Predicate {
    fn from(predicate: core::Predicate) -> Self {
        match predicate {
            core::Predicate::Present { path } => Self::Present { path: Path(path) },
            core::Predicate::Equals { path, expected } => Self::Equals {
                path: Path(path),
                expected: Data(expected),
            },
            core::Predicate::Within { path, allowed } => Self::Within {
                path: Path(path),
                allowed: Allowed(allowed),
            },
            core::Predicate::Contains { path, item } => Self::Contains {
                path: Path(path),
                item: Data(item),
            },
            core::Predicate::Words {
                path,
                min_words,
                max_words,
            } => Self::Words {
                path: Path(path),
                min_words,
                max_words,
            },
            core::Predicate::All { checks } => Self::All {
                checks: Checks(checks),
            },
            core::Predicate::Any { checks } => Self::Any {
                checks: Checks(checks),
            },
            core::Predicate::Not { check } => Self::Not {
                check: Operand(*check),
            },
        }
    }
}

impl From<Predicate> for core::Predicate {
    fn from(predicate: Predicate) -> Self {
        match predicate {
            Predicate::Present { path } => Self::Present { path: path.0 },
            Predicate::Equals { path, expected } => Self::Equals {
                path: path.0,
                expected: expected.0,
            },
            Predicate::Within { path, allowed } => Self::Within {
                path: path.0,
                allowed: allowed.0,
            },
            Predicate::Contains { path, item } => Self::Contains {
                path: path.0,
                item: item.0,
            },
            Predicate::Words {
                path,
                min_words,
                max_words,
            } => Self::Words {
                path: path.0,
                min_words,
                max_words,
            },
            Predicate::All { checks } => Self::All { checks: checks.0 },
            Predicate::Any { checks } => Self::Any { checks: checks.0 },
            Predicate::Not { check } => Self::Not {
                check: Box::new(check.0),
            },
        }
    }
}

#[pymethods]
impl Predicate {
    #[pyo3(signature = (*, observation))]
    fn holds(&self, observation: &Bound<'_, PyAny>) -> PyResult<bool> {
        Ok(core::Predicate::from(self.clone()).holds(&host_value(observation)?))
    }
}

#[pyclass(module = "kernel._native", frozen, from_py_object)]
#[derive(Clone)]
enum Verification {
    Unchecked {},
    Passed {},
    Failed {},
    InvalidSubmission {},
    TimedOut {},
    #[pyo3(constructor = (*, detail))]
    HarnessError {
        detail: String,
    },
}

impl From<core::Verification> for Verification {
    fn from(verification: core::Verification) -> Self {
        match verification {
            core::Verification::Unchecked => Self::Unchecked {},
            core::Verification::Passed => Self::Passed {},
            core::Verification::Failed => Self::Failed {},
            core::Verification::InvalidSubmission => Self::InvalidSubmission {},
            core::Verification::TimedOut => Self::TimedOut {},
            core::Verification::HarnessError { detail } => Self::HarnessError { detail },
        }
    }
}

impl From<Verification> for core::Verification {
    fn from(verification: Verification) -> Self {
        match verification {
            Verification::Unchecked {} => Self::Unchecked,
            Verification::Passed {} => Self::Passed,
            Verification::Failed {} => Self::Failed,
            Verification::InvalidSubmission {} => Self::InvalidSubmission,
            Verification::TimedOut {} => Self::TimedOut,
            Verification::HarnessError { detail } => Self::HarnessError { detail },
        }
    }
}

#[pymethods]
impl Verification {
    #[getter]
    fn status(&self) -> &'static str {
        core::Verification::from(self.clone()).status()
    }
}

#[pyclass(module = "kernel._native", frozen, from_py_object)]
#[derive(Clone)]
struct Outcome(core::Outcome);

#[pymethods]
impl Outcome {
    #[new]
    #[pyo3(signature = (*, changed_files, verification, response, facts))]
    fn new(
        changed_files: Vec<String>,
        verification: Verification,
        response: &Bound<'_, PyAny>,
        facts: Vec<(String, Bound<'_, PyAny>)>,
    ) -> PyResult<Self> {
        let response = host_value(response)?;
        let facts = host_fields(facts)?;
        check_value_depth(&response, 1)?;
        for (_, value) in &facts {
            check_value_depth(value, 1)?;
        }
        Ok(Self(core::Outcome {
            changed_files,
            verification: verification.into(),
            response,
            facts,
        }))
    }
    #[getter]
    fn changed_files(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        tuple(py, self.0.changed_files.clone())
    }
    #[getter]
    fn verification(&self) -> Verification {
        self.0.verification.clone().into()
    }
    #[getter]
    fn response(&self) -> Value {
        self.0.response.clone().into()
    }
    #[getter]
    fn facts(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        tuple(
            py,
            self.0
                .facts
                .iter()
                .map(|(name, value)| (name.clone(), Value::from(value.clone())))
                .collect(),
        )
    }
    fn observation(&self) -> Value {
        self.0.observation().into()
    }
}

#[pyclass(module = "kernel._native", frozen, from_py_object)]
#[derive(Clone)]
enum Assessment {
    #[pyo3(constructor = (*, predicate, observation))]
    Check { predicate: Pred, observation: Data },
    #[pyo3(constructor = (*, checks))]
    All { checks: Assessments },
}

impl From<core::Assessment> for Assessment {
    fn from(assessment: core::Assessment) -> Self {
        match assessment {
            core::Assessment::Check {
                predicate,
                observation,
            } => Self::Check {
                predicate: Pred(predicate),
                observation: Data(observation),
            },
            core::Assessment::All { checks } => Self::All {
                checks: Assessments(checks),
            },
        }
    }
}

impl From<Assessment> for core::Assessment {
    fn from(assessment: Assessment) -> Self {
        match assessment {
            Assessment::Check {
                predicate,
                observation,
            } => Self::Check {
                predicate: predicate.0,
                observation: observation.0,
            },
            Assessment::All { checks } => Self::All { checks: checks.0 },
        }
    }
}

#[pymethods]
impl Assessment {
    #[new]
    #[pyo3(signature = (*, predicate, observation))]
    fn new(py: Python<'_>, predicate: Pred, observation: Data) -> PyResult<Bound<'_, Self>> {
        Self::from(core::Assessment::new(predicate.0, observation.0)).into_pyobject(py)
    }
    fn passed(&self) -> bool {
        core::Assessment::from(self.clone()).passed()
    }
    fn observed(&self) -> Option<Value> {
        core::Assessment::from(self.clone())
            .observed()
            .cloned()
            .map(Value::from)
    }
    fn scores(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        tuple(py, core::Assessment::from(self.clone()).scores())
    }
}

#[pyclass(module = "kernel._native", frozen, from_py_object)]
#[derive(Clone)]
struct SourceFile(core::File);

#[pymethods]
impl SourceFile {
    #[new]
    #[pyo3(signature = (*, path, contents))]
    fn new(path: String, contents: String) -> Self {
        Self(core::File { path, contents })
    }
    #[getter]
    fn path(&self) -> &str {
        &self.0.path
    }
    #[getter]
    fn contents(&self) -> &str {
        &self.0.contents
    }
}

#[pyclass(module = "kernel._native", frozen, from_py_object)]
#[derive(Clone)]
struct Instruction(core::Instruction);

#[pymethods]
impl Instruction {
    #[new]
    #[pyo3(signature = (*, prompt))]
    fn new(prompt: String) -> Self {
        Self(core::Instruction { prompt })
    }
    #[getter]
    fn prompt(&self) -> &str {
        &self.0.prompt
    }
}

#[pyclass(module = "kernel._native", frozen, from_py_object)]
#[derive(Clone)]
struct Verifier(core::Verifier);

#[pymethods]
impl Verifier {
    #[new]
    #[pyo3(signature = (*, inputs, files, tools, command, stdin, timeout_seconds))]
    fn new(
        inputs: Vec<String>,
        files: Vec<SourceFile>,
        tools: Vec<SourceFile>,
        command: Vec<String>,
        stdin: String,
        timeout_seconds: i64,
    ) -> Self {
        Self(core::Verifier {
            inputs,
            files: files.into_iter().map(|file| file.0).collect(),
            tools: tools.into_iter().map(|file| file.0).collect(),
            command,
            stdin,
            timeout_seconds,
        })
    }
    #[getter]
    fn inputs(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        tuple(py, self.0.inputs.clone())
    }
    #[getter]
    fn files(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        tuple(py, self.0.files.iter().cloned().map(SourceFile).collect())
    }
    #[getter]
    fn tools(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        tuple(py, self.0.tools.iter().cloned().map(SourceFile).collect())
    }
    #[getter]
    fn command(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        tuple(py, self.0.command.clone())
    }
    #[getter]
    fn stdin(&self) -> &str {
        &self.0.stdin
    }
    #[getter]
    fn timeout_seconds(&self) -> i64 {
        self.0.timeout_seconds
    }
}

#[pyclass(module = "kernel._native", frozen, from_py_object)]
#[derive(Clone)]
struct TextRule(core::TextRule);

#[pymethods]
impl TextRule {
    #[new]
    #[pyo3(signature = (*, field, min_words, max_words))]
    fn new(field: String, min_words: i64, max_words: i64) -> Self {
        Self(core::TextRule {
            field,
            min_words,
            max_words,
        })
    }
    #[getter]
    fn field(&self) -> &str {
        &self.0.field
    }
    #[getter]
    fn min_words(&self) -> i64 {
        self.0.min_words
    }
    #[getter]
    fn max_words(&self) -> i64 {
        self.0.max_words
    }
}

#[pyclass(module = "kernel._native", frozen, from_py_object)]
#[derive(Clone)]
struct Rubric(core::Rubric);

#[pymethods]
impl Rubric {
    #[new]
    #[pyo3(signature = (*, expected, text_rule))]
    fn new(
        expected: Vec<(String, Bound<'_, PyAny>)>,
        text_rule: Option<TextRule>,
    ) -> PyResult<Self> {
        Ok(Self(core::Rubric {
            expected: host_fields(expected)?,
            text_rule: text_rule.map(|rule| rule.0),
        }))
    }
    #[getter]
    fn expected(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        tuple(
            py,
            self.0
                .expected
                .iter()
                .map(|(name, value)| (name.clone(), Value::from(value.clone())))
                .collect(),
        )
    }
    #[getter]
    fn text_rule(&self) -> Option<TextRule> {
        self.0.text_rule.clone().map(TextRule)
    }
}

#[pyclass(module = "kernel._native", frozen, from_py_object)]
#[derive(Clone)]
enum Criterion {
    #[pyo3(constructor = (*, verifier))]
    Code { verifier: Verifier },
    #[pyo3(constructor = (*, rubric))]
    Answer { rubric: Rubric },
    #[pyo3(constructor = (*, predicate))]
    State { predicate: Pred },
}

impl From<core::Criterion> for Criterion {
    fn from(criterion: core::Criterion) -> Self {
        match criterion {
            core::Criterion::Code { verifier } => Self::Code {
                verifier: Verifier(verifier),
            },
            core::Criterion::Answer { rubric } => Self::Answer {
                rubric: Rubric(rubric),
            },
            core::Criterion::State { predicate } => Self::State {
                predicate: Pred(predicate),
            },
        }
    }
}

impl From<Criterion> for core::Criterion {
    fn from(criterion: Criterion) -> Self {
        match criterion {
            Criterion::Code { verifier } => Self::Code {
                verifier: verifier.0,
            },
            Criterion::Answer { rubric } => Self::Answer { rubric: rubric.0 },
            Criterion::State { predicate } => Self::State {
                predicate: predicate.0,
            },
        }
    }
}

#[pymethods]
impl Criterion {
    fn condition(&self) -> Predicate {
        core::Criterion::from(self.clone()).condition().into()
    }
}

#[pyclass(module = "kernel._native", frozen, from_py_object)]
#[derive(Clone)]
struct Reference(core::Reference);

#[pymethods]
impl Reference {
    #[new]
    #[pyo3(signature = (*, edits, completion))]
    fn new(edits: Vec<SourceFile>, completion: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self(core::Reference {
            edits: edits.into_iter().map(|file| file.0).collect(),
            completion: host_value(completion)?,
        }))
    }
    #[getter]
    fn edits(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        tuple(py, self.0.edits.iter().cloned().map(SourceFile).collect())
    }
    #[getter]
    fn completion(&self) -> Value {
        self.0.completion.clone().into()
    }
}

#[pyclass(module = "kernel._native", frozen, from_py_object)]
#[derive(Clone)]
struct Case(core::Case);

#[pymethods]
impl Case {
    #[getter]
    fn case_id(&self) -> &str {
        &self.0.case_id
    }
    #[getter]
    fn prompt(&self) -> &str {
        &self.0.prompt
    }
    #[getter]
    fn files(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        tuple(py, self.0.files.iter().cloned().map(SourceFile).collect())
    }
}

#[pyclass(module = "kernel._native", frozen, from_py_object)]
#[derive(Clone)]
struct Issue(core::Issue);

#[pymethods]
impl Issue {
    #[getter]
    fn field(&self) -> &str {
        &self.0.field
    }
    #[getter]
    fn message(&self) -> &str {
        &self.0.message
    }
}

fn validation_error(py: Python<'_>, failure: core::ValidationFailure) -> PyErr {
    let error = ValidationError::new_err(failure.to_string());
    let result = (|| -> PyResult<()> {
        error.value(py).setattr("case_id", failure.case_id)?;
        error.value(py).setattr(
            "issues",
            tuple(py, failure.issues.into_iter().map(Issue).collect())?,
        )?;
        Ok(())
    })();
    result.err().unwrap_or(error)
}

fn grading_error(py: Python<'_>, failure: core::GradingFailure) -> PyErr {
    let error = GradingError::new_err(failure.to_string());
    let kind = failure.kind();
    let core::GradingFailure::VerificationHarnessError { detail } = failure;
    let result = (|| -> PyResult<()> {
        error.value(py).setattr("failure", kind)?;
        error.value(py).setattr("detail", detail)?;
        Ok(())
    })();
    result.err().unwrap_or(error)
}

#[pyclass(module = "kernel._native", frozen, from_py_object)]
#[derive(Clone)]
struct Task(core::Task);

#[pymethods]
impl Task {
    #[new]
    #[pyo3(signature = (*, id, fixture, instruction, criterion, allowed_files, reference))]
    fn new(
        id: String,
        fixture: Vec<SourceFile>,
        instruction: Instruction,
        criterion: Criterion,
        allowed_files: Vec<String>,
        reference: Reference,
        py: Python<'_>,
    ) -> PyResult<Self> {
        let criterion = core::Criterion::from(criterion);
        if let core::Criterion::State { predicate } = &criterion {
            check_predicate_depth(predicate, 1)?;
        }
        core::Task::new(
            id,
            fixture.into_iter().map(|file| file.0).collect(),
            instruction.0,
            criterion,
            allowed_files,
            reference.0,
        )
        .map(Self)
        .map_err(|failure| validation_error(py, failure))
    }
    #[getter]
    fn id(&self) -> &str {
        self.0.id()
    }
    #[getter]
    fn fixture(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        tuple(
            py,
            self.0.fixture().iter().cloned().map(SourceFile).collect(),
        )
    }
    #[getter]
    fn instruction(&self) -> Instruction {
        Instruction(self.0.instruction().clone())
    }
    #[getter]
    fn criterion(&self) -> Criterion {
        self.0.criterion().clone().into()
    }
    #[getter]
    fn allowed_files(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        tuple(py, self.0.allowed_files().to_vec())
    }
    #[getter]
    fn reference(&self) -> Reference {
        Reference(self.0.reference().clone())
    }
    fn project(&self) -> Case {
        Case(self.0.project())
    }
    fn checks(&self) -> Option<Verifier> {
        self.0.checks().cloned().map(Verifier)
    }
    #[pyo3(signature = (*, outcome))]
    fn assess(&self, py: Python<'_>, outcome: PyRef<'_, Outcome>) -> PyResult<Assessment> {
        self.0
            .assess(&outcome.0)
            .map(Assessment::from)
            .map_err(|failure| grading_error(py, failure))
    }
    #[pyo3(signature = (*, outcome))]
    fn grade(&self, py: Python<'_>, outcome: PyRef<'_, Outcome>) -> PyResult<Py<PyTuple>> {
        tuple(
            py,
            self.0
                .grade(&outcome.0)
                .map_err(|failure| grading_error(py, failure))?,
        )
    }
    #[pyo3(signature = (*, verification))]
    fn grade_reference(&self, py: Python<'_>, verification: Verification) -> PyResult<Py<PyTuple>> {
        let reference = self.0.reference();
        check_value_depth(&reference.completion, 1)?;
        let outcome = core::Outcome {
            changed_files: reference
                .edits
                .iter()
                .map(|file| file.path.clone())
                .collect(),
            verification: verification.into(),
            response: reference.completion.clone(),
            facts: vec![],
        };
        tuple(
            py,
            self.0
                .grade(&outcome)
                .map_err(|failure| grading_error(py, failure))?,
        )
    }
}

#[pyfunction]
fn version() -> &'static str {
    core::version()
}

#[pyfunction]
fn source_hash() -> &'static str {
    core::source_hash()
}

#[pymodule]
fn _native(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<Value>()?;
    module.add_class::<Record>()?;
    module.add_class::<Predicate>()?;
    module.add_class::<Verification>()?;
    module.add_class::<Outcome>()?;
    module.add_class::<Assessment>()?;
    module.add_class::<SourceFile>()?;
    module.add_class::<Instruction>()?;
    module.add_class::<Verifier>()?;
    module.add_class::<TextRule>()?;
    module.add_class::<Rubric>()?;
    module.add_class::<Criterion>()?;
    module.add_class::<Reference>()?;
    module.add_class::<Case>()?;
    module.add_class::<Issue>()?;
    module.add_class::<Task>()?;
    module.add("ValidationError", module.py().get_type::<ValidationError>())?;
    module.add("GradingError", module.py().get_type::<GradingError>())?;
    module.add(
        "REQUIRED_SCORES",
        tuple(module.py(), core::REQUIRED_SCORES.to_vec())?,
    )?;
    module.add_function(wrap_pyfunction!(version, module)?)?;
    module.add_function(wrap_pyfunction!(source_hash, module)?)?;
    Ok(())
}
