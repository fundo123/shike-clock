#![cfg_attr(not(test), windows_subsystem = "windows")]
mod online_time;
mod render;
mod settings;
mod win;
use online_time::{fetch_beijing_time, retry_interval, sync_interval, OnlineClock};
use render::{GdiPlus, Renderer};
use settings::{auto_light, clamp_position, ColorMode, Settings};
use std::{
    mem::size_of,
    ptr::{null, null_mut},
    sync::mpsc::{self, Receiver, Sender, TryRecvError},
    thread,
    time::Instant,
};
use win::*;

const CLASS: &str = "Shike.Native.Clock.v1";
const TITLE: &str = "拾刻";
const TRAY: u32 = 0x8001;
const DIAG: u32 = 0x8002;
const RESTORE: u32 = 0x8003;
const GWLP_USERDATA: i32 = -21;
const AUTO: usize = 101;
const LIGHT: usize = 102;
const DARK: usize = 103;
const CUSTOM: usize = 104;
const TOP: usize = 201;
const RESET: usize = 202;
const STARTUP: usize = 203;
const QUIT: usize = 299;

struct App {
    hwnd: Handle,
    renderer: Renderer,
    settings: Settings,
    light: bool,
    last: (u64, u64),
    menu_open: bool,
    taskbar_message: u32,
    diagnostics: Option<String>,
    error: Option<String>,
    online_clock: Option<OnlineClock>,
    sync_sender: Sender<Result<i64, String>>,
    sync_receiver: Receiver<Result<i64, String>>,
    sync_pending: bool,
    sync_due: Instant,
}
fn time_text(t: SystemTime) -> String {
    if t.year == 0 {
        "--:--:--".into()
    } else {
        format!("{:02}:{:02}:{:02}", t.hour, t.minute, t.second)
    }
}
fn date_text(t: SystemTime) -> String {
    if t.year == 0 {
        "--月--日 周-".into()
    } else {
        format!(
            "{}月{}日 周{}",
            t.month,
            t.day,
            settings::weekday(t.weekday)
        )
    }
}
fn time_key(t: SystemTime) -> u64 {
    if t.year == 0 {
        0
    } else {
        ((t.year as u64 * 13 + t.month as u64) * 32 + t.day as u64) * 86_400
            + t.hour as u64 * 3_600
            + t.minute as u64 * 60
            + t.second as u64
    }
}
impl App {
    fn color(&self) -> u32 {
        match self.settings.mode {
            ColorMode::Auto => {
                if self.light {
                    0xf4f6f9
                } else {
                    0x171b23
                }
            }
            ColorMode::Light => 0xf4f6f9,
            ColorMode::Dark => 0x171b23,
            ColorMode::Custom => self.settings.custom,
        }
    }
    fn tick(&mut self, force: bool) {
        self.update_online_clock();
        let local = local_time();
        let beijing = self
            .online_clock
            .as_ref()
            .map(OnlineClock::now)
            .unwrap_or_default();
        let key = (time_key(local), time_key(beijing));
        if !force && key == self.last {
            return;
        }
        self.last = key;
        let brightness = if self.settings.mode == ColorMode::Auto {
            self.renderer.sample_background(self.hwnd)
        } else {
            None
        };
        if let Some(b) = brightness {
            self.light = auto_light(self.light, b)
        }
        let color = self.color();
        let result = self
            .renderer
            .draw(local, beijing, color)
            .and_then(|_| self.renderer.update(self.hwnd));
        if let Err(e) = result {
            self.error = Some(e)
        }
        if self.diagnostics.is_some() {
            self.write_diagnostics(local, beijing, brightness)
        }
    }
    fn update_online_clock(&mut self) {
        match self.sync_receiver.try_recv() {
            Ok(Ok(unix_millis)) => {
                self.online_clock = Some(OnlineClock::new(unix_millis));
                self.sync_pending = false;
                self.sync_due = Instant::now() + sync_interval();
            }
            Ok(Err(_)) => {
                self.sync_pending = false;
                self.sync_due = Instant::now() + retry_interval();
            }
            Err(TryRecvError::Disconnected) => {
                self.sync_pending = false;
                self.sync_due = Instant::now() + retry_interval();
            }
            Err(TryRecvError::Empty) => {}
        }
        if !self.sync_pending && Instant::now() >= self.sync_due {
            self.sync_pending = true;
            self.sync_due = Instant::now() + retry_interval();
            let sender = self.sync_sender.clone();
            thread::spawn(move || {
                let _ = sender.send(fetch_beijing_time());
            });
        }
    }
    fn write_diagnostics(&self, local: SystemTime, beijing: SystemTime, brightness: Option<u8>) {
        if let Some(ref path) = self.diagnostics {
            let mut r = Rect::default();
            unsafe { GetWindowRect(self.hwnd, &mut r) };
            let beijing_time = time_text(beijing);
            let beijing_date = date_text(beijing);
            let source = if beijing.year == 0 {
                "waiting_for_network"
            } else {
                "online"
            };
            let data = format!(
                "{{\"time\":\"{beijing_time}\",\"date\":\"{beijing_date}\",\"local_time\":\"{}\",\"local_date\":\"{}\",\"beijing_time\":\"{beijing_time}\",\"beijing_date\":\"{beijing_date}\",\"time_source\":\"{source}\",\"light_text\":{},\"brightness\":{},\"color\":\"{:06X}\",\"x\":{},\"y\":{},\"width\":{},\"height\":{},\"dpi\":{},\"scale\":{},\"error\":{}}}",
                time_text(local),
                date_text(local),
                self.light,
                brightness.map(|v| v.to_string()).unwrap_or("null".into()),
                self.color(),
                r.left,
                r.top,
                self.renderer.surface.width,
                self.renderer.surface.height,
                unsafe { GetDpiForWindow(self.hwnd) },
                self.settings.scale,
                if self.error.is_some() { "true" } else { "null" }
            );
            let _ = std::fs::write(path, data);
        }
    }
    fn remember(&mut self) {
        let mut r = Rect::default();
        unsafe { GetWindowRect(self.hwnd, &mut r) };
        self.settings.x = r.left;
        self.settings.y = r.top;
        self.settings.placed = true;
        let _ = self.settings.save();
    }
    fn resize(&mut self) {
        match Renderer::new(unsafe { GetDpiForWindow(self.hwnd) }, self.settings.scale) {
            Ok(r) => {
                self.renderer = r;
                self.clamp();
                self.tick(true)
            }
            Err(e) => self.error = Some(e),
        }
    }
    fn clamp(&mut self) {
        unsafe {
            let mut r = Rect::default();
            GetWindowRect(self.hwnd, &mut r);
            // The full monitor includes the taskbar. Work-area clamping would
            // push the clock back onto the desktop whenever a drag ends.
            let area = screen_area(self.hwnd);
            let (x, y) = clamp_position(
                r.left,
                r.top,
                self.renderer.surface.width,
                self.renderer.surface.height,
                area,
            );
            SetWindowPos(
                self.hwnd,
                0,
                x,
                y,
                self.renderer.surface.width,
                self.renderer.surface.height,
                0x0014,
            );
        }
    }
    fn raise_if_topmost(&self) {
        if self.settings.topmost {
            unsafe {
                // A drag can leave the taskbar above a topmost window in the
                // z-order. Reinsert the clock at the top when the drag ends.
                SetWindowPos(self.hwnd, -1, 0, 0, 0, 0, 0x0013);
            }
        }
    }
    fn reset(&mut self) {
        unsafe {
            let area = work_area(0);
            let x = area.right - self.renderer.surface.width - 48;
            let y = area.top + 48;
            SetWindowPos(self.hwnd, 0, x.max(area.left), y, 0, 0, 0x0015);
            ShowWindow(self.hwnd, 4);
        }
        self.clamp();
        self.remember();
        self.tick(true);
    }
    fn tray(&self, operation: u32) {
        unsafe {
            let mut data: NotifyIcon = std::mem::zeroed();
            data.size = size_of::<NotifyIcon>() as u32;
            data.hwnd = self.hwnd;
            data.id = 1;
            data.flags = 1 | 2 | 4;
            data.callback = TRAY;
            data.icon = LoadIconW(GetModuleHandleW(null()), resource(1));
            let tip = wide("拾刻 · 右键设置，双击找回时钟");
            data.tip[..tip.len()].copy_from_slice(&tip);
            Shell_NotifyIconW(operation, &data);
        }
    }

