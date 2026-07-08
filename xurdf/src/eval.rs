use super::lexer::*;
use anyhow::{anyhow, Result};
use indexmap::IndexMap;
use pyisheval::{EvalError, Interpreter, Value};
use std::collections::{BTreeMap, HashMap};

#[derive(Clone, Debug)]
pub struct PropertyValue {
    pub raw_value: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum XacroValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    List(Vec<XacroValue>),
    Map(BTreeMap<String, XacroValue>),
}

impl XacroValue {
    pub fn from_raw(raw_value: &str) -> Self {
        if raw_value.eq_ignore_ascii_case("true") {
            Self::Bool(true)
        } else if raw_value.eq_ignore_ascii_case("false") {
            Self::Bool(false)
        } else if let Ok(value) = raw_value.parse::<f64>() {
            Self::Number(value)
        } else {
            Self::String(raw_value.to_string())
        }
    }

    pub fn raw_value(&self) -> String {
        match self {
            Self::Null => String::new(),
            Self::Bool(value) => value.to_string(),
            Self::Number(value) => value.to_string(),
            Self::String(value) => value.clone(),
            Self::List(values) => values
                .iter()
                .map(Self::raw_value)
                .collect::<Vec<_>>()
                .join(" "),
            Self::Map(values) => values
                .iter()
                .map(|(key, value)| format!("{}: {}", key, value.raw_value()))
                .collect::<Vec<_>>()
                .join(", "),
        }
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Self::Null => false,
            Self::Bool(value) => *value,
            Self::Number(value) => *value != 0.0,
            Self::String(value) => !value.is_empty() && !value.eq_ignore_ascii_case("false"),
            Self::List(values) => !values.is_empty(),
            Self::Map(values) => !values.is_empty(),
        }
    }

    fn to_eval_value(&self) -> Value {
        match self {
            Self::Null => Value::StringLit(String::new()),
            Self::Bool(value) => Value::Number(if *value { 1.0 } else { 0.0 }),
            Self::Number(value) => Value::Number(*value),
            Self::String(value) => Value::StringLit(value.clone()),
            Self::List(values) => Value::List(values.iter().map(Self::to_eval_value).collect()),
            Self::Map(values) => {
                let mut map = IndexMap::new();
                for (key, value) in values.iter() {
                    map.insert(key.clone(), value.to_eval_value());
                }
                Value::Dict(map)
            }
        }
    }
}

impl From<&PropertyValue> for XacroValue {
    fn from(value: &PropertyValue) -> Self {
        Self::from_raw(&value.raw_value)
    }
}

