use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{Context as AnyhowContext, Result};
use glob::glob;
use serde_json::json;
use tera::Context;

use crate::config::{Config, LanguageConfig};
use crate::content::{Post, PostRender};
use crate::feed::{generate_rss, strip_html};
use crate::markdown::MarkdownRenderer;
use crate::template::TemplateEngine;

pub struct SiteBuilder {
    config: Config,
    markdown: MarkdownRenderer,
    templates: TemplateEngine,
    output_dir: PathBuf,
    content_dir: PathBuf,
    static_dir: PathBuf,
}

#[derive(serde::Serialize)]
struct PaginationInfo {
    current: usize,
    total: usize,
    prev: Option<String>,
    next: Option<String>,
}

#[derive(serde::Serialize)]
struct TagInfo {
    name: String,
    count: usize,
}

#[derive(serde::Serialize)]
struct SearchIndexEntry {
    title: String,
    slug: String,
    date: String,
    tags: Vec<String>,
    summary: String,
    content: String,
}

impl SiteBuilder {
    pub fn new(project_dir: &Path, config: Config) -> Result<Self> {
        let content_dir = project_dir.join("content");
        let templates_dir = project_dir.join("templates");
        let static_dir = project_dir.join("static");
        let output_dir = project_dir.join("output");
        let theme_name = config.build.syntax_theme.clone();

        Ok(Self {
            markdown: MarkdownRenderer::new(&theme_name),
            templates: TemplateEngine::new(&templates_dir)?,
            output_dir,
            content_dir,
            static_dir,
            config,
        })
    }

    pub fn build_all(&mut self) -> Result<Vec<(String, BuildSummary)>> {
        let start = Instant::now();

        if self.output_dir.exists() {
            std::fs::remove_dir_all(&self.output_dir)?;
        }
        std::fs::create_dir_all(&self.output_dir)?;

        // Build root redirect page
        self.build_redirect()?;

        // Static assets (shared across languages)
        self.copy_static()?;

        let mut summaries = Vec::new();

        for lang in &self.config.site.languages {
            if let Some(lang_config) = self.config.languages.get(lang) {
                let summary = self.build_language(lang, lang_config)?;
                summaries.push((lang.clone(), summary));
            }
        }

        let total = start.elapsed();
        println!("🌐 多语言构建完成，总耗时 {:?}", total);

        Ok(summaries)
    }

    fn build_language(
        &self,
        lang: &str,
        lang_config: &LanguageConfig,
    ) -> Result<BuildSummary> {
        let start = Instant::now();
        let lang_output = self.output_dir.join(lang);
        std::fs::create_dir_all(&lang_output)?;

        // load all posts from unified content/posts/
        let content_dir = self.content_dir.join("posts");
        let mut posts = if content_dir.exists() {
            self.load_posts(&content_dir)?
        } else {
            Vec::new()
        };

        posts.sort_by(|a, b| b.front_matter.date.cmp(&a.front_matter.date));

        let rendered_posts: Vec<Post> = posts
            .into_iter()
            .map(|mut post| {
                post.body_html = self.markdown.render(&post.body_raw);
                post.summary_html = self.generate_summary(&post);
                post
            })
            .collect();

        let post_renders: Vec<PostRender> = rendered_posts
            .iter()
            .map(|p| p.to_render(p.body_raw.len()))
            .collect();

        let all_tags = self.collect_tags(&rendered_posts);

        self.render_post_pages(&rendered_posts, &lang_output, lang, lang_config)?;
        self.render_about_page(&lang_output, lang, lang_config)?;
        self.render_index_pages(&post_renders, &lang_output, lang, lang_config)?;
        self.render_tag_pages(&rendered_posts, &all_tags, &lang_output, lang, lang_config)?;
        self.render_tags_overview(&all_tags, &lang_output, lang, lang_config)?;
        self.generate_feed(&post_renders, lang_config, &lang_output)?;
        self.generate_search_index(&post_renders, &lang_output)?;

        let duration = start.elapsed();
        let summary = BuildSummary {
            posts: rendered_posts.len(),
            tags: all_tags.len(),
            duration,
        };

        Ok(summary)
    }

