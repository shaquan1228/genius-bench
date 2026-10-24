use magnus::rb_sys::AsRawValue;
use magnus::scan_args::{ScanArgsRequired, get_kwargs, scan_args};
use magnus::typed_data::Obj;
use magnus::value::Lazy;
use magnus::{
    Error, ExceptionClass, Float, Integer, IntoValue, RArray, RHash, RModule, RObject, RString,
    Ruby, Symbol, TryConvert, TypedData, Value as RValue, function, method, prelude::*,
    r_hash::ForEach,
};
use semantic_core as core;
use std::collections::HashSet;
use std::fmt;
use std::hash::{DefaultHasher, Hash, Hasher};

trait Wrapped: TypedData {
    type Inner: PartialEq + Hash + fmt::Debug;
    fn inner(&self) -> &Self::Inner;
}

macro_rules! wrapped {
    ($type:ty, $inner:ty) => {
        impl Wrapped for $type {
            type Inner = $inner;
            fn inner(&self) -> &$inner {
                &self.0
            }
        }
    };
}

macro_rules! identity {
    ($class:expr, $type:ty) => {{
        $class.undef_default_alloc_func();
        $class.define_method("==", method!(equal::<$type>, 1))?;
        $class.define_method("eql?", method!(equal::<$type>, 1))?;
        $class.define_method("hash", method!(hash_code::<$type>, 0))?;
        $class.define_method("inspect", method!(inspect::<$type>, 0))?;
    }};
}

fn equal<T: Wrapped>(this: &T, other: RValue) -> bool {
    <&T>::try_convert(other).is_ok_and(|other| this.inner() == other.inner())
}

fn hash_code<T: Wrapped>(this: &T) -> i64 {
    let mut hasher = DefaultHasher::new();
    this.inner().hash(&mut hasher);
    hasher.finish() as i64
}

fn inspect<T: Wrapped>(ruby: &Ruby, this: &T) -> String {
    format!("#<{} {:?}>", T::class(ruby), this.inner())
}

static VALIDATION_ERROR: Lazy<ExceptionClass> =
    Lazy::new(|ruby| error_class(ruby, "ValidationError"));
static GRADING_ERROR: Lazy<ExceptionClass> = Lazy::new(|ruby| error_class(ruby, "GradingError"));

fn error_class(ruby: &Ruby, name: &str) -> ExceptionClass {
    ruby.class_object()
        .const_get::<_, RModule>("GeniusBench")
        .and_then(|module| module.const_get(name))
        .expect("GeniusBench error classes are defined at initialization")
}

fn argument_error(ruby: &Ruby, message: String) -> Error {
    Error::new(ruby.exception_arg_error(), message)
}

fn type_error(ruby: &Ruby, message: String) -> Error {
    Error::new(ruby.exception_type_error(), message)
}

fn unexpected(ruby: &Ruby, expected: &str, value: RValue) -> Error {
    type_error(
        ruby,
        format!("Expected {expected}, got {}", value.class().inspect()),
    )
}

fn check_depth(ruby: &Ruby, depth: usize, kind: &str) -> Result<(), Error> {
    if depth > core::NESTING_LIMIT {
        Err(argument_error(
            ruby,
            format!("{kind} nesting exceeds {} levels", core::NESTING_LIMIT),
        ))
    } else {
        Ok(())
    }
}

fn check_value_depth(ruby: &Ruby, value: &core::Value, depth: usize) -> Result<(), Error> {
    check_depth(ruby, depth + value.nesting_depth(), "Kernel value")
}

fn check_predicate_depth(
    ruby: &Ruby,
    predicate: &core::Predicate,
    depth: usize,
) -> Result<(), Error> {
    check_depth(ruby, depth + predicate.nesting_depth(), "Predicate")
}

fn required<T: ScanArgsRequired>(args: &[RValue], names: &[&str]) -> Result<T, Error> {
    let keywords: RHash = scan_args::<(), (), (), (), RHash, ()>(args)?.keywords;
    Ok(get_kwargs::<_, T, (), ()>(keywords, names, &[])?.required)
}

fn text(ruby: &Ruby, value: RValue) -> Result<String, Error> {
    RString::from_value(value)
        .ok_or_else(|| unexpected(ruby, "a String", value))?
        .to_string()
}

fn integer(ruby: &Ruby, value: RValue) -> Result<i64, Error> {
    Integer::from_value(value)
        .ok_or_else(|| unexpected(ruby, "an Integer", value))?
        .to_i64()
}

fn list(ruby: &Ruby, value: RValue) -> Result<RArray, Error> {
    Ok(RArray::from_value(value)
        .ok_or_else(|| unexpected(ruby, "an Array", value))?
        .dup())
}

fn texts(ruby: &Ruby, value: RValue) -> Result<Vec<String>, Error> {
    let items = list(ruby, value)?;
    (0..items.len())
        .map(|index| text(ruby, items.entry(index as isize)?))
        .collect()
}

fn frozen<T: TypedData>(ruby: &Ruby, data: T) -> Obj<T> {
    let object = ruby.obj_wrap(data);
    object.freeze();
    object
}

fn string(ruby: &Ruby, value: &str) -> RString {
    let string = ruby.str_new(value);
    string.freeze();
    string
}

fn array<T: IntoValue>(ruby: &Ruby, items: impl IntoIterator<Item = T>) -> Result<RArray, Error> {
    let result = ruby.ary_new();
    for item in items {
        result.push(item)?;
    }
    result.freeze();
    Ok(result)
}

fn strings(ruby: &Ruby, items: &[String]) -> Result<RArray, Error> {
    array(ruby, items.iter().map(|item| string(ruby, item)))
}

fn pairs<T: IntoValue>(
    ruby: &Ruby,
    items: impl IntoIterator<Item = (String, T)>,
) -> Result<RArray, Error> {
    let result = ruby.ary_new();
    for (name, item) in items {
        let entry = ruby.ary_new_capa(2);
        entry.push(string(ruby, &name))?;
        entry.push(item)?;
        entry.freeze();
        result.push(entry)?;
    }
    result.freeze();
    Ok(result)
}

fn objects<T: TypedData + Clone>(ruby: &Ruby, value: RValue) -> Result<Vec<T>, Error> {
    let items = list(ruby, value)?;
    (0..items.len())
        .map(|index| Ok(<&T>::try_convert(items.entry(index as isize)?)?.clone()))
        .collect()
}

