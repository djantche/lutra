//! Minimal parser for Valve's text KeyValues format (`.vdf` / `.acf`).

use std::iter::Peekable;
use std::str::Chars;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Str(String),
    Obj(Vec<(String, Value)>),
}

impl Value {
    /// Looks a key up in an object. Steam treats keys as case-insensitive.
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Obj(entries) => entries
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(key))
                .map(|(_, v)| v),
            Value::Str(_) => None,
        }
    }

    pub fn path(&self, keys: &[&str]) -> Option<&Value> {
        keys.iter().try_fold(self, |value, key| value.get(key))
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            Value::Obj(_) => None,
        }
    }

    pub fn entries(&self) -> &[(String, Value)] {
        match self {
            Value::Obj(entries) => entries,
            Value::Str(_) => &[],
        }
    }
}

/// Parses a document into a root object holding its top-level pairs.
pub fn parse(input: &str) -> Result<Value, String> {
    let mut chars = input.chars().peekable();
    parse_object(&mut chars, false).map(Value::Obj)
}

fn skip_trivia(chars: &mut Peekable<Chars>) {
    loop {
        match chars.peek() {
            Some(c) if c.is_whitespace() => {
                chars.next();
            }
            Some('/') => {
                // `//` starts a comment running to the end of the line.
                let mut lookahead = chars.clone();
                lookahead.next();
                if lookahead.peek() == Some(&'/') {
                    for c in chars.by_ref() {
                        if c == '\n' {
                            break;
                        }
                    }
                } else {
                    return;
                }
            }
            _ => return,
        }
    }
}

fn read_string(chars: &mut Peekable<Chars>) -> Result<String, String> {
    chars.next(); // opening quote
    let mut out = String::new();
    loop {
        match chars.next() {
            None => return Err("unterminated string".into()),
            Some('"') => return Ok(out),
            Some('\\') => match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('\\') => out.push('\\'),
                Some('"') => out.push('"'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => return Err("unterminated escape".into()),
            },
            Some(c) => out.push(c),
        }
    }
}

fn parse_object(chars: &mut Peekable<Chars>, nested: bool) -> Result<Vec<(String, Value)>, String> {
    let mut entries = Vec::new();
    loop {
        skip_trivia(chars);
        match chars.peek() {
            None if nested => return Err("missing closing brace".into()),
            None => return Ok(entries),
            Some('}') if nested => {
                chars.next();
                return Ok(entries);
            }
            Some('}') => return Err("unexpected closing brace".into()),
            Some('"') => {
                let key = read_string(chars)?;
                skip_trivia(chars);
                match chars.peek() {
                    Some('{') => {
                        chars.next();
                        entries.push((key, Value::Obj(parse_object(chars, true)?)));
                    }
                    Some('"') => entries.push((key, Value::Str(read_string(chars)?))),
                    _ => return Err(format!("expected a value after key \"{key}\"")),
                }
            }
            Some(other) => return Err(format!("unexpected character '{other}'")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nested_objects_and_strings() {
        let doc = parse(
            "\"AppState\"\n{\n\t\"appid\"\t\t\"42\"\n\t\"Nested\"\n\t{\n\t\t\"k\" \"v\"\n\t}\n}\n",
        )
        .unwrap();
        assert_eq!(
            doc.path(&["AppState", "appid"]).and_then(Value::as_str),
            Some("42")
        );
        assert_eq!(
            doc.path(&["appstate", "nested", "K"])
                .and_then(Value::as_str),
            Some("v")
        );
    }

    #[test]
    fn handles_escapes_and_comments() {
        let doc = parse("// header\n\"a\" \"line\\nbreak \\\"quoted\\\"\" // trailing\n").unwrap();
        assert_eq!(
            doc.get("a").and_then(Value::as_str),
            Some("line\nbreak \"quoted\"")
        );
    }

    #[test]
    fn rejects_unbalanced_input() {
        assert!(parse("\"a\" {").is_err());
        assert!(parse("\"a\" \"b\" }").is_err());
        assert!(parse("\"a\"").is_err());
    }
}
