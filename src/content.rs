use anyhow::{Context, Result};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct FrontMatter {
    pub title: String,
    pub date: NaiveDate,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub draft: bool,
    #[serde(default)]
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PostRender {
    pub title: String,
    pub slug: String,
    pub date: String,
    pub date_iso: String,
    pub tags: Vec<String>,
    pub content_html: String,
    pub summary_html: String,
    pub summary: String,
    pub word_count: usize,
}

#[derive(Debug)]
pub struct Post {
    pub slug: String,
    pub front_matter: FrontMatter,
    pub body_raw: String,
    pub body_html: String,
    pub summary_html: String,
}

impl Post {
    pub fn from_file(path: &Path, slug: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("无法读取文章: {}", path.display()))?;

        let (front_matter, body_raw) = parse_front_matter(&content)?;
        let body_html = String::new();
        let summary_html = String::new();

        Ok(Self {
            slug: slug.to_string(),
            front_matter,
            body_raw,
            body_html,
            summary_html,
        })
    }

    pub fn to_render(&self, word_count: usize) -> PostRender {
        let summary = self.front_matter.summary.clone().unwrap_or_default();
        PostRender {
            title: self.front_matter.title.clone(),
            slug: self.slug.clone(),
            date: self.front_matter.date.format("%Y-%m-%d").to_string(),
            date_iso: self.front_matter.date.format("%Y-%m-%d").to_string(),
            tags: self.front_matter.tags.clone(),
            content_html: self.body_html.clone(),
            summary_html: self.summary_html.clone(),
            summary,
            word_count,
        }
    }
}

fn parse_front_matter(content: &str) -> Result<(FrontMatter, String)> {
    let content = content.trim();
    if !content.starts_with("---") {
        anyhow::bail!("文章缺少 Front Matter (---)");
    }

    let end = content[3..]
        .find("\n---")
        .map(|i| i + 3)
        .ok_or_else(|| anyhow::anyhow!("Front Matter 未正确关闭"))?;

    let yaml_str = &content[3..end].trim();
    let body = content[end + 4..].trim().to_string();

    let front_matter: FrontMatter = serde_yaml::from_str(yaml_str)
        .with_context(|| "Front Matter 格式错误")?;

    Ok((front_matter, body))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_front_matter() {
        let content = "---\ntitle: Hello\ndate: 2024-01-01\ntags: [rust]\n---\n\nHello world";
        let (fm, body) = parse_front_matter(content).unwrap();
        assert_eq!(fm.title, "Hello");
        assert_eq!(fm.date.format("%Y-%m-%d").to_string(), "2024-01-01");
        assert_eq!(fm.tags, vec!["rust"]);
        assert_eq!(body, "Hello world");
    }
}