fn named<T>(
    ruby: &Ruby,
    value: RValue,
    mut convert: impl FnMut(RValue) -> Result<T, Error>,
) -> Result<Vec<(String, T)>, Error> {
    let mut names = Vec::new();
    let items = ruby.ary_new();
    if let Some(mapping) = RHash::from_value(value) {
        let entries = ruby.ary_new_capa(mapping.len() * 2);
        mapping.foreach(|key: RValue, item: RValue| {
            entries.push(key)?;
            entries.push(item)?;
            Ok(ForEach::Continue)
        })?;
        for index in (0..entries.len()).step_by(2) {
            names.push(field_name(ruby, entries.entry(index as isize)?)?);
            items.push(entries.entry::<RValue>(index as isize + 1)?)?;
        }
    } else {
        let entries = RArray::from_value(value)
            .ok_or_else(|| unexpected(ruby, "an Array of name and value pairs or a Hash", value))?
            .dup();
        for index in 0..entries.len() {
            let entry = entries.entry::<RValue>(index as isize)?;
            let pair = RArray::from_value(entry)
                .filter(|pair| pair.len() == 2)
                .ok_or_else(|| unexpected(ruby, "a name and value pair", entry))?;
            names.push(field_name(ruby, pair.entry(0)?)?);
            items.push(pair.entry::<RValue>(1)?)?;
        }
    }
    names
        .into_iter()
        .enumerate()
        .map(|(index, name)| Ok((name, convert(items.entry(index as isize)?)?)))
        .collect()
}

fn field_name(ruby: &Ruby, key: RValue) -> Result<String, Error> {
    if let Some(symbol) = Symbol::from_value(key) {
        return Ok(symbol.name()?.into_owned());
    }
    if let Some(name) = RString::from_value(key) {
        return name.to_string();
    }
    Err(unexpected(
        ruby,
        "a String or Symbol record field name",
        key,
    ))
}

fn convert(
    ruby: &Ruby,
    value: RValue,
    active: &mut HashSet<usize>,
    depth: usize,
) -> Result<core::Value, Error> {
    if let Ok(native) = <&Value>::try_convert(value) {
        check_value_depth(ruby, &native.0, depth)?;
        return Ok(native.0.clone());
    }
    if let Ok(record) = <&Record>::try_convert(value) {
        check_depth(ruby, depth + 1, "Kernel value")?;
        for (_, item) in &record.0 {
            check_value_depth(ruby, item, depth + 1)?;
        }
        return Ok(core::Value::Record(record.0.clone()));
    }
    if value.is_nil() {
        return Ok(core::Value::Null);
    }
    if value.is_kind_of(ruby.class_true_class()) {
        return Ok(core::Value::Bool(true));
    }
    if value.is_kind_of(ruby.class_false_class()) {
        return Ok(core::Value::Bool(false));
    }
    if let Some(number) = Integer::from_value(value) {
        return Ok(core::Value::Int(number.to_i64()?));
    }
    if let Some(number) = Float::from_value(value) {
        return Ok(core::Value::Float(number.to_f64()));
    }
    if let Some(text) = RString::from_value(value) {
        return Ok(core::Value::String(text.to_string()?));
    }
    check_depth(ruby, depth + 1, "Kernel value")?;
    let address = value.as_raw() as usize;
    if !active.insert(address) {
        return Err(argument_error(ruby, "Cyclic kernel value".into()));
    }
    let result = (|| {
        if RHash::from_value(value).is_some() {
            let fields = named(ruby, value, |item| convert(ruby, item, active, depth + 1))?;
            let mut names = HashSet::new();
            for (name, _) in &fields {
                if !names.insert(name) {
                    return Err(argument_error(
                        ruby,
                        format!("Duplicate record field: {name}"),
                    ));
                }
            }
            Ok(core::Value::Record(fields))
        } else if let Some(items) = RArray::from_value(value) {
            let items = items.dup();
            (0..items.len())
                .map(|index| convert(ruby, items.entry(index as isize)?, active, depth + 1))
                .collect::<Result<Vec<_>, Error>>()
                .map(core::Value::List)
        } else if value.respond_to("to_kernel_value", false)? {
            let inner: RValue = value.funcall("to_kernel_value", ())?;
            convert(ruby, inner, active, depth + 1)
        } else {
            Err(unexpected(ruby, "a kernel value", value))
        }
    })();
    active.remove(&address);
    result
}

#[magnus::wrap(class = "GeniusBench::Value", free_immediately, size)]
#[derive(Clone)]
struct Value(core::Value);
wrapped!(Value, core::Value);

fn value_from_ruby(ruby: &Ruby, args: &[RValue]) -> Result<Obj<Value>, Error> {
    let (value,): (RValue,) = required(args, &["value"])?;
    Ok(frozen(
        ruby,
        Value(convert(ruby, value, &mut HashSet::new(), 0)?),
    ))
}

fn value_kind(ruby: &Ruby, this: &Value) -> RString {
    string(
        ruby,
        match this.0 {
            core::Value::Null => "null",
            core::Value::Bool(_) => "bool",
            core::Value::Int(_) => "int",
            core::Value::Float(_) => "float",
            core::Value::String(_) => "string",
            core::Value::List(_) => "list",
            core::Value::Record(_) => "record",
        },
    )
}

fn value_scalar(ruby: &Ruby, this: &Value) -> Result<RValue, Error> {
    Ok(match &this.0 {
        core::Value::Null => ruby.qnil().as_value(),
        core::Value::Bool(value) => value.into_value_with(ruby),
        core::Value::Int(value) => ruby.integer_from_i64(*value).as_value(),
        core::Value::Float(value) => ruby.float_from_f64(*value).as_value(),
        core::Value::String(value) => string(ruby, value).as_value(),
        _ => {
            return Err(type_error(ruby, "Structured values have no scalar".into()));
        }
    })
}

fn values(ruby: &Ruby, items: &[core::Value]) -> Result<RArray, Error> {
    array(
        ruby,
        items.iter().map(|item| frozen(ruby, Value(item.clone()))),
    )
}

fn named_values(ruby: &Ruby, items: &[(String, core::Value)]) -> Result<RArray, Error> {
    pairs(
        ruby,
        items
            .iter()
            .map(|(name, item)| (name.clone(), frozen(ruby, Value(item.clone())))),
    )
}

fn value_items(ruby: &Ruby, this: &Value) -> Result<RArray, Error> {
    let core::Value::List(items) = &this.0 else {
        return Err(type_error(ruby, "Value is not a list".into()));
    };
    values(ruby, items)
}

