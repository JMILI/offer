use anyhow::{Context, Result};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value; // seed JSON
use sqlx::FromRow;
use std::path::PathBuf;

use crate::db::DbPool;
use crate::models::ResumeListItem;

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct JobRow {
    pub id: u64,
    pub position: String,
    pub company: String,
    pub city_mode: String,
    pub experience: String,
    pub source: String,
    pub source_url: String,
    pub company_nature: String,
    pub is_outsourcing: bool,
    pub headcount: String,
    pub listing_status: String,
    pub email: String,
    pub website: String,
    pub hr_wechat: String,
    pub job_description: String,
    pub email_body: String,
    pub selected_resume_id: Option<u64>,
    pub status_applied: bool,
    pub status_written_test: bool,
    pub status_interview1: bool,
    pub status_interview2: bool,
    pub status_offer: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize)]
pub struct JobView {
    pub id: u64,
    pub position: String,
    pub company: String,
    pub city_mode: String,
    pub experience: String,
    pub source: String,
    pub source_url: String,
    pub company_nature: String,
    pub is_outsourcing: bool,
    pub headcount: String,
    pub listing_status: String,
    pub email: String,
    pub website: String,
    pub hr_wechat: String,
    pub job_description: String,
    pub email_body: String,
    pub selected_resume_id: Option<u64>,
    pub status_applied: bool,
    pub status_written_test: bool,
    pub status_interview1: bool,
    pub status_interview2: bool,
    pub status_offer: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl From<JobRow> for JobView {
    fn from(r: JobRow) -> Self {
        Self {
            id: r.id,
            position: r.position,
            company: r.company,
            city_mode: r.city_mode,
            experience: r.experience,
            source: r.source,
            source_url: r.source_url,
            company_nature: r.company_nature,
            is_outsourcing: r.is_outsourcing,
            headcount: r.headcount,
            listing_status: r.listing_status,
            email: r.email,
            website: r.website,
            hr_wechat: r.hr_wechat,
            job_description: r.job_description,
            email_body: r.email_body,
            selected_resume_id: r.selected_resume_id,
            status_applied: r.status_applied,
            status_written_test: r.status_written_test,
            status_interview1: r.status_interview1,
            status_interview2: r.status_interview2,
            status_offer: r.status_offer,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UpsertJobRequest {
    pub position: String,
    pub company: String,
    pub city_mode: Option<String>,
    pub experience: Option<String>,
    pub source: Option<String>,
    pub source_url: Option<String>,
    pub company_nature: Option<String>,
    pub is_outsourcing: Option<bool>,
    pub headcount: Option<String>,
    pub listing_status: Option<String>,
    pub email: Option<String>,
    pub website: Option<String>,
    pub hr_wechat: Option<String>,
    pub job_description: Option<String>,
    pub email_body: Option<String>,
    pub selected_resume_id: Option<u64>,
    pub status_applied: Option<bool>,
    pub status_written_test: Option<bool>,
    pub status_interview1: Option<bool>,
    pub status_interview2: Option<bool>,
    pub status_offer: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct SelectResumeRequest {
    pub resume_id: u64,
}

#[derive(Debug, Serialize)]
pub struct EnsureResumeResponse {
    pub resume_id: u64,
    pub company: String,
    pub version_tag: String,
    pub created: bool,
}

pub fn default_email_body(company: &str, position: &str, job_description: &str, city: &str) -> String {
    let jd = job_description.trim();
    let jd_block = if jd.is_empty() {
        format!("应聘岗位：{position}\n（岗位具体要求待与贵司进一步对齐。）")
    } else {
        // 岗位描述过长时截断，避免把整段 JD 塞进邮件
        let brief = if jd.chars().count() > 280 {
            let s: String = jd.chars().take(280).collect();
            format!("{s}…")
        } else {
            jd.to_string()
        };
        format!("应聘岗位：{position}\n岗位要点：{brief}")
    };
    let mut points = vec![
        "1. 约 2 年 C++ 工程经验，深耕二进制协议解析、性能优化与系统侧开发，可快速迁移到 Rust 生产实践；".to_string(),
        "2. 代表战绩：将异构数据迁移吞吐提升约 6 倍（约 500 万条/1h → 约 10min）；".to_string(),
        "3. 近期用 Rust 完成异步 / FFI 相关实践（含 TUI 调度器等），熟悉所有权模型与跨语言边界；".to_string(),
    ];
    let mut n = 4;
    if !city.trim().is_empty() {
        points.push(format!("{n}. 就职城市/方式意向：{}；", city.trim()));
        n += 1;
    }
    points.push(format!(
        "{n}. 工作地点灵活，可按贵司要求坐班或远程；简历 PDF 已作为附件随本邮件发送。"
    ));
    let points_block = points.join("\n");

    format!(
        "尊敬的{company}招聘负责人 / HR 您好：\n\
         \n\
         我是李江敏，计算机相关专业硕士，求职方向为 Rust / C++ 底层与系统开发。\
         获悉贵司在招「{position}」，对照岗位要求后认为高度契合，特此投递，恳请审阅。\n\
         \n\
         {jd_block}\n\
         \n\
         【匹配说明】\n\
         {points_block}\n\
         \n\
         期待有机会进一步沟通，谢谢！\n\
         \n\
         此致\n\
         敬礼\n\
         \n\
         李江敏\n\
         电话：17691086153\n\
         邮箱：923749851@qq.com"
    )
}

async fn add_column_if_missing(pool: &DbPool, column: &str, ddl: &str) -> Result<()> {
    let exists: Option<(String,)> = sqlx::query_as(
        r#"
        SELECT COLUMN_NAME FROM information_schema.COLUMNS
        WHERE TABLE_SCHEMA = DATABASE()
          AND TABLE_NAME = 'company_jobs'
          AND COLUMN_NAME = ?
        "#,
    )
    .bind(column)
    .fetch_optional(pool)
    .await?;
    if exists.is_none() {
        sqlx::query(ddl)
            .execute(pool)
            .await
            .with_context(|| format!("添加列 {column} 失败"))?;
        tracing::info!("已为 company_jobs 添加列 {column}");
    }
    Ok(())
}

pub async fn ensure_table(pool: &DbPool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS company_jobs (
          id                  BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
          position            VARCHAR(256)    NOT NULL,
          company             VARCHAR(256)    NOT NULL,
          city_mode           VARCHAR(256)    NOT NULL DEFAULT '',
          experience          VARCHAR(128)    NOT NULL DEFAULT '',
          source              VARCHAR(128)    NOT NULL DEFAULT '',
          source_url          VARCHAR(512)    NOT NULL DEFAULT '',
          company_nature      VARCHAR(256)    NOT NULL DEFAULT '',
          is_outsourcing      TINYINT(1)      NOT NULL DEFAULT 0,
          headcount           VARCHAR(128)    NOT NULL DEFAULT '',
          listing_status      VARCHAR(64)     NOT NULL DEFAULT '未上市',
          email               VARCHAR(256)    NOT NULL DEFAULT '',
          website             VARCHAR(512)    NOT NULL DEFAULT '',
          hr_wechat           VARCHAR(128)    NOT NULL DEFAULT '',
          job_description     MEDIUMTEXT      NOT NULL,
          email_body          MEDIUMTEXT      NOT NULL,
          selected_resume_id  BIGINT UNSIGNED NULL,
          status_applied      TINYINT(1)      NOT NULL DEFAULT 0,
          status_written_test TINYINT(1)      NOT NULL DEFAULT 0,
          status_interview1   TINYINT(1)      NOT NULL DEFAULT 0,
          status_interview2   TINYINT(1)      NOT NULL DEFAULT 0,
          status_offer        TINYINT(1)      NOT NULL DEFAULT 0,
          created_at          DATETIME(3)     NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
          updated_at          DATETIME(3)     NOT NULL DEFAULT CURRENT_TIMESTAMP(3)
                                             ON UPDATE CURRENT_TIMESTAMP(3),
          PRIMARY KEY (id),
          KEY idx_company (company),
          KEY idx_updated (updated_at DESC)
        ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci
        "#,
    )
    .execute(pool)
    .await
    .context("创建 company_jobs 表失败")?;

    add_column_if_missing(
        pool,
        "job_description",
        "ALTER TABLE company_jobs ADD COLUMN job_description MEDIUMTEXT NOT NULL AFTER hr_wechat",
    )
    .await?;
    add_column_if_missing(
        pool,
        "source_url",
        "ALTER TABLE company_jobs ADD COLUMN source_url VARCHAR(512) NOT NULL DEFAULT '' AFTER source",
    )
    .await?;
    add_column_if_missing(
        pool,
        "listing_status",
        "ALTER TABLE company_jobs ADD COLUMN listing_status VARCHAR(64) NOT NULL DEFAULT '未上市' AFTER headcount",
    )
    .await?;
    add_column_if_missing(
        pool,
        "selected_resume_id",
        "ALTER TABLE company_jobs ADD COLUMN selected_resume_id BIGINT UNSIGNED NULL AFTER email_body",
    )
    .await?;
    add_column_if_missing(
        pool,
        "status_applied",
        "ALTER TABLE company_jobs ADD COLUMN status_applied TINYINT(1) NOT NULL DEFAULT 0 AFTER selected_resume_id",
    )
    .await?;
    add_column_if_missing(
        pool,
        "status_written_test",
        "ALTER TABLE company_jobs ADD COLUMN status_written_test TINYINT(1) NOT NULL DEFAULT 0 AFTER status_applied",
    )
    .await?;
    add_column_if_missing(
        pool,
        "status_interview1",
        "ALTER TABLE company_jobs ADD COLUMN status_interview1 TINYINT(1) NOT NULL DEFAULT 0 AFTER status_written_test",
    )
    .await?;
    add_column_if_missing(
        pool,
        "status_interview2",
        "ALTER TABLE company_jobs ADD COLUMN status_interview2 TINYINT(1) NOT NULL DEFAULT 0 AFTER status_interview1",
    )
    .await?;
    add_column_if_missing(
        pool,
        "status_offer",
        "ALTER TABLE company_jobs ADD COLUMN status_offer TINYINT(1) NOT NULL DEFAULT 0 AFTER status_interview2",
    )
    .await?;

    // resume_versions.job_id 关联
    let exists: Option<(String,)> = sqlx::query_as(
        r#"
        SELECT COLUMN_NAME FROM information_schema.COLUMNS
        WHERE TABLE_SCHEMA = DATABASE()
          AND TABLE_NAME = 'resume_versions'
          AND COLUMN_NAME = 'job_id'
        "#,
    )
    .fetch_optional(pool)
    .await?;
    if exists.is_none() {
        sqlx::query(
            "ALTER TABLE resume_versions ADD COLUMN job_id BIGINT UNSIGNED NULL AFTER company, ADD KEY idx_job_id (job_id)",
        )
        .execute(pool)
        .await
        .context("添加 resume_versions.job_id 失败")?;
        tracing::info!("已为 resume_versions 添加 job_id 列");
    }

    Ok(())
}

fn seed_path() -> PathBuf {
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        return PathBuf::from(manifest).join("data").join("jobs_seed.json");
    }
    PathBuf::from("data/jobs_seed.json")
}

fn read_seed_items() -> Result<Vec<Value>> {
    let path = seed_path();
    if !path.exists() {
        anyhow::bail!("种子文件不存在: {}", path.display());
    }
    let raw = std::fs::read_to_string(&path)
        .with_context(|| format!("读取种子失败: {}", path.display()))?;
    serde_json::from_str(&raw).context("解析 jobs_seed.json 失败")
}

fn seed_field<'a>(item: &'a Value, key: &str) -> &'a str {
    item.get(key).and_then(|v| v.as_str()).unwrap_or("")
}

pub async fn seed_if_empty(pool: &DbPool) -> Result<usize> {
    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM company_jobs")
        .fetch_one(pool)
        .await?;
    if count > 0 {
        return Ok(0);
    }

    let items = match read_seed_items() {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("{e:#}");
            return Ok(0);
        }
    };

    let mut n = 0;
    for item in items {
        let position = seed_field(&item, "position");
        let company = seed_field(&item, "company");
        let position = if position.is_empty() { "未说明" } else { position };
        let company = if company.is_empty() { "未说明" } else { company };
        let city_mode = seed_field(&item, "city_mode");
        let experience = seed_field(&item, "experience");
        let source = seed_field(&item, "source");
        let source_url = seed_field(&item, "source_url");
        let company_nature = seed_field(&item, "company_nature");
        let is_outsourcing = item
            .get("is_outsourcing")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let headcount = seed_field(&item, "headcount");
        let listing_status = {
            let s = seed_field(&item, "listing_status");
            if s.is_empty() { "未上市" } else { s }
        };
        let email = seed_field(&item, "email");
        let website = seed_field(&item, "website");
        let hr_wechat = seed_field(&item, "hr_wechat");
        let job_description = seed_field(&item, "job_description");
        let email_body = default_email_body(company, position, job_description, city_mode);

        sqlx::query(
            r#"
            INSERT INTO company_jobs
              (position, company, city_mode, experience, source, source_url, company_nature,
               is_outsourcing, headcount, listing_status, email, website, hr_wechat,
               job_description, email_body)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(position)
        .bind(company)
        .bind(city_mode)
        .bind(experience)
        .bind(source)
        .bind(source_url)
        .bind(company_nature)
        .bind(is_outsourcing)
        .bind(headcount)
        .bind(listing_status)
        .bind(email)
        .bind(website)
        .bind(hr_wechat)
        .bind(job_description)
        .bind(&email_body)
        .execute(pool)
        .await?;
        n += 1;
    }
    tracing::info!("已从种子导入 {n} 条岗位");
    Ok(n)
}

/// 用种子补全空字段；若种子中有库内尚不存在的「公司+岗位」则插入
pub async fn sync_descriptions_from_seed(pool: &DbPool) -> Result<usize> {
    let items = match read_seed_items() {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("{e:#}");
            return Ok(0);
        }
    };
    let mut n = 0;
    for item in items {
        let position = seed_field(&item, "position");
        let company = seed_field(&item, "company");
        if position.is_empty() || company.is_empty() {
            continue;
        }
        let job_description = seed_field(&item, "job_description");
        let source_url = seed_field(&item, "source_url");
        let listing_status = seed_field(&item, "listing_status");
        let city_mode = seed_field(&item, "city_mode");
        let experience = seed_field(&item, "experience");
        let source = seed_field(&item, "source");
        let company_nature = seed_field(&item, "company_nature");
        let is_outsourcing = item
            .get("is_outsourcing")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let headcount = seed_field(&item, "headcount");
        let email = seed_field(&item, "email");
        let website = seed_field(&item, "website");
        let hr_wechat = seed_field(&item, "hr_wechat");
        let email_body = default_email_body(company, position, job_description, city_mode);

        let (exists,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM company_jobs WHERE company = ? AND position = ?",
        )
        .bind(company)
        .bind(position)
        .fetch_one(pool)
        .await?;

        if exists == 0 {
            let listing = if listing_status.is_empty() {
                "未上市"
            } else {
                listing_status
            };
            sqlx::query(
                r#"
                INSERT INTO company_jobs
                  (position, company, city_mode, experience, source, source_url, company_nature,
                   is_outsourcing, headcount, listing_status, email, website, hr_wechat,
                   job_description, email_body)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(position)
            .bind(company)
            .bind(city_mode)
            .bind(experience)
            .bind(source)
            .bind(source_url)
            .bind(company_nature)
            .bind(is_outsourcing)
            .bind(headcount)
            .bind(listing)
            .bind(email)
            .bind(website)
            .bind(hr_wechat)
            .bind(job_description)
            .bind(&email_body)
            .execute(pool)
            .await?;
            n += 1;
            continue;
        }

        let result = sqlx::query(
            r#"
            UPDATE company_jobs
            SET
              job_description = CASE
                WHEN ? <> '' AND (job_description IS NULL OR TRIM(job_description) = '') THEN ?
                ELSE job_description END,
              source_url = CASE
                WHEN ? <> '' AND (source_url IS NULL OR TRIM(source_url) = '') THEN ?
                ELSE source_url END,
              listing_status = CASE
                WHEN ? <> '' AND (listing_status IS NULL OR TRIM(listing_status) = '' OR listing_status = '未上市')
                     AND ? <> '未上市' THEN ?
                WHEN listing_status IS NULL OR TRIM(listing_status) = '' THEN COALESCE(NULLIF(?, ''), '未上市')
                ELSE listing_status END,
              website = CASE
                WHEN ? <> '' AND (website IS NULL OR TRIM(website) = '' OR website = '未说明') THEN ?
                ELSE website END,
              email = CASE
                WHEN ? <> '' AND (email IS NULL OR TRIM(email) = '') THEN ?
                ELSE email END,
              hr_wechat = CASE
                WHEN ? <> '' AND (hr_wechat IS NULL OR TRIM(hr_wechat) = '') THEN ?
                ELSE hr_wechat END,
              experience = CASE
                WHEN ? <> '' AND (experience IS NULL OR TRIM(experience) = '' OR experience = '未说明') THEN ?
                ELSE experience END,
              email_body = CASE
                WHEN email_body IS NULL OR TRIM(email_body) = '' THEN ?
                ELSE email_body END
            WHERE company = ? AND position = ?
            "#,
        )
        .bind(job_description)
        .bind(job_description)
        .bind(source_url)
        .bind(source_url)
        .bind(listing_status)
        .bind(listing_status)
        .bind(listing_status)
        .bind(listing_status)
        .bind(website)
        .bind(website)
        .bind(email)
        .bind(email)
        .bind(hr_wechat)
        .bind(hr_wechat)
        .bind(experience)
        .bind(experience)
        .bind(&email_body)
        .bind(company)
        .bind(position)
        .execute(pool)
        .await?;
        n += result.rows_affected() as usize;
    }
    if n > 0 {
        tracing::info!("已从种子同步/补插 {n} 条岗位");
    }
    Ok(n)
}

const SELECT_COLS: &str = r#"
        id, position, company, city_mode, experience, source, source_url, company_nature,
        is_outsourcing, headcount, listing_status, email, website, hr_wechat,
        job_description, email_body, selected_resume_id,
        status_applied, status_written_test, status_interview1, status_interview2, status_offer,
        created_at, updated_at
"#;

pub async fn list_jobs(pool: &DbPool) -> Result<Vec<JobView>> {
    let sql = format!(
        "SELECT {SELECT_COLS} FROM company_jobs ORDER BY city_mode ASC, company ASC, id DESC"
    );
    let rows = sqlx::query_as::<_, JobRow>(&sql).fetch_all(pool).await?;
    Ok(rows.into_iter().map(JobView::from).collect())
}

pub async fn get_job(pool: &DbPool, id: u64) -> Result<Option<JobView>> {
    let sql = format!("SELECT {SELECT_COLS} FROM company_jobs WHERE id = ?");
    let row = sqlx::query_as::<_, JobRow>(&sql)
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(JobView::from))
}

pub async fn create_job(pool: &DbPool, req: UpsertJobRequest) -> Result<JobView> {
    let position = req.position.trim().to_string();
    let company = req.company.trim().to_string();
    if position.is_empty() || company.is_empty() {
        anyhow::bail!("岗位与公司不能为空");
    }
    let city_mode = req.city_mode.unwrap_or_default();
    let experience = req.experience.unwrap_or_default();
    let source = req.source.unwrap_or_default();
    let source_url = req.source_url.unwrap_or_default();
    let company_nature = req.company_nature.unwrap_or_default();
    let is_outsourcing = req.is_outsourcing.unwrap_or(false);
    let headcount = req.headcount.unwrap_or_default();
    let listing_status = req
        .listing_status
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "未上市".into());
    let email = req.email.unwrap_or_default();
    let website = req.website.unwrap_or_default();
    let hr_wechat = req.hr_wechat.unwrap_or_default();
    let job_description = req.job_description.unwrap_or_default();
    let email_body = req
        .email_body
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| {
            default_email_body(&company, &position, &job_description, &city_mode)
        });
    let selected_resume_id = req.selected_resume_id;
    let status_applied = req.status_applied.unwrap_or(false);
    let status_written_test = req.status_written_test.unwrap_or(false);
    let status_interview1 = req.status_interview1.unwrap_or(false);
    let status_interview2 = req.status_interview2.unwrap_or(false);
    let status_offer = req.status_offer.unwrap_or(false);

    let result = sqlx::query(
        r#"
        INSERT INTO company_jobs
          (position, company, city_mode, experience, source, source_url, company_nature,
           is_outsourcing, headcount, listing_status, email, website, hr_wechat,
           job_description, email_body, selected_resume_id,
           status_applied, status_written_test, status_interview1, status_interview2, status_offer)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&position)
    .bind(&company)
    .bind(&city_mode)
    .bind(&experience)
    .bind(&source)
    .bind(&source_url)
    .bind(&company_nature)
    .bind(is_outsourcing)
    .bind(&headcount)
    .bind(&listing_status)
    .bind(&email)
    .bind(&website)
    .bind(&hr_wechat)
    .bind(&job_description)
    .bind(&email_body)
    .bind(selected_resume_id)
    .bind(status_applied)
    .bind(status_written_test)
    .bind(status_interview1)
    .bind(status_interview2)
    .bind(status_offer)
    .execute(pool)
    .await?;

    let id = result.last_insert_id();
    get_job(pool, id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("创建后读取失败"))
}

