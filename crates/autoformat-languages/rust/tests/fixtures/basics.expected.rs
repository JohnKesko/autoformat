use std::collections::HashMap;
use std::io::{self, Read, Write};

/// A configuration value.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Text(String),
    Number(i64),       // a number
    List(Vec<Value>),  // nested
}

impl Value {
    pub fn describe(&self) -> String {
        match self {
            Value::Text(t) => format!("text {}", t),
            Value::Number(n) if *n < 0 => {
                let abs = -n;
                format!("negative {}", abs)
            }
            Value::Number(n) => n.to_string(),
            Value::List(items) => items.iter().map(|item| item.describe()).collect::<Vec<_>>().join(", "),
        }
    }
}

fn parse_arguments(
    arguments: &[String],
    defaults: &HashMap<String, String>,
    strict: bool,
    verbose_output: bool
) -> Result<HashMap<String, String>, String> {
    let mut result = defaults.clone();
    for argument in arguments.iter() {
        if let Some((key, value)) = argument.split_once('=') {
            result.insert(key.to_string(), value.to_string());
        } else if strict {
            return Err(format!("invalid argument: {}", argument));
        }
        else {
            continue;
        }
    }
    let total = result.len()
        + arguments.len()
        + 1;
    if verbose_output { eprintln!("{} entries", total); }
    Ok(result)
}

fn main() {
    let text = "a string with    spaces  and // not a comment";
    let raw = r#"raw "quoted" text"#;
    let values = vec![Value::Number(1), Value::Number(-2), Value::Text(String::from("hello")), Value::List(vec![])];
    /* A block
       comment */
    for v in &values { println!("{}", v.describe()); }
    let _ = (text, raw);
}
