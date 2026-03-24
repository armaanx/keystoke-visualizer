import { useEffect, useMemo, useState, type ReactNode } from "react";
import { BrowserRouter, Navigate, Route, Routes, useLocation, useNavigate, useParams } from "react-router-dom";

type ReportResponse = {
  session: {
    session_id: string;
    session_name: string | null;
    status: "running" | "stopped" | "interrupted" | "failed";
    layout: string;
    started_at: string;
    stopped_at: string | null;
    duration_seconds: number;
    clean_shutdown: boolean;
    privacy_mode: string;
  };
  summary: {
    total_keypresses: number;
    unique_keys: number;
    peak_minute: number | null;
    peak_minute_bucket: string | null;
    avg_keys_per_minute: number;
    dropped_events: number;
  };
  activity: Array<{
    minute_bucket: string;
    keypresses: number;
  }>;
  key_usage: Array<{
    key_id: string;
    display_label: string;
    count: number;
    share_percent: number;
  }>;
  keyboard: {
    layout: string;
    keys: Array<{
      key_id: string;
      label: string;
      count: number;
      intensity: number;
    }>;
  };
  history: HistoryEntry[];
};

type HistoryEntry = {
  session_id: string;
  name: string | null;
  started_at: string;
  total_keypresses: number;
  duration_seconds: number;
  status: string;
};

type ApiError = {
  error: string;
};

const KEYBOARD_TEMPLATE: Array<Array<{ keyId: string; label: string; width: number }>> = [
  [
    { keyId: "Escape", label: "Esc", width: 1 },
    { keyId: "F1", label: "F1", width: 1 },
    { keyId: "F2", label: "F2", width: 1 },
    { keyId: "F3", label: "F3", width: 1 },
    { keyId: "F4", label: "F4", width: 1 },
    { keyId: "F5", label: "F5", width: 1 },
    { keyId: "F6", label: "F6", width: 1 },
    { keyId: "F7", label: "F7", width: 1 },
    { keyId: "F8", label: "F8", width: 1 },
    { keyId: "F9", label: "F9", width: 1 },
    { keyId: "F10", label: "F10", width: 1 },
    { keyId: "F11", label: "F11", width: 1 },
    { keyId: "F12", label: "F12", width: 1 },
  ],
  [
    { keyId: "BackQuote", label: "`", width: 1 },
    { keyId: "Num1", label: "1", width: 1 },
    { keyId: "Num2", label: "2", width: 1 },
    { keyId: "Num3", label: "3", width: 1 },
    { keyId: "Num4", label: "4", width: 1 },
    { keyId: "Num5", label: "5", width: 1 },
    { keyId: "Num6", label: "6", width: 1 },
    { keyId: "Num7", label: "7", width: 1 },
    { keyId: "Num8", label: "8", width: 1 },
    { keyId: "Num9", label: "9", width: 1 },
    { keyId: "Num0", label: "0", width: 1 },
    { keyId: "Minus", label: "-", width: 1 },
    { keyId: "Equal", label: "=", width: 1 },
    { keyId: "Backspace", label: "Backspace", width: 2 },
  ],
  [
    { keyId: "Tab", label: "Tab", width: 1.5 },
    { keyId: "KeyQ", label: "Q", width: 1 },
    { keyId: "KeyW", label: "W", width: 1 },
    { keyId: "KeyE", label: "E", width: 1 },
    { keyId: "KeyR", label: "R", width: 1 },
    { keyId: "KeyT", label: "T", width: 1 },
    { keyId: "KeyY", label: "Y", width: 1 },
    { keyId: "KeyU", label: "U", width: 1 },
    { keyId: "KeyI", label: "I", width: 1 },
    { keyId: "KeyO", label: "O", width: 1 },
    { keyId: "KeyP", label: "P", width: 1 },
    { keyId: "LeftBracket", label: "[", width: 1 },
    { keyId: "RightBracket", label: "]", width: 1 },
    { keyId: "BackSlash", label: "\\", width: 1.5 },
  ],
  [
    { keyId: "CapsLock", label: "Caps", width: 1.75 },
    { keyId: "KeyA", label: "A", width: 1 },
    { keyId: "KeyS", label: "S", width: 1 },
    { keyId: "KeyD", label: "D", width: 1 },
    { keyId: "KeyF", label: "F", width: 1 },
    { keyId: "KeyG", label: "G", width: 1 },
    { keyId: "KeyH", label: "H", width: 1 },
    { keyId: "KeyJ", label: "J", width: 1 },
    { keyId: "KeyK", label: "K", width: 1 },
    { keyId: "KeyL", label: "L", width: 1 },
    { keyId: "SemiColon", label: ";", width: 1 },
    { keyId: "Quote", label: "'", width: 1 },
    { keyId: "Return", label: "Enter", width: 2.25 },
  ],
  [
    { keyId: "ShiftLeft", label: "Shift", width: 2.25 },
    { keyId: "KeyZ", label: "Z", width: 1 },
    { keyId: "KeyX", label: "X", width: 1 },
    { keyId: "KeyC", label: "C", width: 1 },
    { keyId: "KeyV", label: "V", width: 1 },
    { keyId: "KeyB", label: "B", width: 1 },
    { keyId: "KeyN", label: "N", width: 1 },
    { keyId: "KeyM", label: "M", width: 1 },
    { keyId: "Comma", label: ",", width: 1 },
    { keyId: "Dot", label: ".", width: 1 },
    { keyId: "Slash", label: "/", width: 1 },
    { keyId: "ShiftRight", label: "Shift", width: 2.75 },
  ],
  [
    { keyId: "ControlLeft", label: "Ctrl", width: 1.25 },
    { keyId: "MetaLeft", label: "Win", width: 1.25 },
    { keyId: "Alt", label: "Alt", width: 1.25 },
    { keyId: "Space", label: "Space", width: 6.25 },
    { keyId: "AltGr", label: "AltGr", width: 1.25 },
    { keyId: "MetaRight", label: "Win", width: 1.25 },
    { keyId: "Menu", label: "Menu", width: 1.25 },
    { keyId: "ControlRight", label: "Ctrl", width: 1.25 },
  ],
];