    fn set_startup(&mut self, enabled: bool) {
        match sync_startup(enabled) {
            Ok(()) => {
                self.settings.autostart = enabled;
                let _ = self.settings.save();
            }
            Err(error) => unsafe {
                MessageBoxW(
                    self.hwnd,
                    wide(&format!("无法修改开机启动设置。\n{error}")).as_ptr(),
                    wide(TITLE).as_ptr(),
                    0x10,
                );
            },
        }
    }
}

fn sync_startup(enabled: bool) -> Result<(), String> {
    let startup_command = if enabled {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        Some(
            ['"' as u16]
                .into_iter()
                .chain(std::os::windows::ffi::OsStrExt::encode_wide(
                    exe.as_os_str(),
                ))
                .chain(['"' as u16, 0])
                .collect::<Vec<u16>>(),
        )
    } else {
        None
    };
    unsafe {
        const HKEY_CURRENT_USER: Handle = (-2147483647i32) as Handle;
        let mut key = 0;
        let mut disposition = 0;
        let status = RegCreateKeyExW(
            HKEY_CURRENT_USER,
            wide("Software\\Microsoft\\Windows\\CurrentVersion\\Run").as_ptr(),
            0,
            null_mut(),
            0,
            0x0002 | 0x0004,
            null_mut(),
            &mut key,
            &mut disposition,
        );
        if status != 0 {
            return Err(format!("注册表错误 {status}"));
        }
        let value_name = wide("ShikeClock");
        let status = if enabled {
            let mut command = startup_command.unwrap();
            RegSetValueExW(
                key,
                value_name.as_ptr(),
                0,
                1,
                command.as_mut_ptr().cast(),
                (command.len() * 2) as u32,
            )
        } else {
            let result = RegDeleteValueW(key, value_name.as_ptr());
            if result == 2 {
                0
            } else {
                result
            }
        };
        RegCloseKey(key);
        if status != 0 {
            Err(format!("注册表错误 {status}"))
        } else {
            Ok(())
        }
    }
}

