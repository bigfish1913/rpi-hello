//! Drives the real ABI callbacks with a minimal fake host.

use rpi_plugin_sdk::{callbacks, StbString, StbStringRef};
use serde_json::Value;

#[test]
fn the_command_writes_a_message_envelope() {
    let input = r#"{"command":"hello","args":"Ada --excited"}"#;
    let mut slot = StbString::empty();

    let rc = rpi_hello::command::hello_command(
        StbStringRef::from_str(input),
        &mut slot,
        std::ptr::null_mut(),
    );

    assert_eq!(rc, 0);
    // SAFETY: the command wrote one live allocation into `slot`.
    let text = unsafe { slot.to_string_lossy() };
    let value: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(value["kind"], "message");
    assert_eq!(value["text"], "Hello, Ada!!!");
    // SAFETY: exactly one producer allocation is reclaimed here.
    unsafe { callbacks::free_string(slot) };
}

#[test]
fn the_command_degrades_to_world_on_an_empty_envelope() {
    let mut slot = StbString::empty();

    let rc = rpi_hello::command::hello_command(
        StbStringRef::from_str("{}"),
        &mut slot,
        std::ptr::null_mut(),
    );

    assert_eq!(rc, 0);
    // SAFETY: the command wrote one live allocation into `slot`.
    let text = unsafe { slot.to_string_lossy() };
    let value: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(value["text"], "Hello, world!");
    // SAFETY: exactly one producer allocation is reclaimed here.
    unsafe { callbacks::free_string(slot) };
}
