mod builder;
mod config;
mod content;
mod feed;
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
    }
}

fn cmd_init(project_dir: &PathBuf) -> Result<()> {
    let dirs = [
        "content/posts",
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
        project_dir.join("content/posts/hello-world.md"),
        r#"---
title: "星尘之下"
date: 2024-01-01
tags: ["随笔", "essay"]
draft: false
---

欢迎来到 Stellaris Pulvis。

这里是星辰落下的一粒尘埃，也是我们共同生长的一片土壤。

我们聊的是极简的实践、DIY 的乐趣、看过的好书、走过的路，以及那些值得被分享的想法。

每一颗种子都需要阳光和对话才能发芽。欢迎你留下你的声音，我们一起讨论，一起进步。

> 星辰之下，尘埃之上。分享，交流，共同成长。
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
    <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/katex@0.16.11/dist/katex.min.css">
    <script src="https://cdn.jsdelivr.net/npm/katex@0.16.11/dist/katex.min.js"></script>
    <script>
    (function(){var t=localStorage.getItem('stellaris-theme');if(t){document.documentElement.setAttribute('data-theme',t);}else if(window.matchMedia('(prefers-color-scheme:dark)').matches){document.documentElement.setAttribute('data-theme','dark');}})();
    </script>
    <script>
    window.StellarisData = {
      lang: '{{ lang }}',
      searchEmpty: '{{ lang_config.search_empty }}',
    };
    </script>
</head>
<body>
    <canvas id="stardust-canvas"></canvas>
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
            <button id="theme-toggle" class="theme-toggle" aria-label="Toggle theme"></button>
        </nav>
    </header>
    <main>
    {% block content %}{% endblock %}
    </main>
    {% if is_dev %}
    <script src="/js/livereload.js"></script>
    {% endif %}
    <script src="/js/search.js"></script>
    <script src="/js/theme.js"></script>
    <script src="/js/character.js"></script>
    <script>
    document.addEventListener('DOMContentLoaded',function(){
      if(typeof katex==='undefined')return;
      document.querySelectorAll('.math-display,.math').forEach(function(el){
        var display=el.classList.contains('math-display');
        try{katex.render(el.textContent.trim(),el,{displayMode:display,throwOnError:false});
        }catch(e){el.textContent='Error: '+e.message;}
      });
    });
    </script>
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
        data-theme="transparent_dark"
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
    <div class="about-layout">
        <div class="about-character">
            <div class="character" id="site-character" role="img" aria-label="Site mascot">
                <div class="character-face">
                    <div class="character-eyes">
                        <div class="character-eye left-eye">
                            <div class="eyeball">
                                <div class="pupil"></div>
                            </div>
                        </div>
                        <div class="character-eye right-eye">
                            <div class="eyeball">
                                <div class="pupil"></div>
                            </div>
                        </div>
                    </div>
                    <div class="character-mouth">
                        <div class="mouth-shape"></div>
                    </div>
                    <div class="character-blush left-blush"></div>
                    <div class="character-blush right-blush"></div>
                </div>
                <div class="character-body">
                    <div class="body-star">✦</div>
                </div>
                <div class="character-zzz">Z</div>
            </div>
        </div>

        <div class="about-content">
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
        </div>
    </div>
</article>
{% endblock %}"#,
    )?;

    let css = include_str!("../static/css/style.css");
    std::fs::write(project_dir.join("static/css/style.css"), css)?;

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

    let js_theme = include_str!("../static/js/theme.js");
    std::fs::write(project_dir.join("static/js/theme.js"), js_theme)?;

    let js_char = include_str!("../static/js/character.js");
    std::fs::write(project_dir.join("static/js/character.js"), js_char)?;

    println!("✅ 博客项目已初始化: {}", project_dir.display());
    println!("运行 `cargo run -- serve` 启动开发服务器");

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
    builder.set_dev_mode(true);
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
