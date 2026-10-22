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
    if depth > 256 {
        Err(PyValueError::new_err(format!(
            "{kind} nesting exceeds 256 levels"
        )))
    } else {
        Ok(())
    }
}

fn check_value_depth(value: &core::Value, depth: usize) -> PyResult<()> {
    let mut pending = vec![(value, depth)];
    while let Some((value, depth)) = pending.pop() {
        match value {
            core::Value::List(items) => {
                check_depth(depth + 1, "Kernel value")?;
                pending.extend(items.iter().map(|item| (item, depth + 1)));
            }
            core::Value::Record(fields) => {
                check_depth(depth + 1, "Kernel value")?;
                pending.extend(fields.iter().map(|(_, item)| (item, depth + 1)));
            }
            _ => {}
        }
    }
    Ok(())
}

fn check_predicate_depth(predicate: &core::Predicate, depth: usize) -> PyResult<()> {
    let mut pending = vec![(predicate, depth)];
    while let Some((predicate, depth)) = pending.pop() {
        check_depth(depth + 1, "Predicate")?;
        match predicate {
            core::Predicate::All { checks } | core::Predicate::Any { checks } => {
                pending.extend(checks.iter().map(|(_, check)| (check, depth + 1)));
            }
            core::Predicate::Not { check } => pending.push((check, depth + 1)),
            _ => {}
        }
    }
    Ok(())
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
        check_value_depth(&value.0, depth)?;
        return Ok(value.0.clone());
    }
    if let Ok(record) = value.extract::<PyRef<'_, Record>>() {
        check_depth(depth + 1, "Kernel value")?;
        for (_, item) in &record.0 {
            check_value_depth(item, depth + 1)?;
        }
        return Ok(core::Value::Record(record.0.clone()));
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

#[pyclass(module = "kernel._native", frozen, from_py_object)]
#[derive(Clone)]
struct Value(core::Value);

#[pymethods]
impl Value {
    #[staticmethod]
    #[pyo3(signature = (*, value))]
    fn from_python(value: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self(convert(value, &mut HashSet::new(), 0)?))
    }
    #[getter]
    fn kind(&self) -> &'static str {
        match self.0 {
            core::Value::Null => "null",
            core::Value::Bool(_) => "bool",
            core::Value::Int(_) => "int",
            core::Value::Float(_) => "float",
            core::Value::String(_) => "string",
            core::Value::List(_) => "list",
            core::Value::Record(_) => "record",
        }
    }
    #[getter]
    fn scalar(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        Ok(match &self.0 {
            core::Value::Null => py.None(),
            core::Value::Bool(value) => value.into_pyobject(py)?.to_owned().into_any().unbind(),
            core::Value::Int(value) => value.into_pyobject(py)?.into_any().unbind(),
            core::Value::Float(value) => value.into_pyobject(py)?.into_any().unbind(),
            core::Value::String(value) => value.into_pyobject(py)?.into_any().unbind(),
            _ => return Err(PyTypeError::new_err("Structured values have no scalar")),
        })
    }
    #[getter]
    fn items(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        let core::Value::List(items) = &self.0 else {
            return Err(PyTypeError::new_err("Value is not a list"));
        };
        tuple(py, items.iter().cloned().map(Self).collect())
    }
    #[getter]
    fn fields(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        let core::Value::Record(fields) = &self.0 else {
            return Err(PyTypeError::new_err("Value is not a record"));
        };
        tuple(
            py,
            fields
                .iter()
                .map(|(name, item)| (name.clone(), Self(item.clone())))
                .collect(),
        )
    }
    #[pyo3(signature = (*, path))]
    fn get(&self, path: Vec<String>) -> Option<Self> {
        self.0.get(&path).cloned().map(Self)
    }
    #[pyo3(signature = (*, other))]
    fn equal(&self, other: PyRef<'_, Self>) -> bool {
        self.0.equal(&other.0)
    }
}

#[pyclass(module = "kernel._native", frozen, from_py_object)]
#[derive(Clone)]
struct Record(Vec<(String, core::Value)>);

