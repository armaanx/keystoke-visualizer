use crate::model::{CollectorState, SessionSnapshot};
use crate::storage::{AppPaths, Repository};
use anyhow::{Context, Result};
use chrono::Utc;
use rdev::{Event, EventType};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[derive(Clone)]
pub struct DaemonContext {
    pub session_id: String,
    pub control_token: String,
}

#[derive(Deserialize)]
struct ControlRequest {
    token: String,
    command: String,
}

#[derive(Deserialize, Serialize)]
struct ControlResponse {
    ok: bool,
    message: String,
}

pub fn start_flush_worker(
    paths: AppPaths,
    state: Arc<Mutex<CollectorState>>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(1));
            let flush = {
                let mut guard = state.lock().expect("collector state poisoned");
                if !guard.dirty {
                    None
                } else {
                    guard.flush_seq += 1;
                    guard.dirty = false;
                    Some((guard.session_id.clone(), guard.flush_seq, guard.snapshot()))
                }
            };
            if let Some((session_id, flush_seq, snapshot)) = flush
                && let Ok(mut repo) = Repository::open(&paths)
            {
                let _ = repo.flush_session(&session_id, flush_seq, &snapshot);
            }
        }
    })
}

pub fn start_control_server(
    paths: AppPaths,
    state: Arc<Mutex<CollectorState>>,
    context: DaemonContext,
) -> Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0").context("binding daemon control port")?;
    let port = listener
        .local_addr()
        .context("reading daemon control port")?
        .port();
    thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let should_exit =
                        handle_control_stream(stream, &paths, &state, &context).unwrap_or(false);
                    if should_exit {
                        std::process::exit(0);
                    }
                }
                Err(_) => break,
            }
        }
    });
    Ok(port)
}

fn handle_control_stream(
    mut stream: TcpStream,
    paths: &AppPaths,
    state: &Arc<Mutex<CollectorState>>,
    context: &DaemonContext,
) -> Result<bool> {
    let mut reader = BufReader::new(stream.try_clone().context("cloning tcp stream")?);
    let mut body = String::new();
    reader
        .read_line(&mut body)
        .context("reading control request")?;
    let request: ControlRequest =
        serde_json::from_str(body.trim()).context("parsing control request")?;
    if request.token != context.control_token {
        write_response(&mut stream, false, "invalid token")?;
        return Ok(false);
    }

    match request.command.as_str() {
        "status" => {
            write_response(&mut stream, true, "running")?;
            Ok(false)
        }
        "flush" => {
            flush_now(paths, state)?;
            write_response(&mut stream, true, "flushed")?;
            Ok(false)
        }
        "stop" => {
            flush_now(paths, state)?;
            let stopped_at = Utc::now();
            Repository::open(paths)?.mark_session_stopped(&context.session_id, stopped_at, true)?;
            write_response(&mut stream, true, "stopped")?;
            Ok(true)
        }
        _ => {
            write_response(&mut stream, false, "unsupported command")?;
            Ok(false)
        }
    }
}

pub fn send_control_command(port: u16, token: &str, command: &str) -> Result<String> {
    let mut stream = TcpStream::connect(("127.0.0.1", port))
        .with_context(|| format!("connecting to daemon control port {}", port))?;
    let request = serde_json::to_string(&serde_json::json!({
        "token": token,
        "command": command,
    }))
    .context("serializing control request")?;
    writeln!(stream, "{request}").context("sending control request")?;
    stream.shutdown(Shutdown::Write).ok();
    let mut response = String::new();
    BufReader::new(stream)
        .read_line(&mut response)
        .context("reading control response")?;
    let response: ControlResponse =
        serde_json::from_str(response.trim()).context("parsing control response")?;
    if !response.ok {
        anyhow::bail!(response.message);
    }
    Ok(response.message)
}

pub fn handle_event(state: &Arc<Mutex<CollectorState>>, event: Event) {
    let mut guard = state.lock().expect("collector state poisoned");
    match event.event_type {
        EventType::KeyPress(key) => {
            let key_id = format!("{key:?}");
            if guard.pressed_keys.insert(key_id.clone()) {
                let minute_bucket = Utc::now().format("%Y-%m-%d %H:%M").to_string();
                *guard.key_counts.entry(key_id).or_insert(0) += 1;
                *guard.minute_buckets.entry(minute_bucket).or_insert(0) += 1;
                guard.total_keypresses += 1;
                guard.dirty = true;
            }
        }
        EventType::KeyRelease(key) => {
            guard.pressed_keys.remove(&format!("{key:?}"));
        }
        _ => {}
    }
}

pub fn load_collector_state(snapshot: &SessionSnapshot, flush_seq: u64) -> CollectorState {
    CollectorState::from_snapshot(snapshot, flush_seq)
}

fn flush_now(paths: &AppPaths, state: &Arc<Mutex<CollectorState>>) -> Result<()> {
    let (session_id, flush_seq, snapshot) = {
        let mut guard = state.lock().expect("collector state poisoned");
        guard.flush_seq += 1;
        guard.dirty = false;
        (guard.session_id.clone(), guard.flush_seq, guard.snapshot())
    };
    Repository::open(paths)?.flush_session(&session_id, flush_seq, &snapshot)?;
    Ok(())
}

fn write_response(stream: &mut TcpStream, ok: bool, message: &str) -> Result<()> {
    let body = serde_json::to_string(&ControlResponse {
        ok,
        message: message.to_string(),
    })
    .context("serializing control response")?;
    writeln!(stream, "{body}").context("writing control response")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{KeyboardLayout, SessionSnapshot};
    use rdev::{Event, EventType, Key};
    use std::collections::BTreeMap;

    #[test]
    fn handle_event_counts_unique_presses() {
        let snapshot = SessionSnapshot {
            session_id: "abc".to_string(),
            name: None,
            layout: KeyboardLayout::Ansi104,
            status: crate::model::SessionStatus::Running,
            started_at: Utc::now(),
            stopped_at: None,
            last_updated_at: Utc::now(),
            total_keypresses: 0,
            unique_keys: 0,
            dropped_events: 0,
            capture_error: None,
            report_path: None,
            last_flush_at: None,
            clean_shutdown: false,
            key_counts: BTreeMap::new(),
            minute_buckets: BTreeMap::new(),
        };
        let state = Arc::new(Mutex::new(load_collector_state(&snapshot, 0)));
        handle_event(
            &state,
            Event {
                event_type: EventType::KeyPress(Key::KeyA),
                time: std::time::SystemTime::now(),
                name: None,
            },
        );
        handle_event(
            &state,
            Event {
                event_type: EventType::KeyPress(Key::KeyA),
                time: std::time::SystemTime::now(),
                name: None,
            },
        );
        handle_event(
            &state,
            Event {
                event_type: EventType::KeyRelease(Key::KeyA),
                time: std::time::SystemTime::now(),
                name: None,
            },
        );
        handle_event(
            &state,
            Event {
                event_type: EventType::KeyPress(Key::KeyA),
                time: std::time::SystemTime::now(),
                name: None,
            },
        );

        let guard = state.lock().expect("collector state poisoned");
        assert_eq!(guard.total_keypresses, 2);
        assert_eq!(guard.key_counts.get("KeyA"), Some(&2));
    }
}
