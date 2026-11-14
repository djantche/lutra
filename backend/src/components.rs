//! Windows libraries that can be managed in a game's prefix with one click: the short list of
//! those games most often ask for. Winetricks installs most of them; Wine's DLL overrides turn
//! them on and off (nothing is deleted); and the ones that are plain DLL copies can be taken out
//! again by putting Wine's own DLLs back. Winetricks has no uninstall, so this module knows what
//! each component put where. It also knows which prefix is being changed right now, and how far an
//! install got (read back from Winetricks' output).

use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

use serde::Serialize;

use crate::dllfetch;
use crate::wine::RunnerKind;

/// Who installs a component.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Winetricks,
    /// Lutra fetches Microsoft's own DLL (`dllfetch`): the one Winetricks offers is too old.
    Microsoft,
}

/// One offered component and what it leaves in a prefix.
pub struct Component {
    /// Its name for Winetricks, and the key of its texts in the interface.
    pub verb: &'static str,
    pub source: Source,
    /// The DLLs it overrides. Empty when it adds no DLL (fonts): it then has no on/off switch.
    pub dlls: &'static [&'static str],
    /// The load order given to those DLLs.
    pub mode: &'static str,
    /// It only copied these DLLs in place of Wine's own, so putting Wine's back removes it. The
    /// others also ran an installer or registered COM classes, which cannot be undone.
    pub removable: bool,
}

impl Component {
    pub fn togglable(&self) -> bool {
        !self.dlls.is_empty()
    }
}

/// The offered components. Anything else is refused: the verb ends up on a command line, and the
/// DLL names end up in a registry file.
pub const CATALOG: &[Component] = &[
    Component {
        verb: "d3dcompiler_47",
        source: Source::Microsoft,
        dlls: &["d3dcompiler_47"],
        mode: "native",
        removable: true,
    },
    Component {
        verb: "d3dx9",
        source: Source::Winetricks,
        dlls: &[
            "d3dx9_24", "d3dx9_25", "d3dx9_26", "d3dx9_27", "d3dx9_28", "d3dx9_29", "d3dx9_30",
            "d3dx9_31", "d3dx9_32", "d3dx9_33", "d3dx9_34", "d3dx9_35", "d3dx9_36", "d3dx9_37",
            "d3dx9_38", "d3dx9_39", "d3dx9_40", "d3dx9_41", "d3dx9_42", "d3dx9_43",
        ],
        mode: "native",
        removable: true,
    },
    Component {
        verb: "d3dx11_43",
        source: Source::Winetricks,
        dlls: &["d3dx11_43"],
        mode: "native",
        removable: true,
    },
    Component {
        verb: "vcrun2022",
        source: Source::Winetricks,
        dlls: &[
            "concrt140",
            "msvcp140",
            "msvcp140_1",
            "msvcp140_2",
            "msvcp140_atomic_wait",
            "msvcp140_codecvt_ids",
            "vcamp140",
            "vccorlib140",
            "vcomp140",
            "vcruntime140",
            "vcruntime140_1",
        ],
        mode: "native,builtin",
        removable: false,
    },
    Component {
        verb: "corefonts",
        source: Source::Winetricks,
        dlls: &[],
        mode: "",
        removable: false,
    },
    Component {
        verb: "xact",
        source: Source::Winetricks,
        dlls: &[
            "xaudio2_0",
            "xaudio2_1",
            "xaudio2_2",
            "xaudio2_3",
            "xaudio2_4",
            "xaudio2_5",
            "xaudio2_6",
            "xaudio2_7",
            "x3daudio1_0",
            "x3daudio1_1",
            "x3daudio1_2",
            "x3daudio1_3",
            "x3daudio1_4",
            "x3daudio1_5",
            "x3daudio1_6",
            "x3daudio1_7",
            "xapofx1_1",
            "xapofx1_2",
            "xapofx1_3",
            "xapofx1_4",
            "xapofx1_5",
        ],
        mode: "native,builtin",
        removable: false,
    },
];

pub fn find(verb: &str) -> Option<&'static Component> {
    CATALOG.iter().find(|component| component.verb == verb)
}

