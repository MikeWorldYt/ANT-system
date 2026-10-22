// Binary entry point; all logic lives in the library crate.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    folder_manager_lib::run()
}
