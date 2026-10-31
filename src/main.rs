//! 简历服务 + 公司岗位收集：静态页 + MySQL + SMTP
//!
//! 用法：
//!   1. 复制 .env.example 为 .env，填写 DATABASE_URL 与 SMTP_*
//!   2. cargo run
//!
//! 页面：
//!   /            首页（两模块入口）
//!   /resume.html 编辑保存简历
//!   /jobs.html   收集公司岗位信息
//!
//! API:
//!   GET/POST /api/resumes ...
//!   GET/POST /api/jobs  PUT/DELETE /api/jobs/:id
//!   POST /api/jobs/:id/ensure-resume
//!   GET  /api/jobs/:id/resumes
//!   PUT  /api/jobs/:id/selected-resume
//!   POST /api/email/test   → 仅发到 TEST_EMAIL_TO（可附简历）
//!   POST /api/email/send   → 发到公司邮箱（附选中简历）

mod db;
mod email;
mod jobs;
mod models;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use base64::Engine as _;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use email::{MailConfig, SendEmailRequest};
use jobs::{SelectResumeRequest, UpsertJobRequest};
use models::{ApiError, SaveResumeRequest, UploadPdfRequest};

#[derive(Clone)]
struct AppState {
    pool: db::DbPool,
    mail: MailConfig,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse()?))
        .init();

    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "mysql://root:password@127.0.0.1:3306/resume_db".to_string()
    });
    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(4000);
    let open_browser = std::env::var("OPEN_BROWSER")
        .map(|v| v != "0" && v.to_lowercase() != "false")
        .unwrap_or(true);

    let pool = db::connect(&database_url).await?;
    jobs::ensure_table(&pool).await?;
    let seeded = jobs::seed_if_empty(&pool).await?;
    if seeded > 0 {
        println!("已导入岗位种子 {seeded} 条");
    }
    let synced = jobs::sync_descriptions_from_seed(&pool).await?;
    if synced > 0 {
        println!("已同步岗位扩展字段 {synced} 条");
    }

    let mail = MailConfig::from_env()?;
    if mail.is_configured() {
        tracing::info!(
            "SMTP 已配置: {}:{} → 测试收件人 {}",
            mail.host,
            mail.port,
            mail.test_to
        );
    } else {
        tracing::warn!("SMTP 未完整配置：发信前请填写 SMTP_USER / SMTP_PASS / SMTP_FROM");
    }

    let state = Arc::new(AppState { pool, mail });

    let static_dir = resolve_static_dir();
    tracing::info!("静态目录: {}", static_dir.display());

    let app = Router::new()
        .route("/api/health", get(health))
        .route("/api/resumes", get(list_resumes).post(save_resume))
        .route(
            "/api/resumes/{id}",
            get(get_resume).delete(delete_resume),
        )
        .route(
            "/api/resumes/{id}/pdf",
            get(get_resume_pdf).put(upload_resume_pdf),
        )
        .route("/api/jobs", get(list_jobs).post(create_job))
        .route(
            "/api/jobs/{id}",
            get(get_job).put(update_job).delete(delete_job),
        )
        .route("/api/jobs/{id}/ensure-resume", post(ensure_resume))
        .route("/api/jobs/{id}/resumes", get(list_job_resumes))
        .route("/api/jobs/{id}/selected-resume", put(select_resume))
        .route("/api/email/test", post(email_test))
        .route("/api/email/send", post(email_send))
        .fallback_service(ServeDir::new(&static_dir).append_index_html_on_directories(true))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr: SocketAddr = format!("{host}:{port}").parse()?;
    let public_host = if host == "0.0.0.0" || host == "::" {
        "127.0.0.1"
    } else {
        host.as_str()
    };
    let home = format!("http://{public_host}:{port}/");
    tracing::info!("服务已启动: {home} (bind {addr})");
    println!("首页（两模块）: {home}");
    println!("简历模块: http://{public_host}:{port}/resume.html");
    println!("岗位模块: http://{public_host}:{port}/jobs.html");

    if open_browser {
        let _ = open::that(&home);
    }

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

