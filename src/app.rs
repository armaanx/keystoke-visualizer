use crate::collector::{
    DaemonContext, handle_event, load_collector_state, send_control_command, start_control_server,
    start_flush_worker,
};
use crate::assets::load_asset;
use crate::model::{KeyboardLayout, SessionRecord, SessionSnapshot, SessionStatus};
use crate::platform::{process_exists, spawn_daemon};
use crate::storage::{AppPaths, Repository, app_paths};
use crate::web::serve_report_ui;
use crate::{Cli, Commands};
use anyhow::{Context, Result, bail};
use chrono::{Local, Utc};
use rdev::listen;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use uuid::Uuid;

pub fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Start { name, layout } => start_session(name, layout),
        Commands::Status => status_session(),
        Commands::Stop { open } => stop_session(open),
        Commands::Report { session_id, open } => render_existing_report(&session_id, open),
        Commands::List => list_sessions(),
        Commands::Doctor => doctor(),
        Commands::Daemon {
            session_id,
            control_token,
        } => daemon_loop(&session_id, &control_token),
    }
}

fn start_session(name: Option<String>, layout: KeyboardLayout) -> Result<()> {
    let paths = app_paths()?;
    let mut repo = Repository::open(&paths)?;
    reconcile_active_session(&mut repo)?;
    if let Some(active) = repo.active_session()? {
        bail!(
            "session {} is already running with pid {}. Stop it first.",
            active.snapshot.session_id,
            active
                .runtime
                .map(|runtime| runtime.pid)
                .unwrap_or_default()
        );
    }

    let session_id = Uuid::new_v4().simple().to_string();
    let snapshot = SessionSnapshot::new(session_id.clone(), name, layout);
    repo.create_session(&snapshot)?;

    let control_token = Uuid::new_v4().to_string();
    let current_exe = std::env::current_exe().context("discovering current executable")?;
    let pid = spawn_daemon(&current_exe, &session_id, &control_token)?;
    repo.attach_runtime(&session_id, pid, &control_token, Utc::now())?;

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
    let mut repo = Repository::open(&paths)?;
    reconcile_active_session(&mut repo)?;

    if let Some(active) = repo.active_session()? {
        print_session_status(&active);
        return Ok(());
    }

    let recent = repo.recent_sessions()?;
    if let Some(last) = recent.first() {
        println!("No active session.");
        println!();
        print_session_summary(last);
    } else {
        println!("No recorded sessions.");
    }
    Ok(())
}

fn stop_session(open_report: bool) -> Result<()> {
    let paths = app_paths()?;
    let mut repo = Repository::open(&paths)?;
    reconcile_active_session(&mut repo)?;

    let target = match repo.active_session()? {
        Some(active) => StopTarget::Running(active),
        None => match recoverable_session(&repo)? {
            Some(session) => StopTarget::Recoverable(session),
            None => bail!("no active or recoverable session to stop"),
        },
    };

    let session_id = match target {
        StopTarget::Running(record) => stop_running_session(&paths, record)?,
        StopTarget::Recoverable(record) => {
            repo.mark_session_stopped(&record.snapshot.session_id, Utc::now(), false)?;
            record.snapshot.session_id
        }
    };

    let repo = Repository::open(&paths)?;
    let final_record = repo.load_session(&session_id)?;
    println!("Stopped session {}", session_id);
    println!(
        "Total keypresses: {}",
        final_record.snapshot.total_keypresses
    );
    println!("Outcome: {}", final_record.snapshot.status.as_str());
    println!(
        "Clean shutdown: {}",
        yes_no(final_record.snapshot.clean_shutdown)
    );
    if open_report {
        serve_report_ui(paths, &session_id, true)?;
    } else {
        println!("Run `keystroke-visualizer report {session_id} --open` to view the UI.");
    }
    Ok(())
}

fn render_existing_report(session_id: &str, open_report: bool) -> Result<()> {
    let paths = app_paths()?;
    Repository::open(&paths)?.load_session_snapshot(session_id)?;
    serve_report_ui(paths, session_id, open_report)
}