fn value_fields(ruby: &Ruby, this: &Value) -> Result<RArray, Error> {
    let core::Value::Record(fields) = &this.0 else {
        return Err(type_error(ruby, "Value is not a record".into()));
    };
    named_values(ruby, fields)
}

fn value_get(ruby: &Ruby, this: &Value, args: &[RValue]) -> Result<Option<Obj<Value>>, Error> {
    let (path,): (RValue,) = required(args, &["path"])?;
    Ok(this
        .0
        .get(&texts(ruby, path)?)
        .map(|value| frozen(ruby, Value(value.clone()))))
}

fn value_equal_to(this: &Value, args: &[RValue]) -> Result<bool, Error> {
    let (other,): (Obj<Value>,) = required(args, &["other"])?;
    Ok(this.0.equal(&other.0))
}

#[magnus::wrap(class = "GeniusBench::Record", free_immediately, size)]
#[derive(Clone)]
struct Record(Vec<(String, core::Value)>);
wrapped!(Record, Vec<(String, core::Value)>);

fn record_new(ruby: &Ruby, args: &[RValue]) -> Result<Obj<Record>, Error> {
    let (fields,): (RValue,) = required(args, &["fields"])?;
    let mut active = HashSet::new();
    let fields = named(ruby, fields, |item| convert(ruby, item, &mut active, 1))?;
    Ok(frozen(ruby, Record(fields)))
}

fn record_fields(ruby: &Ruby, this: &Record) -> Result<RArray, Error> {
    named_values(ruby, &this.0)
}

fn record_to_kernel_value(ruby: &Ruby, this: &Record) -> Obj<Value> {
    frozen(ruby, Value(core::Value::Record(this.0.clone())))
}

#[magnus::wrap(class = "GeniusBench::Predicate", free_immediately, size)]
#[derive(Clone)]
struct Predicate(core::Predicate);
wrapped!(Predicate, core::Predicate);

fn predicate_present(ruby: &Ruby, args: &[RValue]) -> Result<Obj<Predicate>, Error> {
    let (path,): (RValue,) = required(args, &["path"])?;
    Ok(frozen(
        ruby,
        Predicate(core::Predicate::Present {
            path: texts(ruby, path)?,
        }),
    ))
}

fn predicate_equals(ruby: &Ruby, args: &[RValue]) -> Result<Obj<Predicate>, Error> {
    let (path, expected): (RValue, Obj<Value>) = required(args, &["path", "expected"])?;
    Ok(frozen(
        ruby,
        Predicate(core::Predicate::Equals {
            path: texts(ruby, path)?,
            expected: expected.0.clone(),
        }),
    ))
}

fn predicate_within(ruby: &Ruby, args: &[RValue]) -> Result<Obj<Predicate>, Error> {
    let (path, allowed): (RValue, RValue) = required(args, &["path", "allowed"])?;
    Ok(frozen(
        ruby,
        Predicate(core::Predicate::Within {
            path: texts(ruby, path)?,
            allowed: objects::<Value>(ruby, allowed)?
                .into_iter()
                .map(|item| item.0)
                .collect(),
        }),
    ))
}

fn predicate_contains(ruby: &Ruby, args: &[RValue]) -> Result<Obj<Predicate>, Error> {
    let (path, item): (RValue, Obj<Value>) = required(args, &["path", "item"])?;
    Ok(frozen(
        ruby,
        Predicate(core::Predicate::Contains {
            path: texts(ruby, path)?,
            item: item.0.clone(),
        }),
    ))
}

fn predicate_words(ruby: &Ruby, args: &[RValue]) -> Result<Obj<Predicate>, Error> {
    let (path, min_words, max_words): (RValue, RValue, RValue) =
        required(args, &["path", "min_words", "max_words"])?;
    Ok(frozen(
        ruby,
        Predicate(core::Predicate::Words {
            path: texts(ruby, path)?,
            min_words: integer(ruby, min_words)?,
            max_words: integer(ruby, max_words)?,
        }),
    ))
}

fn predicate_checks(ruby: &Ruby, args: &[RValue]) -> Result<Vec<(String, core::Predicate)>, Error> {
    let (checks,): (RValue,) = required(args, &["checks"])?;
    let checks = named(ruby, checks, |item| {
        Ok(<&Predicate>::try_convert(item)?.0.clone())
    })?;
    for (_, check) in &checks {
        check_predicate_depth(ruby, check, 1)?;
    }
    Ok(checks)
}

fn predicate_all(ruby: &Ruby, args: &[RValue]) -> Result<Obj<Predicate>, Error> {
    let checks = predicate_checks(ruby, args)?;
    Ok(frozen(ruby, Predicate(core::Predicate::All { checks })))
}

fn predicate_any(ruby: &Ruby, args: &[RValue]) -> Result<Obj<Predicate>, Error> {
    let checks = predicate_checks(ruby, args)?;
    Ok(frozen(ruby, Predicate(core::Predicate::Any { checks })))
}

fn predicate_not(ruby: &Ruby, args: &[RValue]) -> Result<Obj<Predicate>, Error> {
    let (check,): (Obj<Predicate>,) = required(args, &["check"])?;
    check_predicate_depth(ruby, &check.0, 1)?;
    Ok(frozen(
        ruby,
        Predicate(core::Predicate::Not {
            check: Box::new(check.0.clone()),
        }),
    ))
}

fn predicate_holds(this: &Predicate, args: &[RValue]) -> Result<bool, Error> {
    let (observation,): (Obj<Value>,) = required(args, &["observation"])?;
    Ok(this.0.holds(&observation.0))
}

fn predicate_kind(ruby: &Ruby, this: &Predicate) -> RString {
    string(
        ruby,
        match this.0 {
            core::Predicate::Present { .. } => "present",
            core::Predicate::Equals { .. } => "equals",
            core::Predicate::Within { .. } => "within",
            core::Predicate::Contains { .. } => "contains",
            core::Predicate::Words { .. } => "words",
            core::Predicate::All { .. } => "all",
            core::Predicate::Any { .. } => "any",
            core::Predicate::Not { .. } => "not",
        },
    )
}

fn predicate_path(ruby: &Ruby, this: &Predicate) -> Result<Option<RArray>, Error> {
    this.0.path().map(|path| strings(ruby, path)).transpose()
}

fn predicate_expected(ruby: &Ruby, this: &Predicate) -> Option<Obj<Value>> {
    match &this.0 {
        core::Predicate::Equals { expected, .. } => Some(frozen(ruby, Value(expected.clone()))),
        _ => None,
    }
}