/// The prefix's 64-bit and 32-bit DLL folders (`system32`, `syswow64`).
fn dll_folder(wine_prefix: &Path, wow64: bool) -> PathBuf {
    wine_prefix
        .join("drive_c/windows")
        .join(if wow64 { "syswow64" } else { "system32" })
}

/// The verbs recorded in a prefix: Winetricks appends one per line to `winetricks.log` at the root
/// of the prefix Wine sees (the `pfx/` folder for Proton), and Lutra does the same for the
/// components it installs itself, so one list says what is there.
pub fn installed(wine_prefix: &Path) -> HashSet<String> {
    fs::read_to_string(wine_prefix.join("winetricks.log"))
        .map(|text| {
            text.lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// Adds a verb to that list, unless it is already there.
pub fn record(wine_prefix: &Path, verb: &str) -> io::Result<()> {
    if installed(wine_prefix).contains(verb) {
        return Ok(());
    }
    use std::io::Write;
    let mut log = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(wine_prefix.join("winetricks.log"))?;
    // Winetricks writes a line per verb: do not glue ours to an unfinished one.
    let unfinished = fs::read(wine_prefix.join("winetricks.log"))
        .is_ok_and(|bytes| bytes.last().is_some_and(|last| *last != b'\n'));
    if unfinished {
        writeln!(log)?;
    }
    writeln!(log, "{verb}")
}

/// Makes Winetricks forget a verb, so that it installs it again when asked.
pub fn forget(wine_prefix: &Path, verb: &str) -> io::Result<()> {
    let path = wine_prefix.join("winetricks.log");
    let Ok(text) = fs::read_to_string(&path) else {
        return Ok(());
    };
    let kept: Vec<&str> = text.lines().filter(|line| line.trim() != verb).collect();
    if kept.len() == text.lines().count() {
        return Ok(());
    }
    let mut rewritten = kept.join("\n");
    if !kept.is_empty() {
        rewritten.push('\n');
    }
    fs::write(path, rewritten)
}

/// Is the installed copy not the one Lutra would install? Only the Microsoft ones can tell: the
/// old `d3dcompiler_47` that Winetricks installs makes every vertex shader of some games fail,
/// and it has a different size from the one Lutra fetches.
pub fn is_outdated(component: &Component, wine_prefix: &Path) -> bool {
    if component.source != Source::Microsoft {
        return false;
    }
    let size = |wow64| {
        fs::metadata(dll_folder(wine_prefix, wow64).join(format!("{}.dll", component.verb)))
            .map(|meta| meta.len())
    };
    size(false).is_ok_and(|size| size != dllfetch::D3DCOMPILER_SIZE)
}

/// Puts a fetched DLL in the prefix, replacing the one there (Wine's own or Winetricks').
pub fn install_dll(source: &Path, wine_prefix: &Path, name: &str) -> io::Result<()> {
    let target = dll_folder(wine_prefix, false).join(format!("{name}.dll"));
    // Not a copy over the old file: it may be a read-only copy of Wine's, or be open.
    let staging = target.with_extension("dll.lutra");
    fs::copy(source, &staging)?;
    make_writable(&staging)?;
    fs::rename(staging, target)
}

/// The DLL overrides in a prefix's `user.reg`: `"*name"="native,builtin"` lines of the
/// `[Software\\Wine\\DllOverrides]` section, keyed by the name without its `*`.
pub fn overrides(user_reg: &str) -> HashMap<String, String> {
    let mut found = HashMap::new();
    let mut inside = false;
    for line in user_reg.lines() {
        if line.starts_with('[') {
            inside = line.starts_with("[Software\\\\Wine\\\\DllOverrides]");
        } else if inside {
            let Some((name, mode)) = line.split_once("\"=\"") else {
                continue;
            };
            let name = name.trim_start_matches('"').trim_start_matches('*');
            found.insert(name.to_string(), mode.trim_end_matches('"').to_string());
        }
    }
    found
}

/// Is the component switched on, that is, do its DLLs load the real ones first?
pub fn is_enabled(component: &Component, overrides: &HashMap<String, String>) -> bool {
    component.togglable()
        && component.dlls.iter().any(|dll| {
            overrides
                .get(*dll)
                .is_some_and(|mode| mode.starts_with("native"))
        })
}

/// The registry file that switches a component on or off, for `regedit /S`. Switching off
/// deletes the overrides (Wine then loads its own DLLs), under both spellings Wine accepts.
pub fn registry_file(component: &Component, enable: bool) -> String {
    let mut text = String::from(
        "Windows Registry Editor Version 5.00\r\n\r\n[HKEY_CURRENT_USER\\Software\\Wine\\DllOverrides]\r\n",
    );
    for dll in component.dlls {
        if enable {
            text.push_str(&format!("\"*{dll}\"=\"{}\"\r\n", component.mode));
        } else {
            text.push_str(&format!("\"*{dll}\"=-\r\n\"{dll}\"=-\r\n"));
        }
    }
    text
}

/// A Linux path as Wine sees it: the `Z:` drive is the root of the file system.
pub fn windows_path(path: &Path) -> String {
    format!("Z:{}", path.to_string_lossy().replace('/', "\\"))
}

/// Where a runner keeps its own Windows DLLs (64-bit, 32-bit). Wine and Proton lay them out as
/// `lib/wine/<arch>-windows`, Proton under its `files/` folder, some builds with `lib64`.
fn builtin_dirs(kind: RunnerKind, runner_path: &Path) -> (Option<PathBuf>, Option<PathBuf>) {
    let Some(parent) = runner_path.parent() else {
        return (None, None);
    };
    let root = match kind {
        RunnerKind::Proton => parent.join("files"),
        _ => parent.parent().unwrap_or(parent).to_path_buf(),
    };
    let first = |candidates: [PathBuf; 2]| candidates.into_iter().find(|dir| dir.is_dir());
    (
        first([
            root.join("lib/wine/x86_64-windows"),
            root.join("lib64/wine/x86_64-windows"),
        ]),
        first([
            root.join("lib/wine/i386-windows"),
            root.join("lib32/wine/i386-windows"),
        ]),
    )
}

/// Takes a component's DLLs out of a prefix: Wine's own copy goes back where the component put
/// its own, or the file is deleted when Wine has none. Refuses to touch anything if the runner's
/// DLLs cannot be found, rather than leave a prefix without them.
pub fn restore_builtin(
    kind: RunnerKind,
    runner_path: &Path,
    wine_prefix: &Path,
    component: &Component,
    win32: bool,
) -> Result<(), String> {
    let (builtin64, builtin32) = builtin_dirs(kind, runner_path);
    // A 64-bit prefix keeps its 64-bit DLLs in `system32` and its 32-bit ones in `syswow64`.
    let places: Vec<(PathBuf, Option<PathBuf>)> = if win32 {
        vec![(dll_folder(wine_prefix, false), builtin32)]
    } else {
        vec![
            (dll_folder(wine_prefix, false), builtin64),
            (dll_folder(wine_prefix, true), builtin32),
        ]
    };
    if places.iter().any(|(_, builtin)| builtin.is_none()) {
        return Err("could not find the runner's own DLLs to put back".into());
    }

    for (folder, builtin) in places {
        let builtin = builtin.unwrap_or_default();
        for dll in component.dlls {
            let target = folder.join(format!("{dll}.dll"));
            let original = builtin.join(format!("{dll}.dll"));
            let result = if original.is_file() {
                fs::copy(&original, &target).and_then(|_| make_writable(&target))
            } else if target.exists() {
                fs::remove_file(&target)
            } else {
                Ok(())
            };
            result.map_err(|error| format!("{}: {error}", target.display()))?;
        }
    }
    Ok(())
}

/// Wine's DLLs are read-only, and a copy would inherit that: it must stay replaceable.
fn make_writable(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o644))
}

/// What is being done to a component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Operation {
    Install,
    Enable,
    Disable,
    Remove,
}

