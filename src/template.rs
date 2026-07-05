use anyhow::{Context as AnyhowContext, Result};
use std::path::Path;
use tera::Tera;

pub struct TemplateEngine {
    tera: Tera,
}

impl TemplateEngine {
    pub fn new(template_dir: &Path) -> Result<Self> {
        let pattern = template_dir
            .join("**/*.*")
            .to_string_lossy()
            .to_string();

        let mut tera = Tera::new(&pattern)
            .with_context(|| format!("无法加载模板: {}", pattern))?;

        tera.autoescape_on(Vec::new());

        Ok(Self { tera })
    }

    pub fn render(&self, template: &str, context: &tera::Context) -> Result<String> {
        self.tera
            .render(template, context)
            .with_context(|| format!("渲染模板 '{}' 失败", template))
    }
}
