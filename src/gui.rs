use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;

use std::sync::Arc;
use eframe::egui::{self, CornerRadius, FontData, FontDefinitions, FontFamily, Frame};

use crate::config::Config;
use crate::content::Post;

#[derive(Clone)]
#[allow(dead_code)]
struct PostInfo {
    path: PathBuf,
    slug: String,
    title: String,
    date: String,
    tags: Vec<String>,
    draft: bool,
    lang: String,
}

#[allow(dead_code)]
enum Msg {
    BuildDone(std::time::Duration),
    BuildError(String),
    DeployDone,
    DeployError(String),
}

pub fn run(project_dir: &PathBuf) {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([900.0, 600.0]),
        ..Default::default()
    };

    let app = StellarisApp::new(project_dir.clone());

    let bundled = project_dir.join("fonts/NotoSansSC.ttf");
    if !bundled.exists() {
        std::fs::create_dir_all(project_dir.join("fonts")).ok();
        let url = "https://github.com/google/fonts/raw/main/ofl/notosanssc/NotoSansSC%5Bwght%5D.ttf";
        let output = std::process::Command::new("curl")
            .args(["-sL", "-o", &bundled.to_string_lossy(), url])
            .output();
        if output.is_err() || !output.unwrap().status.success() {
            let _ = std::fs::remove_file(&bundled);
        }
    }
    let font_path = if bundled.exists() { Some(bundled) } else { None };

    let _ = eframe::run_native(
        "Stellaris Pulvis",
        options,
        Box::new(move |cc| {
            if let Some(ref path) = font_path {
                if let Ok(font_bytes) = std::fs::read(path) {
                    let mut fonts = FontDefinitions::default();
                    fonts.font_data.insert(
                        "cjk".to_string(),
                        Arc::new(FontData::from_owned(font_bytes)),
                    );
                    fonts.families
                        .entry(FontFamily::Proportional)
                        .or_insert_with(Vec::new)
                        .push("cjk".to_string());
                    fonts.families
                        .entry(FontFamily::Monospace)
                        .or_insert_with(Vec::new)
                        .push("cjk".to_string());
                    cc.egui_ctx.set_fonts(fonts);
                }
            }
            Ok(Box::new(app))
        }),
    );
}

#[derive(Clone, Copy, PartialEq)]
enum UiLang {
    Zh,
    En,
}

struct StellarisApp {
    project_dir: PathBuf,
    posts: Vec<PostInfo>,
    #[allow(dead_code)]
    config: Option<Config>,
    msg_rx: mpsc::Receiver<Msg>,
    msg_tx: mpsc::Sender<Msg>,
    status: String,
    building: bool,
    deploying: bool,
    new_post_title: String,
    new_post_lang: String,
    show_new_post: bool,
    filter_drafts: bool,
    ui_lang: UiLang,
}

impl StellarisApp {
    fn new(project_dir: PathBuf) -> Self {
        let (tx, rx) = mpsc::channel();
        let mut app = Self {
            project_dir,
            posts: Vec::new(),
            config: None,
            msg_rx: rx,
            msg_tx: tx,
            status: String::new(),
            building: false,
            deploying: false,
            new_post_title: String::new(),
            new_post_lang: "zh".to_string(),
            show_new_post: false,
            filter_drafts: true,
            ui_lang: UiLang::Zh,
        };
        app.status = app.t("就绪", "Ready");
        app.load_config();
        app.load_posts();
        app
    }

    fn t(&self, zh: &str, en: &str) -> String {
        match self.ui_lang {
            UiLang::Zh => zh.to_string(),
            UiLang::En => en.to_string(),
        }
    }

    fn load_config(&mut self) {
        let config_path = self.project_dir.join("config.toml");
        if let Ok(config) = Config::load(&config_path) {
            self.config = Some(config);
        }
    }

