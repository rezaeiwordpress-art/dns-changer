// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Some(exit_code) = dns_changer_lib::admin::handle_elevated_cli_if_present() {
        std::process::exit(exit_code);
    }

    dns_changer_lib::run();
}