function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route path="/" element={<HomeRedirect />} />
        <Route path="/reports/:sessionId" element={<ReportPage />} />
        <Route path="*" element={<Navigate to="/" replace />} />
      </Routes>
    </BrowserRouter>
  );
}

function HomeRedirect() {
  const location = useLocation();
  const navigate = useNavigate();
  const token = useToken();
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [hasData, setHasData] = useState(false);

  useEffect(() => {
    let cancelled = false;
    fetchJson<HistoryEntry[]>(`/api/sessions/recent?limit=1&token=${token}`)
      .then((history) => {
        if (cancelled) return;
        if (history.length > 0) {
          navigate(`/reports/${history[0].session_id}${location.search}`, { replace: true });
          return;
        }
        setHasData(false);
      })
      .catch((reason: unknown) => {
        if (cancelled) return;
        setError(reason instanceof Error ? reason.message : "Failed to load sessions");
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });

    return () => {
      cancelled = true;
    };
  }, [location.search, navigate, token]);

  if (loading) return <Shell><LoadingState /></Shell>;
  if (error) return <Shell><ErrorState message={error} /></Shell>;
  if (!hasData) {
    return (
      <Shell>
        <EmptyState title="No recorded sessions yet" message="Start and stop a keystroke session from the CLI, then reopen the report UI." />
      </Shell>
    );
  }
  return null;
}

function ReportPage() {
  const { sessionId = "" } = useParams();
  const token = useToken();
  const [data, setData] = useState<ReportResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);

    fetchJson<ReportResponse>(`/api/sessions/${sessionId}/report?token=${token}`)
      .then((report) => {
        if (!cancelled) setData(report);
      })
      .catch((reason: unknown) => {
        if (!cancelled) {
          setError(reason instanceof Error ? reason.message : "Failed to load report");
          setData(null);
        }
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });

    return () => {
      cancelled = true;
    };
  }, [sessionId, token]);

  if (loading) return <Shell><LoadingState /></Shell>;
  if (error) return <Shell><ErrorState message={error} /></Shell>;
  if (!data) {
    return (
      <Shell>
        <EmptyState title="Session not found" message="The requested session could not be loaded from the local SQLite store." />
      </Shell>
    );
  }

  return (
    <Shell>
      <Dashboard report={data} />
    </Shell>
  );
}

