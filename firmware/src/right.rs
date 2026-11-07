#![no_std]
#![no_main]
#[path="ui.rs"]
mod diagnostics;
use rmk::macros::rmk_peripheral;
#[rmk_peripheral(id = 0)]
mod keyboard {}

