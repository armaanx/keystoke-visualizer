use chrono::{DateTime, Utc};
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, ValueEnum)]
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
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ActiveSession {
    pub session_id: String,
    pub pid: u32,
    pub started_at: DateTime<Utc>,
    pub layout: KeyboardLayout,
    pub name: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SessionData {
    pub session_id: String,
    pub name: Option<String>,
    pub layout: KeyboardLayout,
    pub started_at: DateTime<Utc>,
    pub stopped_at: Option<DateTime<Utc>>,
    pub last_updated_at: DateTime<Utc>,
    pub total_keypresses: u64,
    pub unique_keys: usize,
    pub dropped_events: u64,
    pub capture_error: Option<String>,
    pub report_path: Option<String>,
    pub key_counts: BTreeMap<String, u64>,
    pub minute_buckets: BTreeMap<String, u64>,
}

impl SessionData {
    pub fn new(session_id: String, name: Option<String>, layout: KeyboardLayout) -> Self {
        let now = Utc::now();
        Self {
            session_id,
            name,
            layout,
            started_at: now,
            stopped_at: None,
            last_updated_at: now,
            total_keypresses: 0,
            unique_keys: 0,
            dropped_events: 0,
            capture_error: None,
            report_path: None,
            key_counts: BTreeMap::new(),
            minute_buckets: BTreeMap::new(),
        }
    }
}

#[derive(Clone)]
pub struct CollectorState {
    pub session: SessionData,
    pub pressed_keys: HashSet<String>,
    pub dirty: bool,
}

impl CollectorState {
    pub fn new(session: SessionData) -> Self {
        Self {
            session,
            pressed_keys: HashSet::new(),
            dirty: true,
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
