mod builder;
mod config;
mod content;
mod feed;
mod gui;
mod markdown;
mod serve;
mod template;

use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use anyhow::Result;
use clap::{Parser, Subcommand};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::broadcast;
use tracing_subscriber::EnvFilter;

use builder::SiteBuilder;

#[derive(Parser)]
#[command(name = "stellaris", about = "Stellaris Pulvis — A minimalist knowledge-sharing blog")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(short, long, default_value = ".")]
    project: PathBuf,
}

#[derive(Subcommand)]
enum Commands {
    /// Build the static site
    Build,
    /// Build and serve with hot reload
    Serve {
        #[arg(short, long, default_value_t = 8080)]
        port: u16,
    },
    /// Initialize a new blog project
    Init,
    /// Launch the desktop GUI manager
    Gui,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let cli = Cli::parse();
    let project_dir = cli.project;

    match cli.command.unwrap_or(Commands::Build) {
        Commands::Init => cmd_init(&project_dir),
        Commands::Build => cmd_build(&project_dir),
        Commands::Serve { port } => cmd_serve(&project_dir, port).await,
        Commands::Gui => cmd_gui(&project_dir),
    }
}

fn cmd_init(project_dir: &PathBuf) -> Result<()> {
    let dirs = [
        "content/zh/posts",
        "content/en/posts",
        "templates",
        "static/css",
        "static/js",
        "output",
    ];
    for dir in &dirs {
        std::fs::create_dir_all(project_dir.join(dir))?;
    }

    std::fs::write(
        project_dir.join("config.toml"),
        r#"[site]
languages = ["zh", "en"]
default = "zh"

[author]
name = "Your Name"
email = "your@email.com"

[build]
posts_per_page = 10
include_drafts = true
syntax_theme = "base16-ocean.dark"

[languages.zh]
title = "Stellaris Pulvis"
description = "一个极简主义 DIY 爱好者的知识共享站"
nav_home = "首页"
nav_tags = "标签"
nav_about = "关于"
nav_search = "搜索文章"
read_more = "阅读全文"
tags_title = "标签"
comment_title = "留言"
pagination_prev = "上一页"
pagination_next = "下一页"
search_empty = "未找到相关文章"
announcements_title = "公告"

[[languages.zh.announcements]]
title = "🚀 新博客上线"
content = "星尘之下，分享每一个想法，倾听每一种声音，一起成长。"
date = "2026-07"

[languages.en]
title = "Stellaris Pulvis"
description = "A minimalist DIY enthusiast sharing ideas and growing together"
nav_home = "Home"
nav_tags = "Tags"
nav_about = "About"
nav_search = "Search posts"
read_more = "Read more"
tags_title = "Tags"
comment_title = "Comments"
pagination_prev = "Previous"
pagination_next = "Next"
search_empty = "No results found"
announcements_title = "Announcements"

[[languages.en.announcements]]
title = "🚀 New Blog Launched"
content = "Beneath the stars, sharing ideas, listening to voices, growing together."
date = "2026-07"

# giscus (optional) - uncomment and fill in your repo info
# [comment]
# enable = true
# repo = "owner/repo"
# repo_id = "R_kgXXXXXXXXX"
# category = "Announcements"
# category_id = "DIC_kwXXXXXXXXX"
# mapping = "pathname"
# lang = "zh-CN"
"#,
    )?;

    // 中文示例文章
    std::fs::write(
        project_dir.join("content/zh/posts/hello-world.md"),
        r#"---
title: "星尘之下"
date: 2024-01-01
tags: ["随笔"]
draft: false
---

欢迎来到 Stellaris Pulvis。

这里是星辰落下的一粒尘埃，也是我们共同生长的一片土壤。

我们聊的是极简的实践、DIY 的乐趣、看过的好书、走过的路，以及那些值得被分享的想法。

每一颗种子都需要阳光和对话才能发芽。欢迎你留下你的声音，我们一起讨论，一起进步。

> 星辰之下，尘埃之上。分享，交流，共同成长。
"#,
    )?;

    // English sample post
    std::fs::write(
        project_dir.join("content/en/posts/hello-world.md"),
        r#"---
title: "Under the Stardust"
date: 2024-01-01
tags: ["essay"]
draft: false
---

Welcome to Stellaris Pulvis.

This is a speck of stardust fallen from the sky — a patch of soil where we grow together.

Here we talk about minimalist practices, the joy of DIY, good books we've read, paths we've walked, and the ideas worth sharing.

Every seed needs sunlight and conversation to sprout. So your voice is welcome here — let's discuss, share, and grow together.

> Beneath the stars, above the dust. Share, exchange, grow together.
"#,
    )?;

    std::fs::write(
        project_dir.join("templates/base.html"),
        r#"<!DOCTYPE html>
<html lang="{{ lang }}">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{% block title %}{{ lang_config.title }}{% endblock %}</title>
    <meta name="description" content="{% block description %}{{ lang_config.description }}{% endblock %}">
    <link rel="alternate" type="application/rss+xml" title="RSS" href="/{{ lang }}/feed.xml">
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
    <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700;800&family=Noto+Sans+SC:wght@400;500;700&family=JetBrains+Mono:wght@400;500&display=swap" rel="stylesheet">
    <link rel="icon" href="/avatar.svg" type="image/svg+xml">
    <meta property="og:image" content="/avatar.svg">
    <link rel="stylesheet" href="/css/style.css">
</head>
<body>
    <div class="reading-progress"></div>
    <header>
        <nav>
            <a href="/{{ lang }}/" class="site-title">{{ lang_config.title }}</a>
            <div class="nav-links">
                <a href="/{{ lang }}/">{{ lang_config.nav_home }}</a>
                <a href="/{{ lang }}/tags">{{ lang_config.nav_tags }}</a>
                <a href="/{{ lang }}/about/">{{ lang_config.nav_about }}</a>
            </div>
            <div class="search-box">
                <input id="search-input" type="text" placeholder="{{ lang_config.nav_search }}..." autocomplete="off">
                <div id="search-results" class="search-results"></div>
            </div>
        </nav>
    </header>
    <main>
    {% block content %}{% endblock %}
    </main>
    <footer>
        <p>&copy; {{ now() | date(format="%Y") }} {{ config.author.name }}</p>
    </footer>
    <script src="/js/livereload.js"></script>
    <script src="/js/search.js"></script>
</body>
</html>"#,
    )?;

    std::fs::write(
        project_dir.join("templates/index.html"),
        r#"{% extends "base.html" %}
{% block title %}{{ lang_config.title }}{% endblock %}
{% block content %}
<section class="hero">
    <h1>{{ lang_config.title }}</h1>
    <p class="description">{{ lang_config.description }}</p>
</section>
<section class="posts">
{% for post in posts %}
    <article class="post-card">
        <h2><a href="/{{ lang }}/posts/{{ post.slug }}/">{{ post.title }}</a></h2>
        <div class="meta">
            <time datetime="{{ post.date_iso }}">{{ post.date }}</time>
            {% if post.tags %}
            <div class="tags">
                {% for tag in post.tags %}
                <a href="/{{ lang }}/tags/{{ tag }}/" class="tag">{{ tag }}</a>
                {% endfor %}
            </div>
            {% endif %}
        </div>
        <div class="summary">{{ post.summary_html | safe }}</div>
        <a href="/{{ lang }}/posts/{{ post.slug }}/" class="read-more">{{ lang_config.read_more }} →</a>
    </article>
{% endfor %}
</section>
{% if pagination %}
<nav class="pagination">
    {% if pagination.prev %}
    <a href="{{ pagination.prev }}" class="prev">← {{ lang_config.pagination_prev }}</a>
    {% endif %}
    <span class="pages">{{ pagination.current }} / {{ pagination.total }}</span>
    {% if pagination.next %}
    <a href="{{ pagination.next }}" class="next">{{ lang_config.pagination_next }} →</a>
    {% endif %}
</nav>
{% endif %}
{% endblock %}"#,
    )?;

    std::fs::write(
        project_dir.join("templates/post.html"),
        r#"{% extends "base.html" %}
{% block title %}{{ post.title }} - {{ lang_config.title }}{% endblock %}
{% block description %}{{ post.summary }}{% endblock %}
{% block content %}
<article class="post-full">
    <header>
        <h1>{{ post.title }}</h1>
        <div class="meta">
            <time datetime="{{ post.date_iso }}">{{ post.date }}</time>
            {% if post.tags %}
            <div class="tags">
                {% for tag in post.tags %}
                <a href="/{{ lang }}/tags/{{ tag }}/" class="tag">{{ tag }}</a>
                {% endfor %}
            </div>
            {% endif %}
        </div>
    </header>
    <div class="content">
        {{ post.content_html | safe }}
    </div>
</article>
{% if config.comment %}
<section class="post-comments">
    <h2>💬 {{ lang_config.comment_title }}</h2>
    <div class="giscus"></div>
    <script src="https://giscus.app/client.js"
        data-repo="{{ config.comment.repo }}"
        data-repo-id="{{ config.comment.repo_id }}"
        data-category="{{ config.comment.category }}"
        data-category-id="{{ config.comment.category_id }}"
        data-mapping="{{ config.comment.mapping }}"
        data-strict="0"
        data-reactions-enabled="1"
        data-emit-metadata="0"
        data-input-position="bottom"
        data-theme="dark"
        data-lang="{{ config.comment.lang }}"
        crossorigin="anonymous"
        async>
    </script>
</section>
{% endif %}
{% endblock %}"#,
    )?;

    std::fs::write(
        project_dir.join("templates/tags.html"),
        r#"{% extends "base.html" %}
{% block title %}{{ lang_config.tags_title }} - {{ lang_config.title }}{% endblock %}
{% block content %}
<section class="tags-page">
    <h1>🏷️ {{ lang_config.tags_title }}</h1>
    <div class="tags-cloud">
    {% for tag in tags %}
        <a href="/{{ lang }}/tags/{{ tag.name }}/" class="tag-cloud-item">
            {{ tag.name }}
            <span class="count">{{ tag.count }}</span>
        </a>
    {% endfor %}
    </div>
</section>
{% endblock %}"#,
    )?;

    std::fs::write(
        project_dir.join("templates/tag.html"),
        r#"{% extends "base.html" %}
{% block title %}{{ lang_config.tags_title }}: {{ tag.name }} - {{ lang_config.title }}{% endblock %}
{% block content %}
<section class="tag-page">
    <h1>🏷️ <span>{{ tag.name }}</span></h1>
    <section class="posts">
    {% for post in posts %}
        <article class="post-card">
            <h2><a href="/{{ lang }}/posts/{{ post.slug }}/">{{ post.title }}</a></h2>
            <div class="meta">
                <time datetime="{{ post.date_iso }}">{{ post.date }}</time>
            </div>
            <div class="summary">{{ post.summary_html | safe }}</div>
        </article>
    {% endfor %}
    </section>
</section>
{% endblock %}"#,
    )?;

    std::fs::write(
        project_dir.join("templates/about.html"),
        r#"{% extends "base.html" %}
{% block title %}{{ lang_config.nav_about }} — {{ lang_config.title }}{% endblock %}
{% block content %}
<article class="post-full about-page">
    <header>
        <h1>{{ lang_config.nav_about }} {{ lang_config.title }}</h1>
    </header>
    <div class="content">
        {% if lang == "zh" %}
        <h2>✨ 名字的由来</h2>
        <p><strong>Stellaris Pulvis</strong>，拉丁语意为「星尘」。</p>
        <p>这个名字来源于《空洞骑士》中信仰者神龛里的一句话——<strong>「星星与尘土」</strong>。我们都是星尘。</p>
        <p>这个博客便是这样一粒星尘——微小，却承载着值得被分享的光芒。</p>
        <h2>🧑‍🚀 关于我</h2>
        <p>极简主义者和深度 DIY 爱好者。喜欢把系统打磨成最符合心意的模样——<strong>极简、流畅、迅速</strong>。</p>
        <p>建这个博客，是为了分享想法、倾听声音，和大家一起成长。</p>
        <h2>📬 联系方式</h2>
        <p><a href="mailto:starsdusts@foxmail.com">starsdusts@foxmail.com</a></p>
        {% else %}
        <h2>✨ The Story Behind the Name</h2>
        <p><strong>Stellaris Pulvis</strong> — Latin for <strong>«Stardust»</strong>.</p>
        <p>Inspired by a line from <em>Hollow Knight</em>: <strong>«Stars and Dust»</strong>. We are all made of stardust.</p>
        <p>This blog is a speck of that dust — small, yet carrying light worth sharing.</p>
        <h2>🧑‍🚀 About Me</h2>
        <p>A minimalist and DIY enthusiast. Shaping my system until it feels just right — <strong>minimal, smooth, fast</strong>.</p>
        <p>This blog exists to share ideas, listen to voices, and grow together.</p>
        <h2>📬 Contact</h2>
        <p><a href="mailto:starsdusts@foxmail.com">starsdusts@foxmail.com</a></p>
        {% endif %}
    </div>
</article>
{% endblock %}"#,
    )?;

    std::fs::write(
        project_dir.join("static/css/style.css"),
        r#"/* ============================================
   Stellaris — 炫酷博客主题
   ============================================ */

/* ---------- Variables ---------- */
:root {
    --bg: #0f0f1a;
    --bg-card: rgba(255, 255, 255, 0.04);
    --bg-card-hover: rgba(255, 255, 255, 0.08);
    --text: #e2e8f0;
    --text-light: #94a3b8;
    --text-muted: #64748b;
    --border: rgba(255, 255, 255, 0.08);
    --accent-1: #6366f1;
    --accent-2: #8b5cf6;
    --accent-3: #ec4899;
    --glow: rgba(99, 102, 241, 0.3);
    --code-bg: #1a1b2e;
    --radius: 16px;
    --radius-sm: 8px;
    --transition: 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

* { margin: 0; padding: 0; box-sizing: border-box; }
html { scroll-behavior: smooth; }

body {
    font-family: "Inter", "Noto Sans SC", -apple-system, sans-serif;
    line-height: 1.8;
    color: var(--text);
    background: var(--bg);
    min-height: 100vh;
    overflow-x: hidden;
}

body::before {
    content: "";
    position: fixed;
    inset: 0;
    z-index: -1;
    background:
        radial-gradient(ellipse at 20% 50%, rgba(99, 102, 241, 0.12) 0%, transparent 50%),
        radial-gradient(ellipse at 80% 20%, rgba(139, 92, 246, 0.10) 0%, transparent 50%),
        radial-gradient(ellipse at 50% 80%, rgba(236, 72, 153, 0.08) 0%, transparent 50%);
    animation: bgShift 12s ease-in-out infinite alternate;
}

@keyframes bgShift {
    0%   { transform: scale(1) rotate(0deg); }
    50%  { transform: scale(1.05) rotate(1deg); }
    100% { transform: scale(1) rotate(-1deg); }
}

body::after {
    content: "";
    position: fixed;
    inset: 0;
    z-index: -1;
    background-image:
        linear-gradient(rgba(255,255,255,0.02) 1px, transparent 1px),
        linear-gradient(90deg, rgba(255,255,255,0.02) 1px, transparent 1px);
    background-size: 60px 60px;
}

::-webkit-scrollbar { width: 8px; }
::-webkit-scrollbar-track { background: transparent; }
::-webkit-scrollbar-thumb {
    background: linear-gradient(to bottom, var(--accent-1), var(--accent-2));
    border-radius: 4px;
}

::selection { background: rgba(99, 102, 241, 0.3); color: #fff; }

header {
    position: sticky;
    top: 16px;
    z-index: 100;
    margin: 16px 24px 40px;
    border-radius: var(--radius);
    background: rgba(15, 15, 26, 0.7);
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    border: 1px solid var(--border);
    animation: slideDown 0.6s ease-out;
}

@keyframes slideDown {
    from { opacity: 0; transform: translateY(-20px); }
    to   { opacity: 1; transform: translateY(0); }
}

@keyframes fadeIn {
    from { opacity: 0; transform: translateY(12px); }
    to   { opacity: 1; transform: translateY(0); }
}

nav {
    max-width: 820px;
    margin: 0 auto;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 24px;
}

.site-title {
    font-size: 1.25rem;
    font-weight: 800;
    text-decoration: none;
    background: linear-gradient(135deg, var(--accent-1), var(--accent-2), var(--accent-3));
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
    letter-spacing: -0.02em;
    transition: var(--transition);
}

.site-title:hover { filter: brightness(1.2) saturate(1.3); }

.nav-links { display: flex; gap: 2px; }

.nav-links a {
    color: var(--text-light);
    text-decoration: none;
    font-size: 0.9rem;
    font-weight: 500;
    padding: 6px 14px;
    border-radius: 8px;
    transition: var(--transition);
}

.nav-links a:hover {
    color: var(--text);
    background: rgba(255, 255, 255, 0.06);
}

.search-box { position: relative; flex-shrink: 0; }
#search-input {
    width: 180px; padding: 6px 14px 6px 32px; border-radius: 100px;
    border: 1px solid var(--border); background: rgba(255,255,255,0.04);
    color: var(--text); font-size: 0.82rem; font-family: inherit;
    outline: none; transition: var(--transition);
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='14' height='14' viewBox='0 0 24 24' fill='none' stroke='%2364748b' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Ccircle cx='11' cy='11' r='8'/%3E%3Cline x1='21' y1='21' x2='16.65' y2='16.65'/%3E%3C/svg%3E");
    background-repeat: no-repeat; background-position: 10px center;
}
#search-input:focus {
    width: 240px; border-color: rgba(99,102,241,0.4);
    background-color: rgba(255,255,255,0.06);
    box-shadow: 0 0 16px rgba(99,102,241,0.1);
}
#search-input::placeholder { color: var(--text-muted); }
.search-results {
    display: none; position: absolute; top: calc(100% + 8px); right: 0;
    width: 360px; max-height: 400px; overflow-y: auto;
    background: rgba(20,20,36,0.95); backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    border: 1px solid var(--border); border-radius: var(--radius-sm);
    box-shadow: 0 12px 48px rgba(0,0,0,0.4); z-index: 300;
}
.search-results.active { display: block; animation: fadeIn 0.15s ease-out; }
.search-result-item {
    display: block; padding: 12px 16px; text-decoration: none;
    border-bottom: 1px solid var(--border); transition: var(--transition);
}
.search-result-item:last-child { border-bottom: none; }
.search-result-item:hover { background: rgba(255,255,255,0.04); }
.search-result-title {
    display: block; color: var(--text); font-size: 0.9rem;
    font-weight: 600; margin-bottom: 2px;
}
.search-result-title mark {
    background: rgba(99,102,241,0.25); color: var(--accent-1);
    padding: 0 2px; border-radius: 2px;
}
.search-result-meta { display: block; color: var(--text-muted); font-size: 0.78rem; }
.search-result-empty { padding: 20px 16px; color: var(--text-muted); text-align: center; font-size: 0.85rem; }
.search-results::-webkit-scrollbar { width: 4px; }
.search-results::-webkit-scrollbar-thumb { background: var(--accent-1); border-radius: 2px; }

main {
    min-height: 60vh;
    padding: 0 24px 40px;
    max-width: 820px;
    margin: 0 auto;
}

.hero {
    text-align: center;
    padding: 60px 0 48px;
    animation: fadeIn 0.8s ease-out 0.2s both;
}

.hero h1 {
    font-size: clamp(2.2rem, 5vw, 3.2rem);
    font-weight: 800;
    letter-spacing: -0.03em;
    line-height: 1.2;
    background: linear-gradient(135deg, var(--accent-1), var(--accent-2), var(--accent-3), var(--accent-1));
    background-size: 200% 200%;
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
    animation: gradientShift 4s ease-in-out infinite;
    margin-bottom: 12px;
}

@keyframes gradientShift {
    0%, 100% { background-position: 0% 50%; }
    50%      { background-position: 100% 50%; }
}

.hero .description {
    color: var(--text-light);
    font-size: 1.15rem;
    max-width: 500px;
    margin: 0 auto;
}

.posts { display: flex; flex-direction: column; gap: 20px; }

.post-card {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 28px 32px;
    transition: var(--transition);
    animation: fadeIn 0.6s ease-out both;
    position: relative;
    overflow: hidden;
}

.post-card::before {
    content: "";
    position: absolute;
    top: 0; left: 0; right: 0;
    height: 2px;
    background: linear-gradient(90deg, var(--accent-1), var(--accent-2), var(--accent-3));
    opacity: 0;
    transition: var(--transition);
}

.post-card:hover {
    background: var(--bg-card-hover);
    border-color: rgba(255, 255, 255, 0.12);
    transform: translateY(-4px);
    box-shadow: 0 12px 48px rgba(0, 0, 0, 0.3), 0 0 24px var(--glow);
}

.post-card:hover::before { opacity: 1; }
.post-card:nth-child(1) { animation-delay: 0.1s; }
.post-card:nth-child(2) { animation-delay: 0.15s; }
.post-card:nth-child(3) { animation-delay: 0.2s; }
.post-card:nth-child(4) { animation-delay: 0.25s; }
.post-card:nth-child(5) { animation-delay: 0.3s; }

.post-card h2 {
    font-size: 1.35rem;
    font-weight: 700;
    margin-bottom: 10px;
    line-height: 1.4;
}

.post-card h2 a {
    color: var(--text);
    text-decoration: none;
    transition: var(--transition);
}

.post-card h2 a:hover {
    background: linear-gradient(135deg, var(--accent-1), var(--accent-2));
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
}

.meta {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 12px;
    margin-bottom: 14px;
    color: var(--text-muted);
    font-size: 0.85rem;
}

.meta time::before { content: "📅"; font-size: 0.8rem; margin-right: 4px; }

.tags { display: flex; flex-wrap: wrap; gap: 6px; }

.tag {
    display: inline-block;
    padding: 2px 12px;
    border-radius: 100px;
    font-size: 0.78rem;
    font-weight: 500;
    color: var(--accent-1);
    background: rgba(99, 102, 241, 0.1);
    border: 1px solid rgba(99, 102, 241, 0.2);
    text-decoration: none;
    transition: var(--transition);
}

.tag:hover {
    background: rgba(99, 102, 241, 0.2);
    border-color: rgba(99, 102, 241, 0.4);
    box-shadow: 0 0 16px rgba(99, 102, 241, 0.2);
    transform: scale(1.05);
}

.summary { color: var(--text-light); font-size: 0.95rem; line-height: 1.7; }
.summary p { margin-bottom: 0; }

.read-more {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    margin-top: 14px;
    color: var(--accent-1);
    text-decoration: none;
    font-size: 0.9rem;
    font-weight: 600;
    transition: var(--transition);
}

.read-more:hover { color: var(--accent-2); gap: 8px; }

.pagination {
    display: flex;
    justify-content: center;
    align-items: center;
    gap: 12px;
    padding: 32px 0 16px;
    animation: fadeIn 0.6s ease-out;
}

.pagination a {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 8px 20px;
    border-radius: 100px;
    color: var(--text-light);
    text-decoration: none;
    font-size: 0.9rem;
    font-weight: 500;
    border: 1px solid var(--border);
    transition: var(--transition);
}

.pagination a:hover {
    color: var(--text);
    background: var(--bg-card-hover);
    border-color: rgba(255, 255, 255, 0.15);
    transform: translateY(-1px);
}

.pages { color: var(--text-muted); font-size: 0.85rem; padding: 0 4px; }

.post-full { animation: fadeIn 0.6s ease-out; }

.post-full header {
    position: static;
    backdrop-filter: none;
    background: none;
    border: none;
    margin: 0 0 32px;
    padding: 0;
}

.post-full h1 {
    font-size: clamp(1.8rem, 4vw, 2.4rem);
    font-weight: 800;
    letter-spacing: -0.02em;
    line-height: 1.25;
    margin-bottom: 16px;
    background: linear-gradient(135deg, var(--text), var(--accent-1));
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
}

.post-full .meta { margin-bottom: 0; }

.post-comments { margin-top: 48px; padding-top: 32px; border-top: 1px solid var(--border); animation: fadeIn 0.6s ease-out 0.3s both; }
.post-comments h2 { font-size: 1.2rem; font-weight: 700; margin-bottom: 20px; color: var(--text-light); }

.content { line-height: 1.9; font-size: 1.05rem; }

.content h2 {
    font-size: 1.5rem;
    font-weight: 700;
    margin: 2.5rem 0 1rem;
    padding-bottom: 8px;
    border-bottom: 1px solid var(--border);
    letter-spacing: -0.01em;
}

.content h3 { font-size: 1.2rem; font-weight: 700; margin: 2rem 0 0.75rem; color: var(--accent-1); }
.content p { margin-bottom: 1.2rem; }
.content ul, .content ol { margin-bottom: 1.2rem; padding-left: 1.5rem; }
.content li { margin-bottom: 0.35rem; }

.content a {
    color: var(--accent-2);
    text-decoration: none;
    border-bottom: 1px solid rgba(139, 92, 246, 0.25);
    transition: var(--transition);
}

.content a:hover { border-bottom-color: var(--accent-2); }

.content img {
    max-width: 100%;
    height: auto;
    border-radius: var(--radius-sm);
    margin: 1.5rem 0;
    box-shadow: 0 4px 24px rgba(0, 0, 0, 0.3);
    transition: var(--transition);
}

.content img:hover { transform: scale(1.01); }

.content blockquote {
    border-left: 3px solid var(--accent-1);
    padding: 16px 20px;
    margin: 1.5rem 0;
    color: var(--text-light);
    background: rgba(99, 102, 241, 0.06);
    border-radius: 0 var(--radius-sm) var(--radius-sm) 0;
    font-style: italic;
}

.content blockquote p { margin-bottom: 0; }

.content pre {
    background: var(--code-bg);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: var(--radius-sm);
    padding: 20px 24px;
    overflow-x: auto;
    margin: 1.5rem 0;
    font-size: 0.85rem;
    line-height: 1.6;
    box-shadow: 0 4px 24px rgba(0, 0, 0, 0.2);
}

.content code {
    font-family: "JetBrains Mono", "Fira Code", "Cascadia Code", "Consolas", monospace;
    font-size: 0.9em;
}

.content p code, .content li code {
    background: rgba(99, 102, 241, 0.1);
    color: var(--accent-1);
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 0.85em;
    border: 1px solid rgba(99, 102, 241, 0.15);
}

.content hr {
    border: none;
    height: 1px;
    background: linear-gradient(90deg, transparent, var(--border), transparent);
    margin: 2.5rem 0;
}

.content table {
    width: 100%;
    border-collapse: collapse;
    margin: 1.5rem 0;
    font-size: 0.95rem;
}

.content th, .content td {
    padding: 10px 16px;
    text-align: left;
    border-bottom: 1px solid var(--border);
}

.content th {
    background: rgba(255, 255, 255, 0.04);
    font-weight: 600;
    color: var(--text-light);
}

.content tr:hover { background: rgba(255, 255, 255, 0.02); }

/* Tags page */
.tags-page h1 {
    font-size: clamp(1.6rem, 4vw, 2rem);
    font-weight: 800;
    margin-bottom: 28px;
    background: linear-gradient(135deg, var(--accent-1), var(--accent-3));
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
}

.tags-cloud {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    animation: fadeIn 0.6s ease-out;
}

.tag-cloud-item {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 8px 18px;
    border-radius: 100px;
    background: var(--bg-card);
    border: 1px solid var(--border);
    color: var(--text-light);
    text-decoration: none;
    font-size: 0.9rem;
    font-weight: 500;
    transition: var(--transition);
}

.tag-cloud-item:hover {
    background: var(--bg-card-hover);
    border-color: rgba(99, 102, 241, 0.35);
    color: var(--accent-1);
    transform: translateY(-2px);
    box-shadow: 0 4px 16px rgba(99, 102, 241, 0.15);
}

.tag-cloud-item .count {
    background: rgba(255, 255, 255, 0.06);
    padding: 1px 8px;
    border-radius: 100px;
    font-size: 0.75rem;
    color: var(--text-muted);
}

.tag-page h1 {
    font-size: clamp(1.4rem, 3.5vw, 1.8rem);
    font-weight: 700;
    margin-bottom: 24px;
}

.tag-page h1 span {
    background: linear-gradient(135deg, var(--accent-1), var(--accent-2));
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
}

footer {
    text-align: center;
    padding: 32px 24px 24px;
    color: var(--text-muted);
    font-size: 0.85rem;
    border-top: 1px solid var(--border);
    margin-top: 48px;
    max-width: 820px;
    margin-left: auto;
    margin-right: auto;
}

footer p { opacity: 0.7; transition: var(--transition); }
footer:hover p { opacity: 1; }

.reading-progress {
    position: fixed;
    top: 0; left: 0;
    width: 100%;
    height: 3px;
    z-index: 200;
    background: transparent;
}

.reading-progress::after {
    content: "";
    position: absolute;
    top: 0; left: 0;
    height: 100%;
    width: 0;
    background: linear-gradient(90deg, var(--accent-1), var(--accent-2), var(--accent-3));
    animation: progressScroll linear;
    animation-timeline: scroll();
}

@media (max-width: 768px) {
    header { margin: 12px 12px 24px; top: 12px; }
    nav { padding: 12px 16px; flex-wrap: wrap; gap: 8px; }
    .post-card { padding: 20px; }
    .hero { padding: 40px 0 32px; }
    main { padding: 0 12px 24px; }
    .search-box { order: 3; width: 100%; }
    #search-input { width: 100%; }
    #search-input:focus { width: 100%; }
    .search-results { width: calc(100vw - 48px); right: 0; }
}

@media (max-width: 480px) {
    .site-title { font-size: 1.05rem; }
    .nav-links a { padding: 6px 10px; font-size: 0.82rem; }
    .post-card h2 { font-size: 1.15rem; }
}"#,
    )?;

    std::fs::write(
        project_dir.join("static/js/livereload.js"),
        r#"if (window.EventSource) {
    const es = new EventSource('/__reload__');
    es.onmessage = function(e) {
        if (e.data === 'reload') {
            location.reload();
        }
    };
}"#,
    )?;

    std::fs::write(
        project_dir.join("static/js/search.js"),
        r#"(function () {
  var index = [];
  var input = document.getElementById("search-input");
  var results = document.getElementById("search-results");
  if (!input || !results) return;
  fetch("/search.json").then(function (r) { return r.json(); }).then(function (d) { index = d; }).catch(function () {});
  var timer;
  input.addEventListener("input", function () { clearTimeout(timer); timer = setTimeout(search, 200); });
  input.addEventListener("focus", function () { if (results.querySelector(".search-result-item")) results.classList.add("active"); });
  document.addEventListener("click", function (e) { if (!e.target.closest(".search-box")) results.classList.remove("active"); });
  document.addEventListener("keydown", function (e) { if (e.key === "Escape") { results.classList.remove("active"); input.blur(); } });
  function search() {
    var q = input.value.trim().toLowerCase();
    if (q.length < 1) { results.classList.remove("active"); return; }
    var hits = [];
    for (var i = 0; i < index.length; i++) {
      var p = index[i];
      if (p.title.toLowerCase().indexOf(q) !== -1 || p.summary.toLowerCase().indexOf(q) !== -1 || p.content.toLowerCase().indexOf(q) !== -1 || p.tags.some(function (t) { return t.toLowerCase().indexOf(q) !== -1; })) {
        hits.push(p);
        if (hits.length >= 10) break;
      }
    }
    if (hits.length === 0) { results.innerHTML = '<div class="search-result-empty">无结果</div>'; results.classList.add("active"); return; }
    var html = "";
    for (var i = 0; i < hits.length; i++) { var h = hits[i]; html += '<a href="/posts/' + h.slug + '/" class="search-result-item"><span class="search-result-title">' + highlight(h.title, q) + "</span><span class=\"search-result-meta\">" + h.date + "</span></a>"; }
    results.innerHTML = html;
    results.classList.add("active");
  }
  function highlight(text, query) { var idx = text.toLowerCase().indexOf(query); if (idx === -1) return escapeHtml(text); return escapeHtml(text.slice(0, idx)) + "<mark>" + escapeHtml(text.slice(idx, idx + query.length)) + "</mark>" + escapeHtml(text.slice(idx + query.length)); }
  function escapeHtml(s) { var d = document.createElement("div"); d.appendChild(document.createTextNode(s)); return d.innerHTML; }
})();"#,
    )?;

    println!("✅ 博客项目已初始化: {}", project_dir.display());
    println!("运行 `cargo run -- serve` 启动开发服务器");
    println!("运行 `cargo run -- gui`  启动桌面管理程序");

    Ok(())
}

