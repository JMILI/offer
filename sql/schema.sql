-- 简历版本持久化（MySQL 8+）
-- 设计原则：以 HTML 快照完整还原样式；结构化子表便于检索与后续分析。

CREATE DATABASE IF NOT EXISTS resume_db
  DEFAULT CHARACTER SET utf8mb4
  DEFAULT COLLATE utf8mb4_unicode_ci;

USE resume_db;

-- ---------------------------------------------------------------------------
-- 主表：一次「保存」= 一个版本快照
-- 还原展示/再编辑时，优先使用 html_snapshot + photo_data + style_css
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS resume_versions (
  id              BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  company         VARCHAR(128)    NOT NULL COMMENT '面试公司',
  version_tag     VARCHAR(32)     NOT NULL COMMENT '版本号，如 v1 / v2',
  saved_at        DATETIME(3)     NOT NULL COMMENT '用户填写的保存时间',
  note            VARCHAR(512)    NULL COMMENT '备注（可选）',

  -- 样式与内容一体快照（contenteditable 内联加粗/字号等必须靠此还原）
  html_snapshot   MEDIUMTEXT      NOT NULL COMMENT '#resume-root 的完整 innerHTML（含内联加粗/字号）',
  style_css       MEDIUMTEXT      NULL COMMENT '页面 <style> 全文，确保历史版本样式一致',
  photo_data_url  MEDIUMTEXT      NULL COMMENT '证件照 data:URL（还原时写回 img#resume-photo）',

  -- 从页面抽取的摘要，便于列表展示与检索（非还原主路径）
  candidate_name  VARCHAR(64)     NULL,
  job_intent      VARCHAR(128)    NULL,
  modules_json    JSON            NULL COMMENT '结构化模块摘要，见下方注释',

  created_at      DATETIME(3)     NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  updated_at      DATETIME(3)     NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),

  PRIMARY KEY (id),
  UNIQUE KEY uk_company_version (company, version_tag),
  KEY idx_saved_at (saved_at DESC),
  KEY idx_company (company)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci
  COMMENT='简历版本主表：元数据 + 带样式 HTML 快照';

/*
modules_json 建议结构（与页面模块对应，存文本摘要，不替代 html_snapshot）：

{
  "header": {
    "name": "李江敏",
    "job_intent": "Rust, C++ 开发工程师",
    "birth": "1997.02",
    "phone": "...",
    "email": "...",
    "location": "不限城市"
  },
  "education": [
    { "school": "...", "major": "...", "details_html": "..." }
  ],
  "skills": [
    { "category": "Rust 方面", "content": "..." },
    { "category": "C++ 方面", "content": "..." }
  ],
  "work": [
    {
      "company": "西安海讯科技",
      "role": "C++ 开发工程师",
      "date_range": "2025.11 — 2026.07",
      "dept": "...",
      "intro": "...",
      "summary": "...",
      "items": [
        { "title": "1. 轨迹坐标...", "body": "..." }
      ]
    }
  ],
  "campus_projects": [ ... ],
  "ongoing": [ ... ],
  "self_eval": [ "1. ...", "2. ..." ]
}
*/