#[pymethods]
impl Record {
    #[new]
    #[pyo3(signature = (*, fields))]
    fn new(fields: &Bound<'_, PyAny>) -> PyResult<Self> {
        let mut active = HashSet::new();
        let fields = sequence_snapshot(fields)?
            .into_iter()
            .map(|pair| {
                let (name, item): (String, Bound<'_, PyAny>) = pair.extract()?;
                Ok((name, item))
            })
            .collect::<PyResult<Vec<_>>>()?
            .into_iter()
            .map(|(name, item)| Ok((name, convert(&item, &mut active, 1)?)))
            .collect::<PyResult<_>>()?;
        Ok(Self(fields))
    }
    #[getter]
    fn fields(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        tuple(
            py,
            self.0
                .iter()
                .map(|(name, item)| (name.clone(), Value(item.clone())))
                .collect(),
        )
    }
    fn __kernel_value__(&self) -> Value {
        Value(core::Value::Record(self.0.clone()))
    }
}

#[pyclass(module = "kernel._native", frozen, from_py_object)]
#[derive(Clone)]
struct Predicate(core::Predicate);

#[pymethods]
impl Predicate {
    #[staticmethod]
    #[pyo3(signature = (*, path))]
    fn present(path: Vec<String>) -> Self {
        Self(core::Predicate::Present { path })
    }
    #[staticmethod]
    #[pyo3(signature = (*, path, expected))]
    fn equals(path: Vec<String>, expected: Value) -> Self {
        Self(core::Predicate::Equals {
            path,
            expected: expected.0,
        })
    }
    #[staticmethod]
    #[pyo3(signature = (*, path, allowed))]
    fn within(path: Vec<String>, allowed: Vec<Value>) -> Self {
        Self(core::Predicate::Within {
            path,
            allowed: allowed.into_iter().map(|item| item.0).collect(),
        })
    }
    #[staticmethod]
    #[pyo3(signature = (*, path, item))]
    fn contains(path: Vec<String>, item: Value) -> Self {
        Self(core::Predicate::Contains { path, item: item.0 })
    }
    #[staticmethod]
    #[pyo3(signature = (*, path, min_words, max_words))]
    fn words(path: Vec<String>, min_words: i64, max_words: i64) -> Self {
        Self(core::Predicate::Words {
            path,
            min_words,
            max_words,
        })
    }
    #[staticmethod]
    #[pyo3(signature = (*, checks))]
    fn all(checks: Vec<(String, Self)>) -> PyResult<Self> {
        for (_, check) in &checks {
            check_predicate_depth(&check.0, 1)?;
        }
        Ok(Self(core::Predicate::All {
            checks: checks
                .into_iter()
                .map(|(name, check)| (name, check.0))
                .collect(),
        }))
    }
    #[staticmethod]
    #[pyo3(signature = (*, checks))]
    fn any(checks: Vec<(String, Self)>) -> PyResult<Self> {
        for (_, check) in &checks {
            check_predicate_depth(&check.0, 1)?;
        }
        Ok(Self(core::Predicate::Any {
            checks: checks
                .into_iter()
                .map(|(name, check)| (name, check.0))
                .collect(),
        }))
    }
    #[staticmethod]
    #[pyo3(signature = (*, check))]
    fn not_(check: Self) -> PyResult<Self> {
        check_predicate_depth(&check.0, 1)?;
        Ok(Self(core::Predicate::Not {
            check: Box::new(check.0),
        }))
    }
    #[pyo3(signature = (*, observation))]
    fn holds(&self, observation: PyRef<'_, Value>) -> bool {
        self.0.holds(&observation.0)
    }
    #[getter]
    fn kind(&self) -> &'static str {
        match self.0 {
            core::Predicate::Present { .. } => "present",
            core::Predicate::Equals { .. } => "equals",
            core::Predicate::Within { .. } => "within",
            core::Predicate::Contains { .. } => "contains",
            core::Predicate::Words { .. } => "words",
            core::Predicate::All { .. } => "all",
            core::Predicate::Any { .. } => "any",
            core::Predicate::Not { .. } => "not",
        }
    }
    #[getter]
    fn path(&self, py: Python<'_>) -> PyResult<Option<Py<PyTuple>>> {
        self.0
            .path()
            .map(|path| tuple(py, path.to_vec()))
            .transpose()
    }
    #[getter]
    fn expected(&self) -> Option<Value> {
        match &self.0 {
            core::Predicate::Equals { expected, .. } => Some(Value(expected.clone())),
            _ => None,
        }
    }
    #[getter]
    fn item(&self) -> Option<Value> {
        match &self.0 {
            core::Predicate::Contains { item, .. } => Some(Value(item.clone())),
            _ => None,
        }
    }
    #[getter]
    fn allowed(&self, py: Python<'_>) -> PyResult<Option<Py<PyTuple>>> {
        match &self.0 {
            core::Predicate::Within { allowed, .. } => Ok(Some(tuple(
                py,
                allowed.iter().cloned().map(Value).collect(),
            )?)),
            _ => Ok(None),
        }
    }
    #[getter]
    fn min_words(&self) -> Option<i64> {
        match self.0 {
            core::Predicate::Words { min_words, .. } => Some(min_words),
            _ => None,
        }
    }
    #[getter]
    fn max_words(&self) -> Option<i64> {
        match self.0 {
            core::Predicate::Words { max_words, .. } => Some(max_words),
            _ => None,
        }
    }
    #[getter]
    fn checks(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        match &self.0 {
            core::Predicate::All { checks } | core::Predicate::Any { checks } => tuple(
                py,
                checks
                    .iter()
                    .map(|(name, check)| (name.clone(), Self(check.clone())))
                    .collect(),
            ),
            _ => tuple::<Self>(py, vec![]),
        }
    }
    #[getter]
    fn check(&self) -> Option<Self> {
        match &self.0 {
            core::Predicate::Not { check } => Some(Self(*check.clone())),
            _ => None,
        }
    }
}