function Dashboard({ report }: { report: ReportResponse }) {
  const topKeys = report.key_usage.slice(0, 10);
  return (
    <div className="dashboard">
      <header className="hero">
        <div>
          <div className="eyebrow">Keystroke Visualizer</div>
          <h1>{report.session.session_name ?? `Session ${report.session.session_id.slice(0, 8)}`}</h1>
          <p className="hero-copy">
            Local-only keystroke analytics with aggregate counts only. Session status is{" "}
            <span className={`status status-${report.session.status}`}>{report.session.status}</span>.
          </p>
        </div>
        <div className="hero-meta">
          <div className="hero-card">
            <span>Session ID</span>
            <strong>{report.session.session_id.slice(0, 8)}</strong>
          </div>
          <div className="hero-card">
            <span>Started</span>
            <strong>{formatDateTime(report.session.started_at)}</strong>
          </div>
          <div className="hero-card">
            <span>Duration</span>
            <strong>{formatDuration(report.session.duration_seconds)}</strong>
          </div>
        </div>
      </header>

      <section className="stats-grid">
        <MetricCard label="Total Keypresses" value={formatNumber(report.summary.total_keypresses)} note="All keydown events observed in this session." />
        <MetricCard label="Unique Keys" value={String(report.summary.unique_keys)} note="Distinct key identities captured by the collector." />
        <MetricCard label="Peak Minute" value={report.summary.peak_minute ? formatNumber(report.summary.peak_minute) : "n/a"} note={report.summary.peak_minute_bucket ?? "No minute buckets recorded"} />
        <MetricCard label="Avg Keys / Min" value={report.summary.avg_keys_per_minute.toFixed(2)} note="Average intensity over the measured session duration." />
        <MetricCard label="Dropped Events" value={String(report.summary.dropped_events)} note={report.session.clean_shutdown ? "Collector exited cleanly." : "Collector did not report a clean shutdown."} />
      </section>

      <section className="main-grid">
        <Panel title="Activity Timeline" subtitle="Minute-by-minute keystroke volume across the session.">
          <TimelineChart points={report.activity} />
        </Panel>
        <Panel title="Top Keys" subtitle="Most frequently used keys ranked by raw count.">
          <TopKeysList items={topKeys} />
        </Panel>
      </section>

      <section className="content-grid">
        <Panel title="Keyboard Heatmap" subtitle={`Canonical ${report.keyboard.layout} keyboard layout with per-key intensity.`}>
          <KeyboardHeatmap keys={report.keyboard.keys} />
        </Panel>

        <div className="side-grid">
          <Panel title="Recent Sessions" subtitle="Read-only local history from the same device.">
            <SessionHistory currentSessionId={report.session.session_id} sessions={report.history} />
          </Panel>
        </div>
      </section>

      <section className="table-grid">
        <Panel title="Detailed Analytics" subtitle="Every captured key sorted by count and contribution to the session.">
          <DetailedAnalytics items={report.key_usage} />
        </Panel>
      </section>
    </div>
  );
}

function MetricCard({ label, value, note }: { label: string; value: string; note: string }) {
  return (
    <article className="metric-card">
      <span>{label}</span>
      <strong>{value}</strong>
      <p>{note}</p>
    </article>
  );
}

function Panel({ title, subtitle, children }: { title: string; subtitle: string; children: ReactNode }) {
  return (
    <section className="panel">
      <div className="panel-head">
        <div>
          <h2>{title}</h2>
          <p>{subtitle}</p>
        </div>
      </div>
      {children}
    </section>
  );
}