/// What an install is doing: fetching a file, or everything else (unpacking, registering...).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    Download,
    Install,
}

/// How far an install got. `percent` only exists while downloading, and only when the downloader
/// reports it: the other phases give nothing to measure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Progress {
    pub phase: Phase,
    pub percent: Option<u8>,
}

impl Default for Progress {
    fn default() -> Self {
        Self {
            phase: Phase::Install,
            percent: None,
        }
    }
}

/// Follows Winetricks' output. Winetricks announces each download (`Downloading <url> to <dir>`)
/// and lets `curl` draw its meter, which updates one line with `\r`: the percentage is the first
/// number of those lines. Any other step it runs (`Executing ...`) is the install phase.
#[derive(Default)]
pub struct Tracker {
    progress: Progress,
    /// The end of the last chunk, which may stop in the middle of a line.
    partial: String,
}

impl Tracker {
    pub fn progress(&self) -> Progress {
        self.progress
    }

    /// Reads new output; true when the progress changed.
    pub fn feed(&mut self, chunk: &str) -> bool {
        let before = self.progress;
        self.partial.push_str(chunk);
        let Some(end) = self.partial.rfind(['\r', '\n']) else {
            return false;
        };
        let complete: String = self.partial.drain(..=end).collect();
        for line in complete.split(['\r', '\n']) {
            self.line(line.trim());
        }
        self.progress != before
    }

