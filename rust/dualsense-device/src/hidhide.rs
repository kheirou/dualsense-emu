//! Drives HidHide's CLI: hide the real DualSense from games and whitelist this app so it can still read it.

use regex::Regex;
use serde_json::Value;
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::OnceLock;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
pub const HIDHIDE_PACKAGE: &str = "Nefarius.HidHide";
pub const VIGEM_BUS_PACKAGE: &str = "ViGEm.ViGEmBus";

pub fn cli_path() -> PathBuf {
    let pf = std::env::var_os("ProgramFiles")
        .map(PathBuf::from)
        .unwrap_or_default();
    pf.join(r"Nefarius Software Solutions\HidHide\x64\HidHideCLI.exe")
}

pub fn is_installed() -> bool {
    cli_path().is_file()
}

/// Sony VID 054C; PID 0CE6 = DualSense, 0DF2 = DualSense Edge (USB and Bluetooth instance paths).
fn sony_pad() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"(?i)VID[_&](0002)?054C.*PID[_&](0CE6|0DF2)").unwrap())
}

/// The paths end up inside an elevated cmd line, so accept only the characters real instance paths use
/// (no quote, space or % that could break out of the quotes).
fn safe_path() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"^[A-Za-z0-9_{}&\\\-]+$").unwrap())
}

/// Pulls DualSense instance paths out of `HidHideCLI --dev-gaming` JSON.
/// Empty or error text happens when HidHide is installed but its driver is not active yet (needs a restart).
pub fn parse_dualsense_paths(json: &str) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    let Ok(Value::Array(groups)) = serde_json::from_str::<Value>(json) else {
        return found;
    };
    for group in &groups {
        let Some(devices) = group.get("devices").and_then(Value::as_array) else {
            continue;
        };
        for d in devices {
            let Some(path) = d.get("deviceInstancePath").and_then(Value::as_str) else {
                continue;
            };
            if sony_pad().is_match(path)
                && safe_path().is_match(path)
                && !found.iter().any(|f| f == path)
            {
                found.push(path.to_string());
            }
        }
    }
    found
}

/// The HidHideCLI calls joined with && so the first failure stops the rest (and the exit code is the failing one).
pub fn build_chain(hide: bool, pads: &[String], app_path: &str) -> Result<String, String> {
    if app_path.contains('"') || app_path.contains('%') {
        return Err("Unsafe app path".into());
    }
    let cli = cli_path();
    let cli = cli.display();
    let mut cmds: Vec<String> = pads
        .iter()
        .map(|p| {
            format!(
                "\"{cli}\" {} \"{p}\"",
                if hide { "--dev-hide" } else { "--dev-unhide" }
            )
        })
        .collect();
    cmds.push(format!(
        "\"{cli}\" {} \"{app_path}\"",
        if hide { "--app-reg" } else { "--app-unreg" }
    ));
    cmds.push(format!(
        "\"{cli}\" {}",
        if hide { "--cloak-on" } else { "--cloak-off" }
    ));
    Ok(cmds.join(" && "))
}

pub fn is_cloak_on() -> bool {
    run_cli(&["--cloak-state"]).contains("--cloak-on")
}

pub fn find_dualsense() -> Vec<String> {
    parse_dualsense_paths(&run_cli(&["--dev-gaming"]))
}

/// One UAC prompt: hide (or unhide) the pad, whitelist/unwhitelist `app_path`, switch cloaking.
// shortcut: only pads connected right now are hidden; switching USB <-> Bluetooth needs a re-toggle.
pub fn set_hidden(hide: bool, app_path: &str) -> bool {
    let pads = find_dualsense();
    if pads.is_empty() {
        return false;
    }
    match build_chain(hide, &pads, app_path) {
        Ok(chain) => run_elevated("cmd.exe", &format!("/c \"{chain}\"")),
        Err(_) => false,
    }
}

/// Installs a driver package with winget (one UAC prompt). Used for HidHide and ViGEmBus.
pub fn install(winget_id: &str) -> bool {
    run_elevated(
        "winget",
        &format!(
            "install --id {winget_id} -e --accept-package-agreements --accept-source-agreements"
        ),
    )
}

/// Runs `file args` elevated and waits; false when the exit code is not 0 or the user declined the UAC prompt.
fn run_elevated(file: &str, args: &str) -> bool {
    let q = |s: &str| s.replace('\'', "''");
    let script = format!(
        "try {{ $p = Start-Process -FilePath '{}' -ArgumentList '{}' -Verb RunAs -WindowStyle Hidden -Wait -PassThru; exit $p.ExitCode }} catch {{ exit 1 }}",
        q(file),
        q(args)
    );
    Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .is_ok_and(|s| s.success())
}

fn run_cli(args: &[&str]) -> String {
    Command::new(cli_path())
        .args(args)
        .stdout(Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default()
}
