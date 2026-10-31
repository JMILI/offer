use anyhow::{anyhow, Context, Result};
use lettre::message::{Attachment, MultiPart, SinglePart};
use lettre::message::{header::ContentType, Mailbox};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use serde::Deserialize;

use crate::db::{self, DbPool};

#[derive(Clone)]
pub struct MailConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from: String,
    pub test_to: String,
    pub use_starttls: bool,
}

impl MailConfig {
    pub fn from_env() -> Result<Self> {
        let host = std::env::var("SMTP_HOST").unwrap_or_else(|_| "smtp.qq.com".into());
        let port: u16 = std::env::var("SMTP_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(465);
        let username = std::env::var("SMTP_USER").unwrap_or_default();
        let password = std::env::var("SMTP_PASS").unwrap_or_default();
        let from = std::env::var("SMTP_FROM").unwrap_or_else(|_| username.clone());
        let test_to =
            std::env::var("TEST_EMAIL_TO").unwrap_or_else(|_| "923749851@qq.com".into());
        let use_starttls = std::env::var("SMTP_STARTTLS")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        Ok(Self {
            host,
            port,
            username,
            password,
            from,
            test_to,
            use_starttls,
        })
    }

    pub fn is_configured(&self) -> bool {
        !self.host.is_empty()
            && !self.username.is_empty()
            && !self.password.is_empty()
            && !self.from.is_empty()
    }
}

#[derive(Debug, Deserialize)]
pub struct SendEmailRequest {
    pub job_id: Option<u64>,
    /// 指定附带的简历版本；缺省则用岗位 selected_resume_id
    pub resume_id: Option<u64>,
    pub to: Option<String>,
    pub subject: String,
    pub body: String,
}

fn parse_mailbox(addr: &str) -> Result<Mailbox> {
    addr.trim()
        .parse::<Mailbox>()
        .with_context(|| format!("无效邮箱地址: {addr}"))
}

async fn build_transport(
    cfg: &MailConfig,
) -> Result<AsyncSmtpTransport<Tokio1Executor>> {
    let creds = Credentials::new(cfg.username.clone(), cfg.password.clone());
    if cfg.use_starttls || cfg.port == 587 {
        let mailer = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&cfg.host)
            .with_context(|| format!("SMTP STARTTLS 连接失败: {}", cfg.host))?
            .port(cfg.port)
            .credentials(creds)
            .build();
        Ok(mailer)
    } else {
        let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay(&cfg.host)
            .with_context(|| format!("SMTP TLS 连接失败: {}", cfg.host))?
            .port(cfg.port)
            .credentials(creds)
            .build();
        Ok(mailer)
    }
}

struct ResumeAttachment {
    filename: String,
    bytes: Vec<u8>,
    is_pdf: bool,
}

async fn load_resume_attachment(
    pool: &DbPool,
    job_id: Option<u64>,
    resume_id: Option<u64>,
) -> Result<Option<ResumeAttachment>> {
    let mut rid = resume_id;
    if rid.is_none() {
        if let Some(jid) = job_id {
            let row: Option<(Option<u64>,)> =
                sqlx::query_as("SELECT selected_resume_id FROM company_jobs WHERE id = ?")
                    .bind(jid)
                    .fetch_optional(pool)
                    .await?;
            rid = row.and_then(|r| r.0);
        }
    }
    let Some(id) = rid else {
        return Ok(None);
    };
    let detail = db::get_resume(pool, id)
        .await?
        .ok_or_else(|| anyhow!("选中的简历版本不存在 (id={id})"))?;

    let safe_company: String = detail
        .company
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let cand = detail
        .candidate_name
        .clone()
        .unwrap_or_else(|| "李江敏".into());

    // 优先附带服务端已生成的 PDF（保存简历时自动上传，不落本机）
    if let Some(pdf) = db::get_resume_pdf(pool, id).await? {
        let filename = format!("简历_{cand}_{safe_company}_{}.pdf", detail.version_tag);
        return Ok(Some(ResumeAttachment {
            filename,
            bytes: pdf,
            is_pdf: true,
        }));
    }

    // 回退：附 HTML 快照
    let style = detail.style_css.unwrap_or_default();
    let html = format!(
        r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="UTF-8">
<title>简历 - {} - {}</title>
<style>
{}
</style>
</head>
<body>
{}
</body>
</html>"#,
        detail.company, detail.version_tag, style, detail.html_snapshot
    );
    let filename = format!("简历_{cand}_{safe_company}_{}.html", detail.version_tag);

    Ok(Some(ResumeAttachment {
        filename,
        bytes: html.into_bytes(),
        is_pdf: false,
    }))
}