fn predicate_item(ruby: &Ruby, this: &Predicate) -> Option<Obj<Value>> {
    match &this.0 {
        core::Predicate::Contains { item, .. } => Some(frozen(ruby, Value(item.clone()))),
        _ => None,
    }
}

fn predicate_allowed(ruby: &Ruby, this: &Predicate) -> Result<Option<RArray>, Error> {
    match &this.0 {
        core::Predicate::Within { allowed, .. } => values(ruby, allowed).map(Some),
        _ => Ok(None),
    }
}

fn predicate_min_words(this: &Predicate) -> Option<i64> {
    match this.0 {
        core::Predicate::Words { min_words, .. } => Some(min_words),
        _ => None,
    }
}

fn predicate_max_words(this: &Predicate) -> Option<i64> {
    match this.0 {
        core::Predicate::Words { max_words, .. } => Some(max_words),
        _ => None,
    }
}

fn predicates(ruby: &Ruby, checks: &[(String, core::Predicate)]) -> Result<RArray, Error> {
    pairs(
        ruby,
        checks
            .iter()
            .map(|(name, check)| (name.clone(), frozen(ruby, Predicate(check.clone())))),
    )
}

fn predicate_checks_of(ruby: &Ruby, this: &Predicate) -> Result<RArray, Error> {
    match &this.0 {
        core::Predicate::All { checks } | core::Predicate::Any { checks } => {
            predicates(ruby, checks)
        }
        _ => array::<RValue>(ruby, []),
    }
}

fn predicate_check(ruby: &Ruby, this: &Predicate) -> Option<Obj<Predicate>> {
    match &this.0 {
        core::Predicate::Not { check } => Some(frozen(ruby, Predicate(*check.clone()))),
        _ => None,
    }
}

#[magnus::wrap(class = "GeniusBench::Verification", free_immediately, size)]
#[derive(Clone)]
struct Verification(core::Verification);
wrapped!(Verification, core::Verification);

fn verification_unchecked(ruby: &Ruby) -> Obj<Verification> {
    frozen(ruby, Verification(core::Verification::Unchecked))
}

fn verification_passed(ruby: &Ruby) -> Obj<Verification> {
    frozen(ruby, Verification(core::Verification::Passed))
}

fn verification_failed(ruby: &Ruby) -> Obj<Verification> {
    frozen(ruby, Verification(core::Verification::Failed))
}

fn verification_invalid_submission(ruby: &Ruby) -> Obj<Verification> {
    frozen(ruby, Verification(core::Verification::InvalidSubmission))
}

fn verification_timed_out(ruby: &Ruby) -> Obj<Verification> {
    frozen(ruby, Verification(core::Verification::TimedOut))
}

fn verification_harness_error(ruby: &Ruby, args: &[RValue]) -> Result<Obj<Verification>, Error> {
    let (detail,): (RValue,) = required(args, &["detail"])?;
    Ok(frozen(
        ruby,
        Verification(core::Verification::HarnessError {
            detail: text(ruby, detail)?,
        }),
    ))
}

fn verification_status(ruby: &Ruby, this: &Verification) -> RString {
    string(ruby, this.0.status())
}

fn verification_detail(ruby: &Ruby, this: &Verification) -> Option<RString> {
    match &this.0 {
        core::Verification::HarnessError { detail } => Some(string(ruby, detail)),
        _ => None,
    }
}

#[magnus::wrap(class = "GeniusBench::Outcome", free_immediately, size)]
#[derive(Clone)]
struct Outcome(core::Outcome);
wrapped!(Outcome, core::Outcome);

fn outcome_new(ruby: &Ruby, args: &[RValue]) -> Result<Obj<Outcome>, Error> {
    let (changed_files, verification, response, facts): (
        RValue,
        Obj<Verification>,
        Obj<Value>,
        RValue,
    ) = required(
        args,
        &["changed_files", "verification", "response", "facts"],
    )?;
    check_value_depth(ruby, &response.0, 1)?;
    let facts = named(ruby, facts, |item| {
        let value = <&Value>::try_convert(item)?;
        check_value_depth(ruby, &value.0, 1)?;
        Ok(value.0.clone())
    })?;
    Ok(frozen(
        ruby,
        Outcome(core::Outcome {
            changed_files: texts(ruby, changed_files)?,
            verification: verification.0.clone(),
            response: response.0.clone(),
            facts,
        }),
    ))
}

fn outcome_changed_files(ruby: &Ruby, this: &Outcome) -> Result<RArray, Error> {
    strings(ruby, &this.0.changed_files)
}

fn outcome_verification(ruby: &Ruby, this: &Outcome) -> Obj<Verification> {
    frozen(ruby, Verification(this.0.verification.clone()))
}

fn outcome_response(ruby: &Ruby, this: &Outcome) -> Obj<Value> {
    frozen(ruby, Value(this.0.response.clone()))
}

fn outcome_facts(ruby: &Ruby, this: &Outcome) -> Result<RArray, Error> {
    named_values(ruby, &this.0.facts)
}

fn outcome_observation(ruby: &Ruby, this: &Outcome) -> Obj<Value> {
    frozen(ruby, Value(this.0.observation()))
}

#[magnus::wrap(class = "GeniusBench::Assessment", free_immediately, size)]
#[derive(Clone)]
struct Assessment(core::Assessment);
wrapped!(Assessment, core::Assessment);

fn assessment_new(ruby: &Ruby, args: &[RValue]) -> Result<Obj<Assessment>, Error> {
    let (predicate, observation): (Obj<Predicate>, Obj<Value>) =
        required(args, &["predicate", "observation"])?;
    Ok(frozen(
        ruby,
        Assessment(core::Assessment::new(
            predicate.0.clone(),
            observation.0.clone(),
        )),
    ))
}

fn assessment_passed(this: &Assessment) -> bool {
    this.0.passed()
}

fn assessment_observed(ruby: &Ruby, this: &Assessment) -> Option<Obj<Value>> {
    this.0
        .observed()
        .map(|value| frozen(ruby, Value(value.clone())))
}

fn assessment_scores(ruby: &Ruby, this: &Assessment) -> Result<RArray, Error> {
    pairs(ruby, this.0.scores())
}

fn assessment_kind(ruby: &Ruby, this: &Assessment) -> RString {
    string(
        ruby,
        match this.0 {
            core::Assessment::Check { .. } => "check",
            core::Assessment::All { .. } => "all",
        },
    )
}

