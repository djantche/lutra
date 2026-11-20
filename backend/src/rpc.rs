//! The conversation with the Electron shell: one JSON object per line.
//!
//! Requests arrive on stdin as `{"id": 7, "cmd": "list_games", "args": {...}}` and are answered
//! on stdout with `{"id": 7, "ok": ...}` or `{"id": 7, "err": "..."}`. Events go the other way
//! round, as `{"event": "...", "payload": ...}` (see `host::AppHandle::emit`). Nothing else may
//! ever be written to stdout.

use std::io::BufRead;
use std::panic::{AssertUnwindSafe, catch_unwind};

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::applog;
use crate::host::AppHandle;

/// Reads one argument by name. A missing one reads as `null`, which suits optional arguments.
pub fn arg<T: DeserializeOwned>(args: &Value, key: &str) -> Result<T, String> {
    serde_json::from_value(args.get(key).cloned().unwrap_or(Value::Null))
        .map_err(|e| format!("invalid argument `{key}`: {e}"))
}

pub fn to_value<T: Serialize>(value: T) -> Result<Value, String> {
    serde_json::to_value(value).map_err(|e| e.to_string())
}

/// Answers requests until the shell closes our stdin, which is how we learn it is gone.
pub fn serve(app: &AppHandle) {
    for line in std::io::stdin().lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let app = app.clone();
        std::thread::spawn(move || answer(&app, &line));
    }
    applog::info("The shell is gone: exiting");
    std::process::exit(0);
}

fn answer(app: &AppHandle, line: &str) {
    let Ok(request) = serde_json::from_str::<Value>(line) else {
        applog::warn(format!("Unreadable request: {line}"));
        return;
    };
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let command = request.get("cmd").and_then(Value::as_str).unwrap_or("");
    let args = request.get("args").cloned().unwrap_or(Value::Null);
    let result = catch_unwind(AssertUnwindSafe(|| crate::dispatch(app, command, &args)))
        .unwrap_or_else(|_| Err(format!("`{command}` crashed")));
    let reply = match result {
        Ok(value) => json!({ "id": id, "ok": value }),
        Err(message) => json!({ "id": id, "err": message }),
    };
    app.write_line(&reply.to_string());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_argument_reads_as_none_or_fails_with_its_name() {
        let args = json!({ "id": "steam:1" });
        assert_eq!(arg::<Option<u64>>(&args, "sgdbGame").unwrap(), None);
        assert_eq!(arg::<String>(&args, "id").unwrap(), "steam:1");
        assert!(
            arg::<String>(&args, "missing")
                .unwrap_err()
                .contains("missing")
        );
    }
}
