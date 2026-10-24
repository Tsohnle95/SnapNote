use eframe::egui;
use snapnote::{
    annotations::{Mark, Tool},
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
    Copied {
        draft: Draft,
        result: anyhow::Result<()>,
    },
    Finish,
    Recapture,
    Settings,
    Recent,
    Listed(anyhow::Result<Vec<Snippet>>),
}
/// High-contrast palette for the capture popup.
///
/// The popup floats over arbitrary screenshots, so surfaces are opaque and every
/// text pair clears WCAG AA (4.5:1 for text, 3:1 for borders and icons). The
/// contrast test below keeps these values from regressing.
mod theme {
    use eframe::egui::{self, Color32, CornerRadius, Stroke};

    pub const PANEL: Color32 = Color32::from_rgb(25, 28, 33);
    pub const FIELD_BG: Color32 = Color32::from_rgb(20, 23, 27);
    pub const BORDER: Color32 = Color32::from_rgb(112, 121, 135);
    pub const TEXT: Color32 = Color32::from_rgb(242, 244, 247);
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(195, 200, 208);
    pub const TEXT_HINT: Color32 = Color32::from_rgb(170, 177, 188);
    pub const BUTTON_BG: Color32 = Color32::from_rgb(52, 58, 67);
    pub const BUTTON_HOVER: Color32 = Color32::from_rgb(67, 75, 86);
    pub const BUTTON_ACTIVE: Color32 = Color32::from_rgb(42, 48, 56);
    pub const ACCENT: Color32 = Color32::from_rgb(195, 220, 168);
    pub const ACCENT_INK: Color32 = Color32::from_rgb(21, 26, 18);
    pub const SUCCESS: Color32 = Color32::from_rgb(99, 224, 142);
    pub const DANGER: Color32 = Color32::from_rgb(255, 138, 138);
    pub const DEFAULT_BOX: Color32 = Color32::from_rgb(61, 79, 216);

    /// Vibrant box colors that keep white text above 4.5:1.
    pub const VIBRANT: [(&str, Color32); 8] = [
        ("Indigo", Color32::from_rgb(61, 79, 216)),
        ("Violet", Color32::from_rgb(109, 59, 209)),
        ("Magenta", Color32::from_rgb(166, 36, 122)),
        ("Crimson", Color32::from_rgb(179, 36, 60)),
        ("Teal", Color32::from_rgb(15, 118, 110)),
        ("Emerald", Color32::from_rgb(31, 122, 77)),
        ("Ocean", Color32::from_rgb(14, 110, 158)),
        ("Amber", Color32::from_rgb(146, 88, 8)),
    ];

    /// Muted, standard colors for a quieter popup. These are darker than the
    /// vibrant set, so the derived controls gain contrast rather than lose it.
    pub const STANDARD: [(&str, Color32); 8] = [
        ("Graphite", Color32::from_rgb(60, 64, 70)),
        ("Slate", Color32::from_rgb(74, 84, 100)),
        ("Steel", Color32::from_rgb(64, 76, 86)),
        ("Navy", Color32::from_rgb(47, 62, 97)),
        ("Forest", Color32::from_rgb(48, 82, 55)),
        ("Olive", Color32::from_rgb(79, 74, 43)),
        ("Wine", Color32::from_rgb(99, 40, 60)),
        ("Brown", Color32::from_rgb(92, 68, 52)),
    ];

    pub fn mix(a: Color32, b: Color32, amount: f32) -> Color32 {
        let channel = |a: u8, b: u8| (a as f32 * (1.0 - amount) + b as f32 * amount).round() as u8;
        Color32::from_rgb(
            channel(a.r(), b.r()),
            channel(a.g(), b.g()),
            channel(a.b(), b.b()),
        )
    }

    pub fn box_border(fill: Color32) -> Color32 {
        mix(fill, TEXT, 0.7)
    }

    pub fn box_hint(fill: Color32) -> Color32 {
        mix(fill, TEXT, 0.96)
    }

    pub fn button_fill(fill: Color32) -> Color32 {
        mix(fill, Color32::BLACK, 0.55)
    }

    pub fn button_hover(fill: Color32) -> Color32 {
        mix(fill, Color32::BLACK, 0.42)
    }

    pub fn button_active(fill: Color32) -> Color32 {
        mix(fill, Color32::BLACK, 0.64)
    }

    pub fn parse_color(value: &str) -> Option<Color32> {
        let hex = value.trim().trim_start_matches('#');
        if hex.len() != 6 {
            return None;
        }
        let channel = |index: usize| u8::from_str_radix(&hex[index..index + 2], 16).ok();
        Some(Color32::from_rgb(channel(0)?, channel(2)?, channel(4)?))
    }

    pub fn color_hex(color: Color32) -> String {
        format!("#{:02X}{:02X}{:02X}", color.r(), color.g(), color.b())
    }

    pub fn color_swatches(
        ui: &mut egui::Ui,
        current: &mut Color32,
        palette: &[(&str, Color32)],
    ) -> bool {
        let mut changed = false;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(5., 5.);
            for (name, color) in palette {
                let (rect, response) =
                    ui.allocate_exact_size(egui::vec2(20., 20.), egui::Sense::click());
                let selected = *current == *color;
                ui.painter().rect_filled(rect, 5, *color);
                ui.painter().rect_stroke(
                    rect,
                    5,
                    Stroke::new(
                        if selected { 2.0 } else { 1.0 },
                        if selected { TEXT } else { BORDER },
                    ),
                    egui::StrokeKind::Inside,
                );
                if response.clicked() {
                    *current = *color;
                    changed = true;
                }
                response.on_hover_text(*name);
            }
        });
        changed
    }

    pub fn color_name(color: Color32) -> Option<&'static str> {
        VIBRANT
            .into_iter()
            .chain(STANDARD)
            .find_map(|(name, candidate)| (candidate == color).then_some(name))
    }

    pub fn widget(
        weak_bg_fill: Color32,
        bg_stroke: Color32,
        fg: Color32,
    ) -> egui::style::WidgetVisuals {
        egui::style::WidgetVisuals {
            weak_bg_fill,
            bg_fill: weak_bg_fill,
            bg_stroke: Stroke::new(1.0, bg_stroke),
            fg_stroke: Stroke::new(1.0, fg),
            corner_radius: CornerRadius::same(6),
            expansion: 0.0,
        }
    }

    pub fn visuals() -> egui::Visuals {
        let mut visuals = egui::Visuals::dark();
        visuals.override_text_color = Some(TEXT);
        visuals.weak_text_color = Some(TEXT_HINT);
        visuals.weak_text_alpha = 1.0;
        visuals.panel_fill = PANEL;
        visuals.window_fill = PANEL;
        visuals.window_stroke = Stroke::new(1.0, BORDER);
        visuals.extreme_bg_color = FIELD_BG;
        visuals.text_edit_bg_color = Some(FIELD_BG);
        visuals.selection.bg_fill = ACCENT;
        visuals.selection.stroke = Stroke::new(1.0, ACCENT_INK);
        // Disabled toolbar actions must stay legible instead of vanishing.
        visuals.disabled_alpha = 0.72;
        visuals.widgets.noninteractive = widget(PANEL, BORDER, TEXT_MUTED);
        visuals.widgets.inactive = widget(BUTTON_BG, BORDER, TEXT);
        visuals.widgets.hovered = widget(BUTTON_HOVER, TEXT_MUTED, TEXT);
        visuals.widgets.active = widget(BUTTON_ACTIVE, ACCENT, TEXT);
        visuals.widgets.open = widget(BUTTON_HOVER, TEXT_MUTED, TEXT);
        visuals
    }

    #[cfg(test)]
    pub fn contrast_ratio(a: Color32, b: Color32) -> f32 {
        fn linear(channel: u8) -> f32 {
            let channel = channel as f32 / 255.0;
            if channel <= 0.04045 {
                channel / 12.92
            } else {
                ((channel + 0.055) / 1.055).powf(2.4)
            }
        }
        fn luminance(color: Color32) -> f32 {
            0.2126 * linear(color.r()) + 0.7152 * linear(color.g()) + 0.0722 * linear(color.b())
        }
        let (a, b) = (luminance(a), luminance(b));
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }
}

