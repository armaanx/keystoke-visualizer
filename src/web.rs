use crate::api::{build_report_response, recent_history};
use crate::assets::load_asset;
use crate::platform::open_target;
use crate::storage::{AppPaths, Repository};
use anyhow::{Context, Result};
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderValue, Response, StatusCode, header};
use axum::response::{Html, IntoResponse, Json, Redirect};
use axum::routing::get;
use axum::{Router, serve};
use serde::Deserialize;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::net::TcpListener;

const DEFAULT_IDLE_TIMEOUT: Duration = Duration::from_secs(15 * 60);

#[derive(Clone)]
struct WebState {
    paths: AppPaths,
    token: String,
    last_access: Arc<Mutex<Instant>>,
}

#[derive(Deserialize)]
struct TokenQuery {
    token: Option<String>,
}

#[derive(Deserialize)]
struct RecentQuery {
    token: Option<String>,
    limit: Option<usize>,
}

pub fn serve_report_ui(paths: AppPaths, session_id: &str, open_browser: bool) -> Result<()> {
    let repo = Repository::open(&paths)?;
    repo.load_session_snapshot(session_id)
        .with_context(|| format!("report session {} not found", session_id))?;

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("building tokio runtime")?;

    runtime.block_on(async move {
        let token = uuid::Uuid::new_v4().to_string();
        let last_access = Arc::new(Mutex::new(Instant::now()));
        let state = WebState {
            paths: paths.clone(),
            token: token.clone(),
            last_access: Arc::clone(&last_access),
        };

        let app = Router::new()
            .route("/", get(root_redirect))
            .route("/reports/{session_id}", get(report_shell))
            .route("/api/health", get(health))
            .route("/api/sessions/recent", get(recent_sessions))
            .route("/api/sessions/{session_id}/report", get(report_data))
            .fallback(spa_asset_fallback)
            .with_state(state.clone());

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .context("binding report web server")?;
        let address = listener.local_addr().context("reading report address")?;
        let url = format!(
            "http://127.0.0.1:{}/reports/{}?token={}",
            address.port(),
            session_id,
            token
        );

        println!("Report UI: {url}");
        if open_browser {
            open_target(&url)?;
        }

        let last_access_for_shutdown = Arc::clone(&last_access);
        let shutdown = async move {
            loop {
                tokio::time::sleep(Duration::from_secs(5)).await;
                let idle = last_access_for_shutdown
                    .lock()
                    .map(|instant| instant.elapsed())
                    .unwrap_or_default();
                if idle >= DEFAULT_IDLE_TIMEOUT {
                    break;
                }
            }
        };

        serve(listener, app)
            .with_graceful_shutdown(shutdown)
            .await
            .context("running report web server")
    })
}

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({ "ok": true }))
}

async fn root_redirect(
    State(state): State<WebState>,
    Query(query): Query<TokenQuery>,
) -> impl IntoResponse {
    match authorize(&state, query.token.as_deref()) {
        Ok(()) => {
            let repo = match Repository::open(&state.paths) {
                Ok(repo) => repo,
                Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            };
            match repo.recent_sessions_limit(1) {
                Ok(sessions) if !sessions.is_empty() => {
                    let target = format!(
                        "/reports/{}?token={}",
                        sessions[0].snapshot.session_id, state.token
                    );
                    Redirect::temporary(&target).into_response()
                }
                Ok(_) => index_html().into_response(),
                Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            }
        }
        Err(response) => response,
    }
}

async fn report_shell(
    State(state): State<WebState>,
    Path(_session_id): Path<String>,
    Query(query): Query<TokenQuery>,
) -> impl IntoResponse {
    match authorize(&state, query.token.as_deref()) {
        Ok(()) => index_html().into_response(),
        Err(response) => response,
    }
}

async fn report_data(
    State(state): State<WebState>,
    Path(session_id): Path<String>,
    Query(query): Query<TokenQuery>,
) -> impl IntoResponse {
    if let Err(response) = authorize(&state, query.token.as_deref()) {
        return response;
    }

    let repo = match Repository::open(&state.paths) {
        Ok(repo) => repo,
        Err(error) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": error.to_string() })),
            )
                .into_response();
        }
    };

    match build_report_response(&repo, &session_id, 10) {
        Ok(report) => Json(report).into_response(),
        Err(_) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "session not found" })),
        )
            .into_response(),
    }
}

async fn recent_sessions(
    State(state): State<WebState>,
    Query(query): Query<RecentQuery>,
) -> impl IntoResponse {
    if let Err(response) = authorize(&state, query.token.as_deref()) {
        return response;
    }

    let repo = match Repository::open(&state.paths) {
        Ok(repo) => repo,
        Err(error) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": error.to_string() })),
            )
                .into_response();
        }
    };

    match recent_history(&repo, query.limit.unwrap_or(10)) {
        Ok(history) => Json(history).into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn spa_asset_fallback(
    State(state): State<WebState>,
    request: axum::http::Request<Body>,
) -> impl IntoResponse {
    touch(&state);
    let requested = request.uri().path().trim_start_matches('/');
    let Some(asset) = load_asset(requested) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let mut response = Response::new(Body::from(asset.bytes));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(asset.content_type),
    );
    response.into_response()
}

fn index_html() -> Response<Body> {
    let Some(asset) = load_asset("index.html") else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    let mut response = Response::new(Body::from(asset.bytes));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(asset.content_type),
    );
    response
}

fn authorize(state: &WebState, token: Option<&str>) -> std::result::Result<(), axum::response::Response> {
    match token {
        Some(value) if value == state.token => {
            touch(state);
            Ok(())
        }
        _ => Err((StatusCode::UNAUTHORIZED, Html("missing or invalid token")).into_response()),
    }
}

fn touch(state: &WebState) {
    if let Ok(mut guard) = state.last_access.lock() {
        *guard = Instant::now();
    }
}
