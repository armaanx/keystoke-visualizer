use crate::model::{
    CollectorFlush, KeyboardLayout, RuntimeInfo, SessionRecord, SessionSnapshot, SessionStatus,
};
use anyhow::{Context, Result, anyhow, bail};
use chrono::{DateTime, Utc};
use directories::ProjectDirs;
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use std::fs;
use std::path::PathBuf;

const SCHEMA_VERSION: i64 = 1;

#[derive(Clone, Debug)]
pub struct AppPaths {
    pub root: PathBuf,
    pub sessions_dir: PathBuf,
    pub db_path: PathBuf,
}

pub fn app_paths() -> Result<AppPaths> {
    let dirs = ProjectDirs::from("local", "keystroke", "visualizer")
        .context("unable to resolve a writable application data directory")?;
    let root = dirs.data_local_dir().to_path_buf();
    let sessions_dir = root.join("sessions");
    fs::create_dir_all(&sessions_dir).context("creating data directories")?;
    Ok(AppPaths {
        root: root.clone(),
        sessions_dir,
        db_path: root.join("keystroke-visualizer.db"),
    })
}

pub struct Repository {
    conn: Connection,
}

impl Repository {
    pub fn open(paths: &AppPaths) -> Result<Self> {
        if let Some(parent) = paths.db_path.parent() {
            fs::create_dir_all(parent).context("creating database directory")?;
        }
        let conn = Connection::open(&paths.db_path)
            .with_context(|| format!("opening {}", paths.db_path.display()))?;
        conn.pragma_update(None, "journal_mode", "WAL")
            .context("enabling sqlite WAL mode")?;
        conn.pragma_update(None, "foreign_keys", "ON")
            .context("enabling sqlite foreign keys")?;
        let repo = Self { conn };
        repo.migrate()?;
        Ok(repo)
    }