/// Chrome for the Settings window: a quiet sidebar beside a content column,
/// with softly raised cards instead of nested popup surfaces.
mod settings_theme {
    use eframe::egui::{self, Color32, CornerRadius, Stroke};

    pub const BODY: Color32 = Color32::from_rgb(17, 17, 19);
    pub const SIDEBAR: Color32 = Color32::from_rgb(11, 11, 12);
    pub const CARD: Color32 = Color32::from_rgb(28, 28, 30);
    pub const BORDER: Color32 = Color32::from_rgb(44, 44, 46);
    pub const TEXT: Color32 = Color32::from_rgb(242, 242, 247);
    pub const MUTED: Color32 = Color32::from_rgb(152, 152, 159);
    pub const FAINT: Color32 = Color32::from_rgb(108, 108, 115);
    pub const SELECTED: Color32 = Color32::from_rgb(43, 43, 46);
    pub const HOVER: Color32 = Color32::from_rgb(28, 28, 30);
    pub const FIELD: Color32 = Color32::from_rgb(22, 22, 24);
    pub const ACCENT: Color32 = Color32::from_rgb(190, 207, 175);
    pub const ACCENT_INK: Color32 = Color32::from_rgb(28, 35, 23);
    pub const SIDEBAR_WIDTH: f32 = 200.;