#[pyclass(module = "kernel._native", frozen, from_py_object)]
#[derive(Clone)]
struct Verification(core::Verification);

#[pymethods]
impl Verification {
    #[staticmethod]
    fn unchecked() -> Self {
        Self(core::Verification::Unchecked)
    }
    #[staticmethod]
    fn passed() -> Self {
        Self(core::Verification::Passed)
    }
    #[staticmethod]
    fn failed() -> Self {
        Self(core::Verification::Failed)
    }
    #[staticmethod]
    fn invalid_submission() -> Self {
        Self(core::Verification::InvalidSubmission)
    }
    #[staticmethod]
    fn timed_out() -> Self {
        Self(core::Verification::TimedOut)
    }
    #[staticmethod]
    #[pyo3(signature = (*, detail))]
    fn harness_error(detail: String) -> Self {
        Self(core::Verification::HarnessError { detail })
    }
    #[getter]
    fn status(&self) -> &'static str {
        self.0.status()
    }
    #[getter]
    fn detail(&self) -> Option<String> {
        match &self.0 {
            core::Verification::HarnessError { detail } => Some(detail.clone()),
            _ => None,
        }
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
        response: Value,
        facts: Vec<(String, Value)>,
    ) -> PyResult<Self> {
        check_value_depth(&response.0, 1)?;
        for (_, value) in &facts {
            check_value_depth(&value.0, 1)?;
        }
        Ok(Self(core::Outcome {
            changed_files,
            verification: verification.0,
            response: response.0,
            facts: facts
                .into_iter()
                .map(|(name, value)| (name, value.0))
                .collect(),
        }))
    }
    #[getter]
    fn changed_files(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        tuple(py, self.0.changed_files.clone())
    }
    #[getter]
    fn verification(&self) -> Verification {
        Verification(self.0.verification.clone())
    }
    #[getter]
    fn response(&self) -> Value {
        Value(self.0.response.clone())
    }
    #[getter]
    fn facts(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        tuple(
            py,
            self.0
                .facts
                .iter()
                .map(|(name, value)| (name.clone(), Value(value.clone())))
                .collect(),
        )
    }
    fn observation(&self) -> Value {
        Value(self.0.observation())
    }
}

#[pyclass(module = "kernel._native", frozen, from_py_object)]
#[derive(Clone)]
struct Assessment(core::Assessment);

#[pymethods]
impl Assessment {
    #[new]
    #[pyo3(signature = (*, predicate, observation))]
    fn new(predicate: Predicate, observation: Value) -> Self {
        Self(core::Assessment::new(predicate.0, observation.0))
    }
    fn passed(&self) -> bool {
        self.0.passed()
    }
    fn observed(&self) -> Option<Value> {
        self.0.observed().cloned().map(Value)
    }
    fn scores(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        tuple(py, self.0.scores())
    }
    #[getter]
    fn kind(&self) -> &'static str {
        match self.0 {
            core::Assessment::Check { .. } => "check",
            core::Assessment::All { .. } => "all",
        }
    }
    #[getter]
    fn predicate(&self) -> Option<Predicate> {
        match &self.0 {
            core::Assessment::Check { predicate, .. } => Some(Predicate(predicate.clone())),
            _ => None,
        }
    }
    #[getter]
    fn observation(&self) -> Option<Value> {
        match &self.0 {
            core::Assessment::Check { observation, .. } => Some(Value(observation.clone())),
            _ => None,
        }
    }
    #[getter]
    fn checks(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        match &self.0 {
            core::Assessment::All { checks } => tuple(
                py,
                checks
                    .iter()
                    .map(|(name, check)| (name.clone(), Self(check.clone())))
                    .collect(),
            ),
            _ => tuple::<Self>(py, vec![]),
        }
    }
}