fn assessment_predicate(ruby: &Ruby, this: &Assessment) -> Option<Obj<Predicate>> {
    match &this.0 {
        core::Assessment::Check { predicate, .. } => {
            Some(frozen(ruby, Predicate(predicate.clone())))
        }
        _ => None,
    }
}

fn assessment_observation(ruby: &Ruby, this: &Assessment) -> Option<Obj<Value>> {
    match &this.0 {
        core::Assessment::Check { observation, .. } => {
            Some(frozen(ruby, Value(observation.clone())))
        }
        _ => None,
    }
}

fn assessment_checks(ruby: &Ruby, this: &Assessment) -> Result<RArray, Error> {
    match &this.0 {
        core::Assessment::All { checks } => pairs(
            ruby,
            checks
                .iter()
                .map(|(name, check)| (name.clone(), frozen(ruby, Assessment(check.clone())))),
        ),
        _ => array::<RValue>(ruby, []),
    }
}

#[magnus::wrap(class = "GeniusBench::SourceFile", free_immediately, size)]
#[derive(Clone)]
struct SourceFile(core::File);
wrapped!(SourceFile, core::File);

fn file_new(ruby: &Ruby, args: &[RValue]) -> Result<Obj<SourceFile>, Error> {
    let (path, contents): (RValue, RValue) = required(args, &["path", "contents"])?;
    Ok(frozen(
        ruby,
        SourceFile(core::File {
            path: text(ruby, path)?,
            contents: text(ruby, contents)?,
        }),
    ))
}

fn file_path(ruby: &Ruby, this: &SourceFile) -> RString {
    string(ruby, &this.0.path)
}

fn file_contents(ruby: &Ruby, this: &SourceFile) -> RString {
    string(ruby, &this.0.contents)
}

fn files(ruby: &Ruby, items: &[core::File]) -> Result<RArray, Error> {
    array(
        ruby,
        items
            .iter()
            .map(|file| frozen(ruby, SourceFile(file.clone()))),
    )
}

#[magnus::wrap(class = "GeniusBench::Instruction", free_immediately, size)]
#[derive(Clone)]
struct Instruction(core::Instruction);
wrapped!(Instruction, core::Instruction);

fn instruction_new(ruby: &Ruby, args: &[RValue]) -> Result<Obj<Instruction>, Error> {
    let (prompt,): (RValue,) = required(args, &["prompt"])?;
    Ok(frozen(
        ruby,
        Instruction(core::Instruction {
            prompt: text(ruby, prompt)?,
        }),
    ))
}

fn instruction_prompt(ruby: &Ruby, this: &Instruction) -> RString {
    string(ruby, &this.0.prompt)
}

#[magnus::wrap(class = "GeniusBench::Verifier", free_immediately, size)]
#[derive(Clone)]
struct Verifier(core::Verifier);
wrapped!(Verifier, core::Verifier);

fn verifier_new(ruby: &Ruby, args: &[RValue]) -> Result<Obj<Verifier>, Error> {
    let (inputs, files, tools, command, stdin, timeout_seconds): (
        RValue,
        RValue,
        RValue,
        RValue,
        RValue,
        RValue,
    ) = required(
        args,
        &[
            "inputs",
            "files",
            "tools",
            "command",
            "stdin",
            "timeout_seconds",
        ],
    )?;
    Ok(frozen(
        ruby,
        Verifier(core::Verifier {
            inputs: texts(ruby, inputs)?,
            files: objects::<SourceFile>(ruby, files)?
                .into_iter()
                .map(|file| file.0)
                .collect(),
            tools: objects::<SourceFile>(ruby, tools)?
                .into_iter()
                .map(|file| file.0)
                .collect(),
            command: texts(ruby, command)?,
            stdin: text(ruby, stdin)?,
            timeout_seconds: integer(ruby, timeout_seconds)?,
        }),
    ))
}

fn verifier_inputs(ruby: &Ruby, this: &Verifier) -> Result<RArray, Error> {
    strings(ruby, &this.0.inputs)
}

fn verifier_files(ruby: &Ruby, this: &Verifier) -> Result<RArray, Error> {
    files(ruby, &this.0.files)
}

fn verifier_tools(ruby: &Ruby, this: &Verifier) -> Result<RArray, Error> {
    files(ruby, &this.0.tools)
}

fn verifier_command(ruby: &Ruby, this: &Verifier) -> Result<RArray, Error> {
    strings(ruby, &this.0.command)
}

fn verifier_stdin(ruby: &Ruby, this: &Verifier) -> RString {
    string(ruby, &this.0.stdin)
}

fn verifier_timeout_seconds(this: &Verifier) -> i64 {
    this.0.timeout_seconds
}

#[magnus::wrap(class = "GeniusBench::TextRule", free_immediately, size)]
#[derive(Clone)]
struct TextRule(core::TextRule);
wrapped!(TextRule, core::TextRule);

fn text_rule_new(ruby: &Ruby, args: &[RValue]) -> Result<Obj<TextRule>, Error> {
    let (field, min_words, max_words): (RValue, RValue, RValue) =
        required(args, &["field", "min_words", "max_words"])?;
    Ok(frozen(
        ruby,
        TextRule(core::TextRule {
            field: text(ruby, field)?,
            min_words: integer(ruby, min_words)?,
            max_words: integer(ruby, max_words)?,
        }),
    ))
}

fn text_rule_field(ruby: &Ruby, this: &TextRule) -> RString {
    string(ruby, &this.0.field)
}

fn text_rule_min_words(this: &TextRule) -> i64 {
    this.0.min_words
}

fn text_rule_max_words(this: &TextRule) -> i64 {
    this.0.max_words
}

#[magnus::wrap(class = "GeniusBench::Rubric", free_immediately, size)]
#[derive(Clone)]
struct Rubric(core::Rubric);
wrapped!(Rubric, core::Rubric);

fn rubric_new(ruby: &Ruby, args: &[RValue]) -> Result<Obj<Rubric>, Error> {
    let (expected, text_rule): (RValue, Option<Obj<TextRule>>) =
        required(args, &["expected", "text_rule"])?;
    let expected = named(ruby, expected, |item| {
        Ok(<&Value>::try_convert(item)?.0.clone())
    })?;
    Ok(frozen(
        ruby,
        Rubric(core::Rubric {
            expected,
            text_rule: text_rule.map(|rule| rule.0.clone()),
        }),
    ))
}

fn rubric_expected(ruby: &Ruby, this: &Rubric) -> Result<RArray, Error> {
    named_values(ruby, &this.0.expected)
}

