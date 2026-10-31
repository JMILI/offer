use anyhow::{anyhow, Context, Result};
use chrono::{Local, NaiveDateTime};
use serde_json::Value;
use sqlx::{mysql::MySqlPoolOptions, MySql, Pool};

use crate::models::{ResumeDetail, ResumeListItem, SaveResumeRequest, SaveResumeResponse};

pub type DbPool = Pool<MySql>;

pub async fn connect(database_url: &str) -> Result<DbPool> {
    let pool = MySqlPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await
        .context("连接 MySQL 失败，请检查 DATABASE_URL 与库表是否已创建")?;

    // 确保主表存在（子表可先跑 sql/schema.sql）
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS resume_versions (
          id              BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
          company         VARCHAR(128)    NOT NULL,
          job_id          BIGINT UNSIGNED NULL,
          version_tag     VARCHAR(32)     NOT NULL,
          saved_at        DATETIME(3)     NOT NULL,
          note            VARCHAR(512)    NULL,
          html_snapshot   MEDIUMTEXT      NOT NULL,
          style_css       MEDIUMTEXT      NULL,
          photo_data_url  MEDIUMTEXT      NULL,
          candidate_name  VARCHAR(64)     NULL,
          job_intent      VARCHAR(128)    NULL,
          modules_json    JSON            NULL,
          created_at      DATETIME(3)     NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
          updated_at      DATETIME(3)     NOT NULL DEFAULT CURRENT_TIMESTAMP(3)
                                         ON UPDATE CURRENT_TIMESTAMP(3),
          PRIMARY KEY (id),
          UNIQUE KEY uk_company_version (company, version_tag),
          KEY idx_saved_at (saved_at DESC),
          KEY idx_company (company),
          KEY idx_job_id (job_id)
        ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci
        "#,
    )
    .execute(&pool)
    .await
    .context("自动创建 resume_versions 表失败")?;

    // 兼容旧表：补 pdf_data 列（服务端存 PDF，不落本机）
    let exists: Option<(String,)> = sqlx::query_as(
        r#"
        SELECT COLUMN_NAME FROM information_schema.COLUMNS
        WHERE TABLE_SCHEMA = DATABASE()
          AND TABLE_NAME = 'resume_versions'
          AND COLUMN_NAME = 'pdf_data'
        "#,
    )
    .fetch_optional(&pool)
    .await?;
    if exists.is_none() {
        sqlx::query(
            "ALTER TABLE resume_versions ADD COLUMN pdf_data LONGBLOB NULL AFTER modules_json",
        )
        .execute(&pool)
        .await
        .context("添加 resume_versions.pdf_data 失败")?;
        tracing::info!("已为 resume_versions 添加 pdf_data 列");
    }

    Ok(pool)
}

fn parse_saved_at(raw: &str) -> Result<NaiveDateTime> {
    let s = raw.trim();
    let formats = [
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%dT%H:%M:%S%.f",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%dT%H:%M",
        "%Y-%m-%d %H:%M",
        "%Y-%m-%d",
    ];
    for fmt in formats {
        if let Ok(dt) = NaiveDateTime::parse_from_str(s, fmt) {
            return Ok(dt);
        }
        if fmt == "%Y-%m-%d" {
            if let Ok(d) = chrono::NaiveDate::parse_from_str(s, fmt) {
                return Ok(d.and_hms_opt(0, 0, 0).unwrap());
            }
        }
    }
    Err(anyhow!("无法解析时间: {raw}，请使用 2026-08-17T16:00 或 2026-08-17 16:00:00"))
}