    pub fn apply(ui: &mut egui::Ui) {
        let style = ui.style_mut();
        style.spacing.item_spacing = egui::vec2(8., 8.);
        style.spacing.button_padding = egui::vec2(13., 7.);
        style.visuals.override_text_color = Some(TEXT);
        style.visuals.weak_text_color = Some(MUTED);
        style.visuals.panel_fill = BODY;
        style.visuals.window_fill = BODY;
        style.visuals.extreme_bg_color = FIELD;
        style.visuals.text_edit_bg_color = Some(FIELD);
        style.visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(38, 38, 41);
        style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(38, 38, 41);
        style.visuals.widgets.inactive.bg_stroke = Stroke::new(1., BORDER);
        style.visuals.widgets.inactive.fg_stroke = Stroke::new(1., TEXT);
        style.visuals.widgets.inactive.corner_radius = CornerRadius::same(7);
        style.visuals.widgets.hovered.weak_bg_fill = SELECTED;
        style.visuals.widgets.hovered.bg_stroke = Stroke::new(1., MUTED);
        style.visuals.widgets.hovered.fg_stroke = Stroke::new(1., TEXT);
        style.visuals.widgets.hovered.corner_radius = CornerRadius::same(7);
        style.visuals.widgets.active.weak_bg_fill = Color32::from_rgb(52, 52, 56);
        style.visuals.widgets.active.bg_stroke = Stroke::new(1., TEXT);
        style.visuals.widgets.active.corner_radius = CornerRadius::same(7);
        style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(1., BORDER);
        style.visuals.selection.bg_fill = ACCENT;
        style.visuals.selection.stroke = Stroke::new(1., ACCENT_INK);
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Page {
    Popup,
    Settings,
    Recent,
}
#[derive(Clone, Copy, PartialEq)]
enum SettingsTab {
    Settings,
    Appearance,
    Privacy,
}
impl SettingsTab {
    const ALL: [Self; 3] = [Self::Settings, Self::Appearance, Self::Privacy];
    fn label(self) -> &'static str {
        match self {
            Self::Settings => "Settings",
            Self::Appearance => "Appearance",
            Self::Privacy => "Privacy",
        }
    }
    fn icon(self) -> NavIcon {
        match self {
            Self::Settings => NavIcon::Sliders,
            Self::Appearance => NavIcon::Palette,
            Self::Privacy => NavIcon::Shield,
        }
    }
}
#[derive(Clone, Copy)]
enum NavIcon {
    Sliders,
    Palette,
    Shield,
}
#[derive(Clone, Copy, PartialEq)]
enum WindowAction {
    None,
    Close,
    Minimize,
    Zoom,
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
    tool: Tool,
    marks: Vec<Mark>,
    redo_marks: Vec<Mark>,
    message_id: Option<egui::Id>,
    active_mark: Option<Mark>,
    popup_height: f32,
    themed: Option<Page>,
    annotation_color: egui::Color32,
    message_color: egui::Color32,
    settings_tab: SettingsTab,
}
fn post(tx: &Sender<Event>, ctx: &egui::Context, event: Event) {
    let _ = tx.send(event);
    ctx.request_repaint();
}

// Grow the annotation surface with the capture, leaving room for context and actions.
fn preview_bounds(anchor: Option<CursorAnchor>) -> egui::Vec2 {
    let display = anchor.map_or(egui::vec2(1440., 900.), |a| {
        egui::vec2(a.width as f32, a.height as f32)
    });
    egui::vec2(
        (display.x * 0.75).min(960.).min(display.x - 16.).max(1.),
        (display.y * 0.60).min(720.).min(display.y - 230.).max(1.),
    )
}

// Preview ends at the release cursor; the focused message begins directly below it.
// Clamp the complete popup to that display so the input remains reachable at edges.
fn popup_geometry(
    anchor: Option<CursorAnchor>,
    image: egui::Vec2,
) -> (egui::Vec2, Option<egui::Pos2>) {
    let bounds = preview_bounds(anchor);
    let scale = ((bounds.x - 8.) / image.x.max(1.))
        .min(bounds.y / image.y.max(1.))
        .min(1.);
    let preview = image * scale;
    let width = (preview.x + 8.).max(340.).min(bounds.x);
    let size = egui::vec2(width, preview.y + 146.);
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
    let is_settings = page == Page::Settings;
    let code = Arc::new(AtomicI32::new(0));
    let returned_code = code.clone();
    let mut native = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_visible(!is_popup)
            .with_title(if is_settings {
                "SnapNote Settings"
            } else {
                "SnapNote"
            })
            .with_inner_size(if is_popup {
                [340., 300.]
            } else if is_settings {
                [900., 660.]
            } else {
                [500., 380.]
            })
            .with_min_inner_size(if is_settings {
                [640., 480.]
            } else {
                [240., 150.]
            })
            .with_decorations(!is_popup && !is_settings)
            .with_transparent(is_popup || is_settings)
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
            if is_settings {
                cc.egui_ctx
                    .send_viewport_cmd(egui::ViewportCommand::SetTheme(egui::SystemTheme::Dark));
            }
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
            let annotation_color =
                theme::parse_color(&config.ui.annotation_color).unwrap_or(theme::DEFAULT_BOX);
            let message_color =
                theme::parse_color(&config.ui.message_color).unwrap_or(theme::DEFAULT_BOX);
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
                tool: Tool::Move,
                marks: vec![],
                redo_marks: vec![],
                message_id: None,
                active_mark: None,
                popup_height: 0.,
                themed: None,
                annotation_color,
                message_color,
                settings_tab: SettingsTab::Settings,
            }))
        }),
    )
    .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    Ok(returned_code.load(Ordering::SeqCst))
}
impl Popup {
    fn preview_ui(&mut self, ui: &mut egui::Ui) -> f32 {
        let Some(texture) = self.texture.clone() else {
            return 0.;
        };
        let response = ui
            .vertical_centered(|ui| {
                ui.add(
                    egui::Image::new(&texture)
                        .max_size(egui::vec2(
                            ui.available_width() - 4.,
                            preview_bounds(self.anchor).y,
                        ))
                        .maintain_aspect_ratio(true)
                        .corner_radius(8)
                        .sense(egui::Sense::drag()),
                )
            })
            .inner;
        let rect = response.rect;
        ui.add_space(4.);
        let (toolbar_rect, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 36.), egui::Sense::hover());
        let ctx = ui.ctx().clone();
        let pointer = ctx.input(|i| i.pointer.interact_pos());
        let editable = !self.saving && !self.copying && !self.loading;
        if editable {
            let mut toolbar = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(toolbar_rect)
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
            );
            let fill = self.annotation_color;
            let border = theme::box_border(fill);
            egui::Frame::new()
                .fill(fill)
                .stroke(egui::Stroke::new(1., border))
                .corner_radius(8)
                .inner_margin(4)
                .shadow(egui::epaint::Shadow {
                    offset: [0, 2],
                    blur: 8,
                    spread: 0,
                    color: egui::Color32::from_black_alpha(110),
                })
                .show(&mut toolbar, |ui| {
                    let visuals = ui.visuals_mut();
                    visuals.override_text_color = Some(theme::TEXT);
                    visuals.weak_text_color = Some(theme::box_hint(fill));
                    visuals.widgets.noninteractive =
                        theme::widget(theme::button_fill(fill), border, theme::TEXT);
                    visuals.widgets.inactive =
                        theme::widget(theme::button_fill(fill), border, theme::TEXT);
                    visuals.widgets.hovered =
                        theme::widget(theme::button_hover(fill), theme::TEXT, theme::TEXT);
                    visuals.widgets.active =
                        theme::widget(theme::button_active(fill), theme::TEXT, theme::TEXT);
                    visuals.widgets.open =
                        theme::widget(theme::button_hover(fill), theme::TEXT, theme::TEXT);
                    visuals.selection.bg_fill = theme::button_hover(fill);
                    visuals.selection.stroke = egui::Stroke::new(1., theme::TEXT);

                    ui.spacing_mut().item_spacing.x = 4.;
                    ui.spacing_mut().button_padding = egui::vec2(7., 3.);
                    for (tool, label) in [
                        (Tool::Move, "Move"),
                        (Tool::Pen, "Pen"),
                        (Tool::Arrow, "Arrow"),
                        (Tool::Rectangle, "Box"),
                    ] {
                        if ui.selectable_label(self.tool == tool, label).clicked() {
                            self.tool = tool;
                        }
                    }
                    if ui
                        .add_enabled(!self.marks.is_empty(), egui::Button::new("Undo"))
                        .clicked()
                    {
                        self.undo_annotation();
                    }
                    if ui
                        .add_enabled(!self.redo_marks.is_empty(), egui::Button::new("Redo"))
                        .clicked()
                    {
                        self.redo_annotation();
                    }
                    if settings_gear_button(ui).clicked() {
                        self.open_settings(ui.ctx());
                    }
                });
        }
        if editable
            && ctx.input(|i| i.pointer.primary_pressed())
            && pointer.is_some_and(|p| rect.contains(p))
            && let Some(id) = self.message_id
        {
            ctx.memory_mut(|m| m.surrender_focus(id));
        }
        if editable
            && self.tool == Tool::Move
            && ctx.input(|i| {
                i.events
                    .iter()
                    .any(|event| screenshot_drag_started(rect, event))
            })
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }
        if editable && self.tool != Tool::Move {
            // Process ordered events, including drags that press and release in one frame.
            for event in ctx.input(|i| i.events.clone()) {
                if let Some(mark) =
                    drawing_event(&mut self.active_mark, self.tool, rect, toolbar_rect, &event)
                {
                    self.marks.push(mark);
                    self.redo_marks.clear();
                }
            }
        }
        let painter = ui.painter().with_clip_rect(rect);
        for mark in self.marks.iter().chain(self.active_mark.iter()) {
            for (a, b) in mark.segments() {
                painter.line_segment(
                    [
                        rect.min + egui::vec2(a[0] * rect.width(), a[1] * rect.height()),
                        rect.min + egui::vec2(b[0] * rect.width(), b[1] * rect.height()),
                    ],
                    egui::Stroke::new(2.5, egui::Color32::from_rgb(255, 80, 80)),
                );
            }
        }
        rect.height() + 40.
    }
    fn undo_annotation(&mut self) {
        if let Some(mark) = self.marks.pop() {
            self.redo_marks.push(mark);
        }
    }
    fn redo_annotation(&mut self) {
        if let Some(mark) = self.redo_marks.pop() {
            self.marks.push(mark);
        }
    }
    /// Reload the popup surfaces from the saved configuration.
    fn sync_box_colors(&mut self) {
        self.annotation_color =
            theme::parse_color(&self.config.ui.annotation_color).unwrap_or(theme::DEFAULT_BOX);
        self.message_color =
            theme::parse_color(&self.config.ui.message_color).unwrap_or(theme::DEFAULT_BOX);
    }
    /// Open the shared Settings page from inside the capture popup.
    fn open_settings(&mut self, ctx: &egui::Context) {
        self.page = Page::Settings;
        self.preferences = self.config.clone();
        self.directory = self.config.storage.directory.to_string_lossy().into_owned();
        self.show_settings(ctx);
    }
    fn discard(&mut self, ctx: &egui::Context) {
        // The draft owns only this unsaved capture; dropping it removes the PNG
        // and any private annotation backup without publishing metadata.
        self.draft = None;
        self.source = None;
        self.texture = None;
        self.close(ctx);
    }
    fn close(&mut self, ctx: &egui::Context) {
        self.quitting = true;
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
    fn show(&self, ctx: &egui::Context) {
        if self.quitting {
            return;
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    }
    fn show_settings(&self, ctx: &egui::Context) {
        ctx.send_viewport_cmd(egui::ViewportCommand::SetTheme(egui::SystemTheme::Dark));
        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
            egui::WindowLevel::Normal,
        ));
        ctx.send_viewport_cmd(egui::ViewportCommand::Title("SnapNote Settings".into()));
        ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(false));
        let size = egui::vec2(900., 660.);
        ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(egui::vec2(640., 480.)));
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
        // Opening from a capture near a screen edge should not push the larger
        // settings window off that display.
        if let Some(anchor) = self.anchor {
            let left = anchor.left as f32;
            let top = anchor.top as f32;
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(
                (left + (anchor.width as f32 - size.x) / 2.).max(left + 8.),
                (top + (anchor.height as f32 - size.y) / 2.).max(top + 8.),
            )));
        }
        self.show(ctx);
    }
    fn place(&mut self, ctx: &egui::Context) {
        let (size, position) = popup_geometry(self.anchor, self.preview_size);
        ctx.send_viewport_cmd(egui::ViewportCommand::Title("SnapNote".into()));
        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
            egui::WindowLevel::AlwaysOnTop,
        ));
        ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(egui::vec2(240., 150.)));
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
        let marks = self.marks.clone();
        let tx = self.tx.clone();
        let wake = ctx.clone();
        std::thread::spawn(move || {
            let started = std::time::Instant::now();
            let result = draft
                .render_annotations(&marks)
                .and_then(|_| draft.commit(&message));
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
        let mut draft = self.draft.take().unwrap();
        let marks = self.marks.clone();
        std::thread::spawn(move || {
            let result = draft
                .render_annotations(&marks)
                .and_then(|_| clipboard::copy_text(text));
            post(&tx, &wake, Event::Copied { draft, result });
        });
    }
    fn settings_ui(&mut self, ui: &mut egui::Ui) {
        settings_theme::apply(ui);
        ui.spacing_mut().item_spacing = egui::vec2(0., 0.);
        ui.horizontal_top(|ui| {
            settings_sidebar(ui, &mut self.settings_tab);
            egui::Frame::new()
                .fill(settings_theme::BODY)
                .inner_margin(egui::Margin::symmetric(26, 20))
                .show(ui, |ui| {
                    ui.vertical(|ui| self.settings_body(ui));
                });
        });
        match settings_titlebar(ui) {
            WindowAction::None => {}
            WindowAction::Close => self.finish(ui.ctx()),
            WindowAction::Minimize => {
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::Minimized(true));
            }
            WindowAction::Zoom => {
                let maximized = ui
                    .ctx()
                    .input(|input| input.viewport().maximized.unwrap_or(false));
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
            }
        }
        // Borderless windows have no shadow; a hairline keeps the edge defined.
        ui.painter().rect_stroke(
            ui.max_rect(),
            0,
            egui::Stroke::new(1., settings_theme::BORDER),
            egui::StrokeKind::Inside,
        );
    }
    fn settings_body(&mut self, ui: &mut egui::Ui) {
        ui.set_min_height(ui.available_height());
        ui.label(
            egui::RichText::new(self.settings_tab.label())
                .size(26.)
                .strong(),
        );
        ui.add_space(10.);
        ui.separator();
        ui.add_space(14.);
        let footer = 46.;
        let scroll_height = (ui.available_height() - footer).max(90.);
        let tab = self.settings_tab;
        egui::ScrollArea::vertical()
            .id_salt(("settings_scroll", tab.label()))
            .max_height(scroll_height)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                match tab {
                    SettingsTab::Settings => self.settings_general(ui),
                    SettingsTab::Appearance => self.settings_appearance(ui),
                    SettingsTab::Privacy => self.settings_privacy(ui),
                }
            });
        if !self.status.is_empty() {
            ui.add_space(6.);
            let color = if self.status.starts_with("Saved.") {
                settings_theme::ACCENT
            } else {
                theme::DANGER
            };
            ui.label(egui::RichText::new(&self.status).size(12.).color(color));
        }
        ui.add_space(6.);
        ui.separator();
        ui.add_space(8.);
        ui.horizontal(|ui| {
            if self.draft.is_some() && ui.button("Back to capture").clicked() {
                self.page = Page::Popup;
                self.place(ui.ctx());
                self.focus = true;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let save = egui::Button::new(
                    egui::RichText::new("Save changes").color(settings_theme::ACCENT_INK),
                )
                .fill(settings_theme::ACCENT)
                .corner_radius(7);
                if ui.add(save).clicked() {
                    self.save_settings(ui.ctx());
                }
            });
        });
    }
    fn settings_general(&mut self, ui: &mut egui::Ui) {
        settings_section(ui, "Capture");
        settings_card(ui, |ui| {
            settings_row(
                ui,
                "Capture shortcut",
                "Start a capture, or save and close the open one.",
                188.,
                |ui| {
                    ui.add_sized(
                        [188., 32.],
                        egui::TextEdit::singleline(&mut self.preferences.capture.hotkey)
                            .hint_text("Command+B")
                            .margin(egui::vec2(10., 8.)),
                    );
                },
            );
            settings_divider(ui);
            settings_row(
                ui,
                "Screenshot folder",
                "Images, notes, and metadata are saved together here.",
                260.,
                |ui| {
                    ui.add_sized(
                        [260., 32.],
                        egui::TextEdit::singleline(&mut self.directory).margin(egui::vec2(10., 8.)),
                    );
                },
            );
        });
        ui.add_space(10.);
        settings_hint(ui, "Changes are written to config.toml when you save.");
    }
    fn settings_appearance(&mut self, ui: &mut egui::Ui) {
        let mut annotation =
            theme::parse_color(&self.preferences.ui.annotation_color).unwrap_or(theme::DEFAULT_BOX);
        let mut message =
            theme::parse_color(&self.preferences.ui.message_color).unwrap_or(theme::DEFAULT_BOX);
        settings_section(ui, "Box colors");
        settings_card(ui, |ui| {
            settings_row(
                ui,
                "Annotation toolbar",
                "Background of the drawing tools.",
                204.,
                |ui| settings_color_control(ui, &mut annotation),
            );
            settings_divider(ui);
            settings_row(
                ui,
                "Message box",
                "Background of the context field.",
                204.,
                |ui| settings_color_control(ui, &mut message),
            );
        });
        let annotation = theme::color_hex(annotation);
        if annotation != self.preferences.ui.annotation_color {
            self.preferences.ui.annotation_color = annotation;
        }
        let message = theme::color_hex(message);
        if message != self.preferences.ui.message_color {
            self.preferences.ui.message_color = message;
        }
        ui.add_space(10.);
        settings_hint(
            ui,
            "Borders, buttons, and hint text adjust automatically to keep every color readable.",
        );
    }
    fn settings_privacy(&mut self, ui: &mut egui::Ui) {
        settings_section(ui, "Permissions");
        settings_card(ui, |ui| {
            settings_row(
                ui,
                "Screen Recording",
                "Lets SnapNote capture the part of the screen you select.",
                132.,
                |ui| {
                    let enabled = self.draft.is_none() && self.source.is_none() && !self.loading;
                    if ui
                        .add_enabled(enabled, egui::Button::new("Allow access"))
                        .clicked()
                    {
                        self.exit_code.store(11, Ordering::SeqCst);
                        self.close(ui.ctx());
                    }
                },
            );
            settings_divider(ui);
            settings_row_labels(
                ui,
                "Where to grant it",
                "System Settings → Privacy & Security → Screen & System Audio Recording. Relaunch if macOS asks.",
            );
            settings_divider(ui);
            settings_row_labels(
                ui,
                "Which app",
                "Grant access to SnapNote, not the SnapNote Editor helper.",
            );
        });
    }
    fn save_settings(&mut self, ctx: &egui::Context) {
        self.preferences.storage.directory = PathBuf::from(&self.directory);
        self.status = match self.preferences.save(&self.config_path) {
            Ok(()) => {
                self.config = self.preferences.clone();
                self.sync_box_colors();
                "Saved. Shortcut changes apply when Settings closes.".into()
            }
            Err(error) => format!("Settings not saved: {error:#}"),
        };
        if let Some(capture) = self.source.take() {
            self.prepare(ctx, capture);
        }
    }
}
impl eframe::App for Popup {
    fn clear_color(&self, _: &egui::Visuals) -> [f32; 4] {
        [0., 0., 0., 0.]
    }
    fn logic(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        if self.themed != Some(self.page) {
            self.themed = Some(self.page);
            ctx.set_visuals(if self.page == Page::Popup {
                theme::visuals()
            } else {
                egui::Visuals::dark()
            });
        }
        // Closing is terminal: queued IPC/worker completions must never show us again.
        if self.quitting {
            return;
        }
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
            if self.quitting {
                break;
            }
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
                        self.show_settings(ctx);
                    }
                },
                Event::Prepared { result, capture } => {
                    self.loading = false;
                    self.source = capture;
                    match result {
                        Ok(draft) => {
                            let prefix = format!(
                                "{}\nView this screenshot.\n",
                                draft.image_path().display()
                            );
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
                Event::Copied { draft, result } => {
                    self.draft = Some(draft);
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
                Event::Settings => self.open_settings(ctx),
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
            && !self.copying
            && ctx.input(|i| i.pointer.secondary_pressed())
        {
            // Once captured, closing always preserves the image and message.
            self.finish(&ctx);
            return;
        }
        let mut copy_message = false;
        let mut finish_message = false;
        let mut discard_message = false;
        if self.page == Page::Popup {
            let editing_text = self
                .message_id
                .is_some_and(|id| ctx.memory(|m| m.has_focus(id)));
            if !editing_text && !self.saving && !self.copying {
                let redo = ctx.input_mut(|i| {
                    i.consume_key(
                        egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                        egui::Key::Z,
                    )
                });
                let undo = ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Z));
                if redo {
                    self.redo_annotation();
                } else if undo {
                    self.undo_annotation();
                }
            }
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
        } else if self.page == Page::Settings {
            egui::Frame::new().fill(settings_theme::BODY)
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
                    ui.scope(|ui| {
                        let preview_height = self.preview_ui(ui);
                        ui.add_space(6.);
                        let message_fill = self.message_color;
                        let message_box = egui::Frame::new()
                            .fill(message_fill)
                            .stroke(egui::Stroke::new(1., theme::box_border(message_fill)))
                            .corner_radius(8)
                            .inner_margin(egui::Margin::symmetric(9, 8))
                            .shadow(egui::epaint::Shadow {
                                offset: [0, 2],
                                blur: 8,
                                spread: 0,
                                color: egui::Color32::from_black_alpha(110),
                            })
                            .show(ui, |ui| {
                                let visuals = ui.visuals_mut();
                                visuals.override_text_color = Some(theme::TEXT);
                                visuals.weak_text_color = Some(theme::box_hint(message_fill));
                                visuals.selection.bg_fill = theme::button_hover(message_fill);
                                visuals.selection.stroke = egui::Stroke::new(1., theme::TEXT);
                                egui::ScrollArea::vertical()
                                    .id_salt("message_scroll")
                                    .max_height(144.)
                                    .show(ui, |ui| {
                                        ui.add_enabled_ui(!self.loading && !self.saving, |ui| {
                                            let mut output =
                                                egui::TextEdit::multiline(&mut self.message)
                                                    .id_salt("message")
                                                    .frame(egui::Frame::NONE)
                                                    .text_color(theme::TEXT)
                                                    .desired_width(f32::INFINITY)
                                                    .desired_rows(1)
                                                    .cursor_at_end(true)
                                                    .hint_text("Add context…")
                                                    .show(ui);
                                            self.message_id = Some(output.response.id);
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
                                    });
                                if !self.status.is_empty() {
                                    ui.add_space(4.);
                                    ui.small(&self.status);
                                }
                            });
                        // Actions float below the message area rather than living inside it.
                        ui.add_space(6.);
                        ui.allocate_ui_with_layout(
                            egui::vec2(ui.available_width(), 30.),
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                ui.add_enabled_ui(
                                    !self.loading && !self.saving && !self.copying,
                                    |ui| {
                                        discard_message = popup_action(ui, false).clicked();
                                        finish_message |= popup_action(ui, true).clicked();
                                    },
                                );
                            },
                        );
                        let height = preview_height
                            + 6.
                            + message_box.response.rect.height()
                            + 6.
                            + 30.
                            + 24.;
                        if (height - self.popup_height).abs() > 1. {
                            self.popup_height = height;
                            let width = popup_geometry(self.anchor, self.preview_size).0.x;
                            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                                width, height,
                            )));
                            if let (Some(anchor), Some(rect)) =
                                (self.anchor, ctx.input(|i| i.viewport().outer_rect))
                            {
                                let bottom = anchor.top as f32 + anchor.height as f32 - 8.;
                                if rect.top() + height > bottom {
                                    ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(
                                        egui::pos2(rect.left(), bottom - height),
                                    ));
                                }
                            }
                        }
                    });
                }
                Page::Settings => self.settings_ui(ui),
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
        if discard_message {
            self.discard(&ctx);
            return;
        }
        if copy_message {
            self.copy(&ctx);
        }
        if finish_message {
            self.finish(&ctx);
        }
    }
}
fn settings_titlebar(ui: &mut egui::Ui) -> WindowAction {
    let ctx = ui.ctx().clone();
    let area = ui.max_rect();
    let first = area.min + egui::vec2(20., 22.);
    let lights = [
        (
            egui::Rect::from_center_size(first, egui::vec2(12., 12.)),
            egui::Color32::from_rgb(255, 95, 87),
        ),
        (
            egui::Rect::from_center_size(first + egui::vec2(20., 0.), egui::vec2(12., 12.)),
            egui::Color32::from_rgb(254, 188, 46),
        ),
        (
            egui::Rect::from_center_size(first + egui::vec2(40., 0.), egui::vec2(12., 12.)),
            egui::Color32::from_rgb(40, 200, 64),
        ),
    ];
    let title = egui::Rect::from_min_size(area.min, egui::vec2(area.width(), 44.));
    if ctx.input(|input| input.pointer.primary_pressed())
        && let Some(pos) = ctx.input(|input| input.pointer.interact_pos())
        && title.contains(pos)
        && !lights.iter().any(|(rect, _)| rect.expand(3.).contains(pos))
    {
        ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }
    let mut action = WindowAction::None;
    for (index, (rect, color)) in lights.into_iter().enumerate() {
        let response = ui.interact(
            rect.expand(2.),
            ui.make_persistent_id(("settings_window_light", index)),
            egui::Sense::click(),
        );
        ui.painter().circle_filled(rect.center(), 6., color);
        if response.hovered() {
            let ink = egui::Color32::from_rgb(74, 56, 20);
            let center = rect.center();
            let stroke = egui::Stroke::new(1.3, ink);
            match index {
                0 => {
                    ui.painter().line_segment(
                        [
                            center + egui::vec2(-2.2, -2.2),
                            center + egui::vec2(2.2, 2.2),
                        ],
                        stroke,
                    );
                    ui.painter().line_segment(
                        [
                            center + egui::vec2(2.2, -2.2),
                            center + egui::vec2(-2.2, 2.2),
                        ],
                        stroke,
                    );
                }
                1 => {
                    ui.painter().line_segment(
                        [center + egui::vec2(-2.4, 0.), center + egui::vec2(2.4, 0.)],
                        stroke,
                    );
                }
                _ => {
                    ui.painter().line_segment(
                        [center + egui::vec2(-2.4, 0.), center + egui::vec2(2.4, 0.)],
                        stroke,
                    );
                    ui.painter().line_segment(
                        [center + egui::vec2(0., -2.4), center + egui::vec2(0., 2.4)],
                        stroke,
                    );
                }
            }
        }
        if response.clicked() {
            action = match index {
                0 => WindowAction::Close,
                1 => WindowAction::Minimize,
                _ => WindowAction::Zoom,
            };
        }
        let _ = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    }
    action
}