-- ---------------------------------------------------------------------------
-- 可选明细表：专业技术（按类别一行）
-- 用途：按技能关键词搜索；还原仍以 html_snapshot 为准
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS resume_skills (
  id           BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  version_id   BIGINT UNSIGNED NOT NULL,
  sort_order   INT             NOT NULL DEFAULT 0,
  category     VARCHAR(64)     NOT NULL COMMENT '如 Rust 方面 / C++ 方面',
  content      TEXT            NOT NULL,
  PRIMARY KEY (id),
  KEY idx_version (version_id),
  CONSTRAINT fk_skills_version
    FOREIGN KEY (version_id) REFERENCES resume_versions(id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ---------------------------------------------------------------------------
-- 可选明细表：工作经历
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS resume_work (
  id           BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  version_id   BIGINT UNSIGNED NOT NULL,
  sort_order   INT             NOT NULL DEFAULT 0,
  company      VARCHAR(128)    NOT NULL,
  role_title   VARCHAR(128)    NULL,
  date_range   VARCHAR(64)     NULL,
  dept_post    VARCHAR(256)    NULL COMMENT '部门岗位',
  company_intro TEXT           NULL COMMENT '公司介绍',
  job_summary  TEXT            NULL COMMENT '工作描述总述',
  PRIMARY KEY (id),
  KEY idx_version (version_id),
  CONSTRAINT fk_work_version
    FOREIGN KEY (version_id) REFERENCES resume_versions(id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ---------------------------------------------------------------------------
-- 工作经历下的编号事项（如图 1/2/3）
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS resume_work_items (
  id           BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  work_id      BIGINT UNSIGNED NOT NULL,
  sort_order   INT             NOT NULL DEFAULT 0,
  title        VARCHAR(256)    NOT NULL,
  body         TEXT            NOT NULL,
  PRIMARY KEY (id),
  KEY idx_work (work_id),
  CONSTRAINT fk_work_items
    FOREIGN KEY (work_id) REFERENCES resume_work(id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ---------------------------------------------------------------------------
-- 教育 / 在校项目 / 在研 / 自我评价（轻量结构化）
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS resume_sections (
  id           BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  version_id   BIGINT UNSIGNED NOT NULL,
  section_type ENUM('education','campus_project','ongoing','self_eval','other') NOT NULL,
  sort_order   INT             NOT NULL DEFAULT 0,
  title        VARCHAR(256)    NULL,
  subtitle     VARCHAR(256)    NULL,
  date_range   VARCHAR(64)     NULL,
  body_html    MEDIUMTEXT      NULL COMMENT '该条目原始 HTML 片段（含内联样式）',
  PRIMARY KEY (id),
  KEY idx_version_type (version_id, section_type),
  CONSTRAINT fk_sections_version
    FOREIGN KEY (version_id) REFERENCES resume_versions(id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ---------------------------------------------------------------------------
-- 公司岗位收集（投递跟踪）
-- 服务启动时也会自动建表；空表时从 data/jobs_seed.json 导入
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS company_jobs (
  id                  BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  position            VARCHAR(256)    NOT NULL COMMENT '岗位',
  company             VARCHAR(256)    NOT NULL COMMENT '公司',
  city_mode           VARCHAR(256)    NOT NULL DEFAULT '' COMMENT '城市/远程或坐班',
  experience          VARCHAR(128)    NOT NULL DEFAULT '',
  source              VARCHAR(128)    NOT NULL DEFAULT '' COMMENT '检索来源名称',
  source_url          VARCHAR(512)    NOT NULL DEFAULT '' COMMENT '招聘信息链接',
  company_nature      VARCHAR(256)    NOT NULL DEFAULT '' COMMENT '公司性质',
  is_outsourcing      TINYINT(1)      NOT NULL DEFAULT 0 COMMENT '1=外包 0=正规',
  headcount           VARCHAR(128)    NOT NULL DEFAULT '' COMMENT '人数规模',
  listing_status      VARCHAR(64)     NOT NULL DEFAULT '未上市' COMMENT '上市情况',
  email               VARCHAR(256)    NOT NULL DEFAULT '',
  website             VARCHAR(512)    NOT NULL DEFAULT '',
  hr_wechat           VARCHAR(128)    NOT NULL DEFAULT '',
  job_description     MEDIUMTEXT      NOT NULL COMMENT '岗位描述/任职要求',
  email_body          MEDIUMTEXT      NOT NULL COMMENT '投递邮件正文',
  selected_resume_id  BIGINT UNSIGNED NULL COMMENT '当前选中的简历版本',
  status_applied      TINYINT(1)      NOT NULL DEFAULT 0 COMMENT '已投递',
  status_written_test TINYINT(1)      NOT NULL DEFAULT 0 COMMENT '笔试',
  status_interview1   TINYINT(1)      NOT NULL DEFAULT 0 COMMENT '一面',
  status_interview2   TINYINT(1)      NOT NULL DEFAULT 0 COMMENT '二面',
  status_offer        TINYINT(1)      NOT NULL DEFAULT 0 COMMENT '成功接收 offer',
  created_at          DATETIME(3)     NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  updated_at          DATETIME(3)     NOT NULL DEFAULT CURRENT_TIMESTAMP(3)
                                     ON UPDATE CURRENT_TIMESTAMP(3),
  PRIMARY KEY (id),
  KEY idx_company (company),
  KEY idx_updated (updated_at DESC)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci
  COMMENT='公司岗位信息收集表';

-- resume_versions 增加 job_id 关联（服务启动时也会 ALTER）
-- ALTER TABLE resume_versions ADD COLUMN job_id BIGINT UNSIGNED NULL AFTER company;