    pub fn schema_version(&self) -> Result<i64> {
        Ok(self
            .conn
            .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
                row.get::<_, Option<i64>>(0)
            })
            .context("reading schema version")?
            .unwrap_or(0))
    }

    pub fn create_session(&mut self, snapshot: &SessionSnapshot) -> Result<()> {
        let tx = self
            .conn
            .transaction()
            .context("starting session creation tx")?;
        insert_session(&tx, snapshot)?;
        tx.commit().context("committing session creation tx")?;
        Ok(())
    }

    pub fn attach_runtime(
        &mut self,
        session_id: &str,
        pid: u32,
        control_token: &str,
        daemon_started_at: DateTime<Utc>,
    ) -> Result<()> {
        let tx = self
            .conn
            .transaction()
            .context("starting runtime attach tx")?;
        tx.execute(
            "INSERT INTO active_runtime (
                session_id, pid, control_port, control_token, daemon_started_at, heartbeat_at, flush_seq
             ) VALUES (?1, ?2, NULL, ?3, ?4, NULL, 0)",
            params![
                session_id,
                i64::from(pid),
                control_token,
                encode_dt(daemon_started_at),
            ],
        )
        .context("inserting active runtime")?;
        tx.commit().context("committing runtime attach tx")?;
        Ok(())
    }

    pub fn set_runtime_port(&mut self, session_id: &str, control_port: u16) -> Result<()> {
        self.conn
            .execute(
                "UPDATE active_runtime
                 SET control_port = ?2, heartbeat_at = ?3
                 WHERE session_id = ?1",
                params![session_id, i64::from(control_port), encode_dt(Utc::now())],
            )
            .with_context(|| format!("setting runtime port for session {}", session_id))?;
        Ok(())
    }

    pub fn flush_session(
        &mut self,
        session_id: &str,
        flush_seq: u64,
        flush: &CollectorFlush,
    ) -> Result<()> {
        let tx = self.conn.transaction().context("starting flush tx")?;
        tx.execute(
            "UPDATE sessions
             SET total_keypresses = ?2,
                 unique_keys = ?3,
                 dropped_events = ?4,
                 last_updated_at = ?5,
                 last_flush_at = ?6,
                 capture_error = ?7
             WHERE session_id = ?1",
            params![
                session_id,
                flush.total_keypresses as i64,
                flush.unique_keys as i64,
                flush.dropped_events as i64,
                encode_dt(flush.last_updated_at),
                encode_dt(flush.last_flush_at),
                flush.capture_error,
            ],
        )
        .with_context(|| format!("updating session {}", session_id))?;
        tx.execute(
            "DELETE FROM session_key_counts WHERE session_id = ?1",
            params![session_id],
        )
        .with_context(|| format!("clearing key counts for {}", session_id))?;
        for (key_id, count) in &flush.key_counts {
            tx.execute(
                "INSERT INTO session_key_counts (session_id, key_id, count) VALUES (?1, ?2, ?3)",
                params![session_id, key_id, *count as i64],
            )
            .with_context(|| format!("writing key count {} for {}", key_id, session_id))?;
        }
        tx.execute(
            "DELETE FROM session_minute_buckets WHERE session_id = ?1",
            params![session_id],
        )
        .with_context(|| format!("clearing minute buckets for {}", session_id))?;
        for (minute_bucket, count) in &flush.minute_buckets {
            tx.execute(
                "INSERT INTO session_minute_buckets (session_id, minute_bucket, count)
                 VALUES (?1, ?2, ?3)",
                params![session_id, minute_bucket, *count as i64],
            )
            .with_context(|| {
                format!("writing minute bucket {} for {}", minute_bucket, session_id)
            })?;
        }
        tx.execute(
            "UPDATE active_runtime
             SET heartbeat_at = ?2, flush_seq = ?3
             WHERE session_id = ?1",
            params![session_id, encode_dt(Utc::now()), flush_seq as i64],
        )
        .with_context(|| format!("updating runtime heartbeat for {}", session_id))?;
        tx.commit().context("committing flush tx")?;
        Ok(())
    }

    pub fn mark_session_stopped(
        &mut self,
        session_id: &str,
        stopped_at: DateTime<Utc>,
        clean_shutdown: bool,
    ) -> Result<()> {
        let tx = self.conn.transaction().context("starting stop tx")?;
        tx.execute(
            "UPDATE sessions
             SET status = ?2,
                 stopped_at = ?3,
                 last_updated_at = ?3,
                 clean_shutdown = ?4
             WHERE session_id = ?1",
            params![
                session_id,
                SessionStatus::Stopped.as_str(),
                encode_dt(stopped_at),
                bool_to_int(clean_shutdown),
            ],
        )
        .with_context(|| format!("marking session {} stopped", session_id))?;
        tx.execute(
            "DELETE FROM active_runtime WHERE session_id = ?1",
            params![session_id],
        )
        .with_context(|| format!("clearing runtime for {}", session_id))?;
        tx.commit().context("committing stop tx")?;
        Ok(())
    }

    pub fn mark_session_interrupted(
        &mut self,
        session_id: &str,
        capture_error: Option<&str>,
    ) -> Result<()> {
        let now = Utc::now();
        let tx = self.conn.transaction().context("starting interrupt tx")?;
        tx.execute(
            "UPDATE sessions
             SET status = ?2,
                 last_updated_at = ?3,
                 capture_error = COALESCE(?4, capture_error),
                 clean_shutdown = 0
             WHERE session_id = ?1",
            params![
                session_id,
                SessionStatus::Interrupted.as_str(),
                encode_dt(now),
                capture_error,
            ],
        )
        .with_context(|| format!("marking session {} interrupted", session_id))?;
        tx.execute(
            "DELETE FROM active_runtime WHERE session_id = ?1",
            params![session_id],
        )
        .with_context(|| format!("clearing runtime for {}", session_id))?;
        tx.commit().context("committing interrupt tx")?;
        Ok(())
    }

    pub fn mark_session_failed(&mut self, session_id: &str, capture_error: &str) -> Result<()> {
        let now = Utc::now();
        let tx = self.conn.transaction().context("starting fail tx")?;
        tx.execute(
            "UPDATE sessions
             SET status = ?2, last_updated_at = ?3, capture_error = ?4, clean_shutdown = 0
             WHERE session_id = ?1",
            params![
                session_id,
                SessionStatus::Failed.as_str(),
                encode_dt(now),
                capture_error,
            ],
        )
        .with_context(|| format!("marking session {} failed", session_id))?;
        tx.execute(
            "DELETE FROM active_runtime WHERE session_id = ?1",
            params![session_id],
        )
        .with_context(|| format!("clearing runtime for {}", session_id))?;
        tx.commit().context("committing fail tx")?;
        Ok(())
    }

    pub fn load_session(&self, session_id: &str) -> Result<SessionRecord> {
        let snapshot = self.load_session_snapshot(session_id)?;
        let runtime = self.load_runtime(session_id)?;
        Ok(SessionRecord { snapshot, runtime })
    }

    pub fn load_session_snapshot(&self, session_id: &str) -> Result<SessionSnapshot> {
        let mut snapshot = self
            .conn
            .query_row(
                "SELECT session_id, name, layout, status, started_at, stopped_at, last_updated_at,
                        total_keypresses, unique_keys, dropped_events, capture_error, report_path,
                        last_flush_at, clean_shutdown
                 FROM sessions WHERE session_id = ?1",
                params![session_id],
                |row| {
                    Ok(SessionSnapshot {
                        session_id: row.get(0)?,
                        name: row.get(1)?,
                        layout: decode_layout(row.get::<_, String>(2)?)?,
                        status: decode_status(row.get::<_, String>(3)?)?,
                        started_at: decode_dt(row.get::<_, String>(4)?)?,
                        stopped_at: row
                            .get::<_, Option<String>>(5)?
                            .map(decode_dt)
                            .transpose()?,
                        last_updated_at: decode_dt(row.get::<_, String>(6)?)?,
                        total_keypresses: row.get::<_, i64>(7)? as u64,
                        unique_keys: row.get::<_, i64>(8)? as usize,
                        dropped_events: row.get::<_, i64>(9)? as u64,
                        capture_error: row.get(10)?,
                        report_path: row.get(11)?,
                        last_flush_at: row
                            .get::<_, Option<String>>(12)?
                            .map(decode_dt)
                            .transpose()?,
                        clean_shutdown: row.get::<_, i64>(13)? != 0,
                        key_counts: Default::default(),
                        minute_buckets: Default::default(),
                    })
                },
            )
            .optional()
            .with_context(|| format!("loading session {}", session_id))?
            .with_context(|| format!("session {} not found", session_id))?;

        let mut stmt = self
            .conn
            .prepare("SELECT key_id, count FROM session_key_counts WHERE session_id = ?1")
            .context("preparing key count query")?;
        let rows = stmt
            .query_map(params![session_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as u64))
            })
            .context("querying key counts")?;
        for row in rows {
            let (key, count) = row.context("reading key count row")?;
            snapshot.key_counts.insert(key, count);
        }

        let mut stmt = self
            .conn
            .prepare(
                "SELECT minute_bucket, count
                 FROM session_minute_buckets
                 WHERE session_id = ?1
                 ORDER BY minute_bucket",
            )
            .context("preparing minute bucket query")?;
        let rows = stmt
            .query_map(params![session_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as u64))
            })
            .context("querying minute buckets")?;
        for row in rows {
            let (minute, count) = row.context("reading minute bucket row")?;
            snapshot.minute_buckets.insert(minute, count);
        }
        Ok(snapshot)
    }

    pub fn load_runtime(&self, session_id: &str) -> Result<Option<RuntimeInfo>> {
        self.conn
            .query_row(
                "SELECT session_id, pid, control_port, control_token, daemon_started_at, heartbeat_at, flush_seq
                 FROM active_runtime WHERE session_id = ?1",
                params![session_id],
                |row| {
                    Ok(RuntimeInfo {
                        session_id: row.get(0)?,
                        pid: row.get::<_, i64>(1)? as u32,
                        control_port: row
                            .get::<_, Option<i64>>(2)?
                            .map(|value| value as u16),
                        control_token: row.get(3)?,
                        daemon_started_at: decode_dt(row.get::<_, String>(4)?)?,
                        heartbeat_at: row
                            .get::<_, Option<String>>(5)?
                            .map(decode_dt)
                            .transpose()?,
                        flush_seq: row.get::<_, i64>(6)? as u64,
                    })
                },
            )
            .optional()
            .with_context(|| format!("loading runtime {}", session_id))
    }

    pub fn active_session(&self) -> Result<Option<SessionRecord>> {
        let session_id = self
            .conn
            .query_row(
                "SELECT session_id FROM sessions WHERE status = ?1 ORDER BY started_at DESC LIMIT 1",
                params![SessionStatus::Running.as_str()],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .context("loading active session id")?;
        match session_id {
            Some(session_id) => Ok(Some(self.load_session(&session_id)?)),
            None => Ok(None),
        }
    }

    pub fn recent_sessions(&self) -> Result<Vec<SessionRecord>> {
        self.recent_sessions_limit(100)
    }

    pub fn recent_sessions_limit(&self, limit: usize) -> Result<Vec<SessionRecord>> {
        let mut stmt = self
            .conn
            .prepare("SELECT session_id FROM sessions ORDER BY started_at DESC LIMIT ?1")
            .context("preparing recent sessions query")?;
        let ids = stmt
            .query_map(params![limit as i64], |row| row.get::<_, String>(0))
            .context("querying recent sessions")?;
        let mut sessions = Vec::new();
        for id in ids {
            sessions.push(self.load_session(&id.context("reading session id")?)?);
        }
        Ok(sessions)
    }
    fn migrate(&self) -> Result<()> {
        self.conn
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS schema_migrations (
                    version INTEGER PRIMARY KEY,
                    applied_at TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS sessions (
                    session_id TEXT PRIMARY KEY,
                    name TEXT NULL,
                    layout TEXT NOT NULL,
                    status TEXT NOT NULL,
                    started_at TEXT NOT NULL,
                    stopped_at TEXT NULL,
                    last_updated_at TEXT NOT NULL,
                    total_keypresses INTEGER NOT NULL,
                    unique_keys INTEGER NOT NULL,
                    dropped_events INTEGER NOT NULL,
                    capture_error TEXT NULL,
                    report_path TEXT NULL,
                    last_flush_at TEXT NULL,
                    clean_shutdown INTEGER NOT NULL DEFAULT 0
                );
                CREATE TABLE IF NOT EXISTS session_key_counts (
                    session_id TEXT NOT NULL REFERENCES sessions(session_id) ON DELETE CASCADE,
                    key_id TEXT NOT NULL,
                    count INTEGER NOT NULL,
                    PRIMARY KEY (session_id, key_id)
                );
                CREATE TABLE IF NOT EXISTS session_minute_buckets (
                    session_id TEXT NOT NULL REFERENCES sessions(session_id) ON DELETE CASCADE,
                    minute_bucket TEXT NOT NULL,
                    count INTEGER NOT NULL,
                    PRIMARY KEY (session_id, minute_bucket)
                );
                CREATE TABLE IF NOT EXISTS active_runtime (
                    session_id TEXT PRIMARY KEY REFERENCES sessions(session_id) ON DELETE CASCADE,
                    pid INTEGER NOT NULL,
                    control_port INTEGER NULL,
                    control_token TEXT NOT NULL,
                    daemon_started_at TEXT NOT NULL,
                    heartbeat_at TEXT NULL,
                    flush_seq INTEGER NOT NULL DEFAULT 0
                );",
            )
            .context("creating sqlite schema")?;
        let current = self.schema_version()?;
        if current > SCHEMA_VERSION {
            bail!(
                "database schema version {} is newer than supported version {}",
                current,
                SCHEMA_VERSION
            );
        }
        if current < SCHEMA_VERSION {
            self.conn
                .execute(
                    "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
                    params![SCHEMA_VERSION, encode_dt(Utc::now())],
                )
                .context("recording schema migration")?;
        }
        Ok(())
    }
}

