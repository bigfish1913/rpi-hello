//! The `/hello [name] [--excited]` slash command.

use rpi_plugin_sdk::{PluginApi, StbStringRef};
use serde_json::{json, Value};

use crate::greeting;

rpi_plugin_sdk::string_handler! {
    pub fn hello_command(input, out, _user_data) {
        if !out.is_available() {
            return 1;
        }
        let envelope: Value = serde_json::from_str(input).unwrap_or(Value::Null);
        let args = envelope.get("args").and_then(Value::as_str).unwrap_or("");
        let request = greeting::from_command_args(args);
        let text = greeting::greet(&request.name, request.excited);
        out.write(json!({ "kind": "message", "text": text }).to_string());
        0
    }
}

/// Register the slash command. Returns the host status code.
pub fn register(api: &PluginApi) -> i32 {
    let Some(register_command) = api.register_command else {
        return 1;
    };
    register_command(
        StbStringRef::from_str(greeting::COMMAND_NAME),
        StbStringRef::from_str("Print a greeting: /hello [name] [--excited]"),
        hello_command,
    )
}