pub async fn list_resumes(pool: &DbPool) -> Result<Vec<ResumeListItem>> {
    let rows = sqlx::query_as::<_, ResumeListItem>(
        r#"
        SELECT id, company, version_tag, saved_at, note,
               candidate_name, job_intent, created_at, job_id
        FROM resume_versions
        ORDER BY saved_at DESC, id DESC
        "#,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn get_resume(pool: &DbPool, id: u64) -> Result<Option<ResumeDetail>> {
    let row = sqlx::query_as::<_, ResumeDetail>(
        r#"
        SELECT id, company, version_tag, saved_at, note,
               html_snapshot, style_css, photo_data_url,
               candidate_name, job_intent, modules_json, created_at, job_id
        FROM resume_versions
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn save_resume(pool: &DbPool, req: SaveResumeRequest) -> Result<SaveResumeResponse> {
    let company = req.company.trim().to_string();
    let version_tag = req.version_tag.trim().to_string();
    if company.is_empty() {
        return Err(anyhow!("面试公司不能为空"));
    }
    if version_tag.is_empty() {
        return Err(anyhow!("版本号不能为空"));
    }
    if req.html_snapshot.trim().is_empty() {
        return Err(anyhow!("简历内容为空"));
    }

    let saved_at = if req.saved_at.trim().is_empty() {
        Local::now().naive_local()
    } else {
        parse_saved_at(&req.saved_at)?
    };

    let modules: Option<Value> = req.modules_json;
    let job_id = req.job_id;

    let result = sqlx::query(
        r#"
        INSERT INTO resume_versions
          (company, job_id, version_tag, saved_at, note,
           html_snapshot, style_css, photo_data_url,
           candidate_name, job_intent, modules_json)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON DUPLICATE KEY UPDATE
          job_id = COALESCE(VALUES(job_id), job_id),
          saved_at = VALUES(saved_at),
          note = VALUES(note),
          html_snapshot = VALUES(html_snapshot),
          style_css = VALUES(style_css),
          photo_data_url = VALUES(photo_data_url),
          candidate_name = VALUES(candidate_name),
          job_intent = VALUES(job_intent),
          modules_json = VALUES(modules_json)
        "#,
    )
    .bind(&company)
    .bind(job_id)
    .bind(&version_tag)
    .bind(saved_at)
    .bind(req.note.as_deref())
    .bind(&req.html_snapshot)
    .bind(req.style_css.as_deref())
    .bind(req.photo_data_url.as_deref())
    .bind(req.candidate_name.as_deref())
    .bind(req.job_intent.as_deref())
    .bind(modules)
    .execute(pool)
    .await
    .context("写入 resume_versions 失败")?;

    let mut id = result.last_insert_id();
    if id == 0 {
        let existing: (u64,) = sqlx::query_as(
            "SELECT id FROM resume_versions WHERE company = ? AND version_tag = ?",
        )
        .bind(&company)
        .bind(&version_tag)
        .fetch_one(pool)
        .await?;
        id = existing.0;
    }

    // 若带 job_id，自动设为该岗位当前选中简历
    if let Some(jid) = job_id {
        let _ = sqlx::query("UPDATE company_jobs SET selected_resume_id = ? WHERE id = ?")
            .bind(id)
            .bind(jid)
            .execute(pool)
            .await;
    }

    Ok(SaveResumeResponse {
        id,
        company,
        version_tag,
        job_id,
    })
}

pub async fn delete_resume(pool: &DbPool, id: u64) -> Result<bool> {
    let r = sqlx::query("DELETE FROM resume_versions WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(r.rows_affected() > 0)
}

pub async fn save_resume_pdf(pool: &DbPool, id: u64, pdf: &[u8]) -> Result<()> {
    if pdf.is_empty() {
        return Err(anyhow!("PDF 内容为空"));
    }
    if pdf.len() < 5 || &pdf[0..5] != b"%PDF-" {
        return Err(anyhow!("不是有效的 PDF 文件"));
    }
    let result = sqlx::query("UPDATE resume_versions SET pdf_data = ? WHERE id = ?")
        .bind(pdf)
        .bind(id)
        .execute(pool)
        .await
        .context("写入 PDF 失败")?;
    if result.rows_affected() == 0 {
        return Err(anyhow!("简历不存在"));
    }
    Ok(())
}

pub async fn get_resume_pdf(pool: &DbPool, id: u64) -> Result<Option<Vec<u8>>> {
    let row: Option<(Option<Vec<u8>>,)> =
        sqlx::query_as("SELECT pdf_data FROM resume_versions WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?;
    Ok(row.and_then(|r| r.0).filter(|b| !b.is_empty()))
}