fn insert_session(tx: &Transaction<'_>, snapshot: &SessionSnapshot) -> Result<()> {
    tx.execute(
        "INSERT INTO sessions (
            session_id, name, layout, status, started_at, stopped_at, last_updated_at,
            total_keypresses, unique_keys, dropped_events, capture_error, report_path,
            last_flush_at, clean_shutdown
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        params![
            snapshot.session_id,
            snapshot.name,
            snapshot.layout.as_str(),
            snapshot.status.as_str(),
            encode_dt(snapshot.started_at),
            snapshot.stopped_at.map(encode_dt),
            encode_dt(snapshot.last_updated_at),
            snapshot.total_keypresses as i64,
            snapshot.unique_keys as i64,
            snapshot.dropped_events as i64,
            snapshot.capture_error,
            snapshot.report_path,
            snapshot.last_flush_at.map(encode_dt),
            bool_to_int(snapshot.clean_shutdown),
        ],
    )
    .context("inserting session")?;
    Ok(())
}

fn encode_dt(value: DateTime<Utc>) -> String {
    value.to_rfc3339()
}

fn decode_dt(value: String) -> rusqlite::Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(&value)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                value.len(),
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })
}

fn decode_layout(value: String) -> rusqlite::Result<KeyboardLayout> {
    KeyboardLayout::parse(&value).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            value.len(),
            rusqlite::types::Type::Text,
            anyhow!("unknown keyboard layout {}", value).into(),
        )
    })
}