function TimelineChart({ points }: { points: ReportResponse["activity"] }) {
  const path = useMemo(() => buildTimelinePath(points), [points]);
  const peak = useMemo(() => points.reduce((acc, item) => Math.max(acc, item.keypresses), 0), [points]);

  if (points.length === 0) {
    return <EmptyInline message="No minute buckets recorded for this session." />;
  }

  return (
    <div className="chart-shell">
      <svg viewBox="0 0 720 280" className="chart">
        <defs>
          <linearGradient id="timelineGradient" x1="0%" y1="0%" x2="100%" y2="0%">
            <stop offset="0%" stopColor="#6ef2c5" />
            <stop offset="100%" stopColor="#61c3ff" />
          </linearGradient>
        </defs>
        <line x1="48" y1="18" x2="48" y2="236" className="chart-axis" />
        <line x1="48" y1="236" x2="690" y2="236" className="chart-axis" />
        <path d={path.area} className="chart-area" />
        <path d={path.line} className="chart-line" />
        {points.map((point, index) => {
          const x = 48 + (index / Math.max(points.length - 1, 1)) * 642;
          const y = 236 - (point.keypresses / Math.max(peak, 1)) * 190;
          return <circle key={point.minute_bucket} cx={x} cy={y} r="2.2" className="chart-dot" />;
        })}
      </svg>
      <div className="timeline-labels">
        <span>{formatMinuteBucket(points[0]?.minute_bucket)}</span>
        <span>{formatMinuteBucket(points[Math.floor(points.length / 2)]?.minute_bucket)}</span>
        <span>{formatMinuteBucket(points[points.length - 1]?.minute_bucket)}</span>
      </div>
    </div>
  );
}

function TopKeysList({ items }: { items: ReportResponse["key_usage"] }) {
  if (items.length === 0) {
    return <EmptyInline message="No key usage recorded yet." />;
  }

  const max = items[0]?.count ?? 1;
  return (
    <div className="bars">
      {items.map((item) => (
        <div key={item.key_id} className="bar-row">
          <div className="bar-meta">
            <strong>{item.display_label}</strong>
            <span>{formatNumber(item.count)} · {item.share_percent.toFixed(2)}%</span>
          </div>
          <div className="bar-track">
            <div className="bar-fill" style={{ width: `${(item.count / max) * 100}%` }} />
          </div>
        </div>
      ))}
    </div>
  );
}

function KeyboardHeatmap({ keys }: { keys: ReportResponse["keyboard"]["keys"] }) {
  const keyMap = new Map(keys.map((key) => [key.key_id, key]));
  return (
    <div className="keyboard">
      {KEYBOARD_TEMPLATE.map((row, index) => (
        <div key={index} className="keyboard-row">
          {row.map((templateKey) => {
            const match = keyMap.get(templateKey.keyId);
            const intensity = match?.intensity ?? 0;
            return (
              <div
                key={templateKey.keyId}
                className="keyboard-key"
                title={`${templateKey.label}: ${formatNumber(match?.count ?? 0)}`}
                style={{
                  flex: templateKey.width,
                  background: heatColor(intensity),
                  borderColor: intensity > 0.02 ? "rgba(123,255,208,0.28)" : "rgba(255,255,255,0.08)",
                }}
              >
                <span>{templateKey.label}</span>
                <small>{match?.count ?? 0}</small>
              </div>
            );
          })}
        </div>
      ))}
    </div>
  );
}

function SessionHistory({ currentSessionId, sessions }: { currentSessionId: string; sessions: HistoryEntry[] }) {
  const location = useLocation();
  const navigate = useNavigate();
  if (sessions.length === 0) {
    return <EmptyInline message="No other sessions available yet." />;
  }

  return (
    <div className="history-list">
      {sessions.map((session) => (
        <button
          key={session.session_id}
          className={`history-item ${session.session_id === currentSessionId ? "history-item-active" : ""}`}
          onClick={() => navigate(`/reports/${session.session_id}${location.search}`)}
          type="button"
        >
          <div>
            <strong>{session.name ?? `Session ${session.session_id.slice(0, 8)}`}</strong>
            <p>{formatDateTime(session.started_at)}</p>
          </div>
          <div className="history-metrics">
            <span>{formatNumber(session.total_keypresses)}</span>
            <small>{formatDuration(session.duration_seconds)}</small>
          </div>
        </button>
      ))}
    </div>
  );
}

