use anyhow::Result;
use rss::{ChannelBuilder, ItemBuilder, GuidBuilder};

use crate::config::Config;
use crate::config::LanguageConfig;
use crate::content::PostRender;

pub fn generate_rss(
    config: &Config,
    posts: &[PostRender],
    lang_config: &LanguageConfig,
) -> Result<String> {
    let channel = ChannelBuilder::default()
        .title(lang_config.title.clone())
        .link(format!(
            "{}/{}",
            config.site.default,
            config.site.default,
        ))
        .description(lang_config.description.clone())
        .language(config.site.default.clone())
        .items(
            posts
                .iter()
                .map(|post| {
                    let url = format!(
                        "{}/posts/{}/",
                        config.site.default,
                        post.slug
                    );
                    ItemBuilder::default()
                        .title(post.title.clone())
                        .link(url.clone())
                        .description(strip_html(&post.summary_html))
                        .guid(
                            GuidBuilder::default()
                                .value(url.clone())
                                .permalink(true)
                                .build(),
                        )
                        .pub_date(Some(
                            chrono::NaiveDate::parse_from_str(&post.date, "%Y-%m-%d")
                                .unwrap()
                                .and_hms_opt(0, 0, 0)
                                .unwrap()
                                .and_utc()
                                .to_rfc2822(),
                        ))
                        .build()
                })
                .collect::<Vec<_>>(),
        )
        .build();

    let mut buf = Vec::new();
    channel.write_to(&mut buf)?;
    Ok(String::from_utf8(buf)?)
}

pub fn strip_html(html: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(c),
            _ => {}
        }
    }
    result.trim().to_string()
}
