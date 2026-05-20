use godot::prelude::*;
mod nvim;
mod plugin;

struct GodimExtension;

/*
 * This file is the entrypoint for the godot extension. This file is needed
 * for whatever reason for the godot-rust library.
 */

#[gdextension]
unsafe impl ExtensionLibrary for GodimExtension {}