    fn load_posts(&mut self) {
        let dirs = ["zh", "en"];
        let mut posts = Vec::new();

        for lang in dirs {
            let posts_dir = self.project_dir.join("content").join(lang).join("posts");
            if !posts_dir.exists() {
                continue;
            }
            if let Ok(entries) = std::fs::read_dir(&posts_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map(|e| e == "md").unwrap_or(false) {
                        let slug = path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .map(|s| s.to_string());
                        if let Some(slug) = slug {
                            if let Ok(post) = Post::from_file(&path, &slug) {
                                posts.push(PostInfo {
                                    path: path.clone(),
                                    slug,
                                    title: post.front_matter.title,
                                    date: post.front_matter.date.to_string(),
                                    tags: post.front_matter.tags,
                                    draft: post.front_matter.draft,
                                    lang: lang.to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }

        posts.sort_by(|a, b| b.date.cmp(&a.date));
        self.posts = posts;
    }

    fn open_in_editor(&self, path: &PathBuf) {
        let path = path.clone();
        thread::spawn(move || {
            let _ = open::that(&path);
        });
    }

    fn delete_post(&mut self, idx: usize) {
        if idx < self.posts.len() {
            let path = self.posts[idx].path.clone();
            let title = self.posts[idx].title.clone();
            if path.exists() {
                let _ = std::fs::remove_file(&path);
            }
            self.load_posts();
            self.status = format!("{}: {}", self.t("已删除", "Deleted"), title);
        }
    }

    fn create_post(&mut self) {
        if self.new_post_title.trim().is_empty() {
            return;
        }

        let lang = &self.new_post_lang;
        let slug = slugify(&self.new_post_title);
        let posts_dir = self
            .project_dir
            .join("content")
            .join(lang)
            .join("posts");
        let path = posts_dir.join(format!("{}.md", slug));

        if path.exists() {
            self.status = format!("{}: {}.md", self.t("文件已存在", "File already exists"), slug);
            return;
        }

        let today = chrono::Local::now().format("%Y-%m-%d");
        let content = format!(
            r#"---
title: "{}"
date: {}
tags: []
draft: true
---

"#,
            self.new_post_title.trim(),
            today,
        );

        if std::fs::write(&path, &content).is_ok() {
            self.new_post_title.clear();
            self.show_new_post = false;
            self.load_posts();
            self.status = format!(
                "{}: {}.md",
                self.t("已创建，在编辑器中打开", "Created, open in editor"),
                slug
            );
        }
    }

    fn build_site(&mut self) {
        if self.building {
            return;
        }
        self.building = true;
        self.status = self.t("构建中...", "Building...");
        let project_dir = self.project_dir.clone();
        let tx = self.msg_tx.clone();
        thread::spawn(move || {
            let start = std::time::Instant::now();
            let config_path = project_dir.join("config.toml");
            match Config::load(&config_path) {
                Ok(config) => {
                    let mut builder = match crate::builder::SiteBuilder::new(&project_dir, config)
                    {
                        Ok(b) => b,
                        Err(e) => {
                            let _ = tx.send(Msg::BuildError(format!("init failed: {}", e)));
                            return;
                        }
                    };
                    match builder.build_all() {
                        Ok(_) => {
                            let _ = tx.send(Msg::BuildDone(start.elapsed()));
                        }
                        Err(e) => {
                            let _ = tx.send(Msg::BuildError(format!("build failed: {}", e)));
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(Msg::BuildError(format!("config failed: {}", e)));
                }
            }
        });
    }

    fn deploy(&mut self) {
        if self.deploying {
            return;
        }
        self.deploying = true;
        self.status = self.t("部署中...", "Deploying...");
        let project_dir = self.project_dir.clone();
        let tx = self.msg_tx.clone();
        thread::spawn(move || {
            let add = std::process::Command::new("git")
                .args(["-C", &project_dir.to_string_lossy(), "add", "-A"])
                .output();
            if add.is_err() || !add.unwrap().status.success() {
                let _ = tx.send(Msg::DeployError("git add failed".to_string()));
                return;
            }

            let now = chrono::Local::now().format("%Y-%m-%d %H:%M");
            let commit = std::process::Command::new("git")
                .args([
                    "-C",
                    &project_dir.to_string_lossy(),
                    "commit",
                    "--allow-empty",
                    "-m",
                    &format!("deploy: auto update {}", now),
                ])
                .output();
            if commit.is_err() {
                let _ = tx.send(Msg::DeployError("git commit failed".to_string()));
                return;
            }

            let push = std::process::Command::new("git")
                .args(["-C", &project_dir.to_string_lossy(), "push"])
                .output();

            match push {
                Ok(o) if o.status.success() => {
                    let _ = tx.send(Msg::DeployDone);
                }
                Ok(o) => {
                    let stderr = String::from_utf8_lossy(&o.stderr);
                    let _ = tx.send(Msg::DeployError(format!("git push failed: {}", stderr)));
                }
                Err(e) => {
                    let _ = tx.send(Msg::DeployError(format!("git push failed: {}", e)));
                }
            }
        });
    }

    fn process_messages(&mut self) {
        while let Ok(msg) = self.msg_rx.try_recv() {
            match msg {
                Msg::BuildDone(dur) => {
                    self.building = false;
                    self.status = format!(
                        "{}  {:.1?}",
                        self.t("✅ 构建完成，耗时", "✅ Build done in"),
                        dur
                    );
                }
                Msg::BuildError(e) => {
                    self.building = false;
                    self.status = format!("❌ {}", e);
                }
                Msg::DeployDone => {
                    self.deploying = false;
                    self.status = self.t(
                        "✅ 部署完成，已推送到 GitHub",
                        "✅ Deploy done, pushed to GitHub",
                    );
                }
                Msg::DeployError(e) => {
                    self.deploying = false;
                    self.status = format!("❌ {}", e);
                }
            }
        }
    }

    fn sidebar(&mut self, ui: &mut egui::Ui, selected: &mut &'static str) {
        ui.vertical(|ui| {
            ui.heading("Stellaris");
            ui.label("Pulvis");
            ui.separator();

            let items = [
                (self.t("📝 文章", "📝 Posts"), "posts"),
                (self.t("⚡ 构建", "⚡ Build"), "build"),
            ];
            for (label, id) in items {
                if ui
                    .selectable_label(*selected == id, label)
                    .clicked()
                {
                    *selected = id;
                }
            }

            ui.separator();
            if ui.button(self.t("🔄 刷新列表", "🔄 Refresh")).clicked() {
                self.load_posts();
                self.status = self.t("已刷新", "Refreshed");
            }

            // UI language switcher at the bottom
            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.selectable_label(self.ui_lang == UiLang::Zh, "中文").clicked() {
                        self.ui_lang = UiLang::Zh;
                    }
                    if ui.selectable_label(self.ui_lang == UiLang::En, "EN").clicked() {
                        self.ui_lang = UiLang::En;
                    }
                });
            });
        });
    }

    fn posts_view(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.heading(self.t("文章列表", "Posts"));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(self.t("📝 新建文章", "📝 New Post")).clicked() {
                        self.show_new_post = !self.show_new_post;
                    }
                });
            });

            if self.show_new_post {
                Frame::group(ui.style())
                    .fill(ui.style().visuals.extreme_bg_color)
                    .corner_radius(CornerRadius::same(8))
                    .show(ui, |ui| {
                        ui.label(self.t("标题:", "Title:"));
                        ui.text_edit_singleline(&mut self.new_post_title);
                        ui.horizontal(|ui| {
                            ui.label(self.t("语言:", "Language:"));
                            egui::ComboBox::from_id_salt("lang")
                                .selected_text(&self.new_post_lang)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut self.new_post_lang, "zh".to_string(), "中文");
                                    ui.selectable_value(&mut self.new_post_lang, "en".to_string(), "English");
                                });
                        });
                        ui.horizontal(|ui| {
                            if ui.button(self.t("创建", "Create")).clicked() {
                                self.create_post();
                            }
                            if ui.button(self.t("取消", "Cancel")).clicked() {
                                self.show_new_post = false;
                            }
                        });
                    });
                ui.add_space(8.0);
            }

            let show_drafts = self.t("显示草稿", "Show Drafts");
            ui.checkbox(&mut self.filter_drafts, show_drafts);

            ui.separator();

            let filtered: Vec<(usize, PostInfo)> = self
                .posts
                .iter()
                .enumerate()
                .filter(|(_, p)| {
                    if !self.filter_drafts && p.draft {
                        return false;
                    }
                    true
                })
                .map(|(i, p)| (i, p.clone()))
                .collect();

            if filtered.is_empty() {
                ui.label(self.t("暂无文章", "No posts yet"));
                return;
            }

            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    for (orig_idx, post) in &filtered {
                        Frame::group(ui.style())
                            .fill(ui.style().visuals.faint_bg_color)
                            .corner_radius(CornerRadius::same(6))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    if post.draft {
                                        ui.label("📄");
                                    } else {
                                        ui.label("📰");
                                    }

                                    ui.vertical(|ui| {
                                        ui.strong(&post.title);
                                        let lang_label = match post.lang.as_str() {
                                            "zh" => "中文",
                                            _ => "English",
                                        };
                                        let status_label = if post.draft {
                                            self.t("草稿", "Draft")
                                        } else {
                                            self.t("已发布", "Published")
                                        };
                                        ui.label(format!("{}  |  {}  |  {}", post.date, lang_label, status_label));
                                        if !post.tags.is_empty() {
                                            ui.label(format!("🏷️ {}", post.tags.join(", ")));
                                        }
                                    });

                                    let path = post.path.clone();
                                    let idx = *orig_idx;

                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if ui.button("🗑️ 删除")
                                                .on_hover_text(self.t("删除此文章", "Delete this post"))
                                                .clicked()
                                            {
                                                self.delete_post(idx);
                                            }
                                            if ui.button("✏️ 编辑")
                                                .on_hover_text(self.t("在编辑器中打开", "Open in editor"))
                                                .clicked()
                                            {
                                                self.open_in_editor(&path);
                                            }
                                        },
                                    );
                                });
                            });
                        ui.add_space(4.0);
                    }
                });
        });
    }

    fn build_view(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.heading(self.t("构建 & 部署", "Build & Deploy"));
            ui.separator();
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                if ui.add_enabled(!self.building, egui::Button::new(self.t("⚡ 构建", "⚡ Build"))).clicked() {
                    self.build_site();
                }

                if ui.add_enabled(!self.building && !self.deploying, egui::Button::new(self.t("🚀 构建 + 部署", "🚀 Build + Deploy"))).clicked() {
                    self.build_site();
                    self.deploy();
                }
            });

            ui.add_space(8.0);

            if self.building {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(self.t("构建中...", "Building..."));
                });
            }
            if self.deploying {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(self.t("部署中...", "Deploying..."));
                });
            }
            if self.config.is_some() {
                ui.add_space(16.0);
                Frame::group(ui.style())
                    .corner_radius(CornerRadius::same(6))
                    .show(ui, |ui| {
                        ui.heading(self.t("配置信息", "Config Info"));
                        let cfg = self.config.as_ref().unwrap();
                        let lang_text = format!(
                            "{}: {}",
                            self.t("站点默认语言", "Default language"),
                            cfg.site.default
                        );
                        ui.label(lang_text);
                        let author_text = format!("{}: {}", self.t("作者", "Author"), cfg.author.name);
                        ui.label(author_text);
                        if let Some(email) = &cfg.author.email {
                            let email_text = format!("{}: {}", self.t("邮箱", "Email"), email);
                            ui.label(email_text);
                        }
                    });
            }
        });
    }
}

impl eframe::App for StellarisApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.process_messages();

        let mut selected = "posts";

        egui::SidePanel::left("sidebar")
            .resizable(false)
            .default_width(160.0)
            .show(ctx, |ui| {
                self.sidebar(ui, &mut selected);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            match selected {
                "posts" => self.posts_view(ui),
                "build" => self.build_view(ui),
                _ => {}
            }

            // status bar at bottom
            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(&self.status);
                });
            });
        });
    }
}

fn slugify(title: &str) -> String {
    title
        .to_lowercase()
        .trim()
        .replace(|c: char| !c.is_alphanumeric() && c != ' ', "")
        .replace(' ', "-")
}