fn settings_sidebar(ui: &mut egui::Ui, selected: &mut SettingsTab) {
    egui::Frame::new()
        .fill(settings_theme::SIDEBAR)
        .inner_margin(egui::Margin {
            left: 12,
            right: 12,
            top: 46,
            bottom: 14,
        })
        .show(ui, |ui| {
            ui.set_width(settings_theme::SIDEBAR_WIDTH - 24.);
            ui.set_min_height(ui.available_height());
            ui.vertical(|ui| {
                for tab in SettingsTab::ALL {
                    if settings_nav_item(ui, *selected == tab, tab).clicked() {
                        *selected = tab;
                    }
                    ui.add_space(2.);
                }
                let remaining = ui.available_height() - 20.;
                if remaining > 0. {
                    ui.add_space(remaining);
                }
                ui.label(
                    egui::RichText::new(concat!("Version ", env!("CARGO_PKG_VERSION")))
                        .size(10.5)
                        .color(settings_theme::FAINT),
                );
            });
        });
}

fn settings_nav_item(ui: &mut egui::Ui, selected: bool, tab: SettingsTab) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 36.), egui::Sense::click());
    if selected || response.hovered() {
        ui.painter().rect_filled(
            rect,
            8,
            if selected {
                settings_theme::SELECTED
            } else {
                settings_theme::HOVER
            },
        );
    }
    let icon_rect = egui::Rect::from_center_size(
        egui::pos2(rect.left() + 18., rect.center().y),
        egui::vec2(16., 16.),
    );
    let color = if selected {
        settings_theme::TEXT
    } else {
        settings_theme::MUTED
    };
    paint_nav_icon(ui.painter(), icon_rect, color, tab.icon());
    ui.painter().text(
        egui::pos2(rect.left() + 36., rect.center().y),
        egui::Align2::LEFT_CENTER,
        tab.label(),
        egui::FontId::proportional(13.5),
        color,
    );
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn paint_nav_icon(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32, icon: NavIcon) {
    let center = rect.center();
    match icon {
        NavIcon::Sliders => {
            let rows = [rect.top() + 3.5, rect.center().y, rect.bottom() - 3.5];
            for (index, y) in rows.into_iter().enumerate() {
                painter.line_segment(
                    [
                        egui::pos2(rect.left() + 1., y),
                        egui::pos2(rect.right() - 1., y),
                    ],
                    egui::Stroke::new(1.2, color),
                );
                let x = rect.left() + 4. + index as f32 * 3.5;
                painter.circle_filled(egui::pos2(x, y), 1.9, color);
            }
        }
        NavIcon::Palette => {
            painter.circle_stroke(center, 6.5, egui::Stroke::new(1.2, color));
            for (dx, dy) in [(-2.5, -1.5), (2., -2.), (2.5, 1.5), (-1., 2.)] {
                painter.circle_filled(center + egui::vec2(dx, dy), 1.25, color);
            }
        }
        NavIcon::Shield => {
            painter.add(egui::Shape::line(
                vec![
                    egui::pos2(center.x, rect.top() + 1.),
                    egui::pos2(rect.right() - 2., rect.top() + 4.5),
                    egui::pos2(rect.right() - 3., rect.center().y + 4.),
                    egui::pos2(center.x, rect.bottom() - 1.),
                    egui::pos2(rect.left() + 3., rect.center().y + 4.),
                    egui::pos2(rect.left() + 2., rect.top() + 4.5),
                    egui::pos2(center.x, rect.top() + 1.),
                ],
                egui::Stroke::new(1.2, color),
            ));
            painter.add(egui::Shape::line(
                vec![
                    center + egui::vec2(-3., 0.),
                    center + egui::vec2(-0.8, 2.4),
                    center + egui::vec2(3.4, -2.4),
                ],
                egui::Stroke::new(1.3, color),
            ));
        }
    }
}

