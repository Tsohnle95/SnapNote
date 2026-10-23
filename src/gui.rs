use eframe::egui;
use snapnote::{
    capture::{self, Capture, CursorAnchor},
    clipboard,
    config::{self, Config},
    storage::{self, Draft, Snippet},
};
use std::{
    io::BufRead,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicI32, Ordering},
        mpsc::{self, Receiver, Sender},
    },
};

pub struct Options {
    pub capture_file: Option<PathBuf>,
    pub page: String,
    pub status: Option<String>,
    pub capture_completed_ms: Option<u128>,
    pub anchor: Option<CursorAnchor>,
}
enum Event {
    Loaded(anyhow::Result<Option<Capture>>),
    Prepared {
        result: anyhow::Result<Draft>,
        capture: Option<Capture>,
    },
    Finished {
        draft: Draft,
        result: anyhow::Result<Snippet>,
    },
    Copied(anyhow::Result<()>),
    Finish,
    Recapture,
    Settings,
    Recent,
    Listed(anyhow::Result<Vec<Snippet>>),
}
#[derive(PartialEq)]
enum Page {
    Popup,
    Settings,
    Recent,
}
struct Popup {
    tx: Sender<Event>,
    rx: Receiver<Event>,
    config_path: PathBuf,
    config: Config,
    preferences: Config,
    directory: String,
    page: Page,
    texture: Option<egui::TextureHandle>,
    source: Option<Capture>,
    draft: Option<Draft>,
    message: String,
    status: String,
    copied_until: Option<std::time::Instant>,
    loading: bool,
    copying: bool,
    saving: bool,
    finish_requested: bool,
    quitting: bool,
    focus: bool,
    exit_code: Arc<AtomicI32>,
    anchor: Option<CursorAnchor>,
    preview_size: egui::Vec2,
    started: std::time::Instant,
    preview_logged: bool,
    capture_completed_ms: Option<u128>,
    recent: Vec<Snippet>,
}
fn post(tx: &Sender<Event>, ctx: &egui::Context, event: Event) {
    let _ = tx.send(event);
    ctx.request_repaint();
}

