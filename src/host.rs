//! Linux host integration used by Selenite's appliance mode.
//!
//! This module deliberately delegates session policy to Linux. It does not
//! replace init, systemd, the kernel, or the user's permission model.

use std::{
    env,
    ffi::OsStr,
    fs,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use raylib::prelude::*;

use crate::{ui, BUTTON_BG, BUTTON_HOVER_BG, BUTTON_TEXT, TOOLBAR_BG, WINDOW_HEIGHT, WINDOW_WIDTH};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SystemInfo {
    pub hostname: String,
    pub operating_system: String,
    pub kernel: String,
    pub desktop: String,
    pub uptime: String,
    pub memory: String,
    pub load: String,
    pub power: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub state: char,
    pub rss_kib: u64,
    pub command: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DesktopApp {
    pub name: String,
    pub description: String,
    pub executable: Vec<String>,
    pub categories: String,
    pub terminal: bool,
    pub no_display: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceInfo {
    pub name: String,
    pub state: String,
    pub description: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HostSnapshot {
    pub system: SystemInfo,
    pub processes: Vec<ProcessInfo>,
    pub applications: Vec<DesktopApp>,
    pub services: Vec<ServiceInfo>,
    pub mounts: Vec<String>,
    pub network: Vec<String>,
}

fn command_output(command: &mut Command) -> Result<String, String> {
    let output = command.output().map_err(|error| {
        format!(
            "could not run {}: {error}",
            command.get_program().to_string_lossy()
        )
    })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(if stderr.is_empty() {
            format!(
                "{} exited with {}",
                command.get_program().to_string_lossy(),
                output.status
            )
        } else {
            stderr
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn first_line(path: impl AsRef<Path>) -> Option<String> {
    fs::read_to_string(path)
        .ok()?
        .lines()
        .next()
        .map(str::to_owned)
}

fn os_release_value(name: &str) -> Option<String> {
    for line in fs::read_to_string("/etc/os-release").ok()?.lines() {
        if let Some(value) = line.strip_prefix(&format!("{name}=")) {
            return Some(value.trim_matches(['"', '\'']).to_owned());
        }
    }
    None
}

fn parse_kib(path: impl AsRef<Path>, field: &str) -> u64 {
    fs::read_to_string(path)
        .ok()
        .and_then(|contents| {
            contents
                .lines()
                .find(|line| line.starts_with(field))
                .and_then(|line| line.split_whitespace().nth(1))
                .and_then(|value| value.parse().ok())
        })
        .unwrap_or(0)
}

fn human_bytes(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    const GIB: f64 = MIB * 1024.0;
    match bytes as f64 {
        value if value >= GIB => format!("{:.1} GiB", value / GIB),
        value if value >= MIB => format!("{:.1} MiB", value / MIB),
        value if value >= KIB => format!("{:.1} KiB", value / KIB),
        value => format!("{value:.0} B"),
    }
}

fn uptime() -> String {
    let seconds = fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|value| value.split_whitespace().next()?.parse::<f64>().ok())
        .unwrap_or(0.0) as u64;
    let days = seconds / 86_400;
    let hours = (seconds % 86_400) / 3_600;
    let minutes = (seconds % 3_600) / 60;
    if days > 0 {
        format!("{days}d {hours}h {minutes}m")
    } else {
        format!("{hours}h {minutes}m")
    }
}

fn battery_status() -> String {
    let Ok(power_supplies) = fs::read_dir("/sys/class/power_supply") else {
        return "AC / battery status unavailable".into();
    };
    let mut readings = Vec::new();
    for entry in power_supplies.flatten() {
        let path = entry.path();
        if first_line(path.join("type")).as_deref() != Some("Battery") {
            continue;
        }
        let capacity = first_line(path.join("capacity")).unwrap_or_else(|| "?".into());
        let state = first_line(path.join("status")).unwrap_or_else(|| "unknown".into());
        readings.push(format!("{capacity}% ({state})"));
    }
    if readings.is_empty() {
        "AC powered".into()
    } else {
        readings.join(", ")
    }
}

pub fn system_info() -> SystemInfo {
    let total = parse_kib("/proc/meminfo", "MemTotal:") * 1024;
    let available = parse_kib("/proc/meminfo", "MemAvailable:") * 1024;
    SystemInfo {
        hostname: first_line("/proc/sys/kernel/hostname").unwrap_or_else(|| "localhost".into()),
        operating_system: os_release_value("PRETTY_NAME").unwrap_or_else(|| "Linux".into()),
        kernel: first_line("/proc/sys/kernel/osrelease").unwrap_or_else(|| "unknown".into()),
        desktop: env::var("XDG_CURRENT_DESKTOP")
            .or_else(|_| env::var("DESKTOP_SESSION"))
            .unwrap_or_else(|_| {
                if env::var_os("WAYLAND_DISPLAY").is_some() {
                    "Wayland appliance".into()
                } else {
                    "Linux session".into()
                }
            }),
        uptime: uptime(),
        memory: format!(
            "{} / {}",
            human_bytes(total.saturating_sub(available)),
            human_bytes(total)
        ),
        load: first_line("/proc/loadavg")
            .map(|line| {
                line.split_whitespace()
                    .take(3)
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .unwrap_or_else(|| "unavailable".into()),
        power: battery_status(),
    }
}

pub fn processes() -> Vec<ProcessInfo> {
    let Ok(entries) = fs::read_dir("/proc") else {
        return Vec::new();
    };
    let current_uid = unsafe { libc::geteuid() };
    let mut result = entries
        .flatten()
        .filter_map(|entry| {
            let pid = entry.file_name().to_str()?.parse::<u32>().ok()?;
            let metadata = fs::metadata(entry.path()).ok()?;
            if metadata.uid() != current_uid {
                return None;
            }
            let status = fs::read_to_string(entry.path().join("status")).ok()?;
            let value = |name: &str| {
                status
                    .lines()
                    .find(|line| line.starts_with(name))
                    .and_then(|line| line.split_once(':'))
                    .map(|(_, value)| value.trim().to_owned())
            };
            let rss_kib = value("VmRSS:")
                .and_then(|text| text.split_whitespace().next()?.parse().ok())
                .unwrap_or(0);
            let state = value("State:")?.chars().next()?;
            let name = value("Name:").unwrap_or_else(|| "?".into());
            let command = fs::read(entry.path().join("cmdline"))
                .ok()
                .map(|bytes| String::from_utf8_lossy(&bytes).replace('\0', " "))
                .unwrap_or_default();
            Some(ProcessInfo {
                pid,
                name,
                state,
                rss_kib,
                command: command.trim().to_owned(),
            })
        })
        .collect::<Vec<_>>();
    result.sort_by(|left, right| {
        right
            .rss_kib
            .cmp(&left.rss_kib)
            .then(left.pid.cmp(&right.pid))
    });
    result
}

fn desktop_dirs() -> Vec<PathBuf> {
    let home = PathBuf::from(env::var_os("HOME").unwrap_or_else(|| ".".into()));
    let mut roots = vec![env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".local/share"))];
    roots.extend(
        env::var("XDG_DATA_DIRS")
            .unwrap_or_else(|_| "/usr/local/share:/usr/share".into())
            .split(':')
            .filter(|path| !path.is_empty())
            .map(PathBuf::from),
    );
    roots
        .into_iter()
        .map(|root| root.join("applications"))
        .collect()
}

fn parse_exec(value: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut token = String::new();
    let mut quote = None;
    let mut escaped = false;
    for character in value.chars() {
        if escaped {
            token.push(character);
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if quote == Some(character) {
            quote = None;
        } else if character == '"' || character == '\'' {
            quote = Some(character);
        } else if character.is_whitespace() && quote.is_none() {
            if !token.is_empty() {
                result.push(std::mem::take(&mut token));
            }
        } else {
            token.push(character);
        }
    }
    if !token.is_empty() {
        result.push(token);
    }
    result
}

fn unescape_desktop(value: &str) -> String {
    let mut output = String::new();
    let mut escaped = false;
    for character in value.chars() {
        if escaped {
            output.push(match character {
                'n' => '\n',
                'r' => '\r',
                's' => ' ',
                't' => '\t',
                other => other,
            });
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else {
            output.push(character);
        }
    }
    if escaped {
        output.push('\\');
    }
    output
}

fn desktop_entry(path: &Path) -> Option<DesktopApp> {
    let text = fs::read_to_string(path).ok()?;
    let mut section = false;
    let mut values = std::collections::HashMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') {
            if section {
                break;
            }
            section = line == "[Desktop Entry]";
            continue;
        }
        if section {
            if let Some((key, value)) = line.split_once('=') {
                values.insert(key.to_owned(), unescape_desktop(value));
            }
        }
    }
    let executable = parse_exec(values.get("Exec")?);
    if executable.is_empty() || values.get("Type").map(String::as_str) != Some("Application") {
        return None;
    }
    let no_display = values
        .get("NoDisplay")
        .map(|value| value == "true")
        .unwrap_or(false);
    Some(DesktopApp {
        name: values.get("Name").cloned().unwrap_or_else(|| {
            path.file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned()
        }),
        description: values.get("Comment").cloned().unwrap_or_default(),
        executable,
        categories: values.get("Categories").cloned().unwrap_or_default(),
        terminal: values
            .get("Terminal")
            .map(|value| value == "true")
            .unwrap_or(false),
        no_display,
    })
}

pub fn applications() -> Vec<DesktopApp> {
    let mut by_name = std::collections::HashMap::<String, DesktopApp>::new();
    for directory in desktop_dirs() {
        let Ok(entries) = fs::read_dir(directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension() != Some(OsStr::new("desktop")) {
                continue;
            }
            if let Some(app) = desktop_entry(&path) {
                by_name.entry(app.name.to_lowercase()).or_insert(app);
            }
        }
    }
    let mut apps = by_name
        .into_values()
        .filter(|app| !app.no_display)
        .collect::<Vec<_>>();
    apps.sort_by_key(|app| app.name.to_lowercase());
    apps
}

fn command_exists(program: &str) -> bool {
    env::var_os("PATH")
        .map(|paths| env::split_paths(&paths).any(|path| path.join(program).is_file()))
        .unwrap_or(false)
}

fn exec_app_arguments(app: &DesktopApp) -> Vec<String> {
    app.executable
        .iter()
        .filter(|argument| !argument.contains('%'))
        .cloned()
        .collect()
}

pub fn launch_app(app: &DesktopApp) -> Result<(), String> {
    let arguments = exec_app_arguments(app);
    let (program, arguments) = arguments
        .split_first()
        .ok_or("Desktop entry has no executable")?;
    let mut command = if app.terminal {
        let exe = env::current_exe().map_err(|error| error.to_string())?;
        let mut command = Command::new(exe);
        command.arg("--terminal");
        command.arg(program);
        command.args(arguments);
        command
    } else {
        let mut command = Command::new(program);
        command.args(arguments);
        command
    };
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("could not launch {}: {error}", app.name))?;
    notify("Selenite Host", &format!("Launched {}", app.name));
    Ok(())
}

pub fn open_target(target: &Path) -> Result<(), String> {
    let target = target.to_string_lossy();
    let opener = if command_exists("xdg-open") {
        "xdg-open"
    } else {
        "gio"
    };
    let mut command = Command::new(opener);
    if opener == "gio" {
        command.arg("open");
    }
    command
        .arg(target.as_ref())
        .spawn()
        .map_err(|error| error.to_string())?;
    Ok(())
}

pub fn notify(summary: &str, body: &str) {
    let result = Command::new("notify-send")
        .arg("--app-name=Selenite")
        .arg(summary)
        .arg(body)
        .spawn();
    if let Err(error) = result {
        if command_exists("gdbus") {
            let result = Command::new("gdbus")
                .args([
                    "call",
                    "--session",
                    "--dest=org.freedesktop.Notifications",
                    "--object-path=/org/freedesktop/Notifications",
                    "--method=org.freedesktop.Notifications.Notify",
                    "Selenite",
                    "0",
                    "applications-graphics",
                    summary,
                    body,
                    "[]",
                    "{}",
                    "5000",
                ])
                .spawn();
            if let Err(fallback_error) = result {
                eprintln!(
                    "Selenite notification failed ({error}); D-Bus fallback failed: {fallback_error}"
                );
            }
        } else {
            eprintln!("Selenite notification failed: {error}");
        }
    }
}

pub fn dbus_names() -> Result<Vec<String>, String> {
    let output = command_output(Command::new("busctl").args(["--user", "--no-pager", "list"]))?;
    Ok(output.lines().skip(1).map(str::to_owned).collect())
}

pub fn services() -> Vec<ServiceInfo> {
    let Ok(output) = Command::new("systemctl")
        .args([
            "--user",
            "list-units",
            "--type=service",
            "--all",
            "--no-legend",
            "--no-pager",
            "--plain",
        ])
        .output()
    else {
        return Vec::new();
    };
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let fields = line.split_whitespace().collect::<Vec<_>>();
            if fields.len() < 4 {
                return None;
            }
            Some(ServiceInfo {
                name: fields[0].to_owned(),
                state: format!("{} / {}", fields[2], fields[3]),
                description: fields[4..].join(" "),
            })
        })
        .collect()
}

pub fn toggle_service(service: &str) -> Result<(), String> {
    if !service
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || "@_.-:".contains(character))
    {
        return Err("Invalid service name".into());
    }
    let active = command_output(Command::new("systemctl").args(["--user", "is-active", service]))
        .unwrap_or_default();
    let action = if active == "active" || active == "activating" {
        "stop"
    } else {
        "start"
    };
    command_output(Command::new("systemctl").args(["--user", action, service]))?;
    notify(
        "Selenite service",
        &format!("{action} requested for {service}"),
    );
    Ok(())
}

pub fn mounts() -> Vec<String> {
    Command::new("findmnt")
        .args([
            "--real",
            "--noheadings",
            "--raw",
            "--output",
            "TARGET,SOURCE,FSTYPE,SIZE,AVAIL",
        ])
        .output()
        .ok()
        .map(|output| {
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(|line| line.to_owned())
                .collect()
        })
        .unwrap_or_else(|| Vec::new())
}

pub fn network() -> Vec<String> {
    Command::new("ip")
        .args(["-brief", "address"])
        .output()
        .ok()
        .map(|output| {
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(|line| line.to_owned())
                .collect()
        })
        .unwrap_or_else(|| {
            fs::read_dir("/sys/class/net")
                .ok()
                .into_iter()
                .flatten()
                .flatten()
                .filter_map(|entry| entry.file_name().into_string().ok())
                .collect()
        })
}

pub fn terminate_process(pid: u32) -> Result<(), String> {
    if pid == std::process::id() {
        return Err("Selenite will not terminate its own host window".into());
    }
    let result = unsafe { libc::kill(pid as i32, libc::SIGTERM) };
    if result == 0 {
        notify(
            "Selenite Host",
            &format!("Termination requested for PID {pid}"),
        );
        Ok(())
    } else {
        Err(std::io::Error::last_os_error().to_string())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PowerAction {
    Lock,
    Logout,
    Suspend,
    Hibernate,
    Reboot,
    Poweroff,
}

impl PowerAction {
    pub const ALL: [Self; 6] = [
        Self::Lock,
        Self::Logout,
        Self::Suspend,
        Self::Hibernate,
        Self::Reboot,
        Self::Poweroff,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Lock => "Lock session",
            Self::Logout => "Log out",
            Self::Suspend => "Suspend",
            Self::Hibernate => "Hibernate",
            Self::Reboot => "Restart",
            Self::Poweroff => "Power off",
        }
    }
}

pub fn power(action: PowerAction) -> Result<(), String> {
    let mut command = match action {
        PowerAction::Lock => {
            if command_exists("loginctl") {
                let mut command = Command::new("loginctl");
                command.arg("lock-session");
                command
            } else {
                return Err("No Linux session locker is available".into());
            }
        }
        PowerAction::Logout => {
            let session = env::var("XDG_SESSION_ID").map_err(|_| {
                "XDG_SESSION_ID is unavailable; refusing to guess a session".to_owned()
            })?;
            let mut command = Command::new("loginctl");
            command.args(["terminate-session", &session]);
            command
        }
        PowerAction::Suspend => {
            let mut command = Command::new("systemctl");
            command.arg("suspend");
            command
        }
        PowerAction::Hibernate => {
            let mut command = Command::new("systemctl");
            command.arg("hibernate");
            command
        }
        PowerAction::Reboot => {
            let mut command = Command::new("systemctl");
            command.arg("reboot");
            command
        }
        PowerAction::Poweroff => {
            let mut command = Command::new("systemctl");
            command.arg("poweroff");
            command
        }
    };
    command_output(&mut command).map(|_| ())
}

pub fn snapshot() -> HostSnapshot {
    HostSnapshot {
        system: system_info(),
        processes: processes(),
        applications: applications(),
        services: services(),
        mounts: mounts(),
        network: network(),
    }
}

pub fn print_snapshot() -> Result<(), String> {
    let snapshot = snapshot();
    println!("SYSTEM");
    println!(
        "{} — {}",
        snapshot.system.hostname, snapshot.system.operating_system
    );
    println!(
        "Linux {} | {} | uptime {}",
        snapshot.system.kernel, snapshot.system.desktop, snapshot.system.uptime
    );
    println!(
        "Memory {} | load {} | {}",
        snapshot.system.memory, snapshot.system.load, snapshot.system.power
    );
    println!("\nAPPLICATIONS ({})", snapshot.applications.len());
    for app in snapshot.applications {
        println!("{}", app.name);
    }
    println!("\nPROCESSES ({})", snapshot.processes.len());
    for process in snapshot.processes {
        println!(
            "{:>7} {:>10} {:<24} {}",
            process.pid,
            human_bytes(process.rss_kib * 1024),
            process.name,
            process.command
        );
    }
    println!("\nNETWORK");
    for line in snapshot.network {
        println!("{line}");
    }
    println!("\nMOUNTS");
    for line in snapshot.mounts {
        println!("{line}");
    }
    println!("\nUSER SERVICES");
    for service in snapshot.services {
        println!(
            "{} [{}] {}",
            service.name, service.state, service.description
        );
    }
    match dbus_names() {
        Ok(names) => println!("\nDBUS CONNECTIONS ({})", names.len()),
        Err(error) => println!("\nDBUS unavailable: {error}"),
    }
    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Overview,
    Applications,
    Processes,
    Devices,
    Network,
    Services,
    Power,
}

impl Page {
    const ALL: [Self; 7] = [
        Self::Overview,
        Self::Applications,
        Self::Processes,
        Self::Devices,
        Self::Network,
        Self::Services,
        Self::Power,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Applications => "Applications",
            Self::Processes => "Processes",
            Self::Devices => "Devices",
            Self::Network => "Network",
            Self::Services => "Services",
            Self::Power => "Power",
        }
    }
}

pub fn run_window(appliance: bool) -> Result<(), String> {
    let (mut rl, thread) = raylib::init()
        .size(WINDOW_WIDTH, WINDOW_HEIGHT)
        .title(if appliance {
            "Selenite Appliance"
        } else {
            "Selenite Linux Host"
        })
        .resizable()
        .build();
    rl.set_target_fps(30);
    rl.set_exit_key(None);
    rl.set_window_min_size(640, 420);
    ui::fit_window_to_display(&mut rl, WINDOW_WIDTH, WINDOW_HEIGHT);
    if appliance {
        rl.maximize_window();
    }
    ui::set_user_scale(crate::settings::saved_ui_scale());
    ui::load_font(&thread);
    let mut snapshot = snapshot();
    let mut page = Page::Overview;
    let mut selected = 0usize;
    let mut scroll = 0usize;
    let mut query = String::new();
    let mut message = String::new();
    let mut confirmation: Option<PowerAction> = None;
    let mut refresh_at = 0.0;

    while !rl.window_should_close() {
        if rl.get_time() >= refresh_at {
            snapshot = self::snapshot();
            refresh_at = rl.get_time() + 3.0;
        }
        if rl.is_key_pressed(KeyboardKey::KEY_ESCAPE) {
            if confirmation.take().is_some() {
                message = "Cancelled".into();
            } else {
                break;
            }
        }
        for (index, _) in Page::ALL.iter().enumerate() {
            let key = match index {
                0 => KeyboardKey::KEY_ONE,
                1 => KeyboardKey::KEY_TWO,
                2 => KeyboardKey::KEY_THREE,
                3 => KeyboardKey::KEY_FOUR,
                4 => KeyboardKey::KEY_FIVE,
                5 => KeyboardKey::KEY_SIX,
                _ => KeyboardKey::KEY_SEVEN,
            };
            if rl.is_key_pressed(key) {
                page = Page::ALL[index];
                selected = 0;
                scroll = 0;
                query.clear();
                confirmation = None;
            }
        }
        if rl.is_key_pressed(KeyboardKey::KEY_F5) {
            snapshot = self::snapshot();
            message = "Host information refreshed".into();
        }
        if rl.is_key_pressed(KeyboardKey::KEY_O) {
            message = match env::current_exe()
                .map_err(|error| error.to_string())
                .and_then(|program| {
                    Command::new(program)
                        .spawn()
                        .map_err(|error| format!("could not open Selenite workspace: {error}"))
                }) {
                Ok(_) => "Selenite workspace opened".into(),
                Err(error) => error,
            };
        }
        if rl.is_key_pressed(KeyboardKey::KEY_T) {
            message = match env::current_exe()
                .map_err(|error| error.to_string())
                .and_then(|program| {
                    Command::new(program)
                        .arg("--terminal-window")
                        .spawn()
                        .map_err(|error| format!("could not open Selenite Terminal: {error}"))
                }) {
                Ok(_) => "Selenite Terminal opened".into(),
                Err(error) => error,
            };
        }
        if rl.is_key_pressed(KeyboardKey::KEY_DOWN) {
            selected = selected.saturating_add(1);
        }
        if rl.is_key_pressed(KeyboardKey::KEY_UP) {
            selected = selected.saturating_sub(1);
        }
        if page == Page::Applications {
            let previous = query.clone();
            while let Some(character) = rl.get_char_pressed() {
                if !character.is_control() && query.len() < 128 {
                    query.push(character);
                }
            }
            if rl.is_key_pressed(KeyboardKey::KEY_BACKSPACE) {
                query.pop();
            }
            if query != previous {
                selected = 0;
                scroll = 0;
            }
        }
        let applications = snapshot
            .applications
            .iter()
            .filter(|app| {
                let text =
                    format!("{} {} {}", app.name, app.description, app.categories).to_lowercase();
                query
                    .to_lowercase()
                    .split_whitespace()
                    .all(|word| text.contains(word))
            })
            .collect::<Vec<_>>();
        let count = match page {
            Page::Applications => applications.len(),
            Page::Processes => snapshot.processes.len(),
            Page::Services => snapshot.services.len(),
            Page::Power => PowerAction::ALL.len(),
            _ => 0,
        };
        selected = selected.min(count.saturating_sub(1));
        let visible = ((rl.get_screen_height() - 170) / 34).max(1) as usize;
        if selected >= scroll + visible {
            scroll = selected + 1 - visible;
        }
        if selected < scroll {
            scroll = selected;
        }

        if rl.is_key_pressed(KeyboardKey::KEY_ENTER) {
            let result = match page {
                Page::Applications => applications.get(selected).map(|app| launch_app(app)),
                Page::Services => snapshot
                    .services
                    .get(selected)
                    .map(|service| toggle_service(&service.name)),
                Page::Power => {
                    if let Some(action) = confirmation {
                        confirmation = None;
                        Some(power(action))
                    } else {
                        PowerAction::ALL.get(selected).copied().map(|action| {
                            confirmation = Some(action);
                            Ok(())
                        })
                    }
                }
                _ => None,
            };
            if let Some(result) = result {
                match result {
                    Ok(()) if confirmation.is_some() => {
                        message = format!(
                            "Press Enter again to {}",
                            confirmation.unwrap().label().to_lowercase()
                        )
                    }
                    Ok(()) => message = "Action completed".into(),
                    Err(error) => message = error,
                }
            }
        }
        if page == Page::Processes
            && (rl.is_key_pressed(KeyboardKey::KEY_DELETE) || rl.is_key_pressed(KeyboardKey::KEY_X))
        {
            if let Some(process) = snapshot.processes.get(selected) {
                match terminate_process(process.pid) {
                    Ok(()) => message = format!("Termination requested for {}", process.name),
                    Err(error) => message = error,
                }
            }
        }

        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::new(18, 20, 25, 255));
        let width = d.get_screen_width();
        let scale = ui::scale(width, d.get_screen_height());
        let tab_height = 44.0 * scale;
        let tab_width = width as f32 / Page::ALL.len() as f32;
        for (index, candidate) in Page::ALL.iter().enumerate() {
            let active = *candidate == page;
            let rect = Rectangle::new(
                index as f32 * tab_width + 2.0,
                2.0,
                tab_width - 4.0,
                tab_height,
            );
            d.draw_rectangle_rec(rect, if active { BUTTON_HOVER_BG } else { TOOLBAR_BG });
            ui::draw_text_in(
                &mut d,
                &format!("{} {}", index + 1, candidate.label()),
                rect,
                8.0,
                17.0 * scale,
                BUTTON_TEXT,
            );
        }
        let y = tab_height + 10.0;
        let bounds = Rectangle::new(
            12.0,
            y,
            width as f32 - 24.0,
            d.get_screen_height() as f32 - y - 70.0,
        );
        let mut lines = Vec::new();
        match page {
            Page::Overview => {
                lines.push(format!(
                    "{} — {}",
                    snapshot.system.hostname, snapshot.system.operating_system
                ));
                lines.push(format!(
                    "Linux {} | {}",
                    snapshot.system.kernel, snapshot.system.desktop
                ));
                lines.push(format!(
                    "Uptime: {} | Load: {}",
                    snapshot.system.uptime, snapshot.system.load
                ));
                lines.push(format!(
                    "Memory: {} | Power: {}",
                    snapshot.system.memory, snapshot.system.power
                ));
                lines.push(format!(
                    "{} applications | {} user processes | {} mounts | {} services",
                    snapshot.applications.len(),
                    snapshot.processes.len(),
                    snapshot.mounts.len(),
                    snapshot.services.len()
                ));
                lines.push(
                    "Wayland-aware: host policy remains owned by the compositor, DBus and systemd."
                        .into(),
                );
                lines.push("F5 refresh | O workspace | T terminal | 1-7 pages | Esc close".into());
            }
            Page::Applications => {
                lines.push(format!(
                    "Search: {}",
                    if query.is_empty() {
                        "type to filter applications"
                    } else {
                        &query
                    }
                ));
                lines.extend(
                    applications
                        .iter()
                        .skip(scroll)
                        .take(visible)
                        .map(|app| format!("{} — {}", app.name, app.description)),
                );
            }
            Page::Processes => {
                lines.push("Delete/X terminates the selected process with SIGTERM".into());
                lines.extend(
                    snapshot
                        .processes
                        .iter()
                        .skip(scroll)
                        .take(visible)
                        .map(|process| {
                            format!(
                                "{:>7} {:>9} {} ({}) {}",
                                process.pid,
                                human_bytes(process.rss_kib * 1024),
                                process.name,
                                process.state,
                                process.command
                            )
                        }),
                );
            }
            Page::Devices => lines.extend(snapshot.mounts.iter().cloned()),
            Page::Network => lines.extend(snapshot.network.iter().cloned()),
            Page::Services => {
                lines.push("Enter starts/stops the selected user service".into());
                lines.extend(
                    snapshot
                        .services
                        .iter()
                        .skip(scroll)
                        .take(visible)
                        .map(|service| {
                            format!(
                                "{} [{}] {}",
                                service.name, service.state, service.description
                            )
                        }),
                );
            }
            Page::Power => {
                lines.push("Power actions require Enter twice; Escape cancels".into());
                lines.extend(
                    PowerAction::ALL
                        .iter()
                        .map(|action| action.label().to_owned()),
                );
                if let Some(action) = confirmation {
                    lines.push(format!("Confirm: {}", action.label()));
                }
            }
        }
        if lines.is_empty() {
            lines.push("No items reported by the Linux host".into());
        }
        let font = (19.0 * scale).clamp(14.0, 24.0);
        let row = font + 10.0;
        for (index, line) in lines.iter().enumerate() {
            let rect = Rectangle::new(
                bounds.x + 8.0,
                bounds.y + index as f32 * row,
                bounds.width - 16.0,
                row,
            );
            let selectable = matches!(
                page,
                Page::Applications | Page::Processes | Page::Services | Page::Power
            );
            let data_index = index.saturating_sub(usize::from(matches!(
                page,
                Page::Processes | Page::Services | Page::Applications | Page::Power
            )));
            let has_header = matches!(
                page,
                Page::Processes | Page::Services | Page::Applications | Page::Power
            );
            if selectable
                && (!has_header || index > 0)
                && data_index == selected
                && rect.y < bounds.y + bounds.height
            {
                d.draw_rectangle_rec(rect, Color::new(45, 66, 80, 255));
            }
            ui::draw_text_in(
                &mut d,
                &ui::fit(line, font, rect.width),
                rect,
                8.0,
                font,
                Color::RAYWHITE,
            );
        }
        if !message.is_empty() {
            let rect = Rectangle::new(
                8.0,
                d.get_screen_height() as f32 - 48.0,
                width as f32 - 16.0,
                38.0,
            );
            d.draw_rectangle_rec(rect, BUTTON_BG);
            ui::draw_text_in(
                &mut d,
                &ui::fit(&message, font, rect.width - 16.0),
                rect,
                8.0,
                font,
                BUTTON_TEXT,
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_exec_parser_preserves_quoted_arguments() {
        assert_eq!(
            parse_exec(r#"program "two words" --flag='also two' escaped\ space"#),
            ["program", "two words", "--flag=also two", "escaped space"]
        );
    }

    #[test]
    fn field_codes_are_not_passed_as_arguments() {
        let app = DesktopApp {
            name: "Test".into(),
            description: String::new(),
            executable: ["program".into(), "%f".into(), "--fixed".into()].into(),
            categories: String::new(),
            terminal: false,
            no_display: false,
        };
        assert_eq!(exec_app_arguments(&app), ["program", "--fixed"]);
    }

    #[test]
    fn unsafe_service_names_are_rejected() {
        assert!(toggle_service("name;reboot").is_err());
    }
}
