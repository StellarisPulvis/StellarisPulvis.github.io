use std::collections::HashMap;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub site: SiteConfig,
    pub author: AuthorConfig,
    #[serde(default)]
    pub build: BuildConfig,
    #[serde(default)]
    pub comment: Option<CommentConfig>,
    pub languages: HashMap<String, LanguageConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SiteConfig {
    #[serde(default = "default_languages")]
    pub languages: Vec<String>,
    #[serde(default = "default_lang")]
    pub default: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LanguageConfig {
    pub title: String,
    pub description: String,
    pub nav_home: String,
    pub nav_tags: String,
    #[serde(default = "default_nav_about")]
    pub nav_about: String,
    pub nav_search: String,
    pub read_more: String,
    pub tags_title: String,
    pub comment_title: String,
    pub pagination_prev: String,
    pub pagination_next: String,
    pub search_empty: String,
    #[serde(default = "default_announcements_title")]
    pub announcements_title: String,
    #[serde(default)]
    pub stats_posts: String,
    #[serde(default)]
    pub stats_tags: String,
    #[serde(default)]
    pub stats_words: String,
    #[serde(default)]
    pub no_posts: String,
    #[serde(default)]
    pub stats_title: String,
    pub footer_copyright: Option<String>,
    #[serde(default)]
    pub announcements: Vec<Announcement>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Announcement {
    pub title: String,
    pub content: String,
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthorConfig {
    pub name: String,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BuildConfig {
    #[serde(default = "default_posts_per_page")]
    pub posts_per_page: usize,
    #[serde(default = "default_true")]
    pub include_drafts: bool,
    #[serde(default = "default_syntax_theme")]
    pub syntax_theme: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CommentConfig {
    #[serde(default = "default_true")]
    pub enable: bool,
    pub repo: String,
    pub repo_id: String,
    pub category: String,
    pub category_id: String,
    #[serde(default = "default_mapping")]
    pub mapping: String,
    #[serde(default = "default_comment_lang")]
    pub lang: String,
}

fn default_languages() -> Vec<String> {
    vec!["zh".to_string(), "en".to_string()]
}

fn default_lang() -> String {
    "zh".to_string()
}

fn default_comment_lang() -> String {
    "zh-CN".to_string()
}

fn default_posts_per_page() -> usize {
    10
}

fn default_true() -> bool {
    true
}

fn default_nav_about() -> String {
    "关于".to_string()
}

fn default_announcements_title() -> String {
    "公告".to_string()
}

fn default_syntax_theme() -> String {
    "base16-ocean.dark".to_string()
}

fn default_mapping() -> String {
    "pathname".to_string()
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            posts_per_page: 10,
            include_drafts: true,
            syntax_theme: "base16-ocean.dark".to_string(),
        }
    }
}

impl Default for SiteConfig {
    fn default() -> Self {
        Self {
            languages: default_languages(),
            default: default_lang(),
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("无法读取配置文件: {}", path.display()))?;
        let config: Config = toml::from_str(&content)
            .with_context(|| "配置文件格式错误")?;
        Ok(config)
    }
}
