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

type TimelineBar = {
  key: string;
  keypresses: number;
  representativeBucket: string;
};

type KeyboardTemplateRow = {
  keyId: string;
  label: string;
  width?: number;
}[];

const SIGNATURE_KEYBOARD_TEMPLATE: KeyboardTemplateRow[] = [
  [
    { keyId: "KeyQ", label: "Q" },
    { keyId: "KeyW", label: "W" },
    { keyId: "KeyE", label: "E" },
    { keyId: "KeyR", label: "R" },
    { keyId: "KeyT", label: "T" },
    { keyId: "KeyY", label: "Y" },
    { keyId: "KeyU", label: "U" },
    { keyId: "KeyI", label: "I" },
    { keyId: "KeyO", label: "O" },
    { keyId: "KeyP", label: "P" },
  ],
  [
    { keyId: "KeyA", label: "A" },
    { keyId: "KeyS", label: "S" },
    { keyId: "KeyD", label: "D" },
    { keyId: "KeyF", label: "F" },
    { keyId: "KeyG", label: "G" },
    { keyId: "KeyH", label: "H" },
    { keyId: "KeyJ", label: "J" },
    { keyId: "KeyK", label: "K" },
    { keyId: "KeyL", label: "L" },
  ],
  [
    { keyId: "KeyZ", label: "Z" },
    { keyId: "KeyX", label: "X" },
    { keyId: "KeyC", label: "C" },
    { keyId: "KeyV", label: "V" },
    { keyId: "KeyB", label: "B" },
    { keyId: "KeyN", label: "N" },
    { keyId: "KeyM", label: "M" },
  ],
  [{ keyId: "Space", label: "SPACE", width: 6.5 }],
];

const LEFT_HAND_KEYS = new Set(["KeyQ", "KeyW", "KeyE", "KeyR", "KeyT", "KeyA", "KeyS", "KeyD", "KeyF", "KeyG", "KeyZ", "KeyX", "KeyC", "KeyV", "KeyB", "ShiftLeft", "ControlLeft", "Alt", "MetaLeft", "Tab", "CapsLock", "BackQuote", "Num1", "Num2", "Num3", "Num4", "Num5"]);
const RIGHT_HAND_KEYS = new Set(["KeyY", "KeyU", "KeyI", "KeyO", "KeyP", "KeyH", "KeyJ", "KeyK", "KeyL", "KeyN", "KeyM", "ShiftRight", "ControlRight", "AltGr", "MetaRight", "Menu", "Backspace", "Return", "BackSlash", "RightBracket", "LeftBracket", "Minus", "Equal", "Num6", "Num7", "Num8", "Num9", "Num0"]);
const MOBILE_TABS = ["Home", "Metrics", "Keys", "Settings"] as const;

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

  return <Shell><Dashboard report={data} /></Shell>;
}