    fn line(&mut self, line: &str) {
        if line.starts_with("Downloading ") {
            self.progress = Progress {
                phase: Phase::Download,
                percent: None,
            };
        } else if line.starts_with("Executing ") || line.starts_with("Using native override") {
            self.progress = Progress {
                phase: Phase::Install,
                percent: None,
            };
        } else if let Some(percent) = curl_percent(line) {
            self.progress = Progress {
                phase: Phase::Download,
                percent: Some(percent),
            };
        }
    }
}

/// The percentage of a `curl` meter line, such as `34 1453k   34  499k    0     0   326k ...`.
/// The header (`% Total    % Received ...`) and the other lines do not start with a number.
fn curl_percent(line: &str) -> Option<u8> {
    let mut fields = line.split_whitespace();
    let percent: u8 = fields.next()?.parse().ok()?;
    (percent <= 100 && fields.count() >= 8).then_some(percent)
}

/// What was appended to a file since `offset`, which moves forward. Empty when there is nothing
/// new or the file is not there yet.
pub fn read_since(path: &Path, offset: &mut u64) -> String {
    let Ok(mut file) = File::open(path) else {
        return String::new();
    };
    let mut bytes = Vec::new();
    if file.seek(SeekFrom::Start(*offset)).is_err() || file.read_to_end(&mut bytes).is_err() {
        return String::new();
    }
    *offset += bytes.len() as u64;
    String::from_utf8_lossy(&bytes).into_owned()
}

/// A job in progress on a prefix.
#[derive(Clone)]
struct Job {
    verb: String,
    operation: Operation,
    progress: Progress,
}

/// What is being done to a game's prefix right now.
#[derive(Debug, Clone, PartialEq)]
pub struct Current {
    pub verb: String,
    pub operation: Operation,
    pub progress: Progress,
}

/// Games whose prefix is being changed. Two jobs at once in the same prefix would corrupt it.
fn busy() -> MutexGuard<'static, HashMap<String, Job>> {
    static BUSY: OnceLock<Mutex<HashMap<String, Job>>> = OnceLock::new();
    BUSY.get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The job running on a game's prefix, if any.
pub fn current(game_id: &str) -> Option<Current> {
    busy().get(game_id).map(|job| Current {
        verb: job.verb.clone(),
        operation: job.operation,
        progress: job.progress,
    })
}

/// Marks the game's prefix as busy until the returned guard is dropped.
pub fn begin(game_id: &str, verb: &str, operation: Operation) -> Result<Busy, String> {
    let mut busy = busy();
    if let Some(job) = busy.get(game_id) {
        return Err(format!("{} is already being changed", job.verb));
    }
    busy.insert(
        game_id.to_string(),
        Job {
            verb: verb.to_string(),
            operation,
            progress: Progress::default(),
        },
    );
    Ok(Busy(game_id.to_string()))
}

pub struct Busy(String);

impl Busy {
    /// Remembers how far the job is, for a window that opens in the middle of it.
    pub fn set_progress(&self, progress: Progress) {
        if let Some(job) = busy().get_mut(&self.0) {
            job.progress = progress;
        }
    }
}