fn rubric_text_rule(ruby: &Ruby, this: &Rubric) -> Option<Obj<TextRule>> {
    this.0
        .text_rule
        .clone()
        .map(|rule| frozen(ruby, TextRule(rule)))
}

#[magnus::wrap(class = "GeniusBench::Criterion", free_immediately, size)]
#[derive(Clone)]
struct Criterion(core::Criterion);
wrapped!(Criterion, core::Criterion);

fn criterion_code(ruby: &Ruby, args: &[RValue]) -> Result<Obj<Criterion>, Error> {
    let (verifier,): (Obj<Verifier>,) = required(args, &["verifier"])?;
    Ok(frozen(
        ruby,
        Criterion(core::Criterion::Code {
            verifier: verifier.0.clone(),
        }),
    ))
}

fn criterion_answer(ruby: &Ruby, args: &[RValue]) -> Result<Obj<Criterion>, Error> {
    let (rubric,): (Obj<Rubric>,) = required(args, &["rubric"])?;
    Ok(frozen(
        ruby,
        Criterion(core::Criterion::Answer {
            rubric: rubric.0.clone(),
        }),
    ))
}

fn criterion_state(ruby: &Ruby, args: &[RValue]) -> Result<Obj<Criterion>, Error> {
    let (predicate,): (Obj<Predicate>,) = required(args, &["predicate"])?;
    Ok(frozen(
        ruby,
        Criterion(core::Criterion::State {
            predicate: predicate.0.clone(),
        }),
    ))
}

fn criterion_condition(ruby: &Ruby, this: &Criterion) -> Obj<Predicate> {
    frozen(ruby, Predicate(this.0.condition()))
}

fn criterion_kind(ruby: &Ruby, this: &Criterion) -> RString {
    string(
        ruby,
        match this.0 {
            core::Criterion::Code { .. } => "code",
            core::Criterion::Answer { .. } => "answer",
            core::Criterion::State { .. } => "state",
        },
    )
}

fn criterion_verifier(ruby: &Ruby, this: &Criterion) -> Option<Obj<Verifier>> {
    match &this.0 {
        core::Criterion::Code { verifier } => Some(frozen(ruby, Verifier(verifier.clone()))),
        _ => None,
    }
}

fn criterion_rubric(ruby: &Ruby, this: &Criterion) -> Option<Obj<Rubric>> {
    match &this.0 {
        core::Criterion::Answer { rubric } => Some(frozen(ruby, Rubric(rubric.clone()))),
        _ => None,
    }
}

fn criterion_predicate(ruby: &Ruby, this: &Criterion) -> Option<Obj<Predicate>> {
    match &this.0 {
        core::Criterion::State { predicate } => Some(frozen(ruby, Predicate(predicate.clone()))),
        _ => None,
    }
}

#[magnus::wrap(class = "GeniusBench::Reference", free_immediately, size)]
#[derive(Clone)]
struct Reference(core::Reference);
wrapped!(Reference, core::Reference);

fn reference_new(ruby: &Ruby, args: &[RValue]) -> Result<Obj<Reference>, Error> {
    let (edits, completion): (RValue, Obj<Value>) = required(args, &["edits", "completion"])?;
    Ok(frozen(
        ruby,
        Reference(core::Reference {
            edits: objects::<SourceFile>(ruby, edits)?
                .into_iter()
                .map(|file| file.0)
                .collect(),
            completion: completion.0.clone(),
        }),
    ))
}

fn reference_edits(ruby: &Ruby, this: &Reference) -> Result<RArray, Error> {
    files(ruby, &this.0.edits)
}

fn reference_completion(ruby: &Ruby, this: &Reference) -> Obj<Value> {
    frozen(ruby, Value(this.0.completion.clone()))
}

#[magnus::wrap(class = "GeniusBench::Case", free_immediately, size)]
#[derive(Clone)]
struct Case(core::Case);
wrapped!(Case, core::Case);

fn case_case_id(ruby: &Ruby, this: &Case) -> RString {
    string(ruby, &this.0.case_id)
}

fn case_prompt(ruby: &Ruby, this: &Case) -> RString {
    string(ruby, &this.0.prompt)
}

fn case_files(ruby: &Ruby, this: &Case) -> Result<RArray, Error> {
    files(ruby, &this.0.files)
}

#[magnus::wrap(class = "GeniusBench::Issue", free_immediately, size)]
#[derive(Clone)]
struct Issue(core::Issue);
wrapped!(Issue, core::Issue);

fn issue_field(ruby: &Ruby, this: &Issue) -> RString {
    string(ruby, &this.0.field)
}

fn issue_message(ruby: &Ruby, this: &Issue) -> RString {
    string(ruby, &this.0.message)
}

fn validation_error(ruby: &Ruby, failure: core::ValidationFailure) -> Error {
    let class = ruby.get_inner(&VALIDATION_ERROR);
    let message = failure.to_string();
    let build = || -> Result<Error, Error> {
        let exception = class.new_instance((message.as_str(),))?;
        let object = RObject::try_convert(exception.as_value())?;
        object.ivar_set(
            "@case_id",
            failure.case_id.as_deref().map(|id| string(ruby, id)),
        )?;
        object.ivar_set(
            "@issues",
            array(
                ruby,
                failure
                    .issues
                    .iter()
                    .map(|issue| frozen(ruby, Issue(issue.clone()))),
            )?,
        )?;
        Ok(exception.into())
    };
    match build() {
        Ok(error) | Err(error) => error,
    }
}

fn grading_error(ruby: &Ruby, failure: core::GradingFailure) -> Error {
    let class = ruby.get_inner(&GRADING_ERROR);
    let message = failure.to_string();
    let core::GradingFailure::VerificationHarnessError { detail } = &failure;
    let build = || -> Result<Error, Error> {
        let exception = class.new_instance((message.as_str(),))?;
        let object = RObject::try_convert(exception.as_value())?;
        object.ivar_set("@failure", string(ruby, failure.kind()))?;
        object.ivar_set("@detail", string(ruby, detail))?;
        Ok(exception.into())
    };
    match build() {
        Ok(error) | Err(error) => error,
    }
}

fn error_case_id(this: RObject) -> Result<RValue, Error> {
    this.ivar_get("@case_id")
}

fn error_issues(this: RObject) -> Result<RValue, Error> {
    this.ivar_get("@issues")
}

fn error_failure(this: RObject) -> Result<RValue, Error> {
    this.ivar_get("@failure")
}