fn settings_gear_button(ui: &mut egui::Ui) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(28., 28.), egui::Sense::click());
    let visuals = ui.style().interact(&response);
    ui.painter().rect(
        rect,
        6,
        visuals.weak_bg_fill,
        visuals.bg_stroke,
        egui::StrokeKind::Inside,
    );
    let center = rect.center();
    let color = visuals.fg_stroke.color;
    for tooth in 0..8 {
        let angle = std::f32::consts::TAU * tooth as f32 / 8.;
        let direction = egui::vec2(angle.cos(), angle.sin());
        ui.painter().line_segment(
            [center + direction * 5.1, center + direction * 7.3],
            egui::Stroke::new(2.1, color),
        );
    }
    ui.painter()
        .circle_stroke(center, 4.6, egui::Stroke::new(2.0, color));
    ui.painter()
        .circle_filled(center, 1.8, visuals.weak_bg_fill);
    response.on_hover_text("Settings")
}

fn settings_section(ui: &mut egui::Ui, title: &str) {
    ui.label(
        egui::RichText::new(title.to_uppercase())
            .size(10.5)
            .color(settings_theme::MUTED),
    );
    ui.add_space(6.);
}

fn settings_card<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::new()
        .fill(settings_theme::CARD)
        .stroke(egui::Stroke::new(1., settings_theme::BORDER))
        .corner_radius(10)
        .inner_margin(egui::Margin::symmetric(16, 14))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner
}