fn list_sessions() -> Result<()> {
    let paths = app_paths()?;
    let mut repo = Repository::open(&paths)?;
    reconcile_active_session(&mut repo)?;
    let sessions = repo.recent_sessions()?;
    if sessions.is_empty() {
        println!("No recorded sessions.");
        return Ok(());
    }

    for session in sessions {
        let finished = session
            .snapshot
            .stopped_at
            .map(format_local)
            .unwrap_or_else(|| "n/a".to_string());
        println!(
            "{}  {:>8} keys  {:>11}  started {}  stopped {}",
            session.snapshot.session_id,
            session.snapshot.total_keypresses,
            session.snapshot.status.as_str(),
            format_local(session.snapshot.started_at),
            finished
        );
    }
    Ok(())
}

fn doctor() -> Result<()> {
    let paths = app_paths()?;
    let mut repo = Repository::open(&paths)?;
    reconcile_active_session(&mut repo)?;

    println!("Database: {}", paths.db_path.display());
    println!("App data root: {}", paths.root.display());
    println!("Schema version: {}", repo.schema_version()?);
    println!(
        "Sessions directory: {} ({})",
        paths.sessions_dir.display(),
        yes_no(paths.sessions_dir.exists())
    );
    println!(
        "Executable available: {}",
        yes_no(std::env::current_exe().is_ok())
    );
    println!("Capture backend: global keyboard hook via rdev");
    println!(
        "Frontend assets: {}",
        yes_no(load_asset("index.html").is_some())
    );
    println!(
        "Platform support: {}",
        if cfg!(windows) {
            "windows-primary"
        } else {
            "experimental"
        }
    );
    if let Some(active) = repo.active_session()? {
        let runtime = active.runtime.context("active session runtime missing")?;
        println!("Active session: {}", active.snapshot.session_id);
        println!("Active pid: {}", runtime.pid);
        println!(
            "Daemon started: {}",
            format_local(runtime.daemon_started_at)
        );
        println!("Daemon alive: {}", yes_no(process_exists(runtime.pid)));
        println!(
            "Control channel: {}",
            runtime
                .control_port
                .map(|port| port.to_string())
                .unwrap_or_else(|| "pending".to_string())
        );
    } else {
        println!("Active session: none");
    }
    Ok(())
}

fn daemon_loop(session_id: &str, control_token: &str) -> Result<()> {
    let paths = app_paths()?;
    let mut repo = Repository::open(&paths)?;
    let record = repo.load_session(session_id)?;
    let flush_seq = record
        .runtime
        .as_ref()
        .map(|runtime| runtime.flush_seq)
        .unwrap_or(0);
    let state = Arc::new(Mutex::new(load_collector_state(
        &record.snapshot,
        flush_seq,
    )));

    let port = start_control_server(
        paths.clone(),
        Arc::clone(&state),
        DaemonContext {
            session_id: session_id.to_string(),
            control_token: control_token.to_string(),
        },
    )?;
    repo.set_runtime_port(session_id, port)?;
    let _flush_worker = start_flush_worker(paths.clone(), Arc::clone(&state));

    let capture_state = Arc::clone(&state);
    let result = listen(move |event| {
        handle_event(&capture_state, event);
    });

    if let Err(error) = result {
        {
            let mut guard = state.lock().expect("collector state poisoned");
            guard.capture_error = Some(format!("{error:?}"));
            guard.dirty = true;
        }
        let mut repo = Repository::open(&paths)?;
        let (flush_seq, flush) = {
            let mut guard = state.lock().expect("collector state poisoned");
            guard.flush_seq += 1;
            guard.dirty = false;
            (guard.flush_seq, guard.snapshot())
        };
        repo.flush_session(session_id, flush_seq, &flush)?;
        repo.mark_session_failed(session_id, &format!("{error:?}"))?;
        bail!(
            "capture backend failed for session {}: {:?}",
            session_id,
            error
        );
    }
    Ok(())
}

enum StopTarget {
    Running(SessionRecord),
    Recoverable(SessionRecord),
}