function Dashboard({ report }: { report: ReportResponse }) {
  const [showAllRows, setShowAllRows] = useState(false);
  const topKeys = report.key_usage.slice(0, 4);
  const tableItems = showAllRows ? report.key_usage : report.key_usage.slice(0, 25);
  const timelineBars = useMemo(() => buildTimelineBars(report.activity, 20), [report.activity]);
  const safetyIndex = useMemo(() => calculateSafetyIndex(report.summary.total_keypresses, report.summary.dropped_events, report.session.clean_shutdown), [report.session.clean_shutdown, report.summary.dropped_events, report.summary.total_keypresses]);
  const dominantHand = useMemo(() => computeDominantHand(report.keyboard.keys), [report.keyboard.keys]);
  const hotZone = useMemo(() => computeHotZone(report.keyboard.keys), [report.keyboard.keys]);
  const fatigueIndicator = useMemo(() => computeFatigueIndicator(report.key_usage), [report.key_usage]);

  return (
    <div className="report-page">
      <header className="hero-section">
        <div className="hero-copyblock">
          <div className="hero-meta-row">
            <span className={`status-chip status-chip-${report.session.status}`}>{formatStatusLabel(report.session.status)}</span>
            <span className="hero-session-id">ID: {report.session.session_id.slice(0, 12)}</span>
          </div>
          <h1 className="hero-title">{report.session.session_name ?? `Session ${report.session.session_id.slice(0, 8)}`}</h1>
          <p className="hero-description">Recorded session analyzing temporal typing intensity and ergonomic patterns. Data is processed locally with aggregate counts only and no raw keystroke content retained.</p>
        </div>
        <div className="hero-stats">
          <MetricReadout label="Session Duration" value={formatClockDuration(report.session.duration_seconds)} />
          <div className="hero-divider" />
          <MetricReadout label="Safety Index" value={safetyIndex} />
        </div>
      </header>

      <section className="kpi-strip" aria-label="Session summary">
        <KpiCard label="Total Keypresses" value={formatNumber(report.summary.total_keypresses)} accent="strong" detail={report.activity.length > 0 ? `${report.activity.length} minute buckets` : "No minute buckets"} />
        <KpiCard label="Unique Keys" value={String(report.summary.unique_keys)} detail={report.keyboard.layout} />
        <KpiCard label="Peak Minute" value={report.summary.peak_minute ? formatNumber(report.summary.peak_minute) : "n/a"} detail={formatMinuteBucket(report.summary.peak_minute_bucket)} />
        <KpiCard label="Avg. KPM" value={report.summary.avg_keys_per_minute.toFixed(1)} accent="strong" detail={fatigueIndicator} />
      </section>
      <div className="dashboard-grid">
        <div className="primary-column">
          <Panel
            title="Temporal Density Analysis"
            subtitle="Minute-by-minute velocity during the captured session lifecycle."
            headingAdornment={<div className="legend"><LegendItem label="Velocity" tone="legend-bright" /><LegendItem label="Median" tone="legend-muted" /></div>}
          >
            <TimelineChart bars={timelineBars} />
          </Panel>

          <Panel
            title="Biometric Fingerprint (Heatmap)"
            subtitle={`Canonical ${report.keyboard.layout} alpha cluster rendered with discrete usage intensity.`}
            headingAdornment={<HeatLegend />}
            className="heatmap-panel"
          >
            <SignatureHeatmap keys={report.keyboard.keys} />
            <div className="heatmap-metadata">
              <HeatmapStat label="Primary Hand" value={dominantHand} />
              <HeatmapStat label="Hot Zone" value={hotZone} />
              <HeatmapStat label="Fatigue Indicator" value={fatigueIndicator} />
            </div>
          </Panel>
        </div>

        <aside className="secondary-column">
          <Panel title="Top-Key Distribution" subtitle="Most frequent keys ranked by share.">
            <TopKeysList items={topKeys} />
          </Panel>

          <Panel title="Contextual Archive" subtitle="Recent local sessions from this device.">
            <SessionHistory currentSessionId={report.session.session_id} sessions={report.history.slice(0, 3)} />
          </Panel>

          <Panel title="Data Integrity" subtitle="Local-first retention and collector state.">
            <IntegrityPanel report={report} safetyIndex={safetyIndex} />
          </Panel>
        </aside>
      </div>

      <Panel
        title="Detailed Key Metrics"
        subtitle="Captured key usage sorted by contribution to the current session."
        className="table-panel"
        headingAdornment={
          <div className="panel-actions">
            <button className="panel-action" type="button" onClick={() => setShowAllRows((value) => !value)}>{showAllRows ? "Top 25" : "All Keys"}</button>
            <button className="panel-action" type="button" onClick={() => downloadCsv(report.key_usage)}>Download CSV</button>
          </div>
        }
      >
        <DetailedAnalytics items={tableItems} />
      </Panel>
    </div>
  );
}

function Shell({ children }: { children: ReactNode }) {
  return (
    <div className="app-shell">
      <TopBar />
      <SideRail />
      <main className="app-main"><div className="content-canvas">{children}</div></main>
      <MobileNav />
    </div>
  );
}

