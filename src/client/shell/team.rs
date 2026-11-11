//! Fork: team roster. Members stay listed in the Agents panel while their pane is closed.

use std::{process::Stdio, sync::Mutex, time::SystemTime};

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub(super) struct TeamMember {
    pub(super) name: String,
    #[serde(default)]
    pub(super) workspace: String,
}

type Cache = Option<(String, Option<SystemTime>, Vec<TeamMember>)>;

static CACHE: Mutex<Cache> = Mutex::new(None);

/// Roster from the configured file; re-read only when its modification time changes.
pub(super) fn members(file: Option<&str>) -> Vec<TeamMember> {
    let Some(file) = file else {
        return Vec::new();
    };
    let modified = std::fs::metadata(file)
        .and_then(|metadata| metadata.modified())
        .ok();
    let mut cache = CACHE.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some((path, at, members)) = cache.as_ref() {
        if path == file && *at == modified {
            return members.clone();
        }
    }
    let members = std::fs::read_to_string(file)
        .ok()
        .and_then(|text| {
            serde_json::from_str::<Vec<TeamMember>>(text.trim_start_matches('\u{feff}')).ok()
        })
        .unwrap_or_default();
    *cache = Some((file.to_string(), modified, members.clone()));
    members
}

/// Runs `<command> <verb> <args...>` detached, without a console window.
pub(super) fn run_command(command: &[String], verb: &str, args: &[&str]) {
    let Some((program, rest)) = command.split_first() else {
        return;
    };
    let mut process = std::process::Command::new(program);
    process
        .args(rest)
        .arg(verb)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        process.creation_flags(CREATE_NO_WINDOW);
    }
    let _ = process.spawn();
}

type Selection = Option<(String, Option<String>)>;

static SELECTED: Mutex<Selection> = Mutex::new(None);

/// Selects a roster space that has no open pane (or clears the selection).
/// `focused` is the focused live workspace at that moment.
pub(super) fn select_space(label: Option<String>, focused: Option<String>) {
    let mut selected = SELECTED.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    *selected = label.map(|label| (label, focused));
}

/// The selected pane-less space; dropped as soon as the focused workspace changes.
pub(super) fn selected_space(focused: Option<&str>) -> Option<String> {
    let mut selected = SELECTED.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    match selected.as_ref() {
        Some((label, at)) if at.as_deref() == focused => Some(label.clone()),
        Some(_) => {
            *selected = None;
            None
        }
        None => None,
    }
}

pub(super) fn offline_style(
    team: &crate::config::TeamSidebarConfig,
    fallback: ratatui::style::Color,
) -> ratatui::style::Style {
    use ratatui::style::{Modifier, Style};

    let mut style = Style::default().fg(team
        .offline_fg
        .map(|color| color.ratatui())
        .unwrap_or(fallback));
    if team.offline_dim {
        style = style.add_modifier(Modifier::DIM);
    }
    if team.offline_bold {
        style = style.add_modifier(Modifier::BOLD);
    }
    style
}
