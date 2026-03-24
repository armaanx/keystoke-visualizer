use crate::model::{KeyCell, SessionSnapshot};
use chrono::{DateTime, Local, Utc};
use std::collections::BTreeMap;

pub fn build_html_report(session: &SessionSnapshot) -> String {
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
            let share = percentage(*count, session.total_keypresses);
            format!(
                "<tr><td><div class=\"key-name\">{}</div></td><td>{}</td><td>{:.2}%<div class=\"share-bar\"><span style=\"width:{:.2}%\"></span></div></td></tr>",
                html_escape(&pretty_key_name(key)),
                count,
                share,
                share
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
 :root{{--bg:#070b14;--bg2:#11192b;--card:#0f1728dd;--card-2:#131d33;--ink:#eef4ff;--muted:#90a1bf;--line:#25324b;--accent:#6ae2ff;--accent-2:#7d5cff;--accent-3:#4cf3a4;--shadow:0 20px 60px rgba(0,0,0,.38)}}
 *{{box-sizing:border-box}}
 body{{margin:0;font-family:"Aptos","Segoe UI Variable","Segoe UI",system-ui,sans-serif;color:var(--ink);background:
 radial-gradient(circle at top left,rgba(125,92,255,.22),transparent 30%),
 radial-gradient(circle at top right,rgba(106,226,255,.14),transparent 24%),
 radial-gradient(circle at bottom center,rgba(76,243,164,.08),transparent 28%),
 linear-gradient(180deg,var(--bg),var(--bg2) 70%,#09101d)}}
 body::before{{content:"";position:fixed;inset:0;pointer-events:none;background:
 linear-gradient(rgba(255,255,255,.03) 1px,transparent 1px),
 linear-gradient(90deg,rgba(255,255,255,.03) 1px,transparent 1px);background-size:34px 34px;mask-image:linear-gradient(180deg,rgba(0,0,0,.75),transparent 92%);opacity:.18}}
 .wrap{{max-width:1240px;margin:0 auto;padding:34px 22px 64px;position:relative}}
 h1,h2{{margin:0}}
 .hero{{position:relative;overflow:hidden;padding:28px;border-radius:30px;border:1px solid rgba(255,255,255,.08);background:
 linear-gradient(135deg,rgba(19,29,51,.96),rgba(10,17,31,.92)),
 radial-gradient(circle at top right,rgba(106,226,255,.18),transparent 32%);box-shadow:var(--shadow);margin-bottom:22px}}
 .hero::after{{content:"";position:absolute;right:-120px;bottom:-160px;width:360px;height:360px;border-radius:999px;background:radial-gradient(circle,rgba(125,92,255,.26),transparent 70%)}}
 .headline{{display:flex;align-items:flex-start;justify-content:space-between;gap:18px;position:relative;z-index:1}}
 .title-wrap{{max-width:780px}}
 .kicker{{display:inline-flex;align-items:center;gap:10px;padding:8px 12px;border-radius:999px;background:rgba(106,226,255,.08);border:1px solid rgba(106,226,255,.18);text-transform:uppercase;letter-spacing:.14em;color:var(--accent);font-size:12px;margin-bottom:14px}}
 .kicker::before{{content:"";width:8px;height:8px;border-radius:999px;background:var(--accent-3);box-shadow:0 0 18px rgba(76,243,164,.8)}}
 h1{{font-size:clamp(30px,5vw,54px);line-height:1.02;letter-spacing:-.04em;margin-bottom:12px}}
 .sub{{color:var(--muted);font-size:15px;line-height:1.65;max-width:720px}}
 .session-pill{{min-width:190px;padding:16px 18px;border-radius:22px;border:1px solid rgba(255,255,255,.08);background:linear-gradient(180deg,rgba(255,255,255,.08),rgba(255,255,255,.03));backdrop-filter:blur(14px)}}
 .session-label{{color:var(--muted);font-size:12px;text-transform:uppercase;letter-spacing:.12em}}
 .session-value{{font-size:25px;font-weight:750;margin-top:8px}}
 .grid{{display:grid;grid-template-columns:repeat(auto-fit,minmax(210px,1fr));gap:16px;margin-bottom:22px}}
 .card{{position:relative;overflow:hidden;padding:20px;border-radius:24px;border:1px solid rgba(255,255,255,.08);background:linear-gradient(180deg,rgba(15,23,40,.96),rgba(9,15,27,.92));box-shadow:var(--shadow)}}
 .card::before{{content:"";position:absolute;top:0;left:0;right:0;height:1px;background:linear-gradient(90deg,rgba(106,226,255,.55),transparent 60%)}}
 .metric-card{{min-height:146px}}
 .eyebrow{{color:var(--muted);font-size:11px;text-transform:uppercase;letter-spacing:.14em}}
 .big{{font-size:37px;font-weight:760;line-height:1.05;letter-spacing:-.03em;margin-top:16px}}
 .metric-foot{{margin-top:10px;color:var(--muted);font-size:13px;line-height:1.5}}
 .panels{{display:grid;grid-template-columns:1.15fr .85fr;gap:18px}}
 .wide{{grid-column:1/-1}}
 .panel-head{{display:flex;align-items:flex-end;justify-content:space-between;gap:16px;margin-bottom:14px}}
 .panel-copy{{color:var(--muted);font-size:13px;line-height:1.5;max-width:440px}}
 .table-shell{{overflow:auto;border-radius:18px;border:1px solid rgba(255,255,255,.05);background:rgba(255,255,255,.02)}}
 table{{width:100%;border-collapse:collapse;min-width:560px}}
 th,td{{padding:14px 16px;border-bottom:1px solid rgba(255,255,255,.06);text-align:left}}
 th{{color:var(--muted);font-size:12px;font-weight:600;text-transform:uppercase;letter-spacing:.12em;background:rgba(255,255,255,.02)}}
 td:nth-child(2),th:nth-child(2),td:last-child,th:last-child{{text-align:right}}
 tbody tr:hover{{background:rgba(106,226,255,.05)}}
 .key-name{{font-weight:600}}
 .share-bar{{position:relative;height:8px;border-radius:999px;background:rgba(255,255,255,.06);overflow:hidden;margin-top:7px}}
 .share-bar span{{display:block;height:100%;border-radius:inherit;background:linear-gradient(90deg,var(--accent-2),var(--accent));box-shadow:0 0 16px rgba(106,226,255,.22)}}
 .note{{color:var(--muted);font-size:14px;margin-top:10px}}
 svg{{width:100%;height:auto;display:block}}
 @media (max-width:980px){{.headline{{flex-direction:column}}.session-pill{{width:100%}}.panels{{grid-template-columns:1fr}}}}
 @media (max-width:720px){{.wrap{{padding:18px 14px 42px}}.hero{{padding:22px 18px}}.card{{padding:16px;border-radius:20px}}.big{{font-size:30px}}}}
</style></head><body><div class="wrap">
<section class="hero">
<div class="headline">
<div class="title-wrap">
<div class="kicker">Keystroke Session Report</div>
<h1>{title}</h1>
<div class="sub">Session {session_id} • {layout} layout • started {started} • stopped {stopped}. All values come from local aggregate counts with no typed-text capture.</div>
</div>
<div class="session-pill"><div class="session-label">Session ID</div><div class="session-value">{short_session_id}</div></div>
</div>
</section>
<div class="grid">
<div class="card metric-card"><div class="eyebrow">Total Keypresses</div><div class="big">{total}</div><div class="metric-foot">All press events counted across the session lifecycle.</div></div>
<div class="card metric-card"><div class="eyebrow">Unique Keys</div><div class="big">{unique}</div><div class="metric-foot">Distinct key identities observed during capture.</div></div>
<div class="card metric-card"><div class="eyebrow">Duration</div><div class="big">{duration}</div><div class="metric-foot">Measured from session start until stop or last recorded update.</div></div>
<div class="card metric-card"><div class="eyebrow">Peak Minute</div><div class="big">{peak}</div><div class="metric-foot">The busiest minute bucket in the timeline.</div></div>
</div>
<div class="panels">
<section class="card"><div class="panel-head"><h2>Activity Timeline</h2><div class="panel-copy">A neon line plot of minute-by-minute activity, useful for spotting bursts and drop-offs.</div></div>{timeline_svg}<div class="note">This chart shows intensity per bucket rather than cumulative totals.</div></section>
<section class="card"><div class="panel-head"><h2>Top Keys</h2><div class="panel-copy">Most frequent keys ranked by raw volume, normalized against the busiest key.</div></div>{bars_svg}<div class="note">Bars are scaled relative to the top entry.</div></section>
<section class="card wide"><div class="panel-head"><h2>Keyboard Heatmap</h2><div class="panel-copy">Spatial activity view across the active keyboard layout.</div></div>{keyboard_svg}</section>
<section class="card wide"><div class="panel-head"><h2>Per-Key Breakdown</h2><div class="panel-copy">Every captured key with both absolute count and session share.</div></div><div class="table-shell"><table><thead><tr><th>Key</th><th>Count</th><th>Share</th></tr></thead><tbody>{rows}</tbody></table></div></section>
</div></div></body></html>"#,
        title = html_escape(&title),
        session_id = html_escape(&session.session_id),
        short_session_id = html_escape(&session.session_id.chars().take(8).collect::<String>()),
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
            r##"<g><title>{tooltip}</title><rect x="{x}" y="{y}" rx="12" ry="12" width="{w}" height="{h}" fill="{fill}" stroke="rgba(255,255,255,.16)" stroke-width="1.2"/><text x="{text_x}" y="{text_y}" text-anchor="middle" fill="#ecf2ff" font-size="13" font-family="Segoe UI, sans-serif">{label}</text></g>"##,
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
            r##"<text x="0" y="{label_y}" fill="#9eacc7" font-size="13" font-family="Segoe UI, sans-serif">{label}</text><rect x="110" y="{y}" width="320" height="24" rx="8" fill="rgba(255,255,255,.05)"/><rect x="110" y="{y}" width="{width_px}" height="24" rx="8" fill="url(#bar-gradient)"/><text x="{value_x}" y="{label_y}" fill="#eef4ff" font-size="13" font-family="Segoe UI, sans-serif">{count}</text>"##,
            label_y = y + 17.0, label = html_escape(&pretty_key_name(key)), y = y, width_px = width_px.max(8.0), value_x = 120.0 + width_px, count = count
        )
    }).collect::<Vec<_>>().join("");
    format!(
        r##"<svg viewBox="0 0 520 460" role="img" aria-label="Top key chart"><defs><linearGradient id="bar-gradient" x1="0%" x2="100%" y1="0%" y2="0%"><stop offset="0%" stop-color="#7d5cff"/><stop offset="100%" stop-color="#6ae2ff"/></linearGradient></defs>{body}</svg>"##
    )
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
            labels.push_str(&format!(r##"<text x="{x}" y="285" text-anchor="middle" fill="#90a1bf" font-size="11" font-family="Segoe UI, sans-serif">{label}</text>"##, x = x, label = html_escape(minute)));
        }
    }
    format!(
        r###"<svg viewBox="0 0 620 300" role="img" aria-label="Per-minute timeline"><defs><linearGradient id="timeline-gradient" x1="0%" x2="100%" y1="0%" y2="0%"><stop offset="0%" stop-color="#7d5cff"/><stop offset="100%" stop-color="#6ae2ff"/></linearGradient><filter id="line-glow"><feGaussianBlur stdDeviation="4.5" result="blur"/><feMerge><feMergeNode in="blur"/><feMergeNode in="SourceGraphic"/></feMerge></filter></defs><line x1="50" y1="20" x2="50" y2="240" stroke="rgba(255,255,255,.12)"/><line x1="50" y1="240" x2="590" y2="240" stroke="rgba(255,255,255,.12)"/><polyline fill="none" stroke="rgba(106,226,255,.25)" stroke-width="8" filter="url(#line-glow)" points="{points}"/><polyline fill="none" stroke="url(#timeline-gradient)" stroke-width="3.5" points="{points}"/>{labels}</svg>"###,
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
    let cold = (22.0, 32.0, 53.0);
    let hot = (106.0, 226.0, 255.0);
    let r = cold.0 + (hot.0 - cold.0) * ratio;
    let g = cold.1 + (hot.1 - cold.1) * ratio;
    let b = cold.2 + (hot.2 - cold.2) * ratio;
    format!("rgb({:.0}, {:.0}, {:.0})", r, g, b)
}

fn empty_svg(message: &str) -> String {
    format!(
        r##"<svg viewBox="0 0 520 220" role="img"><text x="50%" y="50%" text-anchor="middle" fill="#90a1bf" font-size="16" font-family="Segoe UI, sans-serif">{}</text></svg>"##,
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