pub async fn update_job(pool: &DbPool, id: u64, req: UpsertJobRequest) -> Result<Option<JobView>> {
    let existing = match get_job(pool, id).await? {
        Some(j) => j,
        None => return Ok(None),
    };

    let position = req.position.trim().to_string();
    let company = req.company.trim().to_string();
    if position.is_empty() || company.is_empty() {
        anyhow::bail!("岗位与公司不能为空");
    }
    let city_mode = req.city_mode.unwrap_or(existing.city_mode);
    let experience = req.experience.unwrap_or(existing.experience);
    let source = req.source.unwrap_or(existing.source);
    let source_url = req.source_url.unwrap_or(existing.source_url);
    let company_nature = req.company_nature.unwrap_or(existing.company_nature);
    let is_outsourcing = req.is_outsourcing.unwrap_or(existing.is_outsourcing);
    let headcount = req.headcount.unwrap_or(existing.headcount);
    let listing_status = req.listing_status.unwrap_or(existing.listing_status);
    let email = req.email.unwrap_or(existing.email);
    let website = req.website.unwrap_or(existing.website);
    let hr_wechat = req.hr_wechat.unwrap_or(existing.hr_wechat);
    let job_description = req.job_description.unwrap_or(existing.job_description);
    let email_body = req
        .email_body
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| {
            if existing.email_body.trim().is_empty() {
                default_email_body(&company, &position, &job_description, &city_mode)
            } else {
                existing.email_body.clone()
            }
        });
    let selected_resume_id = req.selected_resume_id.or(existing.selected_resume_id);
    let status_applied = req.status_applied.unwrap_or(existing.status_applied);
    let status_written_test = req.status_written_test.unwrap_or(existing.status_written_test);
    let status_interview1 = req.status_interview1.unwrap_or(existing.status_interview1);
    let status_interview2 = req.status_interview2.unwrap_or(existing.status_interview2);
    let status_offer = req.status_offer.unwrap_or(existing.status_offer);

    sqlx::query(
        r#"
        UPDATE company_jobs SET
          position = ?, company = ?, city_mode = ?, experience = ?, source = ?, source_url = ?,
          company_nature = ?, is_outsourcing = ?, headcount = ?, listing_status = ?,
          email = ?, website = ?, hr_wechat = ?, job_description = ?, email_body = ?,
          selected_resume_id = ?,
          status_applied = ?, status_written_test = ?, status_interview1 = ?,
          status_interview2 = ?, status_offer = ?
        WHERE id = ?
        "#,
    )
    .bind(&position)
    .bind(&company)
    .bind(&city_mode)
    .bind(&experience)
    .bind(&source)
    .bind(&source_url)
    .bind(&company_nature)
    .bind(is_outsourcing)
    .bind(&headcount)
    .bind(&listing_status)
    .bind(&email)
    .bind(&website)
    .bind(&hr_wechat)
    .bind(&job_description)
    .bind(&email_body)
    .bind(selected_resume_id)
    .bind(status_applied)
    .bind(status_written_test)
    .bind(status_interview1)
    .bind(status_interview2)
    .bind(status_offer)
    .bind(id)
    .execute(pool)
    .await?;

    get_job(pool, id).await
}