fn remove_quotation_marks(s: &str) -> &str {
    if s.starts_with('"') && s.ends_with('"') {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

pub fn eval_text(s: &str, symbol_map: &HashMap<String, PropertyValue>) -> String {
    try_eval_text(s, symbol_map, &|expr| {
        if expr == "cwd" {
            Ok(std::env::current_dir()
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned())
        } else {
            Ok(String::new())
        }
    })
    .unwrap()
}

pub fn try_eval_text<F>(
    s: &str,
    symbol_map: &HashMap<String, PropertyValue>,
    resolve_extension: &F,
) -> Result<String>
where
    F: Fn(&str) -> Result<String>,
{
    let values = typed_context_from_properties(symbol_map);
    try_eval_text_with_values(s, &values, resolve_extension, &|_| Ok(None))
}

pub fn try_eval_text_with_values<F, G>(
    s: &str,
    symbol_map: &HashMap<String, XacroValue>,
    resolve_extension: &F,
    resolve_value: &G,
) -> Result<String>
where
    F: Fn(&str) -> Result<String>,
    G: Fn(&str) -> Result<Option<XacroValue>>,
{
    let mut lexer = Lexer::new(s);
    let mut result = vec![];
    while let Some(token) = lexer.next() {
        match token.0 {
            TokenType::Text => result.push(token.1),
            TokenType::Expr => {
                let expr_in = try_eval_text_with_values(
                    token.1.as_str(),
                    symbol_map,
                    resolve_extension,
                    resolve_value,
                )?;
                if let Some(value) = try_eval_expression(&expr_in, symbol_map, resolve_value)? {
                    result.push(remove_quotation_marks(&value.raw_value()).to_owned());
                } else {
                    result.push(format!("${{{}}}", token.1));
                }
            }
            TokenType::Extension => {
                let expr_in = try_eval_text_with_values(
                    token.1.as_str(),
                    symbol_map,
                    resolve_extension,
                    resolve_value,
                )?;
                result.push(resolve_extension(&expr_in)?);
            }
            _ => {}
        }
    }
    Ok(result.join(""))
}

pub fn try_eval_value_with_values<F, G>(
    s: &str,
    symbol_map: &HashMap<String, XacroValue>,
    resolve_extension: &F,
    resolve_value: &G,
) -> Result<XacroValue>
where
    F: Fn(&str) -> Result<String>,
    G: Fn(&str) -> Result<Option<XacroValue>>,
{
    if let Some(expr) = single_expression(s) {
        let expr_in =
            try_eval_text_with_values(&expr, symbol_map, resolve_extension, resolve_value)?;
        if let Some(value) = try_eval_expression(&expr_in, symbol_map, resolve_value)? {
            return Ok(value);
        }
    }

    let value = try_eval_text_with_values(s, symbol_map, resolve_extension, resolve_value)?;
    Ok(XacroValue::from_raw(&value))
}

pub fn get_boolean_value(s: &str, symbol_map: &HashMap<String, PropertyValue>) -> bool {
    try_get_boolean_value(s, symbol_map).unwrap()
}

pub fn try_get_boolean_value(s: &str, symbol_map: &HashMap<String, PropertyValue>) -> Result<bool> {
    let values = typed_context_from_properties(symbol_map);
    try_get_boolean_value_with_values(s, &values, &|_| Ok(String::new()), &|_| Ok(None))
}

pub fn try_get_boolean_value_with_values<F, G>(
    s: &str,
    symbol_map: &HashMap<String, XacroValue>,
    resolve_extension: &F,
    resolve_value: &G,
) -> Result<bool>
where
    F: Fn(&str) -> Result<String>,
    G: Fn(&str) -> Result<Option<XacroValue>>,
{
    if let Some(expr) = single_expression(s) {
        let expr_in =
            try_eval_text_with_values(&expr, symbol_map, resolve_extension, resolve_value)?;
        if let Some(value) = try_eval_expression(&expr_in, symbol_map, resolve_value)? {
            return Ok(value.is_truthy());
        }
    }

    let res_text = try_eval_text_with_values(s, symbol_map, resolve_extension, resolve_value)?;
    if res_text.eq_ignore_ascii_case("true") {
        return Ok(true);
    }
    if res_text.eq_ignore_ascii_case("false") || res_text.is_empty() {
        return Ok(false);
    }
    if let Ok(value) = res_text.parse::<f64>() {
        return Ok(value != 0.0);
    }

    let interp = Interpreter::new();
    interp.eval_boolean(res_text.as_str()).map_err(|e| {
        anyhow!(
            "failed to evaluate boolean expression `{}` from `{}`: {}",
            res_text,
            s,
            e
        )
    })
}

fn typed_context_from_properties(
    symbol_map: &HashMap<String, PropertyValue>,
) -> HashMap<String, XacroValue> {
    symbol_map
        .iter()
        .map(|(name, value)| (name.clone(), XacroValue::from(value)))
        .collect()
}

fn xacro_builtin_radians(args: &[f64]) -> std::result::Result<f64, EvalError> {
    if args.len() != 1 {
        return Err(EvalError::ArgError("radians".to_string()));
    }
    Ok(args[0].to_radians())
}

fn xacro_builtin_degrees(args: &[f64]) -> std::result::Result<f64, EvalError> {
    if args.len() != 1 {
        return Err(EvalError::ArgError("degrees".to_string()));
    }
    Ok(args[0].to_degrees())
}

fn eval_context(symbol_map: &HashMap<String, XacroValue>) -> HashMap<String, Value> {
    let mut context = HashMap::from([
        ("pi".to_string(), Value::Number(std::f64::consts::PI)),
        (
            "radians".to_string(),
            Value::Builtin {
                name: "radians".to_string(),
                func: xacro_builtin_radians,
            },
        ),
        (
            "degrees".to_string(),
            Value::Builtin {
                name: "degrees".to_string(),
                func: xacro_builtin_degrees,
            },
        ),
    ]);
    context.extend(
        symbol_map
            .iter()
            .map(|(name, value)| (name.clone(), value.to_eval_value())),
    );
    context
}

fn try_eval_expression<G>(
    expr: &str,
    symbol_map: &HashMap<String, XacroValue>,
    resolve_value: &G,
) -> Result<Option<XacroValue>>
where
    G: Fn(&str) -> Result<Option<XacroValue>>,
{
    if let Some(value) = resolve_value(expr)? {
        return Ok(Some(value));
    }
    if let Some(value) = lookup_path_expression(expr, symbol_map) {
        return Ok(Some(value));
    }

    let interp = Interpreter::new();
    match interp.eval_with_context(expr, &eval_context(symbol_map)) {
        Ok(value) => Ok(Some(xacro_value_from_eval(value))),
        Err(_) => Ok(None),
    }
}

fn xacro_value_from_eval(value: Value) -> XacroValue {
    match value {
        Value::Number(value) => XacroValue::Number(value),
        Value::StringLit(value) | Value::Var(value) => XacroValue::String(value),
        Value::List(values) | Value::Tuple(values) | Value::Set(values) => {
            XacroValue::List(values.into_iter().map(xacro_value_from_eval).collect())
        }
        Value::Dict(values) => XacroValue::Map(
            values
                .into_iter()
                .map(|(key, value)| (key, xacro_value_from_eval(value)))
                .collect(),
        ),
        value => XacroValue::String(value.to_string()),
    }
}

fn single_expression(s: &str) -> Option<String> {
    let mut lexer = Lexer::new(s);
    let Some(token) = lexer.next() else {
        return None;
    };
    if token.0 != TokenType::Expr || lexer.next().is_some() {
        return None;
    }
    Some(token.1)
}

fn lookup_path_expression(
    expr: &str,
    symbol_map: &HashMap<String, XacroValue>,
) -> Option<XacroValue> {
    let mut cursor = expr.trim();
    let (name, rest) = take_identifier(cursor)?;
    let mut value = symbol_map.get(name)?.clone();
    cursor = rest.trim_start();

    while !cursor.is_empty() {
        if let Some(rest) = cursor.strip_prefix('.') {
            let (key, rest) = take_identifier(rest.trim_start())?;
            value = match value {
                XacroValue::Map(values) => values.get(key)?.clone(),
                _ => return None,
            };
            cursor = rest.trim_start();
        } else if cursor.starts_with('[') {
            let (index, rest) = take_bracket_index(cursor)?;
            value = match value {
                XacroValue::Map(values) => values.get(&index.key()?)?.clone(),
                XacroValue::List(values) => values.get(index.list_index()?)?.clone(),
                _ => return None,
            };
            cursor = rest.trim_start();
        } else {
            return None;
        }
    }

    Some(value)
}

#[derive(Clone, Debug)]
enum PathIndex {
    Key(String),
    ListIndex(usize),
}

impl PathIndex {
    fn key(&self) -> Option<String> {
        match self {
            Self::Key(value) => Some(value.clone()),
            Self::ListIndex(value) => Some(value.to_string()),
        }
    }

    fn list_index(&self) -> Option<usize> {
        match self {
            Self::ListIndex(value) => Some(*value),
            Self::Key(value) => value.parse::<usize>().ok(),
        }
    }
}

fn take_identifier(s: &str) -> Option<(&str, &str)> {
    let mut chars = s.char_indices();
    let (_, first) = chars.next()?;
    if !(first == '_' || first.is_ascii_alphabetic()) {
        return None;
    }

    let mut end = first.len_utf8();
    for (idx, ch) in chars {
        if ch == '_' || ch.is_ascii_alphanumeric() {
            end = idx + ch.len_utf8();
        } else {
            return Some((&s[..idx], &s[idx..]));
        }
    }
    Some((&s[..end], &s[end..]))
}

fn take_bracket_index(s: &str) -> Option<(PathIndex, &str)> {
    let mut quote = None;
    let mut escape = false;
    for (idx, ch) in s.char_indices().skip(1) {
        if escape {
            escape = false;
            continue;
        }
        if ch == '\\' {
            escape = true;
            continue;
        }
        if let Some(q) = quote {
            if ch == q {
                quote = None;
            }
            continue;
        }
        match ch {
            '\'' | '"' => quote = Some(ch),
            ']' => {
                let raw_index = s[1..idx].trim();
                let index = parse_path_index(raw_index)?;
                return Some((index, &s[idx + ch.len_utf8()..]));
            }
            _ => {}
        }
    }
    None
}

fn parse_path_index(raw_index: &str) -> Option<PathIndex> {
    let value = strip_balanced_quotes(raw_index).to_string();
    if value == raw_index {
        if let Ok(index) = value.parse::<usize>() {
            return Some(PathIndex::ListIndex(index));
        }
    }
    Some(PathIndex::Key(value))
}

fn strip_balanced_quotes(value: &str) -> &str {
    if value.len() >= 2
        && ((value.starts_with('\'') && value.ends_with('\''))
            || (value.starts_with('"') && value.ends_with('"')))
    {
        &value[1..value.len() - 1]
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_eval() {
        use super::*;
        let mut context = HashMap::new();
        context.insert(
            "test".to_string(),
            PropertyValue {
                raw_value: "1".to_string(),
            },
        );

        let result = eval_text("${test}", &context);
        assert_eq!(result, "1".to_string());

        let result = eval_text("${test}_", &context);
        assert_eq!(result, "1_".to_string());
    }

    #[test]
    fn evaluates_builtin_pi_and_preserves_unresolved_expr_once() {
        use super::*;
        let context = HashMap::new();

        let result = try_eval_text_with_values(
            "0 ${pi} ${-pi/2.0}",
            &context,
            &|_| Ok(String::new()),
            &|_| Ok(None),
        )
        .unwrap();
        assert_eq!(result, "0 3.141592653589793 -1.5707963267948966");

        let result = try_eval_text_with_values(
            "prefix ${missing}",
            &context,
            &|_| Ok(String::new()),
            &|_| Ok(None),
        )
        .unwrap();
        assert_eq!(result, "prefix ${missing}");
    }

    #[test]
    fn evaluates_xacro_angle_helpers() {
        use super::*;
        let context = HashMap::new();

        let result = try_eval_text_with_values(
            "0 ${radians(90)} ${degrees(pi)}",
            &context,
            &|_| Ok(String::new()),
            &|_| Ok(None),
        )
        .unwrap();
        assert_eq!(result, "0 1.5707963267948966 180");

        let value = try_eval_value_with_values(
            "${radians(180)}",
            &context,
            &|_| Ok(String::new()),
            &|_| Ok(None),
        )
        .unwrap();
        let XacroValue::Number(value) = value else {
            panic!("expected numeric radians result");
        };
        assert!((value - std::f64::consts::PI).abs() < 1e-12);
    }

    #[test]
    fn evaluates_typed_path_values() {
        use super::*;
        let mut nested = BTreeMap::new();
        nested.insert("enabled".to_string(), XacroValue::Bool(true));
        nested.insert("name".to_string(), XacroValue::String("arm".to_string()));
        let mut root = BTreeMap::new();
        root.insert("robot".to_string(), XacroValue::Map(nested));
        root.insert(
            "offsets".to_string(),
            XacroValue::List(vec![XacroValue::Number(1.0), XacroValue::Number(2.0)]),
        );
        let context = HashMap::from([("cfg".to_string(), XacroValue::Map(root))]);

        let result = try_eval_text_with_values(
            "${cfg.robot.name}:${cfg['offsets'][1]}",
            &context,
            &|_| Ok(String::new()),
            &|_| Ok(None),
        )
        .unwrap();
        assert_eq!(result, "arm:2");

        let result = try_get_boolean_value_with_values(
            "${cfg['robot']['enabled']}",
            &context,
            &|_| Ok(String::new()),
            &|_| Ok(None),
        )
        .unwrap();
        assert!(result);
    }

    #[test]
    fn evaluates_membership_expressions() {
        use super::*;
        let mut visual = BTreeMap::new();
        visual.insert(
            "mesh".to_string(),
            XacroValue::String("base.stl".to_string()),
        );
        let mut base = BTreeMap::new();
        base.insert("visual".to_string(), XacroValue::Map(visual));
        let mut meshes = BTreeMap::new();
        meshes.insert("base".to_string(), XacroValue::Map(base));
        let context = HashMap::from([
            ("name".to_string(), XacroValue::String("base".to_string())),
            ("kind".to_string(), XacroValue::String("visual".to_string())),
            ("meshes".to_string(), XacroValue::Map(meshes)),
            (
                "choices".to_string(),
                XacroValue::List(vec![
                    XacroValue::String("visual".to_string()),
                    XacroValue::String("collision".to_string()),
                ]),
            ),
        ]);

        assert!(try_get_boolean_value_with_values(
            "${kind in meshes[name]}",
            &context,
            &|_| Ok(String::new()),
            &|_| Ok(None),
        )
        .unwrap());
        assert!(try_get_boolean_value_with_values(
            "${'visual' in choices}",
            &context,
            &|_| Ok(String::new()),
            &|_| Ok(None),
        )
        .unwrap());
        assert!(try_get_boolean_value_with_values(
            "${'missing' not in meshes[name]}",
            &context,
            &|_| Ok(String::new()),
            &|_| Ok(None),
        )
        .unwrap());
    }
}
