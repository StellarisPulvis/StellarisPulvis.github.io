# Stellaris Pulvis

极简主义博客系统。Markdown 写作，双语言 UI，静态输出。

## 目录结构

```
├── content/posts/       ← 文章 (.md)
├── templates/            ← HTML 模板
├── static/               ← 静态资源 (CSS/JS/图片)
├── config.toml           ← 站点配置
├── src/                  ← 构建工具源码 (Rust)
└── output/               ← 构建产物 (自动生成)
```

## 写博客

在 `content/posts/` 下创建 `.md` 文件：

```markdown
---
title: "我的新文章"
date: 2026-07-01
tags: ["随笔", "DIY"]
draft: true
---

这里是正文，支持 **Markdown** 语法。

- 列表
- 引用
- 代码块
```

### Frontmatter 字段

| 字段    | 必填 | 说明                          |
| ------- | ---- | ------------------------------ |
| title   | 是   | 文章标题                       |
| date    | 是   | 日期，格式 `YYYY-MM-DD`        |
| tags    | 否   | 标签列表，如 `["随笔", "DIY"]` |
| draft   | 否   | 设为 `true` 则构建时跳过        |

### 草稿

将 `draft` 设为 `true`，构建时不会发布。本地预览时可在 `config.toml` 中设置 `include_drafts = true` 来显示。

## 本地预览

```bash
cargo run -- serve
```

打开 http://localhost:8080，修改内容后浏览器自动刷新。

## 构建

```bash
cargo run -- build
```

生成静态文件到 `output/` 目录。

## 发布

```bash
git add -A
git commit -m "更新"
git push
```

GitHub Actions 会自动构建并部署到 GitHub Pages。

## 配置

编辑 `config.toml`：

### 站点信息

```toml
[site]
languages = ["zh", "en"]   # 启用的语言
default = "zh"             # 根路径默认语言

[author]
name = "Your Name"
email = "your@email.com"
```

### 构建选项

```toml
[build]
posts_per_page = 10        # 每页文章数
include_drafts = true      # 构建时是否包含草稿
syntax_theme = "base16-ocean.dark"
```

### 语言文本

```toml
[languages.zh]
title = "Stellaris Pulvis"
description = "一个极简主义 DIY 爱好者的知识共享站"
nav_home = "首页"
nav_tags = "标签"
nav_about = "关于"
read_more = "阅读全文"
search_empty = "未找到相关文章"
```

### 评论（giscus）

取消 `config.toml` 底部注释并填入仓库信息即可启用 GitHub 评论：

```toml
[comment]
enable = true
repo = "你的用户名/你的仓库"
repo_id = "R_kgXXXXXXXXX"
category = "Announcements"
category_id = "DIC_kwXXXXXXXXX"
mapping = "pathname"
lang = "zh-CN"
```

## 文章 slug

文件名决定 URL slug：

- `hello-world.md` → `/zh/posts/hello-world/` / `/en/posts/hello-world/`
- `my-diy-project.md` → `/zh/posts/my-diy-project/` / `/en/posts/my-diy-project/`

## 关于页面

编辑 `templates/about.html`，目前通过 `{% if lang == "zh" %}` 区分中英文内容。

## 提示

- 使用 `cargo run -- init` 初始化新项目
- 构建产物在 `output/`，部署时只需要推送源码，CI 会自动构建
- 文章数量过多时会自动分页