impl Drop for Busy {
    fn drop(&mut self) {
        busy().remove(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What Winetricks printed for a real d3dcompiler_47 install, curl meter included.
    const REAL_OUTPUT: &str = "Executing w_do_call d3dcompiler_47\r\n\
        Downloading https://example.test/a.cab to /tmp/cache\r\n\
        \x20 % Total    % Received % Xferd  Average Speed   Time    Time     Time  Current\r\n\
        \x20                                Dload  Upload   Total   Spent    Left  Speed\r\n\
        \x20 0     0    0     0    0     0      0      0 --:--:-- --:--:-- --:--:--     0\
        \r 34 1453k   34  499k    0     0   326k      0  0:00:04  0:00:01  0:00:03  326k\
        \r100 1453k  100 1453k    0     0   297k      0  0:00:04  0:00:04 --:--:--  336k\r\n\
        /tmp/cache/a.cab: WARNING; possible 8768 extra bytes at end of file.\r\n";

    #[test]
    fn only_catalog_verbs_are_known() {
        assert!(find("d3dcompiler_47").is_some());
        assert!(find("d3dcompiler_47 --force").is_none());
        assert!(find("-q").is_none());
        assert!(find("").is_none());
    }

    #[test]
    fn catalog_names_are_safe_to_write_in_a_registry_file() {
        for component in CATALOG {
            assert!(!component.verb.is_empty());
            for dll in component.dlls {
                assert!(
                    dll.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
                    "{dll}"
                );
            }
            assert_eq!(component.togglable(), !component.mode.is_empty());
            // A Microsoft component is a single DLL named like its verb.
            if component.source == Source::Microsoft {
                assert_eq!(component.dlls, &[component.verb]);
            }
        }
    }

    #[test]
    fn installed_reads_winetricks_log() {
        let dir = tempfile::tempdir().unwrap();
        assert!(installed(dir.path()).is_empty());

        fs::write(
            dir.path().join("winetricks.log"),
            "d3dcompiler_47\n\nvcrun2022\n",
        )
        .unwrap();
        let found = installed(dir.path());
        assert!(found.contains("d3dcompiler_47") && found.contains("vcrun2022"));
        assert_eq!(found.len(), 2);
    }

    #[test]
    fn record_and_forget_keep_one_line_per_verb() {
        let dir = tempfile::tempdir().unwrap();
        let log = dir.path().join("winetricks.log");
        forget(dir.path(), "d3dx9").unwrap();
        assert!(!log.exists());

        record(dir.path(), "d3dcompiler_47").unwrap();
        record(dir.path(), "d3dcompiler_47").unwrap();
        assert_eq!(fs::read_to_string(&log).unwrap(), "d3dcompiler_47\n");

        // Winetricks' last line may have no newline: ours must not be glued to it.
        fs::write(&log, "d3dcompiler_47\nd3dx9").unwrap();
        record(dir.path(), "vcrun2022").unwrap();
        assert_eq!(
            fs::read_to_string(&log).unwrap(),
            "d3dcompiler_47\nd3dx9\nvcrun2022\n"
        );

        forget(dir.path(), "d3dx9").unwrap();
        assert_eq!(
            fs::read_to_string(&log).unwrap(),
            "d3dcompiler_47\nvcrun2022\n"
        );
        forget(dir.path(), "d3dcompiler_47").unwrap();
        forget(dir.path(), "vcrun2022").unwrap();
        assert_eq!(fs::read_to_string(&log).unwrap(), "");
    }

    #[test]
    fn overrides_are_read_from_the_registry_section() {
        let reg = "[Software\\\\Wine\\\\DllOverrides] 1794667755\n#time=1dd744751b430fc\n\
            \"*d3dcompiler_47\"=\"native\"\n\"atl100\"=\"native,builtin\"\n\"atiadlxx\"=\"disabled\"\n\n\
            [Software\\\\Wine\\\\Drivers] 1\n\"*d3dx9_43\"=\"native\"\n";
        let found = overrides(reg);
        assert_eq!(
            found.get("d3dcompiler_47").map(String::as_str),
            Some("native")
        );
        assert_eq!(
            found.get("atl100").map(String::as_str),
            Some("native,builtin")
        );
        // Another section is not an override.
        assert!(!found.contains_key("d3dx9_43"));

        let d3dcompiler = find("d3dcompiler_47").unwrap();
        assert!(is_enabled(d3dcompiler, &found));
        assert!(!is_enabled(find("d3dx9").unwrap(), &found));
        // A builtin-first order is the same as not having the real DLL.
        let builtin =
            overrides("[Software\\\\Wine\\\\DllOverrides] 1\n\"*d3dcompiler_47\"=\"builtin\"\n");
        assert!(!is_enabled(d3dcompiler, &builtin));
        // Fonts have no DLL to switch.
        assert!(!is_enabled(find("corefonts").unwrap(), &found));
    }

    #[test]
    fn registry_files_switch_overrides_on_and_off() {
        let component = find("d3dcompiler_47").unwrap();
        let on = registry_file(component, true);
        assert!(on.starts_with("Windows Registry Editor Version 5.00\r\n"));
        assert!(on.contains("[HKEY_CURRENT_USER\\Software\\Wine\\DllOverrides]\r\n"));
        assert!(on.contains("\"*d3dcompiler_47\"=\"native\"\r\n"));

        let off = registry_file(component, false);
        assert!(off.contains("\"*d3dcompiler_47\"=-\r\n"));
        assert!(off.contains("\"d3dcompiler_47\"=-\r\n"));
        assert!(!off.contains("native"));

        let many = registry_file(find("xact").unwrap(), true);
        assert!(many.contains("\"*xaudio2_7\"=\"native,builtin\"\r\n"));
    }

    #[test]
    fn windows_paths_go_through_the_z_drive() {
        assert_eq!(
            windows_path(Path::new("/home/me/.local/share/lutra/a b.reg")),
            "Z:\\home\\me\\.local\\share\\lutra\\a b.reg"
        );
    }

    /// A prefix and a runner on disk, laid out like Proton's.
    fn proton_setup(dir: &Path) -> (PathBuf, PathBuf) {
        let runner = dir.join("GE-Proton/proton");
        for arch in ["x86_64-windows", "i386-windows"] {
            fs::create_dir_all(dir.join("GE-Proton/files/lib/wine").join(arch)).unwrap();
        }
        let prefix = dir.join("pfx");
        for folder in ["system32", "syswow64"] {
            fs::create_dir_all(prefix.join("drive_c/windows").join(folder)).unwrap();
        }
        (runner, prefix)
    }

    #[test]
    fn removing_puts_the_runners_own_dlls_back() {
        let scratch = tempfile::tempdir().unwrap();
        let (runner, prefix) = proton_setup(scratch.path());
        let lib = scratch.path().join("GE-Proton/files/lib/wine");
        fs::write(lib.join("x86_64-windows/d3dcompiler_47.dll"), "wine64").unwrap();
        fs::write(lib.join("i386-windows/d3dcompiler_47.dll"), "wine32").unwrap();
        let windows = prefix.join("drive_c/windows");
        fs::write(windows.join("system32/d3dcompiler_47.dll"), "native64").unwrap();
        fs::write(windows.join("syswow64/d3dcompiler_47.dll"), "native32").unwrap();

        // The runner's DLLs are read-only: the copy must not be, or it could not be replaced.
        use std::os::unix::fs::PermissionsExt;
        for arch in ["x86_64-windows", "i386-windows"] {
            fs::set_permissions(
                lib.join(arch).join("d3dcompiler_47.dll"),
                fs::Permissions::from_mode(0o555),
            )
            .unwrap();
        }

        let component = find("d3dcompiler_47").unwrap();
        restore_builtin(RunnerKind::Proton, &runner, &prefix, component, false).unwrap();
        let system32 = windows.join("system32/d3dcompiler_47.dll");
        assert_eq!(fs::read_to_string(&system32).unwrap(), "wine64");
        assert_eq!(
            fs::read_to_string(windows.join("syswow64/d3dcompiler_47.dll")).unwrap(),
            "wine32"
        );
        assert_eq!(
            fs::metadata(&system32).unwrap().permissions().mode() & 0o200,
            0o200
        );
    }

    #[test]
    fn a_dll_the_runner_does_not_have_is_deleted() {
        let scratch = tempfile::tempdir().unwrap();
        let (runner, prefix) = proton_setup(scratch.path());
        let file = prefix.join("drive_c/windows/system32/d3dx11_43.dll");
        fs::write(&file, "native").unwrap();

        let component = find("d3dx11_43").unwrap();
        restore_builtin(RunnerKind::Proton, &runner, &prefix, component, false).unwrap();
        assert!(!file.exists());
    }

    #[test]
    fn nothing_is_touched_when_the_runner_has_no_dlls() {
        let scratch = tempfile::tempdir().unwrap();
        let prefix = scratch.path().join("pfx");
        fs::create_dir_all(prefix.join("drive_c/windows/system32")).unwrap();
        let file = prefix.join("drive_c/windows/system32/d3dcompiler_47.dll");
        fs::write(&file, "native").unwrap();

        let runner = scratch.path().join("nowhere/proton");
        let component = find("d3dcompiler_47").unwrap();
        let result = restore_builtin(RunnerKind::Proton, &runner, &prefix, component, true);
        assert!(result.is_err());
        assert_eq!(fs::read_to_string(&file).unwrap(), "native");
    }

    #[test]
    fn a_32_bit_prefix_only_has_system32() {
        let scratch = tempfile::tempdir().unwrap();
        let (runner, prefix) = proton_setup(scratch.path());
        let lib = scratch.path().join("GE-Proton/files/lib/wine");
        fs::write(lib.join("i386-windows/d3dcompiler_47.dll"), "wine32").unwrap();
        let file = prefix.join("drive_c/windows/system32/d3dcompiler_47.dll");
        fs::write(&file, "native32").unwrap();

        let component = find("d3dcompiler_47").unwrap();
        restore_builtin(RunnerKind::Proton, &runner, &prefix, component, true).unwrap();
        assert_eq!(fs::read_to_string(&file).unwrap(), "wine32");
    }

    #[test]
    fn installing_a_dll_replaces_what_is_there() {
        let scratch = tempfile::tempdir().unwrap();
        let (_, prefix) = proton_setup(scratch.path());
        let target = prefix.join("drive_c/windows/system32/d3dcompiler_47.dll");
        fs::write(&target, "old").unwrap();
        let source = scratch.path().join("new.dll");
        fs::write(&source, "new").unwrap();

        install_dll(&source, &prefix, "d3dcompiler_47").unwrap();
        assert_eq!(fs::read_to_string(&target).unwrap(), "new");
        assert!(!target.with_extension("dll.lutra").exists());
    }

    #[test]
    fn an_old_microsoft_dll_is_outdated() {
        let scratch = tempfile::tempdir().unwrap();
        let (_, prefix) = proton_setup(scratch.path());
        let target = prefix.join("drive_c/windows/system32/d3dcompiler_47.dll");
        let component = find("d3dcompiler_47").unwrap();

        // Nothing there: nothing to update.
        assert!(!is_outdated(component, &prefix));
        fs::write(&target, "a different size").unwrap();
        assert!(is_outdated(component, &prefix));
        fs::write(&target, vec![0u8; dllfetch::D3DCOMPILER_SIZE as usize]).unwrap();
        assert!(!is_outdated(component, &prefix));
        // Winetricks' components are never outdated.
        assert!(!is_outdated(find("d3dx9").unwrap(), &prefix));
    }

    #[test]
    fn a_prefix_is_busy_until_the_guard_drops() {
        let guard = begin("test-busy", "d3dx9", Operation::Install).unwrap();
        let now = current("test-busy").unwrap();
        assert_eq!(
            (now.verb.as_str(), now.operation),
            ("d3dx9", Operation::Install)
        );
        assert!(begin("test-busy", "xact", Operation::Remove).is_err());

        guard.set_progress(Progress {
            phase: Phase::Download,
            percent: Some(40),
        });
        assert_eq!(current("test-busy").unwrap().progress.percent, Some(40));

        drop(guard);
        assert_eq!(current("test-busy"), None);
        assert!(begin("test-busy", "xact", Operation::Disable).is_ok());
    }

    #[test]
    fn the_tracker_follows_a_real_download() {
        let mut tracker = Tracker::default();
        // An install starts in the install phase: unpacking and registering are not measurable.
        assert!(!tracker.feed("Executing w_do_call d3dcompiler_47\r\n"));
        assert_eq!(tracker.progress().phase, Phase::Install);

        assert!(tracker.feed("Downloading https://example.test/a.cab to /tmp/cache\r\n"));
        assert_eq!(
            tracker.progress(),
            Progress {
                phase: Phase::Download,
                percent: None
            }
        );

        // The header of curl's meter says nothing yet.
        assert!(!tracker.feed("  % Total    % Received % Xferd  Average Speed   Time\r\n"));
        assert!(tracker.feed(
            " 34 1453k   34  499k    0     0   326k      0  0:00:04  0:00:01  0:00:03  326k\r"
        ));
        assert_eq!(tracker.progress().percent, Some(34));
        assert!(tracker.feed(
            "100 1453k  100 1453k    0     0   297k      0  0:00:04  0:00:04 --:--:--  336k\r\n"
        ));
        assert_eq!(tracker.progress().percent, Some(100));

        assert!(tracker.feed("Executing cabextract -q a.cab\n"));
        assert_eq!(tracker.progress(), Progress::default());
    }

    #[test]
    fn the_tracker_handles_the_whole_output_and_cut_lines() {
        let mut tracker = Tracker::default();
        tracker.feed(REAL_OUTPUT);
        assert_eq!(tracker.progress().percent, Some(100));

        // A chunk that stops inside the number must not read "3" instead of "34".
        let mut tracker = Tracker::default();
        tracker.feed("Downloading https://example.test/a.cab to /tmp\n");
        assert!(!tracker.feed(" 3"));
        assert_eq!(tracker.progress().percent, None);
        tracker
            .feed("4 1453k   34  499k    0     0   326k      0  0:00:04  0:00:01  0:00:03  326k\r");
        assert_eq!(tracker.progress().percent, Some(34));
    }

    /// Real Winetricks output, captured with `winetricks -q d3dx9 > log 2>&1` in an empty cache
    /// so that it downloads: `LUTRA_WINETRICKS_LOG=<log> cargo test -- --ignored winetricks`.
    #[test]
    #[ignore]
    fn winetricks_output_is_followed_from_download_to_install() {
        let path = std::env::var("LUTRA_WINETRICKS_LOG").expect("LUTRA_WINETRICKS_LOG");
        let mut tracker = Tracker::default();
        let (mut offset, mut percents, mut downloads) = (0, Vec::new(), 0);
        // Read it in small pieces, like the install thread does while the file grows.
        let text = read_since(Path::new(&path), &mut offset);
        for chunk in text.as_bytes().chunks(300) {
            tracker.feed(&String::from_utf8_lossy(chunk));
            match tracker.progress() {
                Progress {
                    phase: Phase::Download,
                    percent: Some(p),
                } => percents.push(p),
                Progress {
                    phase: Phase::Download,
                    percent: None,
                } => downloads += 1,
                _ => {}
            }
        }
        assert!(downloads > 0, "no download announced");
        assert!(
            percents.windows(2).all(|w| w[0] <= w[1] || w[1] <= 5),
            "{percents:?}"
        );
        assert_eq!(percents.iter().max(), Some(&100));
        assert_eq!(tracker.progress().phase, Phase::Install);
    }

    #[test]
    fn lines_that_are_not_a_meter_are_ignored() {
        assert_eq!(curl_percent("100 apples"), None);
        assert_eq!(curl_percent("% Total    % Received % Xferd"), None);
        assert_eq!(
            curl_percent("101 1453k 101 1453k 0 0 1 0 0:00:04 0:00:04 --:--:-- 1"),
            None
        );
        assert_eq!(curl_percent(""), None);
    }

    #[test]
    fn read_since_returns_only_what_is_new() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("log.txt");
        let mut offset = 0;
        assert_eq!(read_since(&path, &mut offset), "");

        fs::write(&path, "one\n").unwrap();
        assert_eq!(read_since(&path, &mut offset), "one\n");
        assert_eq!(read_since(&path, &mut offset), "");
        fs::write(&path, "one\ntwo\n").unwrap();
        assert_eq!(read_since(&path, &mut offset), "two\n");
    }
}