unsafe fn append(menu: Handle, id: usize, text: &str, checked: bool) {
    AppendMenuW(menu, if checked { 8 } else { 0 }, id, wide(text).as_ptr());
}
unsafe fn menu(hwnd: Handle, point: Point) {
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut App;
    if ptr.is_null() {
        return;
    }
    (*ptr).menu_open = true;
    // No Rust reference to App survives a modal native menu; its message loop can reenter wndproc.
    let settings = (*ptr).settings.clone();
    let popup = CreatePopupMenu();
    let colors = CreatePopupMenu();
    let sizes = CreatePopupMenu();
    append(
        colors,
        AUTO,
        "自动适应背景",
        settings.mode == ColorMode::Auto,
    );
    AppendMenuW(colors, 0x800, 0, null());
    append(colors, LIGHT, "浅色文字", settings.mode == ColorMode::Light);
    append(colors, DARK, "深色文字", settings.mode == ColorMode::Dark);
    append(
        colors,
        CUSTOM,
        "自选颜色…",
        settings.mode == ColorMode::Custom,
    );
    AppendMenuW(popup, 0x10, colors as usize, wide("文字颜色").as_ptr());
    for (index, (scale, label)) in [
        (75, "小 · 75%"),
        (100, "标准 · 100%"),
        (125, "大 · 125%"),
        (150, "更大 · 150%"),
        (200, "特大 · 200%"),
    ]
    .iter()
    .enumerate()
    {
        append(sizes, 301 + index, label, settings.scale == *scale)
    }
    AppendMenuW(popup, 0x10, sizes as usize, wide("字号").as_ptr());
    AppendMenuW(popup, 0x800, 0, null());
    append(popup, TOP, "始终置顶", settings.topmost);
    append(popup, RESET, "找回到主屏右上角", false);
    AppendMenuW(popup, 0x800, 0, null());
    append(popup, STARTUP, "开机自动启动", settings.autostart);
    AppendMenuW(popup, 0x800, 0, null());
    append(popup, QUIT, "退出拾刻", false);
    SetForegroundWindow(hwnd);
    let command = TrackPopupMenu(popup, 0x100 | 0x2, point.x, point.y, 0, hwnd, null());
    DestroyMenu(popup);
    PostMessageW(hwnd, 0, 0, 0);
    (*ptr).menu_open = false;
    if command != 0 {
        apply_command(hwnd, command as usize)
    }
}
unsafe fn apply_command(hwnd: Handle, id: usize) {
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut App;
    if ptr.is_null() {
        return;
    }
    match id {
        AUTO => (*ptr).settings.mode = ColorMode::Auto,
        LIGHT => (*ptr).settings.mode = ColorMode::Light,
        DARK => (*ptr).settings.mode = ColorMode::Dark,
        CUSTOM => {
            let rgb = (*ptr).settings.custom;
            let mut custom = [0u32; 16];
            let mut cc = ChooseColor {
                size: size_of::<ChooseColor>() as u32,
                owner: hwnd,
                instance: 0,
                result: ((rgb >> 16) & 255) | (rgb & 0xff00) | ((rgb & 255) << 16),
                custom: custom.as_mut_ptr(),
                flags: 1 | 2,
                data: 0,
                hook: 0,
                template: null(),
            };
            (*ptr).menu_open = true;
            let ok = ChooseColorW(&mut cc);
            (*ptr).menu_open = false;
            if ok != 0 {
                (*ptr).settings.mode = ColorMode::Custom;
                (*ptr).settings.custom =
                    ((cc.result & 255) << 16) | (cc.result & 0xff00) | ((cc.result >> 16) & 255)
            }
        }
        TOP => {
            (*ptr).settings.topmost = !(*ptr).settings.topmost;
            SetWindowPos(
                hwnd,
                if (*ptr).settings.topmost { -1 } else { -2 },
                0,
                0,
                0,
                0,
                0x0013,
            );
        }
        RESET => {
            (*ptr).reset();
            return;
        }
        STARTUP => {
            (*ptr).set_startup(!(*ptr).settings.autostart);
            return;
        }
        QUIT => {
            DestroyWindow(hwnd);
            return;
        }
        301..=305 => {
            (*ptr).settings.scale = [75, 100, 125, 150, 200][id - 301];
            (*ptr).resize();
        }
        _ => return,
    }
    (*ptr).remember();
    (*ptr).tick(true);
}

