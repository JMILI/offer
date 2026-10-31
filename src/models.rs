use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct ResumeListItem {
    pub id: u64,
    pub company: String,
    pub version_tag: String,
    pub saved_at: NaiveDateTime,
    pub note: Option<String>,
    pub candidate_name: Option<String>,
    pub job_intent: Option<String>,
    pub created_at: NaiveDateTime,
    pub job_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct ResumeDetail {
    pub id: u64,
    pub company: String,
    pub version_tag: String,
    pub saved_at: NaiveDateTime,
    pub note: Option<String>,
    pub html_snapshot: String,
    pub style_css: Option<String>,
    pub photo_data_url: Option<String>,
    pub candidate_name: Option<String>,
    pub job_intent: Option<String>,
    pub modules_json: Option<Value>,
    pub created_at: NaiveDateTime,
    pub job_id: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct SaveResumeRequest {
    pub company: String,
    pub version_tag: String,
    /// 用户填写的时间，ISO 或 `YYYY-MM-DD HH:MM:SS` / `YYYY-MM-DDTHH:MM`
    pub saved_at: String,
    pub note: Option<String>,
    pub html_snapshot: String,
    pub style_css: Option<String>,
    pub photo_data_url: Option<String>,
    pub candidate_name: Option<String>,
    pub job_intent: Option<String>,
    pub modules_json: Option<Value>,
    /// 关联的公司岗位 id（从岗位页「编辑简历」进入时传入）
    pub job_id: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct SaveResumeResponse {
    pub id: u64,
    pub company: String,
    pub version_tag: String,
    pub job_id: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct UploadPdfRequest {
    /// base64 编码的 PDF 字节（可带 data:application/pdf;base64, 前缀）
    pub pdf_base64: String,
}

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub error: String,
}
