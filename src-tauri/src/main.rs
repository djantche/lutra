// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // `lutra --play <config.json>`: the emulated-game player, launched by Lutra itself.
    let args: Vec<std::ffi::OsString> = std::env::args_os().collect();
    if let Some(index) = args.iter().position(|arg| arg == "--play") {
        let Some(config) = args.get(index + 1) else {
            eprintln!("usage: lutra --play <config.json>");
            std::process::exit(2);
        };
        std::process::exit(lutra_lib::play(std::path::Path::new(config)));
    }
    // `lutra --core-options <core> <language>`: the options a core declares, as JSON.
    if let Some(index) = args.iter().position(|arg| arg == "--core-options") {
        let (Some(core), language) = (args.get(index + 1), args.get(index + 2)) else {
            eprintln!("usage: lutra --core-options <core> [language]");
            std::process::exit(2);
        };
        let language = language
            .and_then(|language| language.to_str())
            .unwrap_or("en");
        std::process::exit(lutra_lib::list_core_options(
            std::path::Path::new(core),
            language,
        ));
    }
    lutra_lib::prepare_rendering();
    lutra_lib::run()
}
