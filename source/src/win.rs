#![allow(non_snake_case, dead_code)]
use std::ffi::c_void;
pub type Handle = isize;
pub type Gp = *mut c_void;

#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}
#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct Size {
    pub cx: i32,
    pub cy: i32,
}
#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}
#[repr(C)]
#[derive(Default)]
pub struct RectF {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}
#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct SystemTime {
    pub year: u16,
    pub month: u16,
    pub weekday: u16,
    pub day: u16,
    pub hour: u16,
    pub minute: u16,
    pub second: u16,
    pub millis: u16,
}
#[repr(C)]
#[derive(Default)]
pub struct Msg {
    pub hwnd: Handle,
    pub message: u32,
    pub wparam: usize,
    pub lparam: isize,
    pub time: u32,
    pub pt: Point,
    pub private: u32,
}
#[repr(C)]
pub struct WndClass {
    pub size: u32,
    pub style: u32,
    pub proc: Option<unsafe extern "system" fn(Handle, u32, usize, isize) -> isize>,
    pub cls_extra: i32,
    pub wnd_extra: i32,
    pub instance: Handle,
    pub icon: Handle,
    pub cursor: Handle,
    pub background: Handle,
    pub menu: *const u16,
    pub name: *const u16,
    pub small_icon: Handle,
}
#[repr(C)]
#[derive(Default)]
pub struct BitmapInfoHeader {
    pub size: u32,
    pub width: i32,
    pub height: i32,
    pub planes: u16,
    pub bit_count: u16,
    pub compression: u32,
    pub size_image: u32,
    pub xppm: i32,
    pub yppm: i32,
    pub clr_used: u32,
    pub clr_important: u32,
}
#[repr(C)]
#[derive(Default)]
pub struct BitmapInfo {
    pub header: BitmapInfoHeader,
    pub colors: [u32; 1],
}
#[repr(C)]
pub struct BlendFunction {
    pub op: u8,
    pub flags: u8,
    pub alpha: u8,
    pub format: u8,
}
#[repr(C)]
#[derive(Default)]
pub struct MonitorInfo {
    pub size: u32,
    pub monitor: Rect,
    pub work: Rect,
    pub flags: u32,
}
#[repr(C)]
pub struct GdiStartup {
    pub version: u32,
    pub callback: usize,
    pub suppress_background: i32,
    pub suppress_codecs: i32,
}
#[repr(C)]
#[derive(Default)]
pub struct LogFont {
    pub height: i32,
    pub width: i32,
    pub escapement: i32,
    pub orientation: i32,
    pub weight: i32,
    pub italic: u8,
    pub underline: u8,
    pub strikeout: u8,
    pub charset: u8,
    pub out_precision: u8,
    pub clip_precision: u8,
    pub quality: u8,
    pub pitch: u8,
    pub face: [u16; 32],
}
#[repr(C)]
#[derive(Default)]
pub struct Guid {
    pub a: u32,
    pub b: u16,
    pub c: u16,
    pub d: [u8; 8],
}
#[repr(C)]
pub struct ChooseColor {
    pub size: u32,
    pub owner: Handle,
    pub instance: Handle,
    pub result: u32,
    pub custom: *mut u32,
    pub flags: u32,
    pub data: isize,
    pub hook: usize,
    pub template: *const u16,
}
#[repr(C)]
pub struct NotifyIcon {
    pub size: u32,
    pub hwnd: Handle,
    pub id: u32,
    pub flags: u32,
    pub callback: u32,
    pub icon: Handle,
    pub tip: [u16; 128],
    pub state: u32,
    pub state_mask: u32,
    pub info: [u16; 256],
    pub version: u32,
    pub title: [u16; 64],
    pub info_flags: u32,
    pub guid: Guid,
    pub balloon_icon: Handle,
}

