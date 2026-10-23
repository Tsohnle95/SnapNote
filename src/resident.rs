//! Windowless, renderer-free owner of tray, shortcut, capture, and editor lifetime.
use anyhow::{Context, Result};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState, hotkey::HotKey};
use snapnote::{
    capture::{self, CapturedFile},
    config::{self, Config},
};
use std::{
    io::Write,
    path::{Path, PathBuf},
    process::{ChildStdin, Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
};
use tray_icon::{
    TrayIcon, TrayIconBuilder,
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    window::WindowId,
};

enum Event {
    Capture,
    Toggle,
    Captured(Result<Option<CapturedFile>>),
    Settings,
    Recent,
    Folder,
    Quit,
    EditorExited {
        code: Option<i32>,
        cleanup: Option<PathBuf>,
    },
}
#[derive(Default)]
struct ShortcutLatch(AtomicBool);
impl ShortcutLatch {
    fn pressed(&self, state: HotKeyState) -> bool {
        match state {
            HotKeyState::Pressed => !self.0.swap(true, Ordering::Relaxed),
            HotKeyState::Released => {
                self.0.store(false, Ordering::Relaxed);
                false
            }
        }
    }
}
fn send_editor(pipe: &mut ChildStdin, command: &str) -> Result<()> {
    match writeln!(pipe, "{command}").and_then(|_| pipe.flush()) {
        // Exit notification owns cleanup. A closing helper isn't a UI error,
        // and reporting this by opening Settings would resurrect a dismissed UI.
        Err(error) if error.kind() == std::io::ErrorKind::BrokenPipe => Ok(()),
        result => result.map_err(Into::into),
    }
}
struct Resident {
    proxy: EventLoopProxy<Event>,
    tray: Option<TrayIcon>,
    manager: Option<GlobalHotKeyManager>,
    hotkey: Option<HotKey>,
    config_path: PathBuf,
    config: Config,
    editor: Option<ChildStdin>,
    editor_is_capture: bool,
    pending_anchor: Option<capture::CursorAnchor>,
    capturing: bool,
    quitting: bool,
    initialized: bool,
    config_problem: Option<String>,
    capture_completed_ms: Option<u128>,
    _lock: std::fs::File,
    started: std::time::Instant,
}
fn lock(path: &Path) -> Result<std::fs::File> {
    let parent = path
        .parent()
        .context("Configuration needs a parent directory")?;
    std::fs::create_dir_all(parent)?;
    let file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(parent.join("instance.lock"))?;
    #[cfg(unix)]
    {
        use std::os::fd::AsRawFd;
        // flock releases automatically even if the process crashes; the file is never unlinked.
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            anyhow::bail!("SnapNote is already running. Use its menu-bar icon.");
        }
    }
    Ok(file)
}
pub fn run(config_path: PathBuf) -> Result<()> {
    let started = std::time::Instant::now();
    let lock = lock(&config_path)?;
    let mut builder = EventLoop::<Event>::with_user_event();
    #[cfg(target_os = "macos")]
    {
        use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};
        builder
            .with_activation_policy(ActivationPolicy::Accessory)
            .with_default_menu(false);
    }
    let event_loop = builder.build()?;
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = Resident {
        proxy: event_loop.create_proxy(),
        tray: None,
        manager: None,
        hotkey: None,
        config_path,
        config: Config::default(),
        editor: None,
        editor_is_capture: false,
        pending_anchor: None,
        capturing: false,
        quitting: false,
        initialized: false,
        config_problem: None,
        capture_completed_ms: None,
        _lock: lock,
        started,
    };
    event_loop.run_app(&mut app)?;
    Ok(())
}
impl Resident {
    fn setup(&mut self) -> Result<()> {
        let menu = Menu::new();
        let title = MenuItem::new("SnapNote", false, None);
        let capture = MenuItem::new("New Capture", true, None);
        let folder = MenuItem::new("Open Capture Folder", true, None);
        let recent = MenuItem::new("Recent Captures", true, None);
        let settings = MenuItem::new("Settings", true, None);
        let quit = MenuItem::new("Quit", true, None);
        menu.append_items(&[
            &title,
            &PredefinedMenuItem::separator(),
            &capture,
            &folder,
            &recent,
            &settings,
            &PredefinedMenuItem::separator(),
            &quit,
        ])?;
        let ids = [
            capture.id().clone(),
            folder.id().clone(),
            recent.id().clone(),
            settings.id().clone(),
            quit.id().clone(),
        ];
        let proxy = self.proxy.clone();
        MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
            let event = if e.id == ids[0] {
                Event::Capture
            } else if e.id == ids[1] {
                Event::Folder
            } else if e.id == ids[2] {
                Event::Recent
            } else if e.id == ids[3] {
                Event::Settings
            } else if e.id == ids[4] {
                Event::Quit
            } else {
                return;
            };
            let _ = proxy.send_event(event);
        }));
        let mut pixels = vec![0u8; 22 * 22 * 4];
        for y in 4..18 {
            for x in 2..20 {
                if y == 4
                    || y == 17
                    || x == 2
                    || x == 19
                    || ((x - 11i32).pow(2) + (y - 11i32).pow(2) < 16)
                {
                    let i = (y * 22 + x) as usize * 4;
                    pixels[i..i + 4].copy_from_slice(&[0, 0, 0, 255]);
                }
            }
        }
        self.tray = Some(
            TrayIconBuilder::new()
                .with_menu(Box::new(menu))
                .with_tooltip("SnapNote")
                .with_icon_templated(tray_icon::Icon::from_rgba(pixels, 22, 22)?)
                .build()?,
        );
        self.manager = Some(GlobalHotKeyManager::new()?);
        let proxy = self.proxy.clone();
        let latch = ShortcutLatch::default();
        GlobalHotKeyEvent::set_event_handler(Some(move |e: GlobalHotKeyEvent| {
            if latch.pressed(e.state) {
                let _ = proxy.send_event(Event::Toggle);
            }
        }));
        self.reload_config()?;
        Ok(())
    }
    fn reload_config(&mut self) -> Result<()> {
        let mut config = Config::load(&self.config_path)?;
        let next: HotKey = config.capture.hotkey.parse()?;
        if self.hotkey != Some(next) {
            let manager = self
                .manager
                .as_ref()
                .context("Shortcut manager unavailable")?;
            if let Err(error) = manager.register(next) {
                // Preserve the working shortcut and persist it alongside any other settings edits.
                if self.hotkey.is_some() {
                    config.capture.hotkey = self.config.capture.hotkey.clone();
                    config.save(&self.config_path)?;
                    self.config = config;
                }
                anyhow::bail!(
                    "Shortcut is unavailable: {error}. Choose a different shortcut in Settings."
                );
            }
            if let Some(old) = self.hotkey
                && let Err(error) = manager.unregister(old)
            {
                let _ = manager.unregister(next);
                return Err(error.into());
            }
            self.hotkey = Some(next);
        }
        self.config = config;
        Ok(())
    }
    fn editor(&mut self, page: &str, path: Option<PathBuf>, status: Option<String>) -> Result<()> {
        if let Some(pipe) = &mut self.editor {
            return send_editor(pipe, page);
        }
        let current = std::env::current_exe()?;
        let bundle_helper = current
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.join("Helpers/SnapNote Editor.app/Contents/MacOS/SnapNote Editor"));
        let helper = bundle_helper
            .filter(|p| p.is_file())
            .unwrap_or_else(|| current.with_file_name("snapnote-editor"));
        let mut command = Command::new(helper);
        command
            .arg("--config")
            .arg(&self.config_path)
            .arg("--page")
            .arg(page)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit());
        if let Some(path) = &path {
            command.arg("--capture-file").arg(path);
            if let Some(anchor) = self.pending_anchor.take() {
                command.arg("--anchor").arg(serde_json::to_string(&anchor)?);
            }
            if let Some(completed) = self.capture_completed_ms.take() {
                command
                    .arg("--capture-completed-ms")
                    .arg(completed.to_string());
            }
        }
        if let Some(message) = status {
            command.arg("--status").arg(message);
        }
        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(error) => {
                if let Some(path) = &path {
                    let _ = std::fs::remove_file(path);
                }
                return Err(error).context("Launch editor");
            }
        };
        self.editor_is_capture = path.is_some();
        self.editor = child.stdin.take();
        let proxy = self.proxy.clone();
        std::thread::spawn(move || {
            let code = child.wait().ok().and_then(|s| s.code());
            let _ = proxy.send_event(Event::EditorExited {
                code,
                cleanup: path,
            });
        });
        Ok(())
    }
    fn capture(&mut self) -> Result<()> {
        if let Some(pipe) = &mut self.editor {
            return send_editor(pipe, "capture");
        }
        if self.capturing {
            return Ok(());
        }
        self.capturing = true;
        let proxy = self.proxy.clone();
        std::thread::spawn(move || {
            let _ = proxy.send_event(Event::Captured(capture::region().map_err(Into::into)));
        });
        Ok(())
    }
    fn report(&mut self, error: anyhow::Error) {
        let message = format!("{error:#}");
        eprintln!("SnapNote: {message}");
        if let Some(tray) = &self.tray {
            let _ = tray.set_tooltip(Some(message.as_str()));
        }
        // The editor is short-lived; errors do not allocate a permanent hidden renderer.
        if let Err(error) = self.editor("settings", None, Some(message)) {
            eprintln!("SnapNote: cannot display error: {error:#}");
        }
    }
}
impl ApplicationHandler<Event> for Resident {
    fn resumed(&mut self, _: &ActiveEventLoop) {
        if self.initialized {
            return;
        }
        self.initialized = true;
        if let Err(error) = self.setup() {
            self.config_problem = Some(format!("{error:#}"));
            self.report(error);
        }
        eprintln!(
            "SnapNote: resident ready in {} ms; no editor or graphics context",
            self.started.elapsed().as_millis()
        );
    }
    fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, _: WindowEvent) {}
    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: Event) {
        let result = match event {
            Event::Capture => self.capture(),
            Event::Toggle => {
                if self.editor_is_capture {
                    if let Some(pipe) = &mut self.editor {
                        send_editor(pipe, "finish")
                    } else {
                        self.capture()
                    }
                } else {
                    self.capture()
                }
            }
            Event::Settings => self.editor("settings", None, None),
            Event::Recent => self.editor("recent", None, None),
            Event::Folder => Config::load(&self.config_path)
                .and_then(|c| config::expand(&c.storage.directory))
                .and_then(|p| crate::open_folder(&p)),
            Event::Quit => {
                if self.capturing {
                    if let Some(tray) = &self.tray {
                        let _ = tray.set_tooltip(Some(
                            "Cancel the native selection with Escape to finish quitting.",
                        ));
                    }
                    self.quitting = true;
                } else if let Some(pipe) = &mut self.editor {
                    self.quitting = true;
                    let _ = writeln!(pipe, "quit");
                    let _ = pipe.flush();
                } else {
                    event_loop.exit();
                }
                Ok(())
            }
            Event::Captured(result) => {
                self.capturing = false;
                if self.quitting {
                    drop(result);
                    event_loop.exit();
                    return;
                }
                match result {
                    Ok(Some(capture)) => {
                        self.pending_anchor = capture.anchor;
                        let completed = capture.completed.elapsed().as_millis();
                        self.capture_completed_ms = Some(
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis()
                                .saturating_sub(completed),
                        );
                        match capture.file.keep() {
                            Ok((file, path)) => {
                                drop(file);
                                eprintln!(
                                    "SnapNote: PNG header checked; capture handed to editor {} ms after capture completion",
                                    completed
                                );
                                self.editor("capture", Some(path), None)
                            }
                            Err(error) => Err(error.into()),
                        }
                    }
                    Ok(None) => Ok(()),
                    Err(error) => Err(error),
                }
            }
            Event::EditorExited { code, cleanup } => {
                self.editor = None;
                self.editor_is_capture = false;
                if let Some(path) = cleanup
                    && let Err(error) = std::fs::remove_file(path)
                    && error.kind() != std::io::ErrorKind::NotFound
                {
                    eprintln!("SnapNote: temporary cleanup failed: {error}");
                }
                eprintln!("SnapNote: editor exited; all editor graphics and buffers released");
                if self.quitting {
                    event_loop.exit();
                    return;
                }
                if let Err(error) = self.reload_config() {
                    let message = format!("{error:#}");
                    if self.config_problem.as_deref() != Some(message.as_str()) {
                        self.config_problem = Some(message);
                        self.report(error);
                    }
                    return;
                }
                self.config_problem = None;
                if code == Some(10) {
                    self.capture()
                } else if code == Some(11) {
                    let granted = capture::request_permission();
                    self.editor("settings",None,Some(if granted { "Screen Recording permission is enabled. Try a new capture.".into() } else { "Enable SnapNote in System Settings → Privacy & Security → Screen & System Audio Recording, then quit and relaunch.".into() }))
                } else {
                    Ok(())
                }
            }
        };
        if let Err(error) = result {
            self.report(error);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn held_shortcut_cannot_reopen_after_close_until_released() {
        let latch = ShortcutLatch::default();
        assert!(latch.pressed(HotKeyState::Pressed));
        for _ in 0..20 {
            assert!(!latch.pressed(HotKeyState::Pressed));
        }
        assert!(!latch.pressed(HotKeyState::Released));
        assert!(latch.pressed(HotKeyState::Pressed));
    }

    #[test]
    fn command_racing_helper_exit_does_not_request_error_ui() {
        let mut child = Command::new("/usr/bin/true")
            .stdin(Stdio::piped())
            .spawn()
            .unwrap();
        let mut pipe = child.stdin.take().unwrap();
        child.wait().unwrap();
        assert_eq!(
            writeln!(pipe, "finish").unwrap_err().kind(),
            std::io::ErrorKind::BrokenPipe
        );
        assert!(send_editor(&mut pipe, "finish").is_ok());
        assert!(send_editor(&mut pipe, "capture").is_ok());
    }
}