function TopBar() {
  return (
    <nav className="topbar">
      <div className="topbar-brand">KEYSTROKE.ANALYTICS</div>
      <div className="topbar-links">
        <span className="topbar-link topbar-link-active">Reports</span>
        <span className="topbar-link">Intelligence</span>
        <span className="topbar-link">Archive</span>
        <span className="topbar-link">Settings</span>
      </div>
      <div className="topbar-actions">
        <button className="toolbar-button" type="button" aria-label="Notifications"><BellIcon /></button>
        <button className="toolbar-button" type="button" aria-label="Security"><ShieldIcon /></button>
        <div className="avatar-badge">AN</div>
      </div>
    </nav>
  );
}

function SideRail() {
  return (
    <aside className="siderail">
      <div className="siderail-brand">
        <div className="siderail-title">ANALYTICS_V1</div>
        <div className="siderail-subtitle">Local-First Encryption</div>
      </div>
      <nav className="siderail-nav">
        <SideNavItem label="Dashboard" />
        <SideNavItem label="Live Stream" active />
        <SideNavItem label="Heatmaps" />
        <SideNavItem label="Privacy Logs" />
        <SideNavItem label="Export" />
      </nav>
      <button className="new-session-button" type="button">New Session</button>
      <div className="siderail-footer">
        <div className="siderail-footer-link">Support</div>
        <div className="siderail-footer-link">System Status</div>
      </div>
    </aside>
  );
}

function SideNavItem({ label, active = false }: { label: string; active?: boolean }) {
  return <div className={`siderail-item${active ? " siderail-item-active" : ""}`}><span className="siderail-marker" /><span>{label}</span></div>;
}

function MobileNav() {
  return (
    <nav className="mobile-nav">
      {MOBILE_TABS.map((label) => (
        <div key={label} className={`mobile-nav-item${label === "Metrics" ? " mobile-nav-item-active" : ""}`}>
          <span className="mobile-nav-dot" />
          <span>{label}</span>
        </div>
      ))}
    </nav>
  );
}

function Panel({ title, subtitle, children, className = "", headingAdornment }: { title: string; subtitle: string; children: ReactNode; className?: string; headingAdornment?: ReactNode; }) {
  return (
    <section className={`panel ${className}`.trim()}>
      <div className="panel-header">
        <div>
          <h2>{title}</h2>
          <p>{subtitle}</p>
        </div>
        {headingAdornment ? <div className="panel-header-side">{headingAdornment}</div> : null}
      </div>
      {children}
    </section>
  );
}

function MetricReadout({ label, value }: { label: string; value: string }) {
  return <div className="hero-readout"><p>{label}</p><strong>{value}</strong></div>;
}

function KpiCard({ label, value, detail, accent = "muted" }: { label: string; value: string; detail: string; accent?: "muted" | "strong"; }) {
  return <article className={`kpi-card kpi-card-${accent}`}><p>{label}</p><div className="kpi-value-row"><strong>{value}</strong><span>{detail}</span></div></article>;
}

function LegendItem({ label, tone }: { label: string; tone: string }) {
  return <span className="legend-item"><span className={`legend-swatch ${tone}`} /><span>{label}</span></span>;
}

function HeatLegend() {
  return <div className="heat-legend"><span>Low</span><div className="heat-legend-bar"><span className="heat-legend-step heat-level-1" /><span className="heat-legend-step heat-level-2" /><span className="heat-legend-step heat-level-3" /><span className="heat-legend-step heat-level-4" /></div><span>High</span></div>;
}

