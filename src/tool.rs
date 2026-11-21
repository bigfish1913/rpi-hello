//! The `hello` tool: `execute` records the response, `poll` returns it once.

use std::ffi::c_void;

use rpi_plugin_sdk::handles::ToolHandles;
use rpi_plugin_sdk::{
    PluginApi, StableToolSchema, StbString, StbStringRef, StepHandle, StepResult, ToolPartialCb,
};
use serde_json::Value;

use crate::greeting;

/// One in-flight tool call. The work is trivial, so `poll` finishes immediately.
struct Job {
    response: String,
}

static JOBS: ToolHandles<Job> = ToolHandles::new();

rpi_plugin_sdk::execute_handler! {
    fn hello_execute(_call_id, params) {
        let value: Value = serde_json::from_str(&params.to_string_lossy()).unwrap_or(Value::Null);
        let request = greeting::from_tool_params(&value);
        let text = greeting::greet(&request.name, request.excited);
        JOBS.insert(Job {
            response: greeting::tool_result(&text),
        })
    }
}

extern "C" fn hello_poll(
    handle: StepHandle,
    _partial: Option<ToolPartialCb>,
    _user_data: *mut c_void,
) -> StepResult {
    if handle.is_null() {
        return StepResult::err(StbString::from_string("null hello handle".into()));
    }
    let Some(job) = JOBS.get(handle) else {
        return StepResult::err(StbString::from_string(
            "invalid or destroyed hello handle".into(),
        ));
    };
    let response = std::mem::take(&mut job.lock().response);
    if response.is_empty() {
        return StepResult::err(StbString::from_string(
            "hello tool polled after completion".into(),
        ));
    }
    StepResult::done(StbString::from_string(response))
}

extern "C" fn hello_cancel(handle: StepHandle) {
    JOBS.cancel(handle);
}

extern "C" fn hello_destroy(handle: StepHandle) {
    drop(JOBS.remove(handle));
}

fn schema() -> StableToolSchema {
    let parameters = serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "name": {
                "type": "string",
                "maxLength": greeting::MAX_NAME_CHARS,
                "description": "Who to greet. Defaults to world when omitted or blank."
            },
            "excited": {
                "type": "boolean",
                "default": false,
                "description": "Append extra emphasis to the greeting."
            }
        }
    });
    StableToolSchema {
        name: StbString::from_string(greeting::TOOL_NAME.into()),
        description: StbString::from_string(
            "Greet a person by name. Use it when the user asks for a hello or a greeting.".into(),
        ),
        parameters: StbString::from_string(parameters.to_string()),
    }
}

/// Register the tool and its prompt metadata. Returns the host status code.
pub fn register(api: &PluginApi) -> i32 {
    let Some(register_tool) = api.register_tool else {
        return 1;
    };
    // The host copies the schema strings during registration; they intentionally
    // live for the rest of the process (the SDK's StbString has no Drop).
    let tool = schema();
    let rc = register_tool(
        &tool,
        hello_execute,
        hello_poll,
        hello_cancel,
        hello_destroy,
        rpi_plugin_sdk::callbacks::free_string,
    );
    if rc != 0 {
        return rc;
    }
    if let Some(register_prompt) = api.register_tool_prompt {
        let rc = register_prompt(
            StbStringRef::from_str(greeting::TOOL_NAME),
            StbStringRef::from_str(
                r#"{"promptSnippet":"Greet a person by name","promptGuidelines":["Use hello when the user asks for a greeting; pass the exact name the user gave."]}"#,
            ),
        );
        if rc != 0 {
            return rc;
        }
    }
    0
}