fn decode_status(value: String) -> rusqlite::Result<SessionStatus> {
    SessionStatus::parse(&value).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            value.len(),
            rusqlite::types::Type::Text,
            anyhow!("unknown session status {}", value).into(),
        )
    })
}

fn bool_to_int(value: bool) -> i64 {
    if value { 1 } else { 0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::KeyboardLayout;
    use tempfile::tempdir;

    fn test_paths() -> Result<AppPaths> {
        let dir = tempdir().context("creating tempdir")?;
        let root = dir.keep();
        let sessions_dir = root.join("sessions");
        fs::create_dir_all(&sessions_dir).context("creating sessions dir")?;
        Ok(AppPaths {
            root: root.clone(),
            sessions_dir,
            db_path: root.join("test.db"),
        })
    }

    #[test]
    fn repository_creates_and_loads_session() -> Result<()> {
        let paths = test_paths()?;
        let mut repo = Repository::open(&paths)?;
        let snapshot = SessionSnapshot::new(
            "abc".to_string(),
            Some("demo".to_string()),
            KeyboardLayout::Ansi104,
        );
        repo.create_session(&snapshot)?;
        repo.attach_runtime("abc", 42, "token", snapshot.started_at)?;

        let loaded = repo.load_session("abc")?;
        assert_eq!(loaded.snapshot.session_id, "abc");
        assert_eq!(loaded.snapshot.name.as_deref(), Some("demo"));
        assert_eq!(loaded.snapshot.status, SessionStatus::Running);
        assert_eq!(loaded.runtime.as_ref().map(|runtime| runtime.pid), Some(42));
        assert_eq!(repo.schema_version()?, SCHEMA_VERSION);
        Ok(())
    }

    #[test]
    fn repository_flushes_and_marks_interrupted() -> Result<()> {
        let paths = test_paths()?;
        let mut repo = Repository::open(&paths)?;
        let snapshot = SessionSnapshot::new("abc".to_string(), None, KeyboardLayout::Ansi104);
        repo.create_session(&snapshot)?;
        repo.attach_runtime("abc", 7, "token", snapshot.started_at)?;
        let mut flush = CollectorFlush {
            total_keypresses: 3,
            unique_keys: 2,
            dropped_events: 0,
            last_updated_at: Utc::now(),
            last_flush_at: Utc::now(),
            capture_error: None,
            key_counts: Default::default(),
            minute_buckets: Default::default(),
        };
        flush.key_counts.insert("KeyA".to_string(), 2);
        flush.key_counts.insert("KeyB".to_string(), 1);
        flush
            .minute_buckets
            .insert("2026-01-01 12:00".to_string(), 3);
        repo.flush_session("abc", 1, &flush)?;
        repo.mark_session_interrupted("abc", Some("daemon died"))?;

        let loaded = repo.load_session("abc")?;
        assert_eq!(loaded.snapshot.status, SessionStatus::Interrupted);
        assert_eq!(loaded.snapshot.total_keypresses, 3);
        assert_eq!(loaded.snapshot.key_counts.get("KeyA"), Some(&2));
        assert!(loaded.runtime.is_none());
        Ok(())
    }
}