function TimelineChart({ bars }: { bars: TimelineBar[] }) {
  if (bars.length === 0) return <EmptyInline message="No minute buckets recorded for this session." />;

  const max = Math.max(...bars.map((bar) => bar.keypresses), 1);
  const median = medianOf(bars.map((bar) => bar.keypresses));
  const labels = buildTimelineLabels(bars);

  return (
    <div className="timeline-shell">
      <div className="timeline-bars" role="img" aria-label="Activity timeline">
        {bars.map((bar, index) => {
          const height = Math.max(10, Math.round((bar.keypresses / max) * 100));
          return (
            <div key={bar.key} className="timeline-bar" title={`${formatMinuteBucket(bar.representativeBucket)}: ${formatNumber(bar.keypresses)}`}>
              <div className={`timeline-fill ${pickTimelineTone(bar.keypresses, max, median)}`} style={{ height: `${height}%` }} />
              {index === bars.length - 1 ? <div className="timeline-last-bar-marker" /> : null}
            </div>
          );
        })}
      </div>
      <div className="timeline-labels">{labels.map((label, index) => <span key={`${label}-${index}`}>{label}</span>)}</div>
    </div>
  );
}

function TopKeysList({ items }: { items: ReportResponse["key_usage"] }) {
  if (items.length === 0) return <EmptyInline message="No key usage recorded yet." />;
  const max = items[0]?.count ?? 1;
  return (
    <div className="top-keys">
      {items.map((item) => (
        <div key={item.key_id} className="top-key-row">
          <div className="top-key-label">{normalizeKeyLabel(item.display_label)}</div>
          <div className="top-key-track">
            <div className="top-key-fill" style={{ width: `${(item.count / max) * 100}%` }} />
          </div>
          <div className="top-key-count">{formatNumber(item.count)}</div>
        </div>
      ))}
    </div>
  );
}

function SignatureHeatmap({ keys }: { keys: ReportResponse["keyboard"]["keys"] }) {
  const keyMap = new Map(keys.map((key) => [key.key_id, key]));
  return <div className="signature-heatmap">{SIGNATURE_KEYBOARD_TEMPLATE.map((row, rowIndex) => <div key={rowIndex} className={`signature-row signature-row-${rowIndex + 1}`}>{row.map((templateKey) => { const match = keyMap.get(templateKey.keyId); return <div key={templateKey.keyId} className={`signature-key heat-level-${pickHeatLevel(match?.intensity ?? 0)}`} style={templateKey.width ? { flex: templateKey.width } : undefined} title={`${templateKey.label}: ${formatNumber(match?.count ?? 0)}`}>{templateKey.label}</div>; })}</div>)}</div>;
}

function HeatmapStat({ label, value }: { label: string; value: string }) {
  return <div className="heatmap-stat"><p>{label}</p><strong>{value}</strong></div>;
}
function SessionHistory({ currentSessionId, sessions }: { currentSessionId: string; sessions: HistoryEntry[] }) {
  const location = useLocation();
  const navigate = useNavigate();
  if (sessions.length === 0) return <EmptyInline message="No recent sessions available yet." />;

  return (
    <div className="archive-list">
      {sessions.map((session) => {
        const isCurrent = session.session_id === currentSessionId;
        return (
          <button
            key={session.session_id}
            className={`archive-item${isCurrent ? " archive-item-current" : ""}`}
            onClick={() => navigate(`/reports/${session.session_id}${location.search}`)}
            type="button"
          >
            <div className="archive-head"><span>{isCurrent ? "Current" : "Archive"}</span><span>{formatHistoryRelativeLabel(session.started_at)}</span></div>
            <strong>{session.name ?? `Session ${session.session_id.slice(0, 8)}`}</strong>
            <p>{formatNumber(session.total_keypresses)} keys • {formatDuration(session.duration_seconds)}</p>
          </button>
        );
      })}
    </div>
  );
}

function IntegrityPanel({ report, safetyIndex }: { report: ReportResponse; safetyIndex: string }) {
  return (
    <div className="integrity-card">
      <div className="integrity-title"><ShieldIcon /><span>Collector integrity score {safetyIndex}</span></div>
      <p>This session is stored locally. No raw keystroke content is captured; only temporal metadata, frequency, and canonical key identities are retained.</p>
      <dl className="integrity-list">
        <div><dt>Privacy Mode</dt><dd>{formatPrivacyMode(report.session.privacy_mode)}</dd></div>
        <div><dt>Shutdown</dt><dd>{report.session.clean_shutdown ? "Clean" : "Interrupted"}</dd></div>
        <div><dt>Dropped Events</dt><dd>{formatNumber(report.summary.dropped_events)}</dd></div>
      </dl>
      <span className="integrity-link">Review Security Protocol<ExternalLinkIcon /></span>
    </div>
  );
}

