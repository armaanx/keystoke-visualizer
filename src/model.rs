use chrono::{DateTime, Utc};
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, ValueEnum, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum KeyboardLayout {
    Ansi104,
    Iso105,
}

impl KeyboardLayout {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ansi104 => "ansi-104",
            Self::Iso105 => "iso-105",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "ansi-104" => Some(Self::Ansi104),
            "iso-105" => Some(Self::Iso105),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SessionStatus {
    Running,
    Stopped,
    Interrupted,
    Failed,
}

impl SessionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Stopped => "stopped",
            Self::Interrupted => "interrupted",
            Self::Failed => "failed",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "running" => Some(Self::Running),
            "stopped" => Some(Self::Stopped),
            "interrupted" => Some(Self::Interrupted),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SessionSnapshot {
    pub session_id: String,
    pub name: Option<String>,
    pub layout: KeyboardLayout,
    pub status: SessionStatus,
    pub started_at: DateTime<Utc>,
    pub stopped_at: Option<DateTime<Utc>>,
    pub last_updated_at: DateTime<Utc>,
    pub total_keypresses: u64,
    pub unique_keys: usize,
    pub dropped_events: u64,
    pub capture_error: Option<String>,
    pub report_path: Option<String>,
    pub last_flush_at: Option<DateTime<Utc>>,
    pub clean_shutdown: bool,
    pub key_counts: BTreeMap<String, u64>,
    pub minute_buckets: BTreeMap<String, u64>,
}

impl SessionSnapshot {
    pub fn new(session_id: String, name: Option<String>, layout: KeyboardLayout) -> Self {
        let now = Utc::now();
        Self {
            session_id,
            name,
            layout,
            status: SessionStatus::Running,
            started_at: now,
            stopped_at: None,
            last_updated_at: now,
            total_keypresses: 0,
            unique_keys: 0,
            dropped_events: 0,
            capture_error: None,
            report_path: None,
            last_flush_at: None,
            clean_shutdown: false,
            key_counts: BTreeMap::new(),
            minute_buckets: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct RuntimeInfo {
    pub session_id: String,
    pub pid: u32,
    pub control_port: Option<u16>,
    pub control_token: String,
    pub daemon_started_at: DateTime<Utc>,
    pub heartbeat_at: Option<DateTime<Utc>>,
    pub flush_seq: u64,
}

#[derive(Clone, Debug)]
pub struct SessionRecord {
    pub snapshot: SessionSnapshot,
    pub runtime: Option<RuntimeInfo>,
}

#[derive(Clone, Debug)]
pub struct CollectorFlush {
    pub total_keypresses: u64,
    pub unique_keys: usize,
    pub dropped_events: u64,
    pub last_updated_at: DateTime<Utc>,
    pub last_flush_at: DateTime<Utc>,
    pub capture_error: Option<String>,
    pub key_counts: BTreeMap<String, u64>,
    pub minute_buckets: BTreeMap<String, u64>,
}

#[derive(Clone)]
pub struct CollectorState {
    pub session_id: String,
    pub total_keypresses: u64,
    pub dropped_events: u64,
    pub capture_error: Option<String>,
    pub key_counts: BTreeMap<String, u64>,
    pub minute_buckets: BTreeMap<String, u64>,
    pub pressed_keys: HashSet<String>,
    pub dirty: bool,
    pub flush_seq: u64,
}

impl CollectorState {
    pub fn from_snapshot(snapshot: &SessionSnapshot, flush_seq: u64) -> Self {
        Self {
            session_id: snapshot.session_id.clone(),
            total_keypresses: snapshot.total_keypresses,
            dropped_events: snapshot.dropped_events,
            capture_error: snapshot.capture_error.clone(),
            key_counts: snapshot.key_counts.clone(),
            minute_buckets: snapshot.minute_buckets.clone(),
            pressed_keys: HashSet::new(),
            dirty: true,
            flush_seq,
        }
    }

    pub fn snapshot(&self) -> CollectorFlush {
        let now = Utc::now();
        CollectorFlush {
            total_keypresses: self.total_keypresses,
            unique_keys: self.key_counts.len(),
            dropped_events: self.dropped_events,
            last_updated_at: now,
            last_flush_at: now,
            capture_error: self.capture_error.clone(),
            key_counts: self.key_counts.clone(),
            minute_buckets: self.minute_buckets.clone(),
        }
    }
}

pub struct KeyCell {
    pub id: &'static str,
    pub label: &'static str,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}