#[pyclass(module = "kernel._native", frozen, from_py_object)]
#[derive(Clone)]
struct File(core::File);

#[pymethods]
impl File {
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
        files: Vec<File>,
        tools: Vec<File>,
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
        tuple(py, self.0.files.iter().cloned().map(File).collect())
    }
    #[getter]
    fn tools(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        tuple(py, self.0.tools.iter().cloned().map(File).collect())
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
    fn new(expected: Vec<(String, Value)>, text_rule: Option<TextRule>) -> Self {
        Self(core::Rubric {
            expected: expected
                .into_iter()
                .map(|(name, value)| (name, value.0))
                .collect(),
            text_rule: text_rule.map(|rule| rule.0),
        })
    }
    #[getter]
    fn expected(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        tuple(
            py,
            self.0
                .expected
                .iter()
                .map(|(name, value)| (name.clone(), Value(value.clone())))
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
struct Criterion(core::Criterion);

#[pymethods]
impl Criterion {
    #[staticmethod]
    #[pyo3(signature = (*, verifier))]
    fn code(verifier: Verifier) -> Self {
        Self(core::Criterion::Code {
            verifier: verifier.0,
        })
    }
    #[staticmethod]
    #[pyo3(signature = (*, rubric))]
    fn answer(rubric: Rubric) -> Self {
        Self(core::Criterion::Answer { rubric: rubric.0 })
    }
    #[staticmethod]
    #[pyo3(signature = (*, predicate))]
    fn state(predicate: Predicate) -> Self {
        Self(core::Criterion::State {
            predicate: predicate.0,
        })
    }
    fn condition(&self) -> Predicate {
        Predicate(self.0.condition())
    }
    #[getter]
    fn kind(&self) -> &'static str {
        match self.0 {
            core::Criterion::Code { .. } => "code",
            core::Criterion::Answer { .. } => "answer",
            core::Criterion::State { .. } => "state",
        }
    }
    #[getter]
    fn verifier(&self) -> Option<Verifier> {
        match &self.0 {
            core::Criterion::Code { verifier } => Some(Verifier(verifier.clone())),
            _ => None,
        }
    }
    #[getter]
    fn rubric(&self) -> Option<Rubric> {
        match &self.0 {
            core::Criterion::Answer { rubric } => Some(Rubric(rubric.clone())),
            _ => None,
        }
    }
    #[getter]
    fn predicate(&self) -> Option<Predicate> {
        match &self.0 {
            core::Criterion::State { predicate } => Some(Predicate(predicate.clone())),
            _ => None,
        }
    }
}

#[pyclass(module = "kernel._native", frozen, from_py_object)]
#[derive(Clone)]
struct Reference(core::Reference);

#[pymethods]
impl Reference {
    #[new]
    #[pyo3(signature = (*, edits, completion))]
    fn new(edits: Vec<File>, completion: Value) -> Self {
        Self(core::Reference {
            edits: edits.into_iter().map(|file| file.0).collect(),
            completion: completion.0,
        })
    }
    #[getter]
    fn edits(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        tuple(py, self.0.edits.iter().cloned().map(File).collect())
    }
    #[getter]
    fn completion(&self) -> Value {
        Value(self.0.completion.clone())
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
        tuple(py, self.0.files.iter().cloned().map(File).collect())
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
    let core::GradingFailure::VerificationHarnessError { detail } = failure;
    let result = (|| -> PyResult<()> {
        error
            .value(py)
            .setattr("failure", "verification_harness_error")?;
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
        fixture: Vec<File>,
        instruction: Instruction,
        criterion: Criterion,
        allowed_files: Vec<String>,
        reference: Reference,
        py: Python<'_>,
    ) -> PyResult<Self> {
        if let core::Criterion::State { predicate } = &criterion.0 {
            check_predicate_depth(predicate, 1)?;
        }
        core::Task::new(
            id,
            fixture.into_iter().map(|file| file.0).collect(),
            instruction.0,
            criterion.0,
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
        tuple(py, self.0.fixture().iter().cloned().map(File).collect())
    }
    #[getter]
    fn instruction(&self) -> Instruction {
        Instruction(self.0.instruction().clone())
    }
    #[getter]
    fn criterion(&self) -> Criterion {
        Criterion(self.0.criterion().clone())
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
            .map(Assessment)
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
    module.add_class::<File>()?;
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