fn error_detail(this: RObject) -> Result<RValue, Error> {
    this.ivar_get("@detail")
}

#[magnus::wrap(class = "GeniusBench::Task", free_immediately, size)]
#[derive(Clone)]
struct Task(core::Task);
wrapped!(Task, core::Task);

fn task_new(ruby: &Ruby, args: &[RValue]) -> Result<Obj<Task>, Error> {
    let (id, fixture, instruction, criterion, allowed_files, reference): (
        RValue,
        RValue,
        Obj<Instruction>,
        Obj<Criterion>,
        RValue,
        Obj<Reference>,
    ) = required(
        args,
        &[
            "id",
            "fixture",
            "instruction",
            "criterion",
            "allowed_files",
            "reference",
        ],
    )?;
    if let core::Criterion::State { predicate } = &criterion.0 {
        check_predicate_depth(ruby, predicate, 1)?;
    }
    core::Task::new(
        text(ruby, id)?,
        objects::<SourceFile>(ruby, fixture)?
            .into_iter()
            .map(|file| file.0)
            .collect(),
        instruction.0.clone(),
        criterion.0.clone(),
        texts(ruby, allowed_files)?,
        reference.0.clone(),
    )
    .map(|task| frozen(ruby, Task(task)))
    .map_err(|failure| validation_error(ruby, failure))
}

fn task_id(ruby: &Ruby, this: &Task) -> RString {
    string(ruby, this.0.id())
}

fn task_fixture(ruby: &Ruby, this: &Task) -> Result<RArray, Error> {
    files(ruby, this.0.fixture())
}

fn task_instruction(ruby: &Ruby, this: &Task) -> Obj<Instruction> {
    frozen(ruby, Instruction(this.0.instruction().clone()))
}

fn task_criterion(ruby: &Ruby, this: &Task) -> Obj<Criterion> {
    frozen(ruby, Criterion(this.0.criterion().clone()))
}

fn task_allowed_files(ruby: &Ruby, this: &Task) -> Result<RArray, Error> {
    strings(ruby, this.0.allowed_files())
}

fn task_reference(ruby: &Ruby, this: &Task) -> Obj<Reference> {
    frozen(ruby, Reference(this.0.reference().clone()))
}

fn task_project(ruby: &Ruby, this: &Task) -> Obj<Case> {
    frozen(ruby, Case(this.0.project()))
}

fn task_checks(ruby: &Ruby, this: &Task) -> Option<Obj<Verifier>> {
    this.0
        .checks()
        .map(|verifier| frozen(ruby, Verifier(verifier.clone())))
}

fn outcome_argument(args: &[RValue]) -> Result<Obj<Outcome>, Error> {
    let (outcome,): (Obj<Outcome>,) = required(args, &["outcome"])?;
    Ok(outcome)
}

fn task_assess(ruby: &Ruby, this: &Task, args: &[RValue]) -> Result<Obj<Assessment>, Error> {
    let outcome = outcome_argument(args)?;
    this.0
        .assess(&outcome.0)
        .map(|assessment| frozen(ruby, Assessment(assessment)))
        .map_err(|failure| grading_error(ruby, failure))
}

fn task_grade(ruby: &Ruby, this: &Task, args: &[RValue]) -> Result<RArray, Error> {
    let outcome = outcome_argument(args)?;
    pairs(
        ruby,
        this.0
            .grade(&outcome.0)
            .map_err(|failure| grading_error(ruby, failure))?,
    )
}

fn version(ruby: &Ruby) -> RString {
    string(ruby, core::version())
}

fn source_hash(ruby: &Ruby) -> RString {
    string(ruby, core::source_hash())
}