fn stop_running_session(paths: &AppPaths, record: SessionRecord) -> Result<String> {
    let runtime = record.runtime.context("active session runtime missing")?;
    if let Some(port) = runtime.control_port {
        match send_control_command(port, &runtime.control_token, "stop") {
            Ok(_) => {
                for _ in 0..10 {
                    if !process_exists(runtime.pid) {
                        break;
                    }
                    thread::sleep(Duration::from_millis(100));
                }
                return Ok(record.snapshot.session_id);
            }
            Err(error) => {
                if !process_exists(runtime.pid) {
                    let mut repo = Repository::open(paths)?;
                    repo.mark_session_interrupted(
                        &record.snapshot.session_id,
                        Some(&format!("daemon exited during stop: {error}")),
                    )?;
                    repo.mark_session_stopped(&record.snapshot.session_id, Utc::now(), false)?;
                    return Ok(record.snapshot.session_id);
                }
                bail!("graceful shutdown failed: {error}");
            }
        }
    }

    if process_exists(runtime.pid) {
        bail!("daemon control channel is not ready yet; retry stop in a moment");
    }

    let mut repo = Repository::open(paths)?;
    repo.mark_session_interrupted(
        &record.snapshot.session_id,
        Some("daemon exited unexpectedly"),
    )?;
    repo.mark_session_stopped(&record.snapshot.session_id, Utc::now(), false)?;
    Ok(record.snapshot.session_id)
}

fn recoverable_session(repo: &Repository) -> Result<Option<SessionRecord>> {
    let sessions = repo.recent_sessions()?;
    Ok(sessions.into_iter().find(|session| {
        session.snapshot.status == SessionStatus::Interrupted
            && session.snapshot.stopped_at.is_none()
    }))
}

fn reconcile_active_session(repo: &mut Repository) -> Result<()> {
    if let Some(active) = repo.active_session()? {
        match active.runtime {
            Some(runtime) if process_exists(runtime.pid) => {}
            Some(_) | None => {
                repo.mark_session_interrupted(
                    &active.snapshot.session_id,
                    Some("daemon process not running; session recovered from sqlite state"),
                )?;
            }
        }
    }
    Ok(())
}

fn print_session_status(record: &SessionRecord) {
    println!("Session: {}", record.snapshot.session_id);
    println!("State:   {}", record.snapshot.status.as_str());
    println!(
        "PID:     {}",
        record
            .runtime
            .as_ref()
            .map(|runtime| runtime.pid.to_string())
            .unwrap_or_else(|| "n/a".to_string())
    );
    println!("Started: {}", format_local(record.snapshot.started_at));
    println!("Updated: {}", format_local(record.snapshot.last_updated_at));
    println!(
        "Flushed: {}",
        record
            .snapshot
            .last_flush_at
            .map(format_local)
            .unwrap_or_else(|| "never".to_string())
    );
    println!("Total:   {}", record.snapshot.total_keypresses);
    println!("Keys:    {}", record.snapshot.unique_keys);
    println!("Clean:   {}", yes_no(record.snapshot.clean_shutdown));
    if let Some(runtime) = &record.runtime {
        println!("Runtime: session {}", runtime.session_id);
        println!("Spawned: {}", format_local(runtime.daemon_started_at));
        println!(
            "Health:  control {} / heartbeat {}",
            runtime
                .control_port
                .map(|port| port.to_string())
                .unwrap_or_else(|| "pending".to_string()),
            runtime
                .heartbeat_at
                .map(format_local)
                .unwrap_or_else(|| "never".to_string())
        );
    }
    if let Some(error) = &record.snapshot.capture_error {
        println!("Error:   {}", error);
    }
}

fn print_session_summary(record: &SessionRecord) {
    println!("Session: {}", record.snapshot.session_id);
    println!("State:   {}", record.snapshot.status.as_str());
    println!("Started: {}", format_local(record.snapshot.started_at));
    println!(
        "Stopped: {}",
        record
            .snapshot
            .stopped_at
            .map(format_local)
            .unwrap_or_else(|| "n/a".to_string())
    );
    println!("Total:   {}", record.snapshot.total_keypresses);
    println!("Clean:   {}", yes_no(record.snapshot.clean_shutdown));
    if let Some(error) = &record.snapshot.capture_error {
        println!("Error:   {}", error);
    }
}

fn format_local(dt: chrono::DateTime<Utc>) -> String {
    dt.with_timezone(&Local)
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}