pub async fn delete_job(pool: &DbPool, id: u64) -> Result<bool> {
    let result = sqlx::query("DELETE FROM company_jobs WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn list_job_resumes(pool: &DbPool, job_id: u64) -> Result<Vec<ResumeListItem>> {
    let job = get_job(pool, job_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("岗位不存在"))?;
    let rows = sqlx::query_as::<_, ResumeListItem>(
        r#"
        SELECT id, company, version_tag, saved_at, note,
               candidate_name, job_intent, created_at, job_id
        FROM resume_versions
        WHERE job_id = ? OR (company = ? AND (job_id IS NULL OR job_id = ?))
        ORDER BY saved_at DESC, id DESC
        "#,
    )
    .bind(job_id)
    .bind(&job.company)
    .bind(job_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn set_selected_resume(
    pool: &DbPool,
    job_id: u64,
    resume_id: u64,
) -> Result<Option<JobView>> {
    let exists: Option<(u64,)> =
        sqlx::query_as("SELECT id FROM resume_versions WHERE id = ?")
            .bind(resume_id)
            .fetch_optional(pool)
            .await?;
    if exists.is_none() {
        anyhow::bail!("简历版本不存在");
    }
    // 绑定 job_id，便于后续按公司筛选
    sqlx::query("UPDATE resume_versions SET job_id = ? WHERE id = ? AND (job_id IS NULL OR job_id = ?)")
        .bind(job_id)
        .bind(resume_id)
        .bind(job_id)
        .execute(pool)
        .await?;

    let result = sqlx::query("UPDATE company_jobs SET selected_resume_id = ? WHERE id = ?")
        .bind(resume_id)
        .bind(job_id)
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Ok(None);
    }
    get_job(pool, job_id).await
}

async fn next_version_tag(pool: &DbPool, company: &str) -> Result<String> {
    let tags: Vec<(String,)> =
        sqlx::query_as("SELECT version_tag FROM resume_versions WHERE company = ?")
            .bind(company)
            .fetch_all(pool)
            .await?;
    let mut max = 0u32;
    for (t,) in tags {
        if let Some(rest) = t.strip_prefix('v').or_else(|| t.strip_prefix('V')) {
            if let Ok(n) = rest.parse::<u32>() {
                max = max.max(n);
            }
        }
    }
    Ok(format!("v{}", max + 1))
}

async fn find_template_resume(
    pool: &DbPool,
) -> Result<Option<(u64, String, Option<String>, Option<String>, Option<String>)>> {
    // (id, html, style, photo, candidate)
    let row: Option<(u64, String, Option<String>, Option<String>, Option<String>)> =
        sqlx::query_as(
            r#"
            SELECT id, html_snapshot, style_css, photo_data_url, candidate_name
            FROM resume_versions
            WHERE company IN ('通用', '通用简历', '模板', 'template')
            ORDER BY saved_at DESC, id DESC
            LIMIT 1
            "#,
        )
        .fetch_optional(pool)
        .await?;
    if row.is_some() {
        return Ok(row);
    }
    let any: Option<(u64, String, Option<String>, Option<String>, Option<String>)> =
        sqlx::query_as(
            r#"
            SELECT id, html_snapshot, style_css, photo_data_url, candidate_name
            FROM resume_versions
            ORDER BY saved_at DESC, id DESC
            LIMIT 1
            "#,
        )
        .fetch_optional(pool)
        .await?;
    Ok(any)
}

/// 为岗位确保一份可编辑简历：已有则返回最新/已选；否则从「通用」模板克隆。
pub async fn ensure_resume_for_job(pool: &DbPool, job_id: u64) -> Result<EnsureResumeResponse> {
    let job = get_job(pool, job_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("岗位不存在"))?;

    if let Some(rid) = job.selected_resume_id {
        let ok: Option<(u64, String, String)> = sqlx::query_as(
            "SELECT id, company, version_tag FROM resume_versions WHERE id = ?",
        )
        .bind(rid)
        .fetch_optional(pool)
        .await?;
        if let Some((id, company, version_tag)) = ok {
            return Ok(EnsureResumeResponse {
                resume_id: id,
                company,
                version_tag,
                created: false,
            });
        }
    }

    // 已有绑定到本岗位的简历 → 选最新
    let existing: Option<(u64, String, String)> = sqlx::query_as(
        r#"
        SELECT id, company, version_tag FROM resume_versions
        WHERE job_id = ?
        ORDER BY saved_at DESC, id DESC
        LIMIT 1
        "#,
    )
    .bind(job_id)
    .fetch_optional(pool)
    .await?;
    if let Some((id, company, version_tag)) = existing {
        sqlx::query("UPDATE company_jobs SET selected_resume_id = ? WHERE id = ?")
            .bind(id)
            .bind(job_id)
            .execute(pool)
            .await?;
        return Ok(EnsureResumeResponse {
            resume_id: id,
            company,
            version_tag,
            created: false,
        });
    }

    let template = find_template_resume(pool)
        .await?
        .ok_or_else(|| {
            anyhow::anyhow!(
                "NO_TEMPLATE:请先在简历模块保存一份「通用」模板（面试公司填「通用」），再点编辑简历"
            )
        })?;

    let (_tid, html, style, photo, cand) = template;
    let version_tag = next_version_tag(pool, &job.company).await?;
    let note = format!("由岗位「{} · {}」从通用模板自动创建", job.company, job.position);

    let result = sqlx::query(
        r#"
        INSERT INTO resume_versions
          (company, job_id, version_tag, saved_at, note,
           html_snapshot, style_css, photo_data_url, candidate_name, job_intent)
        VALUES (?, ?, ?, NOW(3), ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&job.company)
    .bind(job_id)
    .bind(&version_tag)
    .bind(&note)
    .bind(&html)
    .bind(style.as_deref())
    .bind(photo.as_deref())
    .bind(cand.as_deref())
    .bind(job.position.as_str())
    .execute(pool)
    .await
    .context("克隆简历失败")?;

    let resume_id = result.last_insert_id();
    sqlx::query("UPDATE company_jobs SET selected_resume_id = ? WHERE id = ?")
        .bind(resume_id)
        .bind(job_id)
        .execute(pool)
        .await?;

    Ok(EnsureResumeResponse {
        resume_id,
        company: job.company,
        version_tag,
        created: true,
    })
}