fn cmd_gui(project_dir: &PathBuf) -> Result<()> {
    gui::run(project_dir);
    Ok(())
}

fn cmd_build(project_dir: &PathBuf) -> Result<()> {
    let config_path = project_dir.join("config.toml");
    if !config_path.exists() {
        anyhow::bail!("config.toml 不存在，请先运行 `stellaris init`");
    }

    let config = config::Config::load(&config_path)?;
    let mut builder = SiteBuilder::new(project_dir, config)?;
    let summaries = builder.build_all()?;
    for (lang, s) in &summaries {
        println!("✅ [{}] 构建完成: {} 篇文章, 耗时 {:?}", lang, s.posts, s.duration);
    }

    Ok(())
}

async fn cmd_serve(project_dir: &PathBuf, port: u16) -> Result<()> {
    let config_path = project_dir.join("config.toml");
    if !config_path.exists() {
        anyhow::bail!("config.toml 不存在，请先运行 `stellaris init`");
    }

    let config = config::Config::load(&config_path)?;
    let mut builder = SiteBuilder::new(project_dir, config)?;
    let summaries = builder.build_all()?;
    for (lang, s) in &summaries {
        println!("✅ [{}] 构建完成: {} 篇文章, 耗时 {:?}", lang, s.posts, s.duration);
    }

    let (tx, _) = broadcast::channel::<String>(16);
    let tx_clone = tx.clone();

    let watch_dir = project_dir.clone();
    let project_dir_clone = project_dir.clone();
    let tx_watch = tx.clone();

    std::thread::spawn(move || {
        start_watcher(watch_dir, move || {
            tracing::info!("检测到文件变更，正在重新构建...");
            let config_path = project_dir_clone.join("config.toml");
            if let Ok(config) = config::Config::load(&config_path) {
                let mut builder = SiteBuilder::new(&project_dir_clone, config).unwrap();
                match builder.build_all() {
                    Ok(summaries) => {
                        for (lang, s) in &summaries {
                            tracing::info!("[{}] 重建完成: {} 篇文章, 耗时 {:?}", lang, s.posts, s.duration);
                        }
                        let _ = tx_watch.send("reload".to_string());
                    }
                    Err(e) => {
                        tracing::error!("构建失败: {}", e);
                    }
                }
            }
        });
    });

    let output_dir = project_dir.join("output");
    serve::start_server(&output_dir, port, tx_clone).await
}

fn start_watcher<F>(project_dir: PathBuf, callback: F)
where
    F: Fn() + Send + 'static,
{
    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();

    let mut watcher = match RecommendedWatcher::new(tx, Config::default()) {
        Ok(w) => w,
        Err(e) => {
            tracing::error!("无法启动文件监听: {}", e);
            return;
        }
    };

    let dirs_to_watch = [
        project_dir.join("content"),
        project_dir.join("templates"),
        project_dir.join("static"),
        project_dir.join("config.toml"),
    ];

    for dir in &dirs_to_watch {
        if dir.exists() {
            let _ = watcher.watch(dir, RecursiveMode::Recursive);
        }
    }

    tracing::info!("文件监听已启动...");

    let mut last_event = Instant::now();
    for event in rx {
        match event {
            Ok(_) => {
                let now = Instant::now();
                if now.duration_since(last_event) > Duration::from_millis(300) {
                    callback();
                }
                last_event = now;
            }
            Err(e) => {
                tracing::debug!("文件监听事件错误: {}", e);
            }
        }
    }
}
