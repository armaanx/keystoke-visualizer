use crate::model::{KeyCell, SessionData};
use chrono::{DateTime, Local, Utc};
use std::collections::BTreeMap;

pub fn build_html_report(session: &SessionData) -> String {
    let duration_seconds = session
        .stopped_at
        .unwrap_or(session.last_updated_at)
        .signed_duration_since(session.started_at)
        .num_seconds()
        .max(0);
    let top_keys = top_keys(&session.key_counts, 12);
    let timeline_svg = render_timeline_svg(&session.minute_buckets);
    let bars_svg = render_top_keys_svg(&top_keys);
    let keyboard_svg = render_keyboard_svg(&session.key_counts);
    let peak_bucket = session
        .minute_buckets
        .iter()
        .max_by_key(|(_, count)| *count)
        .map(|(minute, count)| format!("{minute} ({count})"))
        .unwrap_or_else(|| "n/a".to_string());
    let mut table_items: Vec<_> = session.key_counts.iter().collect();
    table_items.sort_by(|left, right| right.1.cmp(left.1).then_with(|| left.0.cmp(right.0)));
    let rows = table_items
        .into_iter()
        .map(|(key, count)| {
            format!(
                "<tr><td>{}</td><td>{}</td><td>{:.2}%</td></tr>",
                html_escape(&pretty_key_name(key)),
                count,
                percentage(*count, session.total_keypresses)
            )
        })
        .collect::<Vec<_>>()
        .join("");
    let title = session
        .name
        .clone()
        .unwrap_or_else(|| format!("Session {}", &session.session_id[..8]));

    format!(
        r#"<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title><style>
:root{{--bg:#f7f1e7;--card:#fffdf8;--ink:#1f2328;--muted:#6b7280;--accent:#146c94;--line:#ded6c9}}
*{{box-sizing:border-box}} body{{margin:0;font-family:"Segoe UI",system-ui,sans-serif;color:var(--ink);background:radial-gradient(circle at top left,#fffdf7 0,#fffdf7 20%,transparent 50%),linear-gradient(135deg,#f7f1e7,#f2e7d5)}}
.wrap{{max-width:1200px;margin:0 auto;padding:32px 24px 56px}} h1,h2{{margin:0 0 12px}} .sub{{color:var(--muted);margin-bottom:24px}}
.grid{{display:grid;grid-template-columns:repeat(auto-fit,minmax(180px,1fr));gap:16px;margin-bottom:24px}}
.card{{background:rgba(255,253,248,.9);border:1px solid var(--line);border-radius:18px;padding:18px;box-shadow:0 10px 30px rgba(31,35,40,.06)}}
.eyebrow{{color:var(--muted);font-size:12px;text-transform:uppercase;letter-spacing:.08em}} .big{{font-size:28px;font-weight:700;margin-top:8px}}
.panels{{display:grid;grid-template-columns:1.2fr 1fr;gap:18px;margin-bottom:18px}} .wide{{grid-column:1/-1}}
table{{width:100%;border-collapse:collapse}} th,td{{text-align:left;padding:10px 0;border-bottom:1px solid var(--line)}} th{{color:var(--muted);font-weight:600}}
.note{{color:var(--muted);font-size:14px;margin-top:8px}} svg{{width:100%;height:auto;display:block}} @media (max-width:900px){{.panels{{grid-template-columns:1fr}}}}
</style></head><body><div class="wrap">
<h1>{title}</h1><div class="sub">Session {session_id} • {layout} • started {started} • stopped {stopped}</div>
<div class="grid">
<div class="card"><div class="eyebrow">Total Keypresses</div><div class="big">{total}</div></div>
<div class="card"><div class="eyebrow">Unique Keys</div><div class="big">{unique}</div></div>
<div class="card"><div class="eyebrow">Duration</div><div class="big">{duration}</div></div>
<div class="card"><div class="eyebrow">Peak Minute</div><div class="big">{peak}</div></div>
</div>
<div class="panels">
<section class="card"><h2>Timeline</h2>{timeline_svg}<div class="note">Per-minute activity over the session.</div></section>
<section class="card"><h2>Top Keys</h2>{bars_svg}<div class="note">Most frequently pressed keys.</div></section>
<section class="card wide"><h2>Keyboard Heatmap</h2>{keyboard_svg}</section>
<section class="card wide"><h2>Per-Key Table</h2><table><thead><tr><th>Key</th><th>Count</th><th>Share</th></tr></thead><tbody>{rows}</tbody></table></section>
</div></div></body></html>"#,
        title = html_escape(&title),
        session_id = html_escape(&session.session_id),
        layout = session.layout.as_str(),
        started = format_local(session.started_at),
        stopped = session
            .stopped_at
            .map(format_local)
            .unwrap_or_else(|| "running".to_string()),
        total = session.total_keypresses,
        unique = session.unique_keys,
        duration = human_duration(duration_seconds),
        peak = html_escape(&peak_bucket),
        timeline_svg = timeline_svg,
        bars_svg = bars_svg,
        keyboard_svg = keyboard_svg,
        rows = rows,
    )
}

fn render_keyboard_svg(counts: &BTreeMap<String, u64>) -> String {
    let keys = keyboard_layout();
    let max_count = counts.values().copied().max().unwrap_or(1) as f64;
    let cells = keys.into_iter().map(|key| {
        let count = counts.get(key.id).copied().unwrap_or(0);
        let ratio = count as f64 / max_count;
        let text_x = key.x + key.w / 2.0;
        let text_y = key.y + key.h / 2.0 + 6.0;
        format!(
            r##"<g><title>{tooltip}</title><rect x="{x}" y="{y}" rx="10" ry="10" width="{w}" height="{h}" fill="{fill}" stroke="#cabda7" stroke-width="1.4"/><text x="{text_x}" y="{text_y}" text-anchor="middle" fill="#1f2328" font-size="13" font-family="Segoe UI, sans-serif">{label}</text></g>"##,
            tooltip = html_escape(&format!("{}: {}", key.label, count)),
            x = key.x, y = key.y, w = key.w, h = key.h, fill = heat_color(ratio), text_x = text_x, text_y = text_y, label = html_escape(key.label)
        )
    }).collect::<Vec<_>>().join("");
    format!(r#"<svg viewBox="0 0 1160 370" role="img" aria-label="Keyboard heatmap">{cells}</svg>"#)
}

fn render_top_keys_svg(top_keys: &[(String, u64)]) -> String {
    if top_keys.is_empty() {
        return empty_svg("No key activity recorded yet.");
    }
    let max = top_keys.iter().map(|(_, count)| *count).max().unwrap_or(1) as f64;
    let body = top_keys.iter().enumerate().map(|(index, (key, count))| {
        let y = 20.0 + index as f64 * 34.0;
        let width_px = ((*count as f64) / max) * 320.0;
        format!(
            r##"<text x="0" y="{label_y}" fill="#6b7280" font-size="13" font-family="Segoe UI, sans-serif">{label}</text><rect x="110" y="{y}" width="{width_px}" height="24" rx="8" fill="#146c94"/><text x="{value_x}" y="{label_y}" fill="#1f2328" font-size="13" font-family="Segoe UI, sans-serif">{count}</text>"##,
            label_y = y + 17.0, label = html_escape(&pretty_key_name(key)), y = y, width_px = width_px.max(8.0), value_x = 120.0 + width_px, count = count
        )
    }).collect::<Vec<_>>().join("");
    format!(r#"<svg viewBox="0 0 520 460" role="img" aria-label="Top key chart">{body}</svg>"#)
}

fn render_timeline_svg(buckets: &BTreeMap<String, u64>) -> String {
    if buckets.is_empty() {
        return empty_svg("No timeline data recorded yet.");
    }
    let points: Vec<_> = buckets.iter().collect();
    let max = points.iter().map(|(_, count)| **count).max().unwrap_or(1) as f64;
    let step = if points.len() > 1 {
        540.0 / (points.len() - 1) as f64
    } else {
        540.0
    };
    let mut polyline = String::new();
    let mut labels = String::new();
    for (idx, (minute, count)) in points.iter().enumerate() {
        let x = 50.0 + step * idx as f64;
        let y = 20.0 + 220.0 - ((**count as f64 / max) * 220.0);
        polyline.push_str(&format!("{x},{y} "));
        if idx == 0 || idx == points.len() - 1 || idx % 5 == 0 {
            labels.push_str(&format!(r##"<text x="{x}" y="285" text-anchor="middle" fill="#6b7280" font-size="11" font-family="Segoe UI, sans-serif">{label}</text>"##, x = x, label = html_escape(minute)));
        }
    }
    format!(
        r##"<svg viewBox="0 0 620 300" role="img" aria-label="Per-minute timeline"><line x1="50" y1="20" x2="50" y2="240" stroke="#cabda7"/><line x1="50" y1="240" x2="590" y2="240" stroke="#cabda7"/><polyline fill="none" stroke="#146c94" stroke-width="3" points="{points}"/>{labels}</svg>"##,
        points = polyline.trim_end(),
        labels = labels
    )
}

fn keyboard_layout() -> Vec<KeyCell> {
    let mut keys = Vec::new();
    let mut row = |y: f64, items: &[(&'static str, &'static str, f64)]| {
        let mut x = 20.0;
        for (id, label, units) in items {
            keys.push(KeyCell {
                id,
                label,
                x,
                y,
                w: 58.0 * units - 8.0,
                h: 48.0,
            });
            x += 58.0 * units;
        }
    };
    row(
        20.0,
        &[
            ("Escape", "Esc", 1.0),
            ("F1", "F1", 1.0),
            ("F2", "F2", 1.0),
            ("F3", "F3", 1.0),
            ("F4", "F4", 1.0),
            ("F5", "F5", 1.0),
            ("F6", "F6", 1.0),
            ("F7", "F7", 1.0),
            ("F8", "F8", 1.0),
            ("F9", "F9", 1.0),
            ("F10", "F10", 1.0),
            ("F11", "F11", 1.0),
            ("F12", "F12", 1.0),
        ],
    );
    row(
        78.0,
        &[
            ("BackQuote", "`", 1.0),
            ("Num1", "1", 1.0),
            ("Num2", "2", 1.0),
            ("Num3", "3", 1.0),
            ("Num4", "4", 1.0),
            ("Num5", "5", 1.0),
            ("Num6", "6", 1.0),
            ("Num7", "7", 1.0),
            ("Num8", "8", 1.0),
            ("Num9", "9", 1.0),
            ("Num0", "0", 1.0),
            ("Minus", "-", 1.0),
            ("Equal", "=", 1.0),
            ("Backspace", "Backspace", 2.0),
        ],
    );
    row(
        136.0,
        &[
            ("Tab", "Tab", 1.5),
            ("KeyQ", "Q", 1.0),
            ("KeyW", "W", 1.0),
            ("KeyE", "E", 1.0),
            ("KeyR", "R", 1.0),
            ("KeyT", "T", 1.0),
            ("KeyY", "Y", 1.0),
            ("KeyU", "U", 1.0),
            ("KeyI", "I", 1.0),
            ("KeyO", "O", 1.0),
            ("KeyP", "P", 1.0),
            ("LeftBracket", "[", 1.0),
            ("RightBracket", "]", 1.0),
            ("BackSlash", "\\", 1.5),
        ],
    );
    row(
        194.0,
        &[
            ("CapsLock", "Caps", 1.75),
            ("KeyA", "A", 1.0),
            ("KeyS", "S", 1.0),
            ("KeyD", "D", 1.0),
            ("KeyF", "F", 1.0),
            ("KeyG", "G", 1.0),
            ("KeyH", "H", 1.0),
            ("KeyJ", "J", 1.0),
            ("KeyK", "K", 1.0),
            ("KeyL", "L", 1.0),
            ("SemiColon", ";", 1.0),
            ("Quote", "'", 1.0),
            ("Return", "Enter", 2.25),
        ],
    );
    row(
        252.0,
        &[
            ("ShiftLeft", "Shift", 2.25),
            ("KeyZ", "Z", 1.0),
            ("KeyX", "X", 1.0),
            ("KeyC", "C", 1.0),
            ("KeyV", "V", 1.0),
            ("KeyB", "B", 1.0),
            ("KeyN", "N", 1.0),
            ("KeyM", "M", 1.0),
            ("Comma", ",", 1.0),
            ("Dot", ".", 1.0),
            ("Slash", "/", 1.0),
            ("ShiftRight", "Shift", 2.75),
        ],
    );
    row(
        310.0,
        &[
            ("ControlLeft", "Ctrl", 1.25),
            ("MetaLeft", "Win", 1.25),
            ("Alt", "Alt", 1.25),
            ("Space", "Space", 6.25),
            ("AltGr", "AltGr", 1.25),
            ("MetaRight", "Win", 1.25),
            ("Menu", "Menu", 1.25),
            ("ControlRight", "Ctrl", 1.25),
        ],
    );
    keys
}

fn top_keys(counts: &BTreeMap<String, u64>, limit: usize) -> Vec<(String, u64)> {
    let mut items: Vec<_> = counts
        .iter()
        .map(|(key, count)| (key.clone(), *count))
        .collect();
    items.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    items.truncate(limit);
    items
}

fn pretty_key_name(raw: &str) -> String {
    match raw {
        "Return" => "Enter".to_string(),
        "Space" => "Space".to_string(),
        "Tab" => "Tab".to_string(),
        "CapsLock" => "Caps Lock".to_string(),
        "ShiftLeft" | "ShiftRight" => "Shift".to_string(),
        "ControlLeft" | "ControlRight" => "Ctrl".to_string(),
        "MetaLeft" | "MetaRight" => "Win".to_string(),
        "BackQuote" => "`".to_string(),
        "Minus" => "-".to_string(),
        "Equal" => "=".to_string(),
        "LeftBracket" => "[".to_string(),
        "RightBracket" => "]".to_string(),
        "BackSlash" => "\\".to_string(),
        "SemiColon" => ";".to_string(),
        "Quote" => "'".to_string(),
        "Comma" => ",".to_string(),
        "Dot" => ".".to_string(),
        "Slash" => "/".to_string(),
        _ if raw.starts_with("Key") && raw.len() == 4 => raw[3..].to_string(),
        _ if raw.starts_with("Num") && raw.len() == 4 => raw[3..].to_string(),
        _ => raw.to_string(),
    }
}

fn heat_color(ratio: f64) -> String {
    let ratio = ratio.clamp(0.0, 1.0);
    let cold = (245.0, 239.0, 227.0);
    let hot = (198.0, 40.0, 40.0);
    let r = cold.0 + (hot.0 - cold.0) * ratio;
    let g = cold.1 + (hot.1 - cold.1) * ratio;
    let b = cold.2 + (hot.2 - cold.2) * ratio;
    format!("rgb({:.0}, {:.0}, {:.0})", r, g, b)
}

fn empty_svg(message: &str) -> String {
    format!(
        r##"<svg viewBox="0 0 520 220" role="img"><text x="50%" y="50%" text-anchor="middle" fill="#6b7280" font-size="16" font-family="Segoe UI, sans-serif">{}</text></svg>"##,
        html_escape(message)
    )
}

fn percentage(part: u64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        (part as f64 / total as f64) * 100.0
    }
}

fn human_duration(total_seconds: i64) -> String {
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    if hours > 0 {
        format!("{hours}h {minutes}m {seconds}s")
    } else if minutes > 0 {
        format!("{minutes}m {seconds}s")
    } else {
        format!("{seconds}s")
    }
}

fn html_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn format_local(dt: DateTime<Utc>) -> String {
    dt.with_timezone(&Local)
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}