#[magnus::init]
fn init(ruby: &Ruby) -> Result<(), Error> {
    let module = ruby.define_module("GeniusBench")?;
    let object = ruby.class_object();

    let validation_error = module.define_error("ValidationError", ruby.exception_arg_error())?;
    validation_error.define_method("case_id", method!(error_case_id, 0))?;
    validation_error.define_method("issues", method!(error_issues, 0))?;
    let grading_error = module.define_error("GradingError", ruby.exception_runtime_error())?;
    grading_error.define_method("failure", method!(error_failure, 0))?;
    grading_error.define_method("detail", method!(error_detail, 0))?;
    Lazy::force(&VALIDATION_ERROR, ruby);
    Lazy::force(&GRADING_ERROR, ruby);

    let value = module.define_class("Value", object)?;
    identity!(value, Value);
    value.define_singleton_method("from_ruby", function!(value_from_ruby, -1))?;
    value.define_method("kind", method!(value_kind, 0))?;
    value.define_method("scalar", method!(value_scalar, 0))?;
    value.define_method("items", method!(value_items, 0))?;
    value.define_method("fields", method!(value_fields, 0))?;
    value.define_method("get", method!(value_get, -1))?;
    value.define_method("equal_to?", method!(value_equal_to, -1))?;

    let record = module.define_class("Record", object)?;
    identity!(record, Record);
    record.define_singleton_method("new", function!(record_new, -1))?;
    record.define_method("fields", method!(record_fields, 0))?;
    record.define_method("to_kernel_value", method!(record_to_kernel_value, 0))?;

    let predicate = module.define_class("Predicate", object)?;
    identity!(predicate, Predicate);
    predicate.define_singleton_method("present", function!(predicate_present, -1))?;
    predicate.define_singleton_method("equals", function!(predicate_equals, -1))?;
    predicate.define_singleton_method("within", function!(predicate_within, -1))?;
    predicate.define_singleton_method("contains", function!(predicate_contains, -1))?;
    predicate.define_singleton_method("words", function!(predicate_words, -1))?;
    predicate.define_singleton_method("all", function!(predicate_all, -1))?;
    predicate.define_singleton_method("any", function!(predicate_any, -1))?;
    predicate.define_singleton_method("not", function!(predicate_not, -1))?;
    predicate.define_method("holds?", method!(predicate_holds, -1))?;
    predicate.define_method("kind", method!(predicate_kind, 0))?;
    predicate.define_method("path", method!(predicate_path, 0))?;
    predicate.define_method("expected", method!(predicate_expected, 0))?;
    predicate.define_method("item", method!(predicate_item, 0))?;
    predicate.define_method("allowed", method!(predicate_allowed, 0))?;
    predicate.define_method("min_words", method!(predicate_min_words, 0))?;
    predicate.define_method("max_words", method!(predicate_max_words, 0))?;
    predicate.define_method("checks", method!(predicate_checks_of, 0))?;
    predicate.define_method("check", method!(predicate_check, 0))?;

    let verification = module.define_class("Verification", object)?;
    identity!(verification, Verification);
    verification.define_singleton_method("unchecked", function!(verification_unchecked, 0))?;
    verification.define_singleton_method("passed", function!(verification_passed, 0))?;
    verification.define_singleton_method("failed", function!(verification_failed, 0))?;
    verification.define_singleton_method(
        "invalid_submission",
        function!(verification_invalid_submission, 0),
    )?;
    verification.define_singleton_method("timed_out", function!(verification_timed_out, 0))?;
    verification
        .define_singleton_method("harness_error", function!(verification_harness_error, -1))?;
    verification.define_method("status", method!(verification_status, 0))?;
    verification.define_method("detail", method!(verification_detail, 0))?;

    let outcome = module.define_class("Outcome", object)?;
    identity!(outcome, Outcome);
    outcome.define_singleton_method("new", function!(outcome_new, -1))?;
    outcome.define_method("changed_files", method!(outcome_changed_files, 0))?;
    outcome.define_method("verification", method!(outcome_verification, 0))?;
    outcome.define_method("response", method!(outcome_response, 0))?;
    outcome.define_method("facts", method!(outcome_facts, 0))?;
    outcome.define_method("observation", method!(outcome_observation, 0))?;

    let assessment = module.define_class("Assessment", object)?;
    identity!(assessment, Assessment);
    assessment.define_singleton_method("new", function!(assessment_new, -1))?;
    assessment.define_method("passed?", method!(assessment_passed, 0))?;
    assessment.define_method("observed", method!(assessment_observed, 0))?;
    assessment.define_method("scores", method!(assessment_scores, 0))?;
    assessment.define_method("kind", method!(assessment_kind, 0))?;
    assessment.define_method("predicate", method!(assessment_predicate, 0))?;
    assessment.define_method("observation", method!(assessment_observation, 0))?;
    assessment.define_method("checks", method!(assessment_checks, 0))?;

    let file = module.define_class("SourceFile", object)?;
    identity!(file, SourceFile);
    file.define_singleton_method("new", function!(file_new, -1))?;
    file.define_method("path", method!(file_path, 0))?;
    file.define_method("contents", method!(file_contents, 0))?;

    let instruction = module.define_class("Instruction", object)?;
    identity!(instruction, Instruction);
    instruction.define_singleton_method("new", function!(instruction_new, -1))?;
    instruction.define_method("prompt", method!(instruction_prompt, 0))?;

    let verifier = module.define_class("Verifier", object)?;
    identity!(verifier, Verifier);
    verifier.define_singleton_method("new", function!(verifier_new, -1))?;
    verifier.define_method("inputs", method!(verifier_inputs, 0))?;
    verifier.define_method("files", method!(verifier_files, 0))?;
    verifier.define_method("tools", method!(verifier_tools, 0))?;
    verifier.define_method("command", method!(verifier_command, 0))?;
    verifier.define_method("stdin", method!(verifier_stdin, 0))?;
    verifier.define_method("timeout_seconds", method!(verifier_timeout_seconds, 0))?;

    let text_rule = module.define_class("TextRule", object)?;
    identity!(text_rule, TextRule);
    text_rule.define_singleton_method("new", function!(text_rule_new, -1))?;
    text_rule.define_method("field", method!(text_rule_field, 0))?;
    text_rule.define_method("min_words", method!(text_rule_min_words, 0))?;
    text_rule.define_method("max_words", method!(text_rule_max_words, 0))?;

    let rubric = module.define_class("Rubric", object)?;
    identity!(rubric, Rubric);
    rubric.define_singleton_method("new", function!(rubric_new, -1))?;
    rubric.define_method("expected", method!(rubric_expected, 0))?;
    rubric.define_method("text_rule", method!(rubric_text_rule, 0))?;

    let criterion = module.define_class("Criterion", object)?;
    identity!(criterion, Criterion);
    criterion.define_singleton_method("code", function!(criterion_code, -1))?;
    criterion.define_singleton_method("answer", function!(criterion_answer, -1))?;
    criterion.define_singleton_method("state", function!(criterion_state, -1))?;
    criterion.define_method("condition", method!(criterion_condition, 0))?;
    criterion.define_method("kind", method!(criterion_kind, 0))?;
    criterion.define_method("verifier", method!(criterion_verifier, 0))?;
    criterion.define_method("rubric", method!(criterion_rubric, 0))?;
    criterion.define_method("predicate", method!(criterion_predicate, 0))?;

    let reference = module.define_class("Reference", object)?;
    identity!(reference, Reference);
    reference.define_singleton_method("new", function!(reference_new, -1))?;
    reference.define_method("edits", method!(reference_edits, 0))?;
    reference.define_method("completion", method!(reference_completion, 0))?;

    let case = module.define_class("Case", object)?;
    identity!(case, Case);
    case.define_method("case_id", method!(case_case_id, 0))?;
    case.define_method("prompt", method!(case_prompt, 0))?;
    case.define_method("files", method!(case_files, 0))?;

    let issue = module.define_class("Issue", object)?;
    identity!(issue, Issue);
    issue.define_method("field", method!(issue_field, 0))?;
    issue.define_method("message", method!(issue_message, 0))?;

    let task = module.define_class("Task", object)?;
    identity!(task, Task);
    task.define_singleton_method("new", function!(task_new, -1))?;
    task.define_method("id", method!(task_id, 0))?;
    task.define_method("fixture", method!(task_fixture, 0))?;
    task.define_method("instruction", method!(task_instruction, 0))?;
    task.define_method("criterion", method!(task_criterion, 0))?;
    task.define_method("allowed_files", method!(task_allowed_files, 0))?;
    task.define_method("reference", method!(task_reference, 0))?;
    task.define_method("project", method!(task_project, 0))?;
    task.define_method("checks", method!(task_checks, 0))?;
    task.define_method("assess", method!(task_assess, -1))?;
    task.define_method("grade", method!(task_grade, -1))?;

    module.const_set("NESTING_LIMIT", core::NESTING_LIMIT)?;
    module.const_set(
        "REQUIRED_SCORES",
        array(
            ruby,
            core::REQUIRED_SCORES.iter().map(|name| string(ruby, name)),
        )?,
    )?;
    module.define_singleton_method("version", function!(version, 0))?;
    module.define_singleton_method("source_hash", function!(source_hash, 0))?;
    Ok(())
}
