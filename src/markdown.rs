use pulldown_cmark::{html, CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use syntect::html::highlighted_html_for_string;
use syntect::parsing::SyntaxSet;
use syntect::highlighting::ThemeSet;

pub struct MarkdownRenderer {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
    theme_name: String,
}

impl MarkdownRenderer {
    pub fn new(theme_name: &str) -> Self {
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
            theme_name: theme_name.to_string(),
        }
    }

    pub fn render(&self, text: &str) -> String {
        let mut buf = String::new();
        let parser = Parser::new_ext(text, Options::all());

        let mut events_buf = Vec::new();
        let mut in_code_block = false;
        let mut code_lang = String::new();
        let mut code_text = String::new();

        for event in parser {
            match event {
                Event::Start(Tag::CodeBlock(kind)) => {
                    if !events_buf.is_empty() {
                        html::push_html(&mut buf, events_buf.drain(..));
                    }
                    in_code_block = true;
                    code_lang = match kind {
                        CodeBlockKind::Fenced(info) => info.to_string(),
                        CodeBlockKind::Indented => String::new(),
                    };
                    code_text.clear();
                }
                Event::End(TagEnd::CodeBlock) => {
                    in_code_block = false;

                    if let Some(syntax) = self.syntax_set.find_syntax_by_token(&code_lang) {
                        let theme = self
                            .theme_set
                            .themes
                            .get(&self.theme_name)
                            .unwrap_or_else(|| {
                                self.theme_set.themes.values().next().unwrap()
                            });
                        if let Ok(highlighted) = highlighted_html_for_string(
                            &code_text,
                            &self.syntax_set,
                            syntax,
                            theme,
                        ) {
                            buf.push_str(&highlighted);
                        }
                    } else {
                        buf.push_str("<pre><code class=\"language-");
                        buf.push_str(&html_escape(&code_lang));
                        buf.push_str("\">");
                        buf.push_str(&html_escape(&code_text));
                        buf.push_str("</code></pre>");
                    }
                }
                Event::Text(t) if in_code_block => {
                    code_text.push_str(&t);
                }
                _ => {
                    if !in_code_block {
                        events_buf.push(event);
                    }
                }
            }
        }

        if !events_buf.is_empty() {
            html::push_html(&mut buf, events_buf.drain(..));
        }

        buf
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
