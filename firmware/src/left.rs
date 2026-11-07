#![no_std]
#![no_main]
#[path="ui.rs"]
mod diagnostics;
use rmk::macros::rmk_central;
#[rmk_central]
mod keyboard {}

