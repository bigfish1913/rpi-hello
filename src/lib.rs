#![forbid(unsafe_code)]
// The `export_plugin!` / `string_handler!` trampolines have raw-pointer ABI
// parameters; the SDK owns and validates them. Same allowance as the official
// extension crates.
#![allow(clippy::not_unsafe_ptr_arg_deref)]
//! `rpi-hello` — the smallest useful rpi extension.
//!
//! It registers two capabilities through the stable plugin ABI:
//!
//! * a `hello` tool (so the model can greet people), and
//! * a `/hello` slash command (so the user can do the same by hand).
//!
//! The crate is deliberately dependency-light (`rpi-plugin-sdk` + `serde_json`)
//! and touches neither the network nor the filesystem.

pub mod command;
pub mod greeting;
mod tool;

use rpi_plugin_sdk::{export_plugin, StbStringRef};

export_plugin!(|api| {
    // Optional metadata: load priority and the platforms this extension supports.
    if let Some(declare) = api.declare {
        let rc = declare(StbStringRef::from_str(
            r#"{"priority":50,"platforms":["linux","macos","windows"]}"#,
        ));
        if rc != 0 {
            return rc;
        }
    }

    let rc = tool::register(api);
    if rc != 0 {
        return rc;
    }
    command::register(api)
});
