use owo_colors::OwoColorize;
use serde::Serialize;
use std::io::{self, Write};

pub fn output_json<T: Serialize>(value: &T) {
    match serde_json::to_string_pretty(value) {
        Ok(json) => {
            let stdout = io::stdout();
            let mut handle = stdout.lock();
            let _ = writeln!(handle, "{}", json);
        }
        Err(e) => {
            eprintln!("Error: failed to serialize JSON: {e}");
            std::process::exit(1);
        }
    }
}

pub fn output_json_error(command: &str, error: &str) {
    let val = serde_json::json!({
        "success": false,
        "command": command,
        "error": error,
    });
    output_json(&val);
}

pub fn print_success(msg: &str) {
    println!("{}", msg.green());
}

pub fn print_error(msg: &str) {
    eprintln!("{}", msg.red());
}

pub fn print_warning(msg: &str) {
    println!("{}", msg.yellow());
}