fn resolve_static_dir() -> PathBuf {
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        return PathBuf::from(manifest);
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

async fn health(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(serde_json::json!({
        "ok": true,
        "smtp_configured": state.mail.is_configured(),
        "test_email_to": state.mail.test_to,
    }))
}

async fn list_resumes(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match db::list_resumes(&state.pool).await {
        Ok(rows) => (StatusCode::OK, Json(serde_json::json!(rows))).into_response(),
        Err(e) => api_err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

async fn get_resume(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u64>,
) -> impl IntoResponse {
    match db::get_resume(&state.pool, id).await {
        Ok(Some(row)) => (StatusCode::OK, Json(serde_json::json!(row))).into_response(),
        Ok(None) => api_err(StatusCode::NOT_FOUND, "简历不存在".into()),
        Err(e) => api_err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

async fn save_resume(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SaveResumeRequest>,
) -> impl IntoResponse {
    match db::save_resume(&state.pool, req).await {
        Ok(resp) => (StatusCode::OK, Json(serde_json::json!(resp))).into_response(),
        Err(e) => {
            let msg = e.to_string();
            let status = if msg.contains("不能为空") || msg.contains("无法解析") {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            api_err(status, msg)
        }
    }
}

async fn delete_resume(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u64>,
) -> impl IntoResponse {
    match db::delete_resume(&state.pool, id).await {
        Ok(true) => (StatusCode::OK, Json(serde_json::json!({ "ok": true }))).into_response(),
        Ok(false) => api_err(StatusCode::NOT_FOUND, "简历不存在".into()),
        Err(e) => api_err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

async fn upload_resume_pdf(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u64>,
    Json(req): Json<UploadPdfRequest>,
) -> impl IntoResponse {
    let raw = req.pdf_base64.trim();
    let b64 = raw
        .split(',')
        .next_back()
        .unwrap_or(raw)
        .trim();
    let bytes = match base64::engine::general_purpose::STANDARD.decode(b64) {
        Ok(b) => b,
        Err(e) => return api_err(StatusCode::BAD_REQUEST, format!("PDF base64 无效: {e}")),
    };
    match db::save_resume_pdf(&state.pool, id, &bytes).await {
        Ok(()) => (
            StatusCode::OK,
            Json(serde_json::json!({ "ok": true, "bytes": bytes.len() })),
        )
            .into_response(),
        Err(e) => {
            let msg = e.to_string();
            let status = if msg.contains("不存在") || msg.contains("有效") || msg.contains("空") {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            api_err(status, msg)
        }
    }
}

async fn get_resume_pdf(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u64>,
) -> impl IntoResponse {
    match db::get_resume_pdf(&state.pool, id).await {
        Ok(Some(bytes)) => {
            let mut res = axum::response::Response::new(bytes.into());
            *res.status_mut() = StatusCode::OK;
            res.headers_mut().insert(
                header::CONTENT_TYPE,
                header::HeaderValue::from_static("application/pdf"),
            );
            res.headers_mut().insert(
                header::CONTENT_DISPOSITION,
                header::HeaderValue::from_static("inline; filename=\"resume.pdf\""),
            );
            res
        }
        Ok(None) => api_err(
            StatusCode::NOT_FOUND,
            "该版本尚未生成 PDF，请先在简历页保存一次".into(),
        ),
        Err(e) => api_err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

async fn list_jobs(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match jobs::list_jobs(&state.pool).await {
        Ok(rows) => (StatusCode::OK, Json(serde_json::json!(rows))).into_response(),
        Err(e) => api_err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

async fn get_job(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u64>,
) -> impl IntoResponse {
    match jobs::get_job(&state.pool, id).await {
        Ok(Some(row)) => (StatusCode::OK, Json(serde_json::json!(row))).into_response(),
        Ok(None) => api_err(StatusCode::NOT_FOUND, "岗位不存在".into()),
        Err(e) => api_err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

async fn create_job(
    State(state): State<Arc<AppState>>,
    Json(req): Json<UpsertJobRequest>,
) -> impl IntoResponse {
    match jobs::create_job(&state.pool, req).await {
        Ok(row) => (StatusCode::OK, Json(serde_json::json!(row))).into_response(),
        Err(e) => {
            let msg = e.to_string();
            let status = if msg.contains("不能为空") {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            api_err(status, msg)
        }
    }
}

async fn update_job(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u64>,
    Json(req): Json<UpsertJobRequest>,
) -> impl IntoResponse {
    match jobs::update_job(&state.pool, id, req).await {
        Ok(Some(row)) => (StatusCode::OK, Json(serde_json::json!(row))).into_response(),
        Ok(None) => api_err(StatusCode::NOT_FOUND, "岗位不存在".into()),
        Err(e) => {
            let msg = e.to_string();
            let status = if msg.contains("不能为空") {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            api_err(status, msg)
        }
    }
}

async fn delete_job(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u64>,
) -> impl IntoResponse {
    match jobs::delete_job(&state.pool, id).await {
        Ok(true) => (StatusCode::OK, Json(serde_json::json!({ "ok": true }))).into_response(),
        Ok(false) => api_err(StatusCode::NOT_FOUND, "岗位不存在".into()),
        Err(e) => api_err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

async fn ensure_resume(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u64>,
) -> impl IntoResponse {
    match jobs::ensure_resume_for_job(&state.pool, id).await {
        Ok(resp) => (StatusCode::OK, Json(serde_json::json!(resp))).into_response(),
        Err(e) => {
            let msg = e.to_string();
            let status = if msg.contains("NO_TEMPLATE") || msg.contains("不存在") {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            api_err(status, msg)
        }
    }
}

async fn list_job_resumes(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u64>,
) -> impl IntoResponse {
    match jobs::list_job_resumes(&state.pool, id).await {
        Ok(rows) => (StatusCode::OK, Json(serde_json::json!(rows))).into_response(),
        Err(e) => {
            let msg = e.to_string();
            let status = if msg.contains("不存在") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            api_err(status, msg)
        }
    }
}

async fn select_resume(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u64>,
    Json(req): Json<SelectResumeRequest>,
) -> impl IntoResponse {
    match jobs::set_selected_resume(&state.pool, id, req.resume_id).await {
        Ok(Some(row)) => (StatusCode::OK, Json(serde_json::json!(row))).into_response(),
        Ok(None) => api_err(StatusCode::NOT_FOUND, "岗位不存在".into()),
        Err(e) => {
            let msg = e.to_string();
            let status = if msg.contains("不存在") {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            api_err(status, msg)
        }
    }
}

async fn email_test(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SendEmailRequest>,
) -> impl IntoResponse {
    match email::send_test(&state.mail, &state.pool, req).await {
        Ok(message) => {
            (StatusCode::OK, Json(serde_json::json!({ "ok": true, "message": message }))).into_response()
        }
        Err(e) => api_err(StatusCode::BAD_REQUEST, e.to_string()),
    }
}

async fn email_send(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SendEmailRequest>,
) -> impl IntoResponse {
    match email::send_company(&state.mail, &state.pool, req).await {
        Ok(message) => {
            (StatusCode::OK, Json(serde_json::json!({ "ok": true, "message": message }))).into_response()
        }
        Err(e) => api_err(StatusCode::BAD_REQUEST, e.to_string()),
    }
}

fn api_err(status: StatusCode, error: String) -> axum::response::Response {
    (status, Json(ApiError { error })).into_response()
}
