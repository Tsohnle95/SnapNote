use super::{CaptureError, CapturedFile};
use std::process::Command;

#[repr(C)]
#[derive(Clone, Copy)]
struct Point {
    x: f64,
    y: f64,
}
#[repr(C)]
struct Size {
    width: f64,
    height: f64,
}
#[repr(C)]
struct Rect {
    origin: Point,
    size: Size,
}
#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGEventCreate(source: *const std::ffi::c_void) -> *const std::ffi::c_void;
    fn CGEventGetLocation(event: *const std::ffi::c_void) -> Point;
    fn CGGetDisplaysWithPoint(point: Point, max: u32, displays: *mut u32, count: *mut u32) -> i32;
    fn CGMainDisplayID() -> u32;
    fn CGDisplayBounds(display: u32) -> Rect;
    fn CGPreflightScreenCaptureAccess() -> bool;
    fn CGRequestScreenCaptureAccess() -> bool;
}
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFRelease(object: *const std::ffi::c_void);
}
fn cursor_anchor() -> Option<super::CursorAnchor> {
    // One-shot cursor sampling after selection; no event tap or input monitoring.
    unsafe {
        let event = CGEventCreate(std::ptr::null());
        if event.is_null() {
            return None;
        }
        let point = CGEventGetLocation(event);
        CFRelease(event);
        let mut display = CGMainDisplayID();
        let mut count = 0;
        CGGetDisplaysWithPoint(point, 1, &mut display, &mut count);
        let bounds = CGDisplayBounds(display);
        Some(super::CursorAnchor {
            x: point.x,
            y: point.y,
            left: bounds.origin.x,
            top: bounds.origin.y,
            width: bounds.size.width,
            height: bounds.size.height,
        })
    }
}
pub fn has_permission() -> bool {
    // These parameterless CoreGraphics APIs are available on supported macOS versions.
    unsafe { CGPreflightScreenCaptureAccess() }
}
pub fn request_permission() -> bool {
    unsafe { CGRequestScreenCaptureAccess() }
}
pub fn region() -> Result<Option<CapturedFile>, CaptureError> {
    let file = super::temporary_file()?;
    let output = Command::new("/usr/sbin/screencapture")
        .args(["-i", "-s", "-x"])
        .arg(file.path())
        .output()?;
    let completed = std::time::Instant::now();
    let anchor = cursor_anchor();
    let diagnostics = [output.stderr, output.stdout].concat();
    let result = super::inspect(file, output.status.success(), &diagnostics);
    if (result.as_ref().is_ok_and(Option::is_none) || result.is_err()) && !has_permission() {
        return Err(CaptureError::PermissionDenied);
    }
    result.map(|capture| {
        capture.map(|mut capture| {
            capture.completed = completed;
            capture.anchor = anchor;
            capture
        })
    })
}
