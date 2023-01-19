use once_cell::sync::Lazy;
use regex::Regex;

static DOLLAR_DOLLAR_BRACE_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\$\$+(\{|\()").unwrap());
static EXPR_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\$\{[^\}]*\}").unwrap());
static EXTENSION_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\$\([^\)]*\)").unwrap());
static TEXT_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"[^$]+|\$[^{($]+|\$$").unwrap());

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TokenType {
    DollarDollarBrace,
    Expr,
    Extension,
    Text,
}

pub struct Lexer {
    input_str: String,
    regexes: Vec<(TokenType, Regex)>,
    position: usize,
}

impl Lexer {
    pub fn new(input_str: &str) -> Lexer {
        Lexer {
            input_str: input_str.to_string(),
            regexes: vec![
                (
                    TokenType::DollarDollarBrace,
                    DOLLAR_DOLLAR_BRACE_REGEX.clone(),
                ),
                (TokenType::Expr, EXPR_REGEX.clone()),
                (TokenType::Extension, EXTENSION_REGEX.clone()),
                (TokenType::Text, TEXT_REGEX.clone()),
            ],
            position: 0,
        }
    }
    pub fn next(&mut self) -> Option<(TokenType, String)> {
        for (token_type, regex) in self.regexes.iter() {
            if let Some(m) = regex.captures(&self.input_str[self.position..]) {
                if let Some(m) = m.get(0) {
                    self.position += m.end();
                    let m = m.as_str();
                    let m = match token_type {
                        TokenType::DollarDollarBrace => &m[1..],
                        TokenType::Expr => &m[2..m.len() - 1],
                        TokenType::Extension => &m[2..m.len() - 1],
                        TokenType::Text => m,
                    };
                    return Some((token_type.clone(), m.to_string()));
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_lexer() {
        use super::*;
        let input_str = "hello ${world}!";
        let mut lexer = Lexer::new(input_str);
        assert_eq!(lexer.next(), Some((TokenType::Text, "hello ".to_string())));
        assert_eq!(lexer.next(), Some((TokenType::Expr, "world".to_string())));
        assert_eq!(lexer.next(), Some((TokenType::Text, "!".to_string())));
        assert_eq!(lexer.next(), None);
    }
}