async fn send_raw(
    cfg: &MailConfig,
    to: &str,
    subject: &str,
    body: &str,
    attachment: Option<ResumeAttachment>,
) -> Result<()> {
    if !cfg.is_configured() {
        return Err(anyhow!(
            "未配置 SMTP：请在 .env 填写 SMTP_USER / SMTP_PASS / SMTP_FROM（QQ 邮箱授权码）"
        ));
    }
    let from = parse_mailbox(&cfg.from)?;
    let to_mb = parse_mailbox(to)?;

    let builder = Message::builder()
        .from(from)
        .to(to_mb)
        .subject(subject);

    let email = if let Some(att) = attachment {
        let text = SinglePart::builder()
            .header(ContentType::TEXT_PLAIN)
            .body(body.to_string());
        let ctype = if att.is_pdf {
            "application/pdf"
                .parse::<ContentType>()
                .expect("application/pdf")
        } else {
            ContentType::TEXT_HTML
        };
        let file = Attachment::new(att.filename).body(att.bytes, ctype);
        builder
            .multipart(
                MultiPart::mixed()
                    .singlepart(text)
                    .singlepart(file),
            )
            .context("构造带附件邮件失败")?
    } else {
        builder
            .header(ContentType::TEXT_PLAIN)
            .body(body.to_string())
            .context("构造邮件失败")?
    };

    let mailer = build_transport(cfg).await?;
    mailer.send(email).await.context("SMTP 发送失败")?;
    Ok(())
}

/// 测试发信：固定发到 TEST_EMAIL_TO，可附带选中简历。
pub async fn send_test(
    cfg: &MailConfig,
    pool: &DbPool,
    req: SendEmailRequest,
) -> Result<String> {
    let subject = if req.subject.contains("测试") {
        req.subject
    } else {
        format!("【测试投递】{}", req.subject)
    };
    let att = load_resume_attachment(pool, req.job_id, req.resume_id).await?;
    let att_note = if att.is_some() {
        "（已附带简历 HTML）"
    } else {
        "（未附带简历：请先为岗位创建/选择简历版本）"
    };
    send_raw(cfg, &cfg.test_to, &subject, &req.body, att).await?;
    Ok(format!("测试邮件已发送至 {} {}", cfg.test_to, att_note))
}

/// 正式发信：发往公司邮箱，自动附带选中简历。
pub async fn send_company(
    cfg: &MailConfig,
    pool: &DbPool,
    req: SendEmailRequest,
) -> Result<String> {
    let to = req
        .to
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("缺少公司收件邮箱 to"))?;

    if to.eq_ignore_ascii_case(&cfg.test_to) {
        return Err(anyhow!(
            "正式发信目标不能是测试邮箱；请使用「测试发信」按钮"
        ));
    }
    if !to.contains('@') {
        return Err(anyhow!("公司邮箱无效: {to}"));
    }

    let att = load_resume_attachment(pool, req.job_id, req.resume_id)
        .await?
        .ok_or_else(|| {
            anyhow!("请先点击「编辑简历」并保存，或在下拉框选择简历版本后再发送")
        })?;
    let fname = att.filename.clone();
    send_raw(cfg, to, &req.subject, &req.body, Some(att)).await?;

    // 标记已投递
    if let Some(jid) = req.job_id {
        let _ = sqlx::query("UPDATE company_jobs SET status_applied = 1 WHERE id = ?")
            .bind(jid)
            .execute(pool)
            .await;
    }

    Ok(format!("已向公司发送邮件: {to}（附件 {fname}）"))
}