fn settings_row_labels(ui: &mut egui::Ui, title: &str, description: &str) {
    ui.label(egui::RichText::new(title).size(14.).strong());
    ui.add_space(2.);
    ui.label(
        egui::RichText::new(description)
            .size(12.)
            .color(settings_theme::MUTED),
    );
}

fn settings_row<R>(
    ui: &mut egui::Ui,
    title: &str,
    description: &str,
    control_width: f32,
    add_control: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let available = ui.available_width();
    if available < 400. || available < control_width + 180. {
        settings_row_labels(ui, title, description);
        ui.add_space(8.);
        return add_control(ui);
    }
    let spacing = ui.spacing().item_spacing.x;
    let label_width = (available - control_width - spacing).max(140.);
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(
            egui::vec2(label_width, 0.),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.set_min_width(label_width);
                ui.set_max_width(label_width);
                settings_row_labels(ui, title, description);
            },
        );
        ui.allocate_ui_with_layout(
            egui::vec2(control_width, 0.),
            egui::Layout::right_to_left(egui::Align::Center),
            add_control,
        )
        .inner
    })
    .inner
}

fn settings_divider(ui: &mut egui::Ui) {
    ui.add_space(4.);
    ui.separator();
    ui.add_space(2.);
}

fn settings_hint(ui: &mut egui::Ui, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .size(11.5)
            .color(settings_theme::FAINT),
    );
}

