use crate::model::{SessionRecord, SessionSnapshot};
use crate::report::{keyboard_layout, pretty_key_name, sorted_key_usage};
use crate::storage::Repository;
use anyhow::Result;
use chrono::Utc;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ReportResponse {
    pub session: SessionHeaderDto,
    pub summary: SummaryDto,
    pub activity: Vec<ActivityPointDto>,
    pub key_usage: Vec<KeyUsageEntryDto>,
    pub keyboard: KeyboardHeatmapDto,
    pub history: Vec<HistoryEntryDto>,
}

#[derive(Debug, Serialize)]
pub struct SessionHeaderDto {
    pub session_id: String,
    pub session_name: Option<String>,
    pub status: String,
    pub layout: String,
    pub started_at: String,
    pub stopped_at: Option<String>,
    pub duration_seconds: i64,
    pub clean_shutdown: bool,
    pub privacy_mode: &'static str,
}

#[derive(Debug, Serialize)]
pub struct SummaryDto {
    pub total_keypresses: u64,
    pub unique_keys: usize,
    pub peak_minute: Option<u64>,
    pub peak_minute_bucket: Option<String>,
    pub avg_keys_per_minute: f64,
    pub dropped_events: u64,
}

#[derive(Debug, Serialize)]
pub struct ActivityPointDto {
    pub minute_bucket: String,
    pub keypresses: u64,
}

#[derive(Debug, Serialize)]
pub struct KeyUsageEntryDto {
    pub key_id: String,
    pub display_label: String,
    pub count: u64,
    pub share_percent: f64,
}

#[derive(Debug, Serialize)]
pub struct KeyboardHeatmapDto {
    pub layout: String,
    pub keys: Vec<KeyboardHeatmapKeyDto>,
}

#[derive(Debug, Serialize)]
pub struct KeyboardHeatmapKeyDto {
    pub key_id: String,
    pub label: String,
    pub count: u64,
    pub intensity: f64,
}

#[derive(Debug, Serialize)]
pub struct HistoryEntryDto {
    pub session_id: String,
    pub name: Option<String>,
    pub started_at: String,
    pub total_keypresses: u64,
    pub duration_seconds: i64,
    pub status: String,
}

pub fn build_report_response(
    repo: &Repository,
    session_id: &str,
    history_limit: usize,
) -> Result<ReportResponse> {
    let snapshot = repo.load_session_snapshot(session_id)?;
    let mut history = recent_history(repo, history_limit)?;
    history.retain(|entry| entry.session_id != session_id);
    history.truncate(history_limit);

    let key_usage = build_key_usage(&snapshot);
    let activity = snapshot
        .minute_buckets
        .iter()
        .map(|(minute_bucket, keypresses)| ActivityPointDto {
            minute_bucket: minute_bucket.clone(),
            keypresses: *keypresses,
        })
        .collect::<Vec<_>>();
    let peak = snapshot
        .minute_buckets
        .iter()
        .max_by_key(|(_, count)| *count)
        .map(|(bucket, count)| (bucket.clone(), *count));
    let duration_seconds = duration_seconds(&snapshot);
    let keyboard = build_keyboard_heatmap(&snapshot);

    Ok(ReportResponse {
        session: SessionHeaderDto {
            session_id: snapshot.session_id.clone(),
            session_name: snapshot.name.clone(),
            status: snapshot.status.as_str().to_string(),
            layout: snapshot.layout.as_str().to_string(),
            started_at: snapshot.started_at.to_rfc3339(),
            stopped_at: snapshot.stopped_at.map(|value| value.to_rfc3339()),
            duration_seconds,
            clean_shutdown: snapshot.clean_shutdown,
            privacy_mode: "aggregate-only",
        },
        summary: SummaryDto {
            total_keypresses: snapshot.total_keypresses,
            unique_keys: snapshot.unique_keys,
            peak_minute: peak.as_ref().map(|(_, count)| *count),
            peak_minute_bucket: peak.map(|(bucket, _)| bucket),
            avg_keys_per_minute: average_keys_per_minute(snapshot.total_keypresses, duration_seconds),
            dropped_events: snapshot.dropped_events,
        },
        activity,
        key_usage,
        keyboard,
        history,
    })
}

pub fn recent_history(repo: &Repository, limit: usize) -> Result<Vec<HistoryEntryDto>> {
    let sessions = repo.recent_sessions_limit(limit)?;
    Ok(sessions.into_iter().map(history_from_record).collect())
}

fn build_key_usage(snapshot: &SessionSnapshot) -> Vec<KeyUsageEntryDto> {
    sorted_key_usage(&snapshot.key_counts)
        .into_iter()
        .map(|(key_id, count)| KeyUsageEntryDto {
            display_label: pretty_key_name(&key_id),
            share_percent: percentage(count, snapshot.total_keypresses),
            key_id,
            count,
        })
        .collect()
}

fn build_keyboard_heatmap(snapshot: &SessionSnapshot) -> KeyboardHeatmapDto {
    let max = snapshot.key_counts.values().copied().max().unwrap_or(0) as f64;
    let keys = keyboard_layout(snapshot.layout)
        .into_iter()
        .map(|cell| {
            let count = snapshot.key_counts.get(cell.id).copied().unwrap_or(0);
            let intensity = if max > 0.0 { count as f64 / max } else { 0.0 };
            KeyboardHeatmapKeyDto {
                key_id: cell.id.to_string(),
                label: cell.label.to_string(),
                count,
                intensity,
            }
        })
        .collect();

    KeyboardHeatmapDto {
        layout: snapshot.layout.as_str().to_string(),
        keys,
    }
}

fn history_from_record(record: SessionRecord) -> HistoryEntryDto {
    let duration = duration_seconds(&record.snapshot);
    HistoryEntryDto {
        session_id: record.snapshot.session_id,
        name: record.snapshot.name,
        started_at: record.snapshot.started_at.to_rfc3339(),
        total_keypresses: record.snapshot.total_keypresses,
        duration_seconds: duration,
        status: record.snapshot.status.as_str().to_string(),
    }
}

fn duration_seconds(snapshot: &SessionSnapshot) -> i64 {
    snapshot
        .stopped_at
        .unwrap_or(snapshot.last_updated_at)
        .signed_duration_since(snapshot.started_at)
        .num_seconds()
        .max(0)
}

fn average_keys_per_minute(total_keypresses: u64, duration_seconds: i64) -> f64 {
    let minutes = (duration_seconds as f64 / 60.0).max(1.0 / 60.0);
    ((total_keypresses as f64 / minutes) * 100.0).round() / 100.0
}

fn percentage(part: u64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        ((part as f64 / total as f64) * 10000.0).round() / 100.0
    }
}

#[allow(dead_code)]
fn _now() -> String {
    Utc::now().to_rfc3339()
}