// Preview ends at the release cursor; the focused message begins directly below it.
// Clamp the complete popup to that display so the input remains reachable at edges.
fn popup_geometry(
    anchor: Option<CursorAnchor>,
    image: egui::Vec2,
) -> (egui::Vec2, Option<egui::Pos2>) {
    let width = anchor.map_or(340., |a| (a.width as f32 - 16.).clamp(240., 340.));
    let scale = ((width - 8.) / image.x.max(1.))
        .min(220. / image.y.max(1.))
        .min(1.);
    let preview = image * scale;
    let size = egui::vec2(width, preview.y + 130.);
    let position = anchor.map(|a| {
        let left = a.left as f32;
        let top = a.top as f32;
        let max_x = (left + a.width as f32 - size.x - 8.).max(left + 8.);
        let max_y = (top + a.height as f32 - size.y - 8.).max(top + 8.);
        egui::pos2(
            (a.x as f32 - width / 2.).clamp(left + 8., max_x),
            (a.y as f32 - preview.y - 4.).clamp(top + 8., max_y),
        )
    });
    (size, position)
}
pub fn run(
    config_path: PathBuf,
    loaded: anyhow::Result<Config>,
    options: Options,
) -> anyhow::Result<i32> {
    let started = std::time::Instant::now();
    let (config, warning) = match loaded {
        Ok(config) => (config, String::new()),
        Err(error) => (Config::default(), format!("Configuration error: {error:#}")),
    };
    let page = match options.page.as_str() {
        "settings" => Page::Settings,
        "recent" => Page::Recent,
        _ => Page::Popup,
    };
    let is_popup = page == Page::Popup;
    let code = Arc::new(AtomicI32::new(0));
    let returned_code = code.clone();
    let mut native = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_visible(!is_popup)
            .with_inner_size(if is_popup { [340., 300.] } else { [500., 380.] })
            .with_min_inner_size([240., 150.])
            .with_decorations(!is_popup)
            .with_transparent(is_popup)
            .with_always_on_top(),
        ..Default::default()
    };
    #[cfg(target_os = "macos")]
    {
        native.event_loop_builder = Some(Box::new(|builder| {
            use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};
            builder
                .with_activation_policy(ActivationPolicy::Accessory)
                .with_default_menu(false);
        }));
    }
    let pending = options.capture_file.is_some();
    eframe::run_native(
        "SnapNote",
        native,
        Box::new(move |cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            let (tx, rx) = mpsc::channel();
            let pipe = tx.clone();
            let wake = cc.egui_ctx.clone();
            std::thread::spawn(move || {
                for line in std::io::stdin().lock().lines() {
                    let event = match line.as_deref() {
                        Ok("finish" | "quit") => Event::Finish,
                        Ok("capture") => Event::Recapture,
                        Ok("settings") => Event::Settings,
                        Ok("recent") => Event::Recent,
                        _ => continue,
                    };
                    post(&pipe, &wake, event);
                }
            });
            if let Some(path) = options.capture_file {
                let tx = tx.clone();
                let wake = cc.egui_ctx.clone();
                std::thread::spawn(move || {
                    post(
                        &tx,
                        &wake,
                        Event::Loaded(capture::adopt(path).map_err(Into::into)),
                    )
                });
            }
            if page == Page::Recent {
                let _ = tx.send(Event::Recent);
            }
            Ok(Box::new(Popup {
                tx,
                rx,
                config_path,
                directory: config.storage.directory.to_string_lossy().into_owned(),
                preferences: config.clone(),
                config,
                page,
                texture: None,
                source: None,
                draft: None,
                message: String::new(),
                status: options.status.unwrap_or(warning),
                copied_until: None,
                loading: pending,
                copying: false,
                saving: false,
                finish_requested: false,
                quitting: false,
                focus: false,
                exit_code: code,
                anchor: options.anchor,
                preview_size: egui::vec2(300., 180.),
                started,
                preview_logged: false,
                capture_completed_ms: options.capture_completed_ms,
                recent: vec![],
            }))
        }),
    )
    .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    Ok(returned_code.load(Ordering::SeqCst))
}
impl Popup {
    fn close(&mut self, ctx: &egui::Context) {
        self.quitting = true;
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
    fn show(&self, ctx: &egui::Context) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    }
    fn place(&mut self, ctx: &egui::Context) {
        let (size, position) = popup_geometry(self.anchor, self.preview_size);
        ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
        if let Some(position) = position {
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(position));
        }
    }
    fn prepare(&mut self, ctx: &egui::Context, capture: Capture) {
        self.loading = true;
        let root = config::expand(&self.config.storage.directory);
        let tx = self.tx.clone();
        let wake = ctx.clone();
        std::thread::spawn(move || {
            let result = root.and_then(|root| storage::begin(&root, capture.file.path()));
            let capture = if result.is_err() {
                Some(capture)
            } else {
                drop(capture);
                None
            };
            post(&tx, &wake, Event::Prepared { result, capture });
        });
    }
    fn finish(&mut self, ctx: &egui::Context) {
        self.finish_requested = true;
        if self.loading || self.saving || self.copying {
            return;
        }
        let Some(mut draft) = self.draft.take() else {
            if self.source.is_some() {
                self.finish_requested = false;
                self.status="Could not save the image. Change the folder in menu-bar Settings, then try Cmd+B again.".into();
                return;
            }
            self.close(ctx);
            return;
        };
        self.saving = true;
        let message = self.message.clone();
        let tx = self.tx.clone();
        let wake = ctx.clone();
        std::thread::spawn(move || {
            let started = std::time::Instant::now();
            let result = draft.commit(&message);
            eprintln!(
                "SnapNote: message finalized in {} ms",
                started.elapsed().as_millis()
            );
            post(&tx, &wake, Event::Finished { draft, result });
        });
    }
    fn copy(&mut self, ctx: &egui::Context) {
        if self.draft.is_none() || self.copying || self.saving {
            return;
        }
        self.copying = true;
        let text = clipboard::format(
            config::Mode::Agent,
            &self.draft.as_ref().unwrap().image_path(),
            &self.message,
        )
        .expect("agent formatting always returns text");
        let tx = self.tx.clone();
        let wake = ctx.clone();
        std::thread::spawn(move || post(&tx, &wake, Event::Copied(clipboard::copy_text(text))));
    }
    fn settings_ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("SnapNote Settings");
        ui.label("Capture / save-and-close shortcut");
        ui.text_edit_singleline(&mut self.preferences.capture.hotkey);
        ui.label("Screenshot folder");
        ui.text_edit_singleline(&mut self.directory);
        if ui.button("Save Settings").clicked() {
            self.preferences.storage.directory = PathBuf::from(&self.directory);
            self.status = match self.preferences.save(&self.config_path) {
                Ok(()) => {
                    self.config = self.preferences.clone();
                    "Saved. The shortcut applies when this popup closes.".into()
                }
                Err(error) => format!("Settings not saved: {error:#}"),
            };
            if let Some(capture) = self.source.take() {
                self.prepare(ui.ctx(), capture);
            }
        }
        if self.draft.is_some() && ui.button("Back to capture").clicked() {
            self.page = Page::Popup;
            self.place(ui.ctx());
            self.focus = true;
        }
        ui.separator();
        if ui
            .add_enabled(
                self.draft.is_none() && self.source.is_none() && !self.loading,
                egui::Button::new("Request Screen Recording Permission"),
            )
            .clicked()
        {
            self.exit_code.store(11, Ordering::SeqCst);
            self.close(ui.ctx());
        }
        ui.small("Enable SnapNote in System Settings → Privacy & Security → Screen & System Audio Recording. Relaunch after granting access if needed.");
        if !self.status.is_empty() {
            ui.label(&self.status);
        }
    }
}
impl eframe::App for Popup {
    fn clear_color(&self, _: &egui::Visuals) -> [f32; 4] {
        [0., 0., 0., 0.]
    }
    fn logic(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        if self
            .copied_until
            .is_some_and(|until| std::time::Instant::now() >= until)
        {
            self.copied_until = None;
        }
        if self.page == Page::Popup && self.loading {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }
        if !self.quitting && ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.finish(ctx);
        }
        while let Ok(event) = self.rx.try_recv() {
            match event {
                Event::Finish => self.finish(ctx),
                Event::Recapture => {
                    self.exit_code.store(10, Ordering::SeqCst);
                    self.finish(ctx);
                }
                Event::Loaded(result) => match result {
                    Ok(Some(mut capture)) => {
                        self.preview_size =
                            egui::vec2(capture.image.width() as f32, capture.image.height() as f32);
                        self.texture = Some(ctx.load_texture(
                            "capture",
                            egui::ColorImage::from_rgba_unmultiplied(
                                [
                                    capture.image.width() as usize,
                                    capture.image.height() as usize,
                                ],
                                capture.image.as_raw(),
                            ),
                            Default::default(),
                        ));
                        capture.image = image::RgbaImage::new(0, 0);
                        self.prepare(ctx, capture);
                    }
                    Ok(None) => {
                        self.loading = false;
                        self.close(ctx);
                    }
                    Err(error) => {
                        self.loading = false;
                        self.status = format!("{error:#}");
                        self.page = Page::Settings;
                        self.show(ctx);
                    }
                },
                Event::Prepared { result, capture } => {
                    self.loading = false;
                    self.source = capture;
                    match result {
                        Ok(draft) => {
                            let prefix = format!("{}\n", draft.image_path().display());
                            self.message = format!("{prefix}{}", self.message);
                            self.draft = Some(draft);
                            self.page = Page::Popup;
                            self.focus = true;
                            self.place(ctx);
                            self.show(ctx);
                            if self.finish_requested {
                                self.finish(ctx);
                            }
                        }
                        Err(error) => {
                            self.status = format!(
                                "Image not saved: {error:#}. Open menu-bar Settings to change the folder."
                            );
                            self.show(ctx);
                        }
                    }
                }
                Event::Copied(result) => {
                    self.copying = false;
                    match result {
                        Ok(()) => {
                            self.status.clear();
                            let duration = std::time::Duration::from_secs(1);
                            self.copied_until = Some(std::time::Instant::now() + duration);
                            ctx.request_repaint_after(duration);
                        }
                        Err(error) => {
                            self.copied_until = None;
                            self.status = format!("Clipboard failed: {error:#}");
                        }
                    }
                    if self.finish_requested {
                        self.finish(ctx);
                    }
                }
                Event::Finished { draft, result } => {
                    self.saving = false;
                    match result {
                        Ok(_) => {
                            drop(draft);
                            self.close(ctx);
                        }
                        Err(error) => {
                            self.draft = Some(draft);
                            self.finish_requested = false;
                            self.status =
                                format!("Could not save: {error:#}. Press Cmd+B to retry.");
                            self.show(ctx);
                        }
                    }
                }
                Event::Settings => {
                    self.page = Page::Settings;
                    self.preferences = self.config.clone();
                    self.directory = self.config.storage.directory.to_string_lossy().into_owned();
                    ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(true));
                    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(500., 380.)));
                    self.show(ctx);
                }
                Event::Recent => {
                    self.page = Page::Recent;
                    self.show(ctx);
                    let root = config::expand(&self.config.storage.directory);
                    let tx = self.tx.clone();
                    let wake = ctx.clone();
                    std::thread::spawn(move || {
                        post(
                            &tx,
                            &wake,
                            Event::Listed(root.and_then(|p| storage::list(&p))),
                        )
                    });
                }
                Event::Listed(result) => match result {
                    Ok(snippets) => self.recent = snippets,
                    Err(error) => self.status = format!("{error:#}"),
                },
            }
        }
    }
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if self.page == Page::Popup
            && !self.loading
            && !self.saving
            && ctx.input(|i| i.pointer.secondary_pressed())
        {
            // Explicit cancellation: dropping the uncommitted draft removes its PNG.
            // Do not publish metadata or modify the user's clipboard.
            self.draft = None;
            self.source = None;
            self.texture = None;
            self.close(&ctx);
            return;
        }
        let mut copy_message = false;
        let mut finish_message = false;
        if self.page == Page::Popup {
            copy_message = ctx.input_mut(|i| {
                let control_c = i.consume_key(egui::Modifiers::CTRL, egui::Key::C);
                // winit converts native Cmd+C to Event::Copy before egui sees it.
                // Consume it here so TextEdit cannot replace the full payload
                // with just the selected text.
                let command_c = i
                    .events
                    .iter()
                    .any(|event| matches!(event, egui::Event::Copy));
                i.events.retain(|event| !matches!(event, egui::Event::Copy));
                control_c || command_c
            });
            finish_message = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        }
        let panel_frame = if self.page == Page::Popup {
            egui::Frame::NONE
        } else {
            egui::Frame::new().fill(egui::Color32::from_rgb(30, 32, 38))
        };
        egui::CentralPanel::default()
            .frame(panel_frame)
            .show(ui, |ui| match self.page {
                Page::Popup => {
                    if self.texture.is_some() && !self.preview_logged {
                        self.preview_logged = true;
                        eprintln!(
                            "SnapNote: popup initialization to preview draw {} ms",
                            self.started.elapsed().as_millis()
                        );
                        if let Some(completed) = self.capture_completed_ms.take() {
                            let now = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis();
                            eprintln!(
                                "SnapNote: native capture completion to popup draw {} ms",
                                now.saturating_sub(completed)
                            );
                        }
                    }
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            if let Some(texture) = &self.texture {
                                let preview = ui.add(
                                    egui::Image::new(texture)
                                        .max_size(egui::vec2(ui.available_width() - 4., 220.))
                                        .maintain_aspect_ratio(true)
                                        .corner_radius(8)
                                        .sense(egui::Sense::drag()),
                                );
                                let preview = preview.on_hover_cursor(egui::CursorIcon::Grab);
                                // AppKit requires the original mouse-down event, before
                                // egui's drag-distance threshold has been crossed.
                                if preview.is_pointer_button_down_on()
                                    && ctx.input(|i| i.pointer.primary_pressed())
                                {
                                    ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                                }
                            }
                        });
                        ui.add_space(6.);
                        egui::Frame::new()
                            .fill(egui::Color32::from_rgb(30, 32, 38))
                            .corner_radius(12)
                            .inner_margin(10)
                            .show(ui, |ui| {
                                ui.add_enabled_ui(!self.loading && !self.saving, |ui| {
                                    let mut output = egui::TextEdit::multiline(&mut self.message)
                                        .id_salt("message")
                                        .frame(egui::Frame::NONE)
                                        .desired_width(f32::INFINITY)
                                        .desired_rows(4)
                                        .cursor_at_end(true)
                                        .hint_text("Add context…")
                                        .show(ui);
                                    if self.focus {
                                        output.response.request_focus();
                                        output.state.cursor.set_char_range(Some(
                                            egui::text::CCursorRange::one(
                                                egui::text::CCursor::new(
                                                    self.message.chars().count(),
                                                ),
                                            ),
                                        ));
                                        output.state.store(ui.ctx(), output.response.id);
                                        self.focus = false;
                                    }
                                });
                                if !self.status.is_empty() {
                                    ui.small(&self.status);
                                }
                            });
                    });
                }
                Page::Settings => {
                    egui::Frame::new()
                        .inner_margin(12)
                        .show(ui, |ui| self.settings_ui(ui));
                }
                Page::Recent => {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.heading("Recent snapshots");
                        for snippet in self.recent.iter().take(50) {
                            ui.label(&snippet.metadata.id);
                            ui.label(snippet.text.chars().take(150).collect::<String>());
                            ui.separator();
                        }
                        if !self.status.is_empty() {
                            ui.label(&self.status);
                        }
                    });
                }
            });
        if self.page == Page::Popup && self.copied_until.is_some() {
            egui::Area::new(egui::Id::unique("copied_feedback"))
                .anchor(egui::Align2::RIGHT_BOTTOM, [-10., -8.])
                .interactable(false)
                .show(&ctx, |ui| {
                    let green = egui::Color32::from_rgb(95, 220, 135);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.;
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(12., 12.), egui::Sense::hover());
                        ui.painter().add(egui::Shape::line(
                            vec![
                                rect.min + egui::vec2(1., 6.),
                                rect.min + egui::vec2(5., 10.),
                                rect.min + egui::vec2(11., 2.),
                            ],
                            egui::Stroke::new(1.7, green),
                        ));
                        ui.label(egui::RichText::new("Copied").color(green).small());
                    });
                });
        }
        // TextEdit must consume pending typing/paste before we snapshot the message.
        if copy_message {
            self.copy(&ctx);
        }
        if finish_message {
            self.finish(&ctx);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn popup_places_preview_above_cursor_and_clamps_to_monitor() {
        let anchor = CursorAnchor {
            x: 700.,
            y: 400.,
            left: 0.,
            top: 0.,
            width: 1440.,
            height: 900.,
        };
        let (size, pos) = popup_geometry(Some(anchor), egui::vec2(1000., 600.));
        let pos = pos.unwrap();
        assert!(pos.y < anchor.y as f32);
        assert!(pos.y + size.y > anchor.y as f32);
        assert!(pos.x >= 8.);
        for (x, y) in [(1., 1.), (1439., 899.)] {
            let (_, pos) = popup_geometry(
                Some(CursorAnchor { x, y, ..anchor }),
                egui::vec2(1000., 600.),
            );
            let pos = pos.unwrap();
            assert!(pos.x >= 8. && pos.y >= 8.);
            assert!(pos.x + size.x <= 1440. && pos.y + size.y <= 900.);
        }
    }
}
