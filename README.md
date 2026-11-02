# offer — 求职工作台

首页两个模块：

1. **编辑保存简历** — 预览 / 编辑 / 导出 PDF，版本写入 MySQL  
2. **收集公司岗位信息** — 表格维护岗位与联系方式；测试发信与正式发信分离  

## 启动

```bash
cd deployProject/offer

# 1) 环境变量：仓库里已有可用 .env 时跳过；没有就复制模板再填
[ -f .env ] || cp .env.example .env    # DATABASE_URL / SMTP_* 必填

# 2) 首次：建库 + 建表（MySQL 8，库名 offer）
mysql -uroot -p -e "CREATE DATABASE IF NOT EXISTS offer DEFAULT CHARSET utf8mb4;"
mysql -uroot -p offer < sql/schema.sql

# 3) 启动（空库首次启动会自动从 data/jobs_seed.json 导入种子岗位）
cargo run
```

浏览器打开 `http://127.0.0.1:4000/`：

| 路径 | 说明 |
|------|------|
| `/` | 首页路由 |
| `/resume.html` | 简历模块 |
| `/jobs.html` | 岗位收集模块 |

后端监听 `HOST=0.0.0.0`、`PORT=4000`（`src/main.rs` 的默认值）。`cp .env.example .env` 后模板里的 `HOST=127.0.0.1` 只允许本机访问，需要局域网或隧道访问时改成 `0.0.0.0`。

公网隧道（Sealtun `offer-ui` → 本地 4000）如已配置可继续用。

## SMTP（发信必填）

QQ 邮箱请使用**授权码**（不是登录密码）：

```env
SMTP_HOST=smtp.qq.com
SMTP_PORT=465
SMTP_USER=你的QQ号@qq.com
SMTP_PASS=授权码
SMTP_FROM=你的QQ号@qq.com
TEST_EMAIL_TO=923749851@qq.com
```

| 按钮 | 行为 |
|------|------|
| **测试邮件发送** | 固定发到 `TEST_EMAIL_TO`（默认 `923749851@qq.com`，与 `src/email.rs` 一致），验证 SMTP |
| **发送邮件** | 发到该公司邮箱；需输入「确认发送」二次确认 |

两按钮独立，避免误把测试邮件打到公司。

## 岗位表字段

岗位 / 公司 / 就职城市（远程·坐班）/ 经验 / 检索来源 / 公司性质 / 外包或正规 / 人数规模 / 公司邮件 / 官网 / HR 微信 / 邮件内容 / 测试发信 / 发送邮件  

邮件正文默认按李江敏简历生成规范求职信，可逐行编辑。空库启动时从 `data/jobs_seed.json` 导入种子。

## API

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/health` | 健康检查（含 smtp_configured） |
| GET/POST | `/api/resumes` | 简历列表 / 保存 |
| GET/DELETE | `/api/resumes/:id` | 读 / 删简历 |
| GET/POST | `/api/jobs` | 岗位列表 / 新增 |
| GET/PUT/DELETE | `/api/jobs/:id` | 读 / 改 / 删岗位 |
| POST | `/api/email/test` | 测试发信 → TEST_EMAIL_TO |
| POST | `/api/email/send` | 正式发信 → 公司邮箱 |

## 项目结构

```text
offer/
├── index.html          # 首页两模块入口
├── resume.html         # 简历编辑器
├── jobs.html           # 岗位表格
├── data/jobs_seed.json # 岗位种子
├── sql/schema.sql
└── src/
    ├── main.rs
    ├── db.rs / models.rs
    ├── jobs.rs
    └── email.rs
```