#[link(name = "user32")]
extern "system" {
    pub fn RegisterClassExW(class: *const WndClass) -> u16;
    pub fn CreateWindowExW(
        ex: u32,
        class: *const u16,
        title: *const u16,
        style: u32,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        parent: Handle,
        menu: Handle,
        instance: Handle,
        param: *mut c_void,
    ) -> Handle;
    pub fn DefWindowProcW(hwnd: Handle, msg: u32, w: usize, l: isize) -> isize;
    pub fn GetMessageW(msg: *mut Msg, hwnd: Handle, min: u32, max: u32) -> i32;
    pub fn TranslateMessage(msg: *const Msg) -> i32;
    pub fn DispatchMessageW(msg: *const Msg) -> isize;
    pub fn PostQuitMessage(code: i32);
    pub fn DestroyWindow(hwnd: Handle) -> i32;
    pub fn ShowWindow(hwnd: Handle, cmd: i32) -> i32;
    pub fn SetWindowLongPtrW(hwnd: Handle, index: i32, value: isize) -> isize;
    pub fn GetWindowLongPtrW(hwnd: Handle, index: i32) -> isize;
    pub fn SetTimer(hwnd: Handle, id: usize, ms: u32, callback: usize) -> usize;
    pub fn KillTimer(hwnd: Handle, id: usize) -> i32;
    pub fn GetWindowRect(hwnd: Handle, rect: *mut Rect) -> i32;
    pub fn SetWindowPos(
        hwnd: Handle,
        after: Handle,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        flags: u32,
    ) -> i32;
    pub fn GetDC(hwnd: Handle) -> Handle;
    pub fn ReleaseDC(hwnd: Handle, dc: Handle) -> i32;
    pub fn UpdateLayeredWindow(
        hwnd: Handle,
        dst_dc: Handle,
        dst: *const Point,
        size: *const Size,
        src_dc: Handle,
        src: *const Point,
        key: u32,
        blend: *const BlendFunction,
        flags: u32,
    ) -> i32;
    pub fn LoadCursorW(instance: Handle, name: *const u16) -> Handle;
    pub fn LoadIconW(instance: Handle, name: *const u16) -> Handle;
    pub fn ReleaseCapture() -> i32;
    pub fn SendMessageW(hwnd: Handle, msg: u32, w: usize, l: isize) -> isize;
    pub fn PostMessageW(hwnd: Handle, msg: u32, w: usize, l: isize) -> i32;
    pub fn SetForegroundWindow(hwnd: Handle) -> i32;
    pub fn GetCursorPos(point: *mut Point) -> i32;
    pub fn MonitorFromPoint(point: Point, flags: u32) -> Handle;
    pub fn MonitorFromWindow(hwnd: Handle, flags: u32) -> Handle;
    pub fn GetMonitorInfoW(monitor: Handle, info: *mut MonitorInfo) -> i32;
    pub fn GetDpiForWindow(hwnd: Handle) -> u32;
    pub fn SetProcessDpiAwarenessContext(value: isize) -> i32;
    pub fn CreatePopupMenu() -> Handle;
    pub fn AppendMenuW(menu: Handle, flags: u32, id: usize, text: *const u16) -> i32;
    pub fn TrackPopupMenu(
        menu: Handle,
        flags: u32,
        x: i32,
        y: i32,
        reserved: i32,
        hwnd: Handle,
        rect: *const Rect,
    ) -> i32;
    pub fn DestroyMenu(menu: Handle) -> i32;
    pub fn FindWindowW(class: *const u16, title: *const u16) -> Handle;
    pub fn MessageBoxW(hwnd: Handle, text: *const u16, title: *const u16, flags: u32) -> i32;
    pub fn RegisterWindowMessageW(name: *const u16) -> u32;
}
#[link(name = "kernel32")]
extern "system" {
    pub fn GetModuleHandleW(name: *const u16) -> Handle;
    pub fn GetLocalTime(time: *mut SystemTime);
    pub fn GetLastError() -> u32;
}
#[link(name = "advapi32")]
extern "system" {
    pub fn RegCreateKeyExW(
        key: Handle,
        subkey: *const u16,
        reserved: u32,
        class: *mut u16,
        options: u32,
        access: u32,
        security: *mut c_void,
        result: *mut Handle,
        disposition: *mut u32,
    ) -> i32;
    pub fn RegSetValueExW(
        key: Handle,
        name: *const u16,
        reserved: u32,
        value_type: u32,
        data: *const u8,
        data_size: u32,
    ) -> i32;
    pub fn RegDeleteValueW(key: Handle, name: *const u16) -> i32;
    pub fn RegCloseKey(key: Handle) -> i32;
}
#[link(name = "gdi32")]
extern "system" {
    pub fn CreateCompatibleDC(dc: Handle) -> Handle;
    pub fn CreateDIBSection(
        dc: Handle,
        info: *const BitmapInfo,
        usage: u32,
        bits: *mut *mut c_void,
        section: Handle,
        offset: u32,
    ) -> Handle;
    pub fn SelectObject(dc: Handle, object: Handle) -> Handle;
    pub fn DeleteObject(object: Handle) -> i32;
    pub fn DeleteDC(dc: Handle) -> i32;
    pub fn BitBlt(
        dst: Handle,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        src: Handle,
        sx: i32,
        sy: i32,
        rop: u32,
    ) -> i32;
    pub fn GdiFlush() -> i32;
}
#[link(name = "gdiplus")]
extern "system" {
    pub fn GdiplusStartup(token: *mut usize, input: *const GdiStartup, output: *mut c_void) -> i32;
    pub fn GdiplusShutdown(token: usize);
    pub fn GdipCreateBitmapFromScan0(
        w: i32,
        h: i32,
        stride: i32,
        format: i32,
        scan: *mut u8,
        bitmap: *mut Gp,
    ) -> i32;
    pub fn GdipGetImageGraphicsContext(image: Gp, graphics: *mut Gp) -> i32;
    pub fn GdipSetTextRenderingHint(graphics: Gp, hint: i32) -> i32;
    pub fn GdipSetSmoothingMode(graphics: Gp, mode: i32) -> i32;
    pub fn GdipCreateFontFamilyFromName(name: *const u16, collection: Gp, family: *mut Gp) -> i32;
    pub fn GdipDeleteFontFamily(family: Gp) -> i32;
    pub fn GdipCreateFont(family: Gp, size: f32, style: i32, unit: i32, font: *mut Gp) -> i32;
    pub fn GdipCreateFontFromLogfontW(dc: Handle, logfont: *const LogFont, font: *mut Gp) -> i32;
    pub fn GdipDeleteFont(font: Gp) -> i32;
    pub fn GdipCreateStringFormat(attrs: i32, language: u16, format: *mut Gp) -> i32;
    pub fn GdipSetStringFormatAlign(format: Gp, align: i32) -> i32;
    pub fn GdipDeleteStringFormat(format: Gp) -> i32;
    pub fn GdipCreateSolidFill(color: u32, brush: *mut Gp) -> i32;
    pub fn GdipDeleteBrush(brush: Gp) -> i32;
    pub fn GdipDrawString(
        graphics: Gp,
        text: *const u16,
        length: i32,
        font: Gp,
        rect: *const RectF,
        format: Gp,
        brush: Gp,
    ) -> i32;
    pub fn GdipDeleteGraphics(graphics: Gp) -> i32;
    pub fn GdipDisposeImage(image: Gp) -> i32;
    pub fn GdipSaveImageToFile(
        image: Gp,
        file: *const u16,
        encoder: *const Guid,
        params: *const c_void,
    ) -> i32;
}
#[link(name = "shell32")]
extern "system" {
    pub fn Shell_NotifyIconW(message: u32, data: *const NotifyIcon) -> i32;
}
#[link(name = "comdlg32")]
extern "system" {
    pub fn ChooseColorW(color: *mut ChooseColor) -> i32;
}

pub fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}
// MAKEINTRESOURCEW: Win32 encodes an integer resource identifier in the pointer value.
pub fn resource(id: u16) -> *const u16 {
    id as usize as *const u16
}
pub fn local_time() -> SystemTime {
    let mut t = SystemTime::default();
    unsafe { GetLocalTime(&mut t) };
    t
}
pub fn work_area(hwnd: Handle) -> Rect {
    monitor_area(hwnd, true)
}
pub fn screen_area(hwnd: Handle) -> Rect {
    monitor_area(hwnd, false)
}
fn monitor_area(hwnd: Handle, work_only: bool) -> Rect {
    unsafe {
        let mut info = MonitorInfo {
            size: std::mem::size_of::<MonitorInfo>() as u32,
            ..Default::default()
        };
        let monitor = if hwnd == 0 {
            MonitorFromPoint(Point { x: 0, y: 0 }, 2)
        } else {
            MonitorFromWindow(hwnd, 2)
        };
        if GetMonitorInfoW(monitor, &mut info) == 0 {
            Rect {
                left: 0,
                top: 0,
                right: 1920,
                bottom: 1080,
            }
        } else {
            if work_only {
                info.work
            } else {
                info.monitor
            }
        }
    }
}
