use crate::model::{ActiveSession, CollectorState, KeyboardLayout, SessionData};
use crate::platform::{open_path, process_exists, spawn_daemon, terminate_process};
use crate::report::build_html_report;
use crate::{Cli, Commands};
use anyhow::{Context, Result, bail};
use chrono::{Local, Utc};
use directories::ProjectDirs;
use rdev::{Event, EventType, listen};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use uuid::Uuid;

#[derive(Clone)]
struct AppPaths {
    sessions_dir: PathBuf,
    active_session_path: PathBuf,
}

pub fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Start { name, layout } => start_session(name, layout),
        Commands::Status => status_session(),
        Commands::Stop { open } => stop_session(open),
        Commands::Report { session_id, open } => render_existing_report(&session_id, open),
        Commands::List => list_sessions(),
        Commands::Daemon { session_id } => daemon_loop(&session_id),
    }
}

fn start_session(name: Option<String>, layout: KeyboardLayout) -> Result<()> {
    let paths = app_paths()?;
    if let Some(active) = read_active_session(&paths)? {
        if process_exists(active.pid) {
            bail!(
                "session {} is already running with pid {}. Stop it first.",
                active.session_id,
                active.pid
            );
        }
        fs::remove_file(&paths.active_session_path).ok();
    }

    let session_id = Uuid::new_v4().simple().to_string();
    let session = SessionData::new(session_id.clone(), name.clone(), layout);
    write_session(&paths, &session)?;

    let current_exe = std::env::current_exe().context("discovering current executable")?;
    let pid = spawn_daemon(&current_exe, &session_id)?;
    let active = ActiveSession {
        session_id: session_id.clone(),
        pid,
        started_at: session.started_at,
        layout,
        name,
    };
    write_json(&paths.active_session_path, &active)?;

    println!(
        "Started session {} using {} layout.",
        session_id,
        layout.as_str()
    );
    println!("Run `keystroke-visualizer stop --open` when you want the report.");
    Ok(())
}

fn status_session() -> Result<()> {
    let paths = app_paths()?;
    let Some(active) = read_active_session(&paths)? else {
        println!("No active session.");
        return Ok(());
    };

    let session = read_session(&paths, &active.session_id)?;
    let state = if process_exists(active.pid) {
        "running"
    } else {
        "stale"
    };
    println!("Session: {}", active.session_id);
    println!("State:   {}", state);
    println!("PID:     {}", active.pid);
    println!("Started: {}", format_local(session.started_at));
    println!("Updated: {}", format_local(session.last_updated_at));
    println!("Total:   {}", session.total_keypresses);
    println!("Keys:    {}", session.unique_keys);
    if let Some(error) = session.capture_error {
        println!("Error:   {}", error);
    }
    Ok(())
}

fn stop_session(open_report: bool) -> Result<()> {
    let paths = app_paths()?;
    let active = read_active_session(&paths)?.context("no active session to stop")?;

    if process_exists(active.pid) {
        terminate_process(active.pid)?;
        thread::sleep(Duration::from_millis(300));
    }

    let mut session = read_session(&paths, &active.session_id)?;
    session.stopped_at = Some(Utc::now());
    session.last_updated_at = Utc::now();

    let report_path = report_path(&paths, &session.session_id);
    fs::write(&report_path, build_html_report(&session)).context("writing report")?;
    session.report_path = Some(report_path.to_string_lossy().to_string());
    write_session(&paths, &session)?;
    fs::remove_file(&paths.active_session_path).ok();

    println!("Stopped session {}", session.session_id);
    println!("Total keypresses: {}", session.total_keypresses);
    println!("Report: {}", report_path.display());
    if open_report {
        open_path(&report_path)?;
    }
    Ok(())
}

fn render_existing_report(session_id: &str, open_report: bool) -> Result<()> {
    let paths = app_paths()?;
    let mut session = read_session(&paths, session_id)?;
    let report_path = report_path(&paths, session_id);
    fs::write(&report_path, build_html_report(&session)).context("writing report")?;
    session.report_path = Some(report_path.to_string_lossy().to_string());
    write_session(&paths, &session)?;
    println!("Report: {}", report_path.display());
    if open_report {
        open_path(&report_path)?;
    }
    Ok(())
}

