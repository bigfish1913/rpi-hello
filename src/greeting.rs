//! Pure greeting logic, kept separate from the ABI so it can be unit tested.

use serde_json::{json, Value};

/// Names longer than this are truncated (Unicode scalar values, not bytes).
pub const MAX_NAME_CHARS: usize = 120;

pub const TOOL_NAME: &str = "hello";
pub const COMMAND_NAME: &str = "hello";

/// A parsed greeting request, shared by the tool and the slash command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub name: String,
    pub excited: bool,
}

impl Default for Request {
    fn default() -> Self {
        Self {
            name: "world".to_string(),
            excited: false,
        }
    }
}

/// Build the greeting text. Blank names fall back to `world`.
pub fn greet(name: &str, excited: bool) -> String {
    let name = name.trim();
    let name = if name.is_empty() { "world" } else { name };
    let emphasis = if excited { "!!!" } else { "!" };
    format!("Hello, {name}{emphasis}")
}

/// Read the tool arguments: `{ "name": string, "excited": bool }`.
pub fn from_tool_params(params: &Value) -> Request {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .map(normalize)
        .unwrap_or_else(|| "world".to_string());
    let excited = params
        .get("excited")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    Request { name, excited }
}

/// Parse `/hello Ada --excited`, given everything after the command name.
pub fn from_command_args(args: &str) -> Request {
    let mut excited = false;
    let mut words = Vec::new();
    for word in args.split_whitespace() {
        match word {
            "--excited" | "-e" => excited = true,
            other => words.push(other),
        }
    }
    Request {
        name: normalize(&words.join(" ")),
        excited,
    }
}

/// The `AgentToolResult` envelope expected from a poll `Done` payload.
pub fn tool_result(text: &str) -> String {
    json!({
        "content": [{ "type": "text", "text": text }],
        "details": {
            "kind": "rpi-hello",
            "version": env!("CARGO_PKG_VERSION"),
        },
    })
    .to_string()
}

fn normalize(value: &str) -> String {
    let trimmed: String = value.chars().take(MAX_NAME_CHARS).collect();
    let trimmed = trimmed.trim();
    if trimmed.is_empty() {
        "world".to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greets_a_real_name_once() {
        assert_eq!(greet("Ada", false), "Hello, Ada!");
        assert_eq!(greet("  Ada  ", false), "Hello, Ada!");
    }

    #[test]
    fn excited_adds_emphasis_and_blank_names_become_world() {
        assert_eq!(greet("Ada", true), "Hello, Ada!!!");
        assert_eq!(greet("   ", false), "Hello, world!");
        assert_eq!(greet("", true), "Hello, world!!!");
    }

    #[test]
    fn tool_params_default_to_world_and_ignore_wrong_types() {
        assert_eq!(from_tool_params(&json!({})), Request::default());
        assert_eq!(
            from_tool_params(&json!({"name": 42, "excited": "yes"})),
            Request::default()
        );
        assert_eq!(
            from_tool_params(&json!({"name": "Ada", "excited": true})),
            Request {
                name: "Ada".into(),
                excited: true
            }
        );
    }

    #[test]
    fn command_args_accept_the_excited_flag_in_any_position() {
        assert_eq!(
            from_command_args("Ada --excited"),
            Request {
                name: "Ada".into(),
                excited: true
            }
        );
        assert_eq!(
            from_command_args("-e Ada Lovelace"),
            Request {
                name: "Ada Lovelace".into(),
                excited: true
            }
        );
        assert_eq!(from_command_args(""), Request::default());
    }

    #[test]
    fn long_names_are_truncated_by_characters_not_bytes() {
        let long = "世".repeat(MAX_NAME_CHARS + 20);
        let request = from_command_args(&long);
        assert_eq!(request.name.chars().count(), MAX_NAME_CHARS);
        assert!(request.name.is_char_boundary(request.name.len()));
    }

    #[test]
    fn tool_result_is_the_agent_tool_result_envelope() {
        let value: Value = serde_json::from_str(&tool_result("Hello, Ada!")).unwrap();
        assert_eq!(value["content"][0]["type"], "text");
        assert_eq!(value["content"][0]["text"], "Hello, Ada!");
        assert_eq!(value["details"]["kind"], "rpi-hello");
        assert_eq!(value["details"]["version"], env!("CARGO_PKG_VERSION"));
    }
}
