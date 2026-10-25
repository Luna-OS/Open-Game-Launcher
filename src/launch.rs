//! Spiele starten – und bei Problemen genau erklären, was los ist.

use crate::model::{CustomGame, Game, LaunchTarget};
use crate::problem::Problem;
use crate::{epic, sys};
use std::path::Path;
use std::process::{Child, Command};
use std::time::{Duration, Instant};

/// Steam- oder Epic-Spiel über den jeweiligen Launcher starten.
pub fn launch_store(game: &Game) -> Result<(), Problem> {
    match &game.launch {
        LaunchTarget::SteamRun(id) | LaunchTarget::SteamInstall(id) => {
            if !sys::protocol_registered("steam") {
                return Err(Problem::steam_missing());
            }
            let uri = if matches!(game.launch, LaunchTarget::SteamRun(_)) {
                format!("steam://rungameid/{id}")
            } else {
                format!("steam://install/{id}")
            };
            sys::open_url(&uri).map_err(|code| Problem::launcher_uri_failed(&game.name, "Steam", code, &uri))
        }
        LaunchTarget::Epic(uri) | LaunchTarget::EpicInstall(uri) => {
            if !epic::launcher_installed() {
                return Err(Problem::epic_missing());
            }
            sys::open_url(uri).map_err(|code| Problem::launcher_uri_failed(&game.name, "Epic Games", code, uri))
        }
        LaunchTarget::Custom => Err(Problem::unknown(&game.name, "Internal error: custom game without data")),
    }
}

/// Prüft ein eigenes Spiel, bevor die Sicherheitsfrage gestellt wird.
pub fn precheck_custom(c: &CustomGame) -> Result<(), Problem> {
    let exe = c.exe.trim().trim_matches('"');
    if exe.is_empty() {
        return Err(Problem::exe_missing(&c.name, "(-)", true));
    }
    if !Path::new(exe).exists() {
        return Err(Problem::exe_missing(&c.name, exe, sys::drive_exists(exe)));
    }
    let wd = c.workdir.trim().trim_matches('"');
    if !wd.is_empty() && !Path::new(wd).is_dir() {
        return Err(Problem::workdir_missing(&c.name, wd));
    }
    Ok(())
}

/// Eigenes Spiel starten. Gibt bei normalen EXEs den Prozess zurück, damit wir
/// einen Absturz direkt nach dem Start erkennen und erklären können.
pub fn launch_custom(c: &CustomGame, force_admin: bool) -> Result<Option<Child>, Problem> {
    precheck_custom(c)?;
    let exe = c.exe.trim().trim_matches('"').to_string();
    let path = Path::new(&exe);
    let wd = c.workdir.trim().trim_matches('"');
    let workdir = if wd.is_empty() {
        path.parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default()
    } else {
        wd.to_string()
    };
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();
    let is_exe = ext == "exe" || ext == "com";
    let as_admin = c.admin || force_admin;

    if as_admin || !is_exe {
        return shell_start(c, &exe, &workdir, as_admin).map(|_| None);
    }

    let mut cmd = Command::new(path);
    if !workdir.is_empty() {
        cmd.current_dir(&workdir);
    }
    let args = c.args.trim();
    if !args.is_empty() {
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.raw_arg(args);
        }
        #[cfg(not(windows))]
        {
            cmd.args(args.split_whitespace());
        }
    }
    match cmd.spawn() {
        Ok(child) => Ok(Some(child)),
        Err(e) => Err(Problem::from_os_error(&c.name, &exe, e.raw_os_error(), &e.to_string())),
    }
}

fn shell_start(c: &CustomGame, exe: &str, workdir: &str, as_admin: bool) -> Result<(), Problem> {
    #[cfg(windows)]
    {
        let verb = if as_admin { "runas" } else { "open" };
        crate::winffi::shell_execute(Some(verb), exe, Some(c.args.trim()), Some(workdir))
            .map_err(|code| Problem::from_shell_code(&c.name, exe, code, as_admin))
    }
    #[cfg(not(windows))]
    {
        let _ = (workdir, as_admin);
        Command::new(exe)
            .spawn()
            .map(|_| ())
            .map_err(|e| Problem::from_os_error(&c.name, exe, e.raw_os_error(), &e.to_string()))
    }
}

/// Beobachtet ein gestartetes Spiel ein paar Sekunden lang.
/// Stürzt es sofort ab, gibt es eine verständliche Erklärung zurück.
pub fn watch(mut child: Child, name: String, exe: String) -> Option<Problem> {
    let start = Instant::now();
    let limit = Duration::from_secs(10);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let code = status.code().map(|c| c as u32).unwrap_or(0);
                return Problem::from_exit_code(&name, &exe, code);
            }
            Ok(None) => {
                if start.elapsed() >= limit {
                    return None;
                }
                std::thread::sleep(Duration::from_millis(250));
            }
            Err(_) => return None,
        }
    }
}
