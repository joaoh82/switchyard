// Prevents an additional console window on Windows in release. DO NOT REMOVE.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    yardsort_lib::print_env_and_exit_if_asked();
    yardsort_lib::run()
}