function DetailedAnalytics({ items }: { items: ReportResponse["key_usage"] }) {
  if (items.length === 0) {
    return <EmptyInline message="No detailed key usage available." />;
  }

  return (
    <div className="table-shell">
      <table>
        <thead>
          <tr>
            <th>Key</th>
            <th>Count</th>
            <th>Share</th>
          </tr>
        </thead>
        <tbody>
          {items.map((item) => (
            <tr key={item.key_id}>
              <td>{item.display_label}</td>
              <td>{formatNumber(item.count)}</td>
              <td>
                <div className="share-cell">
                  <span>{item.share_percent.toFixed(2)}%</span>
                  <div className="share-track">
                    <div className="share-fill" style={{ width: `${Math.min(item.share_percent, 100)}%` }} />
                  </div>
                </div>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function Shell({ children }: { children: ReactNode }) {
  return (
    <div className="app-shell">
      <div className="background-grid" />
      <main className="container">{children}</main>
    </div>
  );
}

function LoadingState() {
  return (
    <div className="state-card">
      <div className="spinner" />
      <h2>Loading local report</h2>
      <p>Reading session data from the Rust backend and rendering the dashboard.</p>
    </div>
  );
}

function ErrorState({ message }: { message: string }) {
  return (
    <div className="state-card">
      <h2>Unable to render report</h2>
      <p>{message}</p>
    </div>
  );
}

function EmptyState({ title, message }: { title: string; message: string }) {
  return (
    <div className="state-card">
      <h2>{title}</h2>
      <p>{message}</p>
    </div>
  );
}

function EmptyInline({ message }: { message: string }) {
  return <div className="empty-inline">{message}</div>;
}

function useToken() {
  const location = useLocation();
  return useMemo(() => new URLSearchParams(location.search).get("token") ?? "", [location.search]);
}

async function fetchJson<T>(url: string): Promise<T> {
  const response = await fetch(url);
  if (!response.ok) {
    const payload = (await response.json().catch(() => null)) as ApiError | null;
    throw new Error(payload?.error ?? `HTTP ${response.status}`);
  }
  return (await response.json()) as T;
}

function buildTimelinePath(points: ReportResponse["activity"]) {
  if (points.length === 0) return { line: "", area: "" };
  const max = Math.max(...points.map((point) => point.keypresses), 1);
  const coordinates = points.map((point, index) => {
    const x = 48 + (index / Math.max(points.length - 1, 1)) * 642;
    const y = 236 - (point.keypresses / max) * 190;
    return { x, y };
  });
  const line = coordinates
    .map((point, index) => `${index === 0 ? "M" : "L"} ${point.x.toFixed(2)} ${point.y.toFixed(2)}`)
    .join(" ");
  const area = `${line} L 690 236 L 48 236 Z`;
  return { line, area };
}

function heatColor(intensity: number) {
  const clamped = Math.max(0, Math.min(1, intensity));
  const cold = [19, 30, 52];
  const hot = [111, 242, 197];
  const rgb = cold.map((value, index) => Math.round(value + (hot[index] - value) * clamped));
  return `linear-gradient(180deg, rgba(${rgb.join(",")}, 0.92), rgba(12, 18, 33, 0.92))`;
}

function formatDateTime(value: string | null) {
  if (!value) return "running";
  return new Date(value).toLocaleString(undefined, {
    month: "short",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function formatMinuteBucket(value?: string) {
  if (!value) return "n/a";
  const date = new Date(value.replace(" ", "T"));
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" });
}

function formatDuration(totalSeconds: number) {
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;
  if (hours > 0) return `${hours}h ${minutes}m ${seconds}s`;
  if (minutes > 0) return `${minutes}m ${seconds}s`;
  return `${seconds}s`;
}

function formatNumber(value: number) {
  return value.toLocaleString();
}

export default App;