fn settings_color_control(ui: &mut egui::Ui, current: &mut egui::Color32) {
    ui.vertical(|ui| {
        ui.set_min_width(196.);
        ui.label(
            egui::RichText::new("Vibrant")
                .size(10.)
                .color(settings_theme::FAINT),
        );
        theme::color_swatches(ui, current, &theme::VIBRANT);
        ui.add_space(3.);
        ui.label(
            egui::RichText::new("Standard")
                .size(10.)
                .color(settings_theme::FAINT),
        );
        theme::color_swatches(ui, current, &theme::STANDARD);
        ui.add_space(4.);
        let name = theme::color_name(*current).unwrap_or("Custom");
        ui.label(
            egui::RichText::new(format!("{name} · {}", theme::color_hex(*current)))
                .size(10.5)
                .color(settings_theme::MUTED),
        );
    });
}

// Use the press position directly: native window dragging must start on the press
// frame, before egui's drag threshold, across the entire screenshot.
fn screenshot_drag_started(rect: egui::Rect, event: &egui::Event) -> bool {
    matches!(event, egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed: true,
        ..
    } if rect.contains(*pos))
}

fn drawing_event(
    active: &mut Option<Mark>,
    tool: Tool,
    rect: egui::Rect,
    toolbar: egui::Rect,
    event: &egui::Event,
) -> Option<Mark> {
    match *event {
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: true,
            ..
        } if rect.contains(pos) && !toolbar.contains(pos) => {
            *active = Some(Mark {
                tool,
                points: vec![normalized_point(rect, pos)],
            });
        }
        egui::Event::PointerMoved(pos) => {
            if let Some(mark) = active {
                extend_mark(mark, normalized_point(rect, pos));
            }
        }
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            ..
        } => {
            if let Some(mut mark) = active.take() {
                extend_mark(&mut mark, normalized_point(rect, pos));
                return Some(mark);
            }
        }
        _ => {}
    }
    None
}
fn normalized_point(rect: egui::Rect, p: egui::Pos2) -> [f32; 2] {
    [
        ((p.x - rect.left()) / rect.width()).clamp(0., 1.),
        ((p.y - rect.top()) / rect.height()).clamp(0., 1.),
    ]
}
fn extend_mark(mark: &mut Mark, point: [f32; 2]) {
    if mark.tool == Tool::Pen || mark.points.len() == 1 {
        mark.points.push(point);
    } else {
        *mark.points.last_mut().unwrap() = point;
    }
}
fn popup_action(ui: &mut egui::Ui, save: bool) -> egui::Response {
    let label = if save {
        "Save and close"
    } else {
        "Close and delete capture"
    };
    let color = if save { theme::SUCCESS } else { theme::DANGER };
    let response = ui.add(
        egui::Button::new("")
            .min_size(egui::vec2(32., 28.))
            .stroke(egui::Stroke::new(1., color)),
    );
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, ui.is_enabled(), label));
    let origin = response.rect.center() - egui::vec2(6., 6.);
    let stroke = egui::Stroke::new(1.7, color);
    if save {
        ui.painter().add(egui::Shape::line(
            vec![
                origin + egui::vec2(1., 6.),
                origin + egui::vec2(5., 10.),
                origin + egui::vec2(11., 2.),
            ],
            stroke,
        ));
    } else {
        ui.painter().line_segment(
            [origin + egui::vec2(2., 2.), origin + egui::vec2(10., 10.)],
            stroke,
        );
        ui.painter().line_segment(
            [origin + egui::vec2(10., 2.), origin + egui::vec2(2., 10.)],
            stroke,
        );
    }
    response.on_hover_text(label)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn move_drag_starts_across_the_entire_screenshot() {
        let rect = egui::Rect::from_min_size(egui::pos2(10., 10.), egui::vec2(900., 500.));
        for pos in [
            rect.left_top(),
            rect.center_top() + egui::vec2(0., 2.),
            rect.center(),
            rect.right_bottom() - egui::vec2(1., 1.),
        ] {
            assert!(screenshot_drag_started(
                rect,
                &egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                }
            ));
        }
        assert!(!screenshot_drag_started(
            rect,
            &egui::Event::PointerButton {
                pos: rect.center_bottom() + egui::vec2(0., 20.),
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            }
        ));
    }

    #[test]
    fn fast_drag_retains_press_and_release_in_one_frame() {
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(300., 200.));
        let toolbar = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(300., 30.));
        for tool in [Tool::Pen, Tool::Arrow, Tool::Rectangle] {
            let mut active = None;
            let mut result = None;
            for event in [
                egui::Event::PointerButton {
                    pos: egui::pos2(60., 60.),
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
                egui::Event::PointerMoved(egui::pos2(150., 100.)),
                egui::Event::PointerButton {
                    pos: egui::pos2(240., 160.),
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                },
            ] {
                result = drawing_event(&mut active, tool, rect, toolbar, &event).or(result);
            }
            let mark = result.unwrap();
            assert_eq!(mark.points.first(), Some(&[0.2, 0.3]));
            assert_eq!(mark.points.last(), Some(&[0.8, 0.8]));
            assert!(active.is_none());
            assert!(!mark.segments().is_empty());
        }
    }
    #[test]
    fn large_captures_get_a_larger_annotation_surface() {
        let small = popup_geometry(None, egui::vec2(300., 180.)).0;
        let large = popup_geometry(None, egui::vec2(2400., 1600.)).0;
        assert_eq!(small.x, 340.);
        assert!(large.x > 800. && large.y > 600.);
        assert!(large.x <= 960. && large.y <= 686.);
        let tall = popup_geometry(None, egui::vec2(500., 3000.)).0;
        assert_eq!(tall.y, 686.);
        assert_eq!(tall.x, 340.);
    }

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

    #[test]
    fn popup_theme_keeps_annotation_and_text_boxes_readable() {
        fn assert_contrast(label: &str, fg: egui::Color32, bg: egui::Color32, floor: f32) {
            let ratio = theme::contrast_ratio(fg, bg);
            assert!(
                ratio >= floor,
                "{label} contrast {ratio:.2}:1 is below the {floor:.1}:1 floor"
            );
        }

        let visuals = theme::visuals();
        let text = visuals.text_color();
        // Every selectable box color must keep its text and controls legible.
        for (name, fill) in theme::VIBRANT.into_iter().chain(theme::STANDARD) {
            assert_contrast(&format!("{name} message text"), text, fill, 4.5);
            assert_contrast(
                &format!("{name} hint text"),
                theme::box_hint(fill),
                fill,
                4.5,
            );
            assert_contrast(
                &format!("{name} box border"),
                theme::box_border(fill),
                fill,
                3.0,
            );
            for (state, button) in [
                ("inactive", theme::button_fill(fill)),
                ("hovered", theme::button_hover(fill)),
                ("active", theme::button_active(fill)),
            ] {
                assert_contrast(&format!("{name} {state} button text"), text, button, 4.5);
                assert_contrast(
                    &format!("{name} {state} button border"),
                    theme::box_border(fill),
                    button,
                    3.0,
                );
            }
            assert_contrast(
                &format!("{name} selected tool"),
                text,
                theme::button_hover(fill),
                4.5,
            );
        }
        assert_contrast("default box text", text, theme::DEFAULT_BOX, 4.5);
        // Uncolored surfaces still need readable text and borders.
        assert_contrast("panel text", text, theme::PANEL, 4.5);
        assert_contrast("panel border", theme::BORDER, theme::PANEL, 3.0);
        assert_contrast("save icon", theme::SUCCESS, theme::BUTTON_BG, 3.0);
        assert_contrast("delete icon", theme::DANGER, theme::BUTTON_BG, 3.0);
    }

    #[test]
    fn box_colors_round_trip_through_hex() {
        for (name, color) in theme::VIBRANT.into_iter().chain(theme::STANDARD) {
            let hex = theme::color_hex(color);
            assert_eq!(
                theme::parse_color(&hex),
                Some(color),
                "{name} did not round-trip through {hex}"
            );
        }
        assert_eq!(theme::parse_color("#3d4fd8"), Some(theme::DEFAULT_BOX));
        assert_eq!(theme::parse_color("nonsense"), None);
        assert_eq!(
            theme::color_hex(theme::DEFAULT_BOX),
            config::DEFAULT_BOX_COLOR
        );
    }

    #[test]
    fn toolbar_controls_fit_the_minimum_popup_width() {
        // popup_geometry keeps the popup at least 340 points wide; the toolbar
        // frame reserves 4 points of inner margin on each side.
        let available = 340. - 8.;
        let ctx = egui::Context::default();
        let mut used = 0.0;
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            ui.set_width(available);
            ui.set_visuals(theme::visuals());
            ui.spacing_mut().item_spacing.x = 4.;
            ui.spacing_mut().button_padding = egui::vec2(7., 3.);
            let left = ui.cursor().left();
            for label in ["Move", "Pen", "Arrow", "Box"] {
                used = ui.selectable_label(false, label).rect.right() - left;
            }
            for label in ["Undo", "Redo"] {
                used = ui.button(label).rect.right() - left;
            }
            used = settings_gear_button(ui).rect.right() - left;
        });
        output.textures_delta.clear();
        assert!(
            used <= available,
            "toolbar controls need {used:.0} points but only {available:.0} are available"
        );
    }
}