    fn build_redirect(&self) -> Result<()> {
        let languages = &self.config.site.languages;
        let default = &self.config.site.default;

        let cases: String = languages
            .iter()
            .map(|l| {
                format!(
                    "  if (lang.startsWith('{}')) {{ window.location.href = '/{}/'; }}",
                    lang_code_to_bcp47(l),
                    l
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        let html = format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width,initial-scale=1.0">
<title>Redirecting...</title>
<script>
var lang = navigator.language || navigator.userLanguage || '';
{}
  window.location.href = '/{}/';
</script>
</head>
<body>
<p>Redirecting...</p>
</body>
</html>"#,
            cases, default
        );

        std::fs::write(self.output_dir.join("index.html"), &html)?;
        Ok(())
    }

    fn load_posts(&self, content_dir: &Path) -> Result<Vec<Post>> {
        let pattern = content_dir
            .join("**/*.md")
            .to_string_lossy()
            .to_string();

        let mut posts = Vec::new();

        for entry in glob(&pattern).context("搜索文章文件失败")? {
            let path = entry?;
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();

            let post = Post::from_file(&path, &stem)?;

            if post.front_matter.draft && !self.config.build.include_drafts {
                continue;
            }

            posts.push(post);
        }

        Ok(posts)
    }

    fn render_post_pages(
        &self,
        posts: &[Post],
        output_dir: &Path,
        lang: &str,
        lang_config: &LanguageConfig,
    ) -> Result<()> {
        for post in posts {
            let mut context = Context::new();
            context.insert("lang_config", lang_config);
            context.insert("lang", lang);
            context.insert("config", &self.config);
            context.insert("post", &post.to_render(post.body_raw.len()));

            let html = self
                .templates
                .render("post.html", &context)
                .with_context(|| format!("渲染文章 '{}' 失败", post.slug))?;

            let output_path = output_dir
                .join("posts")
                .join(&post.slug)
                .join("index.html");
            std::fs::create_dir_all(output_path.parent().unwrap())?;
            std::fs::write(&output_path, &html)?;
        }
        Ok(())
    }

    fn render_index_pages(
        &self,
        posts: &[PostRender],
        output_dir: &Path,
        lang: &str,
        lang_config: &LanguageConfig,
    ) -> Result<()> {
        let per_page = self.config.build.posts_per_page;
        let total_pages = (posts.len() + per_page - 1) / per_page;
        let prefix = format!("/{}/", lang);

        // Compute stats for the index page
        let mut tag_counts: HashMap<String, usize> = HashMap::new();
        let mut total_words = 0usize;
        for post in posts {
            total_words += post.word_count;
            for tag in &post.tags {
                *tag_counts.entry(tag.clone()).or_default() += 1;
            }
        }
        let mut all_tags: Vec<TagInfo> = tag_counts
            .into_iter()
            .map(|(name, count)| TagInfo { name, count })
            .collect();
        all_tags.sort_by(|a, b| a.name.cmp(&b.name));

        let stats = json!({
            "total_posts": posts.len(),
            "total_tags": all_tags.len(),
            "total_words": total_words,
        });

        for page_num in 0..total_pages.max(1) {
            let start = page_num * per_page;
            let end = (start + per_page).min(posts.len());
            let page_posts = if posts.is_empty() {
                &[]
            } else {
                &posts[start..end]
            };

            let mut context = Context::new();
            context.insert("lang_config", lang_config);
            context.insert("lang", lang);
            context.insert("config", &self.config);
            context.insert("posts", page_posts);
            context.insert("stats", &stats);
            context.insert("all_tags", &all_tags);

            if total_pages > 1 {
                let pagination = PaginationInfo {
                    current: page_num + 1,
                    total: total_pages,
                    prev: if page_num > 0 {
                        Some(if page_num == 1 {
                            prefix.clone()
                        } else {
                            format!("{}page/{}/", prefix, page_num)
                        })
                    } else {
                        None
                    },
                    next: if page_num + 1 < total_pages {
                        Some(format!("{}page/{}/", prefix, page_num + 2))
                    } else {
                        None
                    },
                };
                context.insert("pagination", &pagination);
            }

            let html = self.templates.render("index.html", &context)?;

            let output_path = if page_num == 0 {
                output_dir.join("index.html")
            } else {
                let dir = output_dir.join("page").join((page_num + 1).to_string());
                std::fs::create_dir_all(&dir)?;
                dir.join("index.html")
            };
            std::fs::write(&output_path, &html)?;
        }
        Ok(())
    }

    fn collect_tags(&self, posts: &[Post]) -> Vec<TagInfo> {
        let mut tag_counts: HashMap<String, usize> = HashMap::new();
        for post in posts {
            for tag in &post.front_matter.tags {
                *tag_counts.entry(tag.clone()).or_default() += 1;
            }
        }
        let mut tags: Vec<TagInfo> = tag_counts
            .into_iter()
            .map(|(name, count)| TagInfo { name, count })
            .collect();
        tags.sort_by(|a, b| a.name.cmp(&b.name));
        tags
    }

    fn render_tag_pages(
        &self,
        posts: &[Post],
        all_tags: &[TagInfo],
        output_dir: &Path,
        lang: &str,
        lang_config: &LanguageConfig,
    ) -> Result<()> {
        for tag_info in all_tags {
            let tag_posts: Vec<PostRender> = posts
                .iter()
                .filter(|p| p.front_matter.tags.contains(&tag_info.name))
                .map(|p| p.to_render(p.body_raw.len()))
                .collect();

            let mut context = Context::new();
            context.insert("lang_config", lang_config);
            context.insert("lang", lang);
            context.insert("config", &self.config);
            context.insert("tag", tag_info);
            context.insert("posts", &tag_posts);

            let html = self.templates.render("tag.html", &context)?;

            let output_path = output_dir
                .join("tags")
                .join(&tag_info.name)
                .join("index.html");
            std::fs::create_dir_all(output_path.parent().unwrap())?;
            std::fs::write(&output_path, &html)?;
        }
        Ok(())
    }

    fn render_tags_overview(
        &self,
        tags: &[TagInfo],
        output_dir: &Path,
        lang: &str,
        lang_config: &LanguageConfig,
    ) -> Result<()> {
        let mut context = Context::new();
        context.insert("lang_config", lang_config);
        context.insert("lang", lang);
        context.insert("config", &self.config);
        context.insert("tags", tags);

        let html = self.templates.render("tags.html", &context)?;

        let output_path = output_dir.join("tags").join("index.html");
        std::fs::create_dir_all(output_path.parent().unwrap())?;
        std::fs::write(&output_path, &html)?;
        Ok(())
    }

    fn render_about_page(
        &self,
        output_dir: &Path,
        lang: &str,
        lang_config: &LanguageConfig,
    ) -> Result<()> {
        let mut context = Context::new();
        context.insert("lang_config", lang_config);
        context.insert("lang", lang);
        context.insert("config", &self.config);

        let html = self.templates.render("about.html", &context)?;

        let output_path = output_dir.join("about").join("index.html");
        std::fs::create_dir_all(output_path.parent().unwrap())?;
        std::fs::write(&output_path, &html)?;
        Ok(())
    }

    fn generate_feed(
        &self,
        posts: &[PostRender],
        lang_config: &LanguageConfig,
        output_dir: &Path,
    ) -> Result<()> {
        let rss = generate_rss(&self.config, posts, lang_config)?;
        std::fs::write(output_dir.join("feed.xml"), &rss)?;
        Ok(())
    }

    fn generate_search_index(&self, posts: &[PostRender], output_dir: &Path) -> Result<()> {
        let index: Vec<SearchIndexEntry> = posts
            .iter()
            .map(|p| {
                SearchIndexEntry {
                    title: p.title.clone(),
                    slug: p.slug.clone(),
                    date: p.date.clone(),
                    tags: p.tags.clone(),
                    summary: strip_html(&p.summary_html),
                    content: strip_html(&p.content_html),
                }
            })
            .collect();

        let json = serde_json::to_string(&index)?;
        std::fs::write(output_dir.join("search.json"), &json)?;
        Ok(())
    }

    fn copy_static(&self) -> Result<()> {
        if !self.static_dir.exists() {
            return Ok(());
        }
        Self::copy_dir(&self.static_dir, &self.output_dir)?;
        Ok(())
    }

    fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let ft = entry.file_type()?;
            let file_name = entry.file_name();
            let dst_path = dst.join(&file_name);
            if ft.is_dir() {
                std::fs::create_dir_all(&dst_path)?;
                Self::copy_dir(&entry.path(), &dst_path)?;
            } else {
                std::fs::copy(entry.path(), &dst_path)?;
            }
        }
        Ok(())
    }

    fn generate_summary(&self, post: &Post) -> String {
        if let Some(ref summary) = post.front_matter.summary {
            return self.markdown.render(summary);
        }

        let body = &post.body_raw;
        let mut in_paragraph = false;
        let mut paragraph = String::new();

        for line in body.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                if in_paragraph {
                    break;
                }
            } else {
                in_paragraph = true;
                paragraph.push_str(line);
                paragraph.push(' ');
                if paragraph.len() > 300 {
                    break;
                }
            }
        }

        self.markdown.render(paragraph.trim())
    }
}



#[derive(Debug)]
#[allow(dead_code)]
pub struct BuildSummary {
    pub posts: usize,
    pub tags: usize,
    pub duration: std::time::Duration,
}

fn lang_code_to_bcp47(lang: &str) -> &str {
    match lang {
        "zh" => "zh",
        "en" => "en",
        _ => lang,
    }
}