fn list_sessions() -> Result<()> {
    let paths = app_paths()?;
    let mut sessions = Vec::new();
    if paths.sessions_dir.exists() {
        for entry in fs::read_dir(&paths.sessions_dir).context("reading sessions directory")? {
            let entry = entry?;
            let path = entry.path().join("session.json");
            if path.exists() {
                if let Ok(session) = read_json::<SessionData>(&path) {
                    sessions.push(session);
                }
            }
        }
    }
    sessions.sort_by_key(|session| session.started_at);
    sessions.reverse();
    if sessions.is_empty() {
        println!("No recorded sessions.");
        return Ok(());
    }
    for session in sessions {
        let finished = session
            .stopped_at
            .map(format_local)
            .unwrap_or_else(|| "running".to_string());
        println!(
            "{}  {:>8} keys  started {}  {}",
            session.session_id,
            session.total_keypresses,
            format_local(session.started_at),
            finished
        );
    }
    Ok(())
}

fn daemon_loop(session_id: &str) -> Result<()> {
    let paths = app_paths()?;
    let session = read_session(&paths, session_id)?;
    let state = Arc::new(Mutex::new(CollectorState::new(session)));

    {
        let flush_state = Arc::clone(&state);
        let flush_paths = paths.clone();
        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_secs(1));
                let snapshot = {
                    let mut guard = flush_state.lock().expect("collector state poisoned");
                    if !guard.dirty {
                        None
                    } else {
                        guard.session.unique_keys = guard.session.key_counts.len();
                        guard.dirty = false;
                        Some(guard.session.clone())
                    }
                };
                if let Some(snapshot) = snapshot {
                    let _ = write_session(&flush_paths, &snapshot);
                }
            }
        });
    }

    let capture_state = Arc::clone(&state);
    let result = listen(move |event| {
        let _ = handle_event(&capture_state, event);
    });

    if let Err(error) = result {
        let mut guard = state.lock().expect("collector state poisoned");
        guard.session.capture_error = Some(format!("{error:?}"));
        guard.session.last_updated_at = Utc::now();
        guard.session.unique_keys = guard.session.key_counts.len();
        write_session(&paths, &guard.session)?;
        bail!(
            "capture backend failed for session {}: {:?}",
            session_id,
            error
        );
    }
    Ok(())
}

fn handle_event(state: &Arc<Mutex<CollectorState>>, event: Event) -> Result<()> {
    let mut guard = state.lock().expect("collector state poisoned");
    match event.event_type {
        EventType::KeyPress(key) => {
            let key_id = format!("{key:?}");
            if guard.pressed_keys.insert(key_id.clone()) {
                let minute_bucket = Utc::now().format("%Y-%m-%d %H:%M").to_string();
                *guard.session.key_counts.entry(key_id).or_insert(0) += 1;
                *guard
                    .session
                    .minute_buckets
                    .entry(minute_bucket)
                    .or_insert(0) += 1;
                guard.session.total_keypresses += 1;
                guard.session.last_updated_at = Utc::now();
                guard.session.unique_keys = guard.session.key_counts.len();
                guard.dirty = true;
            }
        }
        EventType::KeyRelease(key) => {
            guard.pressed_keys.remove(&format!("{key:?}"));
        }
        _ => {}
    }
    Ok(())
}

fn app_paths() -> Result<AppPaths> {
    let dirs = ProjectDirs::from("local", "keystroke", "visualizer")
        .context("unable to resolve a writable application data directory")?;
    let root = dirs.data_local_dir().to_path_buf();
    let sessions_dir = root.join("sessions");
    fs::create_dir_all(&sessions_dir).context("creating data directories")?;
    Ok(AppPaths {
        sessions_dir,
        active_session_path: root.join("active-session.json"),
    })
}

fn session_dir(paths: &AppPaths, session_id: &str) -> PathBuf {
    paths.sessions_dir.join(session_id)
}

fn session_file_path(paths: &AppPaths, session_id: &str) -> PathBuf {
    session_dir(paths, session_id).join("session.json")
}

fn report_path(paths: &AppPaths, session_id: &str) -> PathBuf {
    session_dir(paths, session_id).join("report.html")
}

fn read_active_session(paths: &AppPaths) -> Result<Option<ActiveSession>> {
    if !paths.active_session_path.exists() {
        return Ok(None);
    }
    Ok(Some(read_json(&paths.active_session_path)?))
}

fn read_session(paths: &AppPaths, session_id: &str) -> Result<SessionData> {
    read_json(&session_file_path(paths, session_id))
}

fn write_session(paths: &AppPaths, session: &SessionData) -> Result<()> {
    let session_dir = session_dir(paths, &session.session_id);
    fs::create_dir_all(&session_dir).context("creating session directory")?;
    write_json(&session_file_path(paths, &session.session_id), session)
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let text = serde_json::to_string_pretty(value).context("serializing json")?;
    fs::write(path, text).with_context(|| format!("writing {}", path.display()))
}

fn format_local(dt: chrono::DateTime<Utc>) -> String {
    dt.with_timezone(&Local)
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}