unsafe extern "system" fn wndproc(hwnd: Handle, msg: u32, w: usize, l: isize) -> isize {
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut App;
    if ptr.is_null() {
        return DefWindowProcW(hwnd, msg, w, l);
    }
    if msg == (*ptr).taskbar_message {
        (*ptr).tray(0);
        return 0;
    }
    match msg {
        0x0014 => 1, // No opaque erase or background brush.
        0x0084 => 1, // Client area; the one-alpha plane accepts drags between glyphs.
        0x0021 => 3, // MA_NOACTIVATE: dragging the clock preserves the working app's focus.
        0x0201 => {
            ReleaseCapture();
            SendMessageW(hwnd, 0x00A1, 2, l);
            0
        }
        0x0203 => 0,
        0x007B => {
            let point = if l == -1 {
                let mut p = Point::default();
                GetCursorPos(&mut p);
                p
            } else {
                Point {
                    x: l as i16 as i32,
                    y: (l >> 16) as i16 as i32,
                }
            };
            menu(hwnd, point);
            0
        }
        0x0111 => {
            apply_command(hwnd, w & 0xffff);
            0
        }
        0x0113 => {
            if !(*ptr).menu_open {
                (*ptr).tick(false)
            }
            0
        }
        0x0232 => {
            (*ptr).clamp();
            (*ptr).raise_if_topmost();
            (*ptr).remember();
            (*ptr).tick(true);
            0
        }
        0x02E0 => {
            let r = *(l as *const Rect);
            SetWindowPos(hwnd, 0, r.left, r.top, 0, 0, 0x0015);
            (*ptr).resize();
            (*ptr).remember();
            0
        }
        0x007E | 0x001A => {
            (*ptr).clamp();
            (*ptr).tick(true);
            0
        }
        0x001E | 0x0218 => {
            (*ptr).tick(true);
            DefWindowProcW(hwnd, msg, w, l)
        }
        TRAY => {
            match l as u32 {
                0x0203 => {
                    (*ptr).reset();
                }
                0x0205 | 0x007B => {
                    let mut p = Point::default();
                    GetCursorPos(&mut p);
                    menu(hwnd, p);
                }
                _ => {}
            }
            0
        }
        DIAG => {
            (*ptr).tick(true);
            0
        }
        RESTORE => {
            (*ptr).reset();
            0
        }
        0x0010 => {
            DestroyWindow(hwnd);
            0
        }
        0x0002 => {
            KillTimer(hwnd, 1);
            (*ptr).remember();
            (*ptr).tray(2);
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, w, l),
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let _gdi = GdiPlus::start()?;
    if let Some(index) = args.iter().position(|s| s == "--render") {
        let path = args.get(index + 1).ok_or("--render needs a PNG filename")?;
        let mut renderer = Renderer::new(96, 100)?;
        let color = if args.iter().any(|s| s == "--dark") {
            0x171b23
        } else {
            0xf4f6f9
        };
        renderer.draw(
            local_time(),
            OnlineClock::new(fetch_beijing_time()?).now(),
            color,
        )?;
        return renderer.surface.save_png(path);
    }
    unsafe {
        SetProcessDpiAwarenessContext(-4);
        let class = wide(CLASS);
        let title = wide(TITLE);
        let existing = FindWindowW(class.as_ptr(), null());
        if existing != 0 {
            PostMessageW(existing, RESTORE, 0, 0);
            return Ok(());
        }
        let instance = GetModuleHandleW(null());
        let icon = LoadIconW(instance, resource(1));
        let wc = WndClass {
            size: size_of::<WndClass>() as u32,
            style: 8,
            proc: Some(wndproc),
            cls_extra: 0,
            wnd_extra: 0,
            instance,
            icon,
            cursor: LoadCursorW(0, resource(32646)),
            background: 0,
            menu: null(),
            name: class.as_ptr(),
            small_icon: icon,
        };
        if RegisterClassExW(&wc) == 0 {
            return Err(format!("RegisterClassEx: {}", GetLastError()));
        }
        let settings = Settings::load();
        sync_startup(settings.autostart)?;
        let (sync_sender, sync_receiver) = mpsc::channel();
        let area = work_area(0);
        let initial = if settings.placed {
            (settings.x, settings.y)
        } else {
            (area.right - 132, area.top + 48)
        };
        let hwnd = CreateWindowExW(
            0x00080000 | 0x00000080 | 0x08000000 | if settings.topmost { 0x00000008 } else { 0 },
            class.as_ptr(),
            title.as_ptr(),
            0x80000000,
            initial.0,
            initial.1,
            84,
            36,
            0,
            0,
            instance,
            null_mut(),
        );
        if hwnd == 0 {
            return Err(format!("CreateWindowEx: {}", GetLastError()));
        }
        let renderer = match Renderer::new(GetDpiForWindow(hwnd), settings.scale) {
            Ok(r) => r,
            Err(e) => {
                DestroyWindow(hwnd);
                return Err(e);
            }
        };
        let diagnostics = args
            .iter()
            .position(|s| s == "--diagnostic-file")
            .and_then(|i| args.get(i + 1).cloned());
        let mut app = Box::new(App {
            hwnd,
            renderer,
            settings,
            light: true,
            last: (0, 0),
            menu_open: false,
            taskbar_message: RegisterWindowMessageW(wide("TaskbarCreated").as_ptr()),
            diagnostics,
            error: None,
            online_clock: None,
            sync_sender,
            sync_receiver,
            sync_pending: false,
            sync_due: Instant::now(),
        });
        let ptr = &mut *app as *mut App;
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, ptr as isize);
        app.clamp();
        SetWindowPos(
            hwnd,
            if app.settings.topmost { -1 } else { -2 },
            0,
            0,
            0,
            0,
            0x0013,
        );
        app.tick(true);
        if let Some(ref e) = app.error {
            let error = e.clone();
            DestroyWindow(hwnd);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            return Err(error);
        }
        ShowWindow(hwnd, 4);
        app.tray(0);
        SetTimer(hwnd, 1, 250, 0);
        let mut msg = Msg::default();
        loop {
            let result = GetMessageW(&mut msg, 0, 0, 0);
            if result <= 0 {
                break;
            }
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
        drop(app);
    }
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        unsafe {
            MessageBoxW(
                0,
                wide(&format!("无法启动拾刻。\n{error}")).as_ptr(),
                wide(TITLE).as_ptr(),
                0x10,
            );
        }
    }
}
