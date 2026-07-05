use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;

use eframe::egui::{self, CornerRadius, Frame};

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
    let _ = eframe::run_native(
        "Stellaris Pulvis",
        options,
        Box::new(|_cc| Ok(Box::new(app))),
    );
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
    filter_lang: String,
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
            status: "就绪".to_string(),
            building: false,
            deploying: false,
            new_post_title: String::new(),
            new_post_lang: "zh".to_string(),
            show_new_post: false,
            filter_drafts: true,
            filter_lang: "all".to_string(),
        };
        app.load_config();
        app.load_posts();
        app
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
            self.status = format!("已删除: {}", title);
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
            self.status = format!("文件已存在: {}.md", slug);
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
            self.status = format!("已创建: {}.md，双击在编辑器中打开", slug);
        }
    }

    fn build_site(&mut self) {
        if self.building {
            return;
        }
        self.building = true;
        self.status = "构建中...".to_string();
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
                            let _ = tx.send(Msg::BuildError(format!("初始化失败: {}", e)));
                            return;
                        }
                    };
                    match builder.build_all() {
                        Ok(_) => {
                            let _ = tx.send(Msg::BuildDone(start.elapsed()));
                        }
                        Err(e) => {
                            let _ = tx.send(Msg::BuildError(format!("构建失败: {}", e)));
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(Msg::BuildError(format!("读取配置失败: {}", e)));
                }
            }
        });
    }

    fn deploy(&mut self) {
        if self.deploying {
            return;
        }
        self.deploying = true;
        self.status = "部署中...".to_string();
        let project_dir = self.project_dir.clone();
        let tx = self.msg_tx.clone();
        thread::spawn(move || {
            let add = std::process::Command::new("git")
                .args(["-C", &project_dir.to_string_lossy(), "add", "-A"])
                .output();
            if add.is_err() || !add.unwrap().status.success() {
                let _ = tx.send(Msg::DeployError("git add 失败".to_string()));
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
                let _ = tx.send(Msg::DeployError("git commit 失败".to_string()));
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
                    let _ = tx.send(Msg::DeployError(format!("git push 失败: {}", stderr)));
                }
                Err(e) => {
                    let _ = tx.send(Msg::DeployError(format!("git push 失败: {}", e)));
                }
            }
        });
    }

    fn process_messages(&mut self) {
        while let Ok(msg) = self.msg_rx.try_recv() {
            match msg {
                Msg::BuildDone(dur) => {
                    self.building = false;
                    self.status = format!("✅ 构建完成，耗时 {:.1?}", dur);
                }
                Msg::BuildError(e) => {
                    self.building = false;
                    self.status = format!("❌ {}", e);
                }
                Msg::DeployDone => {
                    self.deploying = false;
                    self.status = "✅ 部署完成，已推送到 GitHub".to_string();
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

            let items = [("📝 文章", "posts"), ("⚡ 构建", "build")];
            for (label, id) in items {
                if ui
                    .selectable_label(*selected == id, label)
                    .clicked()
                {
                    *selected = id;
                }
            }

            ui.separator();
            if ui.button("🔄 刷新列表").clicked() {
                self.load_posts();
                self.status = "已刷新".to_string();
            }
        });
    }

    fn posts_view(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.heading("文章列表");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("📝 新建文章").clicked() {
                        self.show_new_post = !self.show_new_post;
                    }
                });
            });

            if self.show_new_post {
                Frame::group(ui.style())
                    .fill(ui.style().visuals.extreme_bg_color)
                    .corner_radius(CornerRadius::same(8))
                    .show(ui, |ui| {
                        ui.label("标题:");
                        ui.text_edit_singleline(&mut self.new_post_title);
                        ui.horizontal(|ui| {
                            ui.label("语言:");
                            egui::ComboBox::from_id_salt("lang")
                                .selected_text(&self.new_post_lang)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut self.new_post_lang, "zh".to_string(), "中文");
                                    ui.selectable_value(&mut self.new_post_lang, "en".to_string(), "English");
                                });
                        });
                        ui.horizontal(|ui| {
                            if ui.button("创建").clicked() {
                                self.create_post();
                            }
                            if ui.button("取消").clicked() {
                                self.show_new_post = false;
                            }
                        });
                    });
                ui.add_space(8.0);
            }

            ui.separator();

            ui.horizontal(|ui| {
                ui.checkbox(&mut self.filter_drafts, "显示草稿");
                ui.label("语言:");
                egui::ComboBox::from_id_salt("filter_lang")
                    .selected_text(&self.filter_lang)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.filter_lang, "all".to_string(), "全部");
                        ui.selectable_value(&mut self.filter_lang, "zh".to_string(), "中文");
                        ui.selectable_value(&mut self.filter_lang, "en".to_string(), "English");
                    });
            });

            ui.separator();

            let filtered: Vec<(usize, PostInfo)> = self
                .posts
                .iter()
                .enumerate()
                .filter(|(_, p)| {
                    if self.filter_lang != "all" && p.lang != self.filter_lang {
                        return false;
                    }
                    if !self.filter_drafts && p.draft {
                        return false;
                    }
                    true
                })
                .map(|(i, p)| (i, p.clone()))
                .collect();

            if filtered.is_empty() {
                ui.label("暂无文章");
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
                                        ui.label(format!(
                                            "{}  |  {}  |  {}",
                                            post.date,
                                            if post.lang == "zh" { "中文" } else { "English" },
                                            if post.draft { "草稿" } else { "已发布" },
                                        ));
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
                                                .on_hover_text("删除此文章")
                                                .clicked()
                                            {
                                                self.delete_post(idx);
                                            }
                                            if ui.button("✏️ 编辑")
                                                .on_hover_text("在编辑器中打开")
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
            ui.heading("构建 & 部署");
            ui.separator();
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                if ui.add_enabled(!self.building, egui::Button::new("⚡ 构建")).clicked() {
                    self.build_site();
                }

                if ui.add_enabled(!self.building && !self.deploying, egui::Button::new("🚀 构建 + 部署")).clicked() {
                    self.build_site();
                    self.deploy();
                }
            });

            ui.add_space(8.0);

            if self.building {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("构建中...");
                });
            }
            if self.deploying {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("部署中...");
                });
            }
            if self.config.is_some() {
                ui.add_space(16.0);
                Frame::group(ui.style())
                    .corner_radius(CornerRadius::same(6))
                    .show(ui, |ui| {
                        ui.heading("配置信息");
                        let cfg = self.config.as_ref().unwrap();
                        ui.label(format!("站点标题: {}", cfg.site.default));
                        ui.label(format!("作者: {}", cfg.author.name));
                        if let Some(email) = &cfg.author.email {
                            ui.label(format!("邮箱: {}", email));
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