function DetailedAnalytics({ items }: { items: ReportResponse["key_usage"] }) {
  if (items.length === 0) return <EmptyInline message="No detailed key usage available." />;
  const max = items[0]?.count ?? 1;

  return (
    <div className="table-shell">
      <table>
        <thead>
          <tr>
            <th>Key Descriptor</th>
            <th>Total Count</th>
            <th>Global Share</th>
            <th className="column-compact">Usage Tier</th>
            <th className="column-compact column-right">Rank</th>
          </tr>
        </thead>
        <tbody>
          {items.map((item, index) => (
            <tr key={item.key_id}>
              <td className="table-key">{item.display_label}</td>
              <td>{formatNumber(item.count)}</td>
              <td>
                <div className="share-cell">
                  <span>{item.share_percent.toFixed(2)}%</span>
                  <div className="share-track">
                    <div className="share-fill" style={{ width: `${Math.max(4, (item.count / max) * 100)}%` }} />
                  </div>
                </div>
              </td>
              <td className="column-compact"><UsageTier item={item} maxCount={max} /></td>
              <td className="column-compact column-right"><RankBadge index={index} count={item.count} maxCount={max} /></td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function UsageTier({ item, maxCount }: { item: ReportResponse["key_usage"][number]; maxCount: number }) {
  const ratio = item.count / Math.max(maxCount, 1);
  let label = "Stable";
  let tone = "usage-stable";
  if (ratio > 0.75) {
    label = "Optimal";
    tone = "usage-optimal";
  } else if (ratio > 0.35) {
    label = "Elevated";
    tone = "usage-elevated";
  } else if (ratio < 0.1) {
    label = "Low";
    tone = "usage-low";
  }
  return <span className={`usage-pill ${tone}`}><span className="usage-dot" />{label}</span>;
}

function RankBadge({ index, count, maxCount }: { index: number; count: number; maxCount: number }) {
  const ratio = count / Math.max(maxCount, 1);
  if (ratio > 0.75) return <span className="rank-indicator"><ArrowUpIcon />#{index + 1}</span>;
  if (ratio < 0.12) return <span className="rank-indicator rank-indicator-muted"><ArrowDownIcon />#{index + 1}</span>;
  return <span className="rank-indicator rank-indicator-neutral"><MinusIcon />#{index + 1}</span>;
}

function LoadingState() {
  return <div className="state-card"><div className="state-spinner" /><h2>Loading local report</h2><p>Reading session data from the Rust backend and reconstructing the analytics canvas.</p></div>;
}

function ErrorState({ message }: { message: string }) {
  return <div className="state-card"><h2>Unable to render report</h2><p>{message}</p></div>;
}

function EmptyState({ title, message }: { title: string; message: string }) {
  return <div className="state-card"><h2>{title}</h2><p>{message}</p></div>;
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

function buildTimelineBars(points: ReportResponse["activity"], maxBars: number): TimelineBar[] {
  if (points.length === 0) return [];
  const bucketCount = Math.min(maxBars, points.length);
  const bars: TimelineBar[] = [];

  for (let index = 0; index < bucketCount; index += 1) {
    const start = Math.floor((index * points.length) / bucketCount);
    const end = Math.floor(((index + 1) * points.length) / bucketCount);
    const slice = points.slice(start, Math.max(end, start + 1));
    const total = slice.reduce((sum, point) => sum + point.keypresses, 0);
    const representative = slice[Math.floor(slice.length / 2)] ?? slice[0];
    bars.push({ key: `${representative.minute_bucket}-${index}`, keypresses: Math.round(total / slice.length), representativeBucket: representative.minute_bucket });
  }

  return bars;
}

function buildTimelineLabels(bars: TimelineBar[]) {
  if (bars.length === 0) return ["n/a"];
  return [0, 0.25, 0.5, 0.75, 1].map((position) => {
    const index = Math.min(bars.length - 1, Math.round(position * (bars.length - 1)));
    return formatMinuteBucket(bars[index]?.representativeBucket);
  });
}

function pickTimelineTone(value: number, max: number, median: number) {
  const ratio = value / Math.max(max, 1);
  if (ratio > 0.92) return "timeline-fill-peak";
  if (value >= median) return "timeline-fill-high";
  if (ratio > 0.35) return "timeline-fill-mid";
  return "timeline-fill-low";
}

function calculateSafetyIndex(total: number, dropped: number, cleanShutdown: boolean) {
  const observed = total + dropped;
  const captureRatio = observed === 0 ? 100 : (total / observed) * 100;
  const adjusted = cleanShutdown ? Math.min(100, captureRatio + 0.2) : Math.max(0, captureRatio - 1.5);
  return `${adjusted.toFixed(1)}%`;
}

function computeDominantHand(keys: ReportResponse["keyboard"]["keys"]) {
  let left = 0;
  let right = 0;
  keys.forEach((key) => {
    if (LEFT_HAND_KEYS.has(key.key_id)) left += key.count;
    if (RIGHT_HAND_KEYS.has(key.key_id)) right += key.count;
  });
  const total = left + right;
  if (total === 0 || left === right) return "Balanced";
  return left > right ? `Left (${((left / total) * 100).toFixed(1)}%)` : `Right (${((right / total) * 100).toFixed(1)}%)`;
}

function computeHotZone(keys: ReportResponse["keyboard"]["keys"]) {
  const zones = { upper: 0, home: 0, lower: 0, thumb: 0 };
  keys.forEach((key) => {
    if (["KeyQ", "KeyW", "KeyE", "KeyR", "KeyT", "KeyY", "KeyU", "KeyI", "KeyO", "KeyP"].includes(key.key_id)) zones.upper += key.count;
    if (["KeyA", "KeyS", "KeyD", "KeyF", "KeyG", "KeyH", "KeyJ", "KeyK", "KeyL"].includes(key.key_id)) zones.home += key.count;
    if (["KeyZ", "KeyX", "KeyC", "KeyV", "KeyB", "KeyN", "KeyM"].includes(key.key_id)) zones.lower += key.count;
    if (key.key_id === "Space") zones.thumb += key.count;
  });
  const dominant = Object.entries(zones).sort((left, right) => right[1] - left[1])[0]?.[0] ?? "home";
  if (dominant === "upper") return "Upper Row";
  if (dominant === "lower") return "Lower Row";
  if (dominant === "thumb") return "Thumb Cluster";
  return "Home Row - Center";
}

function computeFatigueIndicator(items: ReportResponse["key_usage"]) {
  const backspace = items.find((item) => item.key_id === "Backspace" || item.display_label.toLowerCase().includes("backspace"));
  const ratio = backspace?.share_percent ?? 0;
  if (ratio > 6) return "Elevated";
  if (ratio > 2.5) return "Moderate";
  return "Low Threshold";
}

function pickHeatLevel(intensity: number) {
  const clamped = Math.max(0, Math.min(1, intensity));
  if (clamped === 0) return 0;
  if (clamped < 0.2) return 1;
  if (clamped < 0.45) return 2;
  if (clamped < 0.7) return 3;
  return 4;
}
function downloadCsv(items: ReportResponse["key_usage"]) {
  const lines = ["key_id,display_label,count,share_percent"];
  items.forEach((item) => {
    lines.push([
      item.key_id,
      escapeCsv(item.display_label),
      item.count.toString(),
      item.share_percent.toFixed(4),
    ].map((field) => (field.includes(",") || field.includes('"') ? `"${field.replaceAll('"', '""')}"` : field)).join(","));
  });
  const blob = new Blob([lines.join("\n")], { type: "text/csv;charset=utf-8" });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = "key-metrics.csv";
  link.click();
  URL.revokeObjectURL(url);
}

function escapeCsv(value: string) {
  return value.replaceAll("\n", " ");
}

function medianOf(values: number[]) {
  if (values.length === 0) return 0;
  const sorted = [...values].sort((left, right) => left - right);
  const midpoint = Math.floor(sorted.length / 2);
  return sorted.length % 2 === 0 ? (sorted[midpoint - 1] + sorted[midpoint]) / 2 : sorted[midpoint];
}

function formatStatusLabel(value: ReportResponse["session"]["status"]) {
  if (value === "running") return "Running";
  if (value === "stopped") return "Completed";
  if (value === "interrupted") return "Interrupted";
  return "Failed";
}

function formatPrivacyMode(value: string) {
  return value.split(/[_-\s]+/).filter(Boolean).map((part) => part.charAt(0).toUpperCase() + part.slice(1)).join(" ");
}

function normalizeKeyLabel(label: string) {
  if (label.length <= 3) return label.toUpperCase();
  if (label === "Space" || label === "Spacebar") return "SP";
  return label.slice(0, 2).toUpperCase();
}

function formatDateTime(value: string | null) {
  if (!value) return "running";
  return new Date(value).toLocaleString(undefined, { month: "short", day: "2-digit", hour: "2-digit", minute: "2-digit" });
}

function formatMinuteBucket(value: string | null | undefined) {
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

function formatClockDuration(totalSeconds: number) {
  const hours = Math.floor(totalSeconds / 3600).toString().padStart(2, "0");
  const minutes = Math.floor((totalSeconds % 3600) / 60).toString().padStart(2, "0");
  const seconds = Math.floor(totalSeconds % 60).toString().padStart(2, "0");
  return `${hours}:${minutes}:${seconds}`;
}

function formatNumber(value: number) {
  return value.toLocaleString();
}

function formatHistoryRelativeLabel(value: string) {
  const sessionDate = new Date(value);
  if (Number.isNaN(sessionDate.getTime())) return formatDateTime(value);
  const now = new Date();
  const differenceDays = Math.floor((now.getTime() - sessionDate.getTime()) / 86_400_000);
  if (differenceDays <= 0) return formatMinuteBucket(value);
  if (differenceDays === 1) return "Yesterday";
  return `${differenceDays}d ago`;
}

function BellIcon() {
  return <svg className="inline-icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M12 4a4 4 0 0 0-4 4v2.4c0 .7-.2 1.3-.7 1.8L6 13.5V15h12v-1.5l-1.3-1.3c-.5-.5-.7-1.1-.7-1.8V8a4 4 0 0 0-4-4Z" /><path d="M10 18a2 2 0 0 0 4 0" /></svg>;
}

function ShieldIcon() {
  return <svg className="inline-icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M12 3 6 5.5v5.4c0 4.2 2.4 8 6 9.8 3.6-1.8 6-5.6 6-9.8V5.5L12 3Z" /><path d="m9.5 12 1.8 1.8 3.5-3.8" /></svg>;
}

function ExternalLinkIcon() {
  return <svg className="inline-icon inline-icon-sm" viewBox="0 0 24 24" aria-hidden="true"><path d="M8 8h8v8" /><path d="m8 16 8-8" /></svg>;
}

function ArrowUpIcon() {
  return <svg className="inline-icon inline-icon-sm" viewBox="0 0 24 24" aria-hidden="true"><path d="m6 14 6-6 6 6" /></svg>;
}

function ArrowDownIcon() {
  return <svg className="inline-icon inline-icon-sm" viewBox="0 0 24 24" aria-hidden="true"><path d="m6 10 6 6 6-6" /></svg>;
}

function MinusIcon() {
  return <svg className="inline-icon inline-icon-sm" viewBox="0 0 24 24" aria-hidden="true"><path d="M6 12h12" /></svg>;
}

export default App;
