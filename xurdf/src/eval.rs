use super::lexer::*;
use evalexpr::*;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct PropertyValue {
    pub raw_value: String,
}

fn remove_quotation_marks(s: &str) -> &str {
    if s.starts_with('"') && s.ends_with('"') {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

#[allow(unused_must_use)]
pub fn eval_text(s: &str, symbol_map: &HashMap<String, PropertyValue>) -> String {
    let mut context = HashMapContext::new();
    for (name, value) in symbol_map.iter() {
        let parsed_value = value.raw_value.parse::<f64>();
        if let Ok(v) = parsed_value {
            context.set_value(name.clone(), Value::from(v));
        } else {
            context.set_value(name.clone(), Value::from(value.raw_value.clone()));
        }
    }
    let mut lexer = Lexer::new(s);
    let mut result = vec![];
    while let Some(token) = lexer.next() {
        match token.0 {
            TokenType::Text => result.push(token.1),
            TokenType::Expr => {
                let expr_in = eval_text(token.1.replace("'", "\"").as_str(), symbol_map);
                let expr = eval_with_context(&expr_in, &context);
                if let Ok(e) = expr {
                    result.push(remove_quotation_marks(&e.to_string()).to_owned());
                } else {
                    result.push(s.to_owned());
                }
            }
            TokenType::Extension => {
                let expr_in = eval_text(token.1.replace("'", "\"").as_str(), symbol_map);
                if expr_in == "cwd" {
                    result.push(
                        std::env::current_dir()
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .to_owned(),
                    );
                }
            }
            _ => {}
        }
    }
    result.join("")
}

pub fn get_boolean_value(s: &str, symbol_map: &HashMap<String, PropertyValue>) -> bool {
    let res_text = eval_text(s, symbol_map);
    eval_boolean(res_text.as_str()).unwrap_or(false)
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
}
