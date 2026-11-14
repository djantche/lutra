//! Lutra's own activity log (`<data>/logs/lutra.log`): what the app did and what went wrong.
//! Game output goes to one log per game instead; see `spawn_process` in `lib.rs`.

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::{Mutex, OnceLock};

/// Past this size the log is moved to `lutra.old.log` at startup.
const MAX_BYTES: u64 = 2 * 1024 * 1024;

static LOG: OnceLock<Mutex<File>> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Info,
    Warn,
    Error,
}

impl Level {
    pub fn parse(value: &str) -> Self {
        match value {
            "error" => Level::Error,
            "warn" => Level::Warn,
            _ => Level::Info,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
        }
    }
}

pub fn init(path: &Path) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    if fs::metadata(path).is_ok_and(|meta| meta.len() > MAX_BYTES) {
        let _ = fs::rename(path, path.with_extension("old.log"));
    }
    let file = OpenOptions::new().create(true).append(true).open(path)?;
    let _ = LOG.set(Mutex::new(file));
    Ok(())
}

fn format_line(time: &str, level: Level, message: &str) -> String {
    // One entry per line, whatever the message contains.
    let message = message.replace('\n', " | ");
    format!("{time} {:<5} {message}\n", level.as_str())
}

pub fn write(level: Level, message: &str) {
    let time = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let line = format_line(&time, level, message);
    eprint!("{line}");
    if let Some(Ok(mut file)) = LOG.get().map(Mutex::lock) {
        let _ = file.write_all(line.as_bytes());
    }
}

pub fn info(message: impl AsRef<str>) {
    write(Level::Info, message.as_ref());
}

pub fn warn(message: impl AsRef<str>) {
    write(Level::Warn, message.as_ref());
}

pub fn error(message: impl AsRef<str>) {
    write(Level::Error, message.as_ref());
}

/// Empties a log file. Lutra's own is open for appending, so what it writes next starts the file
/// again from the top.
pub fn clear(path: &Path) -> std::io::Result<()> {
    match OpenOptions::new().write(true).truncate(true).open(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        other => other.map(|_| ()),
    }
}

/// The end of a log file, starting on a whole line. `truncated` says older lines were left out.
pub fn read_tail(path: &Path, max_bytes: u64) -> std::io::Result<(String, bool)> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok((String::new(), false));
        }
        Err(error) => return Err(error),
    };
    let length = file.metadata()?.len();
    let truncated = length > max_bytes;
    if truncated {
        file.seek(SeekFrom::Start(length - max_bytes))?;
    }

    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    let mut text = String::from_utf8_lossy(&bytes).into_owned();
    if let Some(newline) = text.find('\n').filter(|_| truncated) {
        text.drain(..=newline);
    }
    Ok((text, truncated))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_have_a_time_a_padded_level_and_stay_on_one_line() {
        assert_eq!(
            format_line("2026-01-01 10:00:00", Level::Info, "Started"),
            "2026-01-01 10:00:00 INFO  Started\n"
        );
        assert_eq!(format_line("t", Level::Error, "a\nb"), "t ERROR a | b\n");
    }

    #[test]
    fn levels_from_the_frontend_default_to_info() {
        assert_eq!(Level::parse("error"), Level::Error);
        assert_eq!(Level::parse("warn"), Level::Warn);
        assert_eq!(Level::parse("whatever"), Level::Info);
    }

    #[test]
    fn reading_the_tail_starts_on_a_whole_line() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("game.log");
        fs::write(&path, "first line\nsecond line\nthird line\n").unwrap();

        assert_eq!(
            read_tail(&path, 1024).unwrap(),
            ("first line\nsecond line\nthird line\n".into(), false)
        );
        assert_eq!(read_tail(&path, 15).unwrap(), ("third line\n".into(), true));
        assert_eq!(
            read_tail(&dir.path().join("missing.log"), 10).unwrap(),
            (String::new(), false)
        );
    }
}
