use crate::{date_text, time_text, win::*};
use std::{
    ffi::c_void,
    ptr::{null, null_mut},
    slice,
};

fn checked(status: i32, what: &str) -> Result<(), String> {
    if status == 0 {
        Ok(())
    } else {
        Err(format!("{what}: GDI+ status {status}"))
    }
}
pub struct GdiPlus(usize);
impl GdiPlus {
    pub fn start() -> Result<Self, String> {
        unsafe {
            let mut t = 0;
            checked(
                GdiplusStartup(
                    &mut t,
                    &GdiStartup {
                        version: 1,
                        callback: 0,
                        suppress_background: 0,
                        suppress_codecs: 0,
                    },
                    null_mut(),
                ),
                "GDI+ startup",
            )?;
            Ok(Self(t))
        }
    }
}
impl Drop for GdiPlus {
    fn drop(&mut self) {
        unsafe { GdiplusShutdown(self.0) }
    }
}

pub struct Surface {
    pub width: i32,
    pub height: i32,
    pub pixels: *mut u32,
    pub dc: Handle,
    bitmap: Handle,
    old: Handle,
}
impl Surface {
    pub fn new(w: i32, h: i32) -> Result<Self, String> {
        unsafe {
            let dc = CreateCompatibleDC(0);
            if dc == 0 {
                return Err("CreateCompatibleDC failed".into());
            }
            let info = BitmapInfo {
                header: BitmapInfoHeader {
                    size: 40,
                    width: w,
                    height: -h,
                    planes: 1,
                    bit_count: 32,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut bits: *mut c_void = null_mut();
            let bitmap = CreateDIBSection(dc, &info, 0, &mut bits, 0, 0);
            if bitmap == 0 || bits.is_null() {
                DeleteDC(dc);
                return Err("CreateDIBSection failed".into());
            }
            let old = SelectObject(dc, bitmap);
            Ok(Self {
                width: w,
                height: h,
                pixels: bits.cast(),
                dc,
                bitmap,
                old,
            })
        }
    }
    pub fn data(&self) -> &[u32] {
        unsafe { slice::from_raw_parts(self.pixels, (self.width * self.height) as usize) }
    }
    pub fn data_mut(&mut self) -> &mut [u32] {
        unsafe { slice::from_raw_parts_mut(self.pixels, (self.width * self.height) as usize) }
    }
    pub fn save_png(&self, path: &str) -> Result<(), String> {
        unsafe {
            let mut image = null_mut();
            checked(
                GdipCreateBitmapFromScan0(
                    self.width,
                    self.height,
                    self.width * 4,
                    0xE200B,
                    self.pixels.cast(),
                    &mut image,
                ),
                "PNG image",
            )?;
            let png = Guid {
                a: 0x557cf406,
                b: 0x1a04,
                c: 0x11d3,
                d: [0x9a, 0x73, 0, 0, 0xf8, 0x1e, 0xf3, 0x2e],
            };
            let result = checked(
                GdipSaveImageToFile(image, wide(path).as_ptr(), &png, null()),
                "Save PNG",
            );
            GdipDisposeImage(image);
            result
        }
    }
}
impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.dc, self.old);
            DeleteObject(self.bitmap);
            DeleteDC(self.dc);
        }
    }
}

struct Font(Gp);
impl Font {
    fn semibold(size: f32) -> Result<Self, String> {
        unsafe {
            let mut lf = LogFont {
                height: -(size.round() as i32),
                weight: 600,
                charset: 1,
                quality: 4,
                ..Default::default()
            };
            let name = wide("Bahnschrift");
            lf.face[..name.len()].copy_from_slice(&name);
            let dc = GetDC(0);
            let mut font = null_mut();
            let result = GdipCreateFontFromLogfontW(dc, &lf, &mut font);
            ReleaseDC(0, dc);
            if result == 0 {
                Ok(Self(font))
            } else {
                Self::new(&["Segoe UI", "Bahnschrift"], size)
            }
        }
    }
    fn new(names: &[&str], size: f32) -> Result<Self, String> {
        unsafe {
            for name in names {
                let mut family = null_mut();
                if GdipCreateFontFamilyFromName(wide(name).as_ptr(), null_mut(), &mut family) != 0 {
                    continue;
                }
                let mut font = null_mut();
                let status = GdipCreateFont(family, size, 0, 2, &mut font);
                GdipDeleteFontFamily(family);
                if status == 0 {
                    return Ok(Self(font));
                }
            }
            Err("No usable system font".into())
        }
    }
}
impl Drop for Font {
    fn drop(&mut self) {
        unsafe {
            GdipDeleteFont(self.0);
        }
    }
}

pub struct Renderer {
    pub surface: Surface,
    capture: Surface,
    mask: Vec<u8>,
    factor: f32,
    time_font: Font,
    detail_font: Font,
}
impl Renderer {
    pub fn new(dpi: u32, scale: u32) -> Result<Self, String> {
        // Keep the compact composition at 62.5% of its original size.
        let factor = (dpi.max(96) as f32 / 96.) * (scale as f32 / 100.) * 0.625;
        let w = (320. * factor).round() as i32;
        let h = (100. * factor).round() as i32;
        Ok(Self {
            surface: Surface::new(w, h)?,
            capture: Surface::new(w, h)?,
            mask: vec![0; (w * h) as usize],
            factor,
            time_font: Font::semibold(52. * factor)?,
            detail_font: Font::new(
                &[
                    "Noto Sans SC Medium",
                    "等线",
                    "Microsoft YaHei UI",
                    "Segoe UI",
                ],
                32. * factor,
            )?,
        })
    }
    pub fn draw(
        &mut self,
        local: SystemTime,
        beijing: SystemTime,
        color: u32,
    ) -> Result<(), String> {
        unsafe {
            let w = self.surface.width;
            let h = self.surface.height;
            let sw = w * 2;
            let sh = h * 2;
            let mut high = vec![0u32; (sw * sh) as usize];
            let mut image = null_mut();
            let mut g = null_mut();
            checked(
                GdipCreateBitmapFromScan0(
                    sw,
                    sh,
                    sw * 4,
                    0xE200B,
                    high.as_mut_ptr().cast(),
                    &mut image,
                ),
                "Text surface",
            )?;
            if let Err(e) = checked(GdipGetImageGraphicsContext(image, &mut g), "Text context") {
                GdipDisposeImage(image);
                return Err(e);
            }
            GdipSetTextRenderingHint(g, 4);
            GdipSetSmoothingMode(g, 4);
            let mut format = null_mut();
            let mut time_brush = null_mut();
            let mut detail_brush = null_mut();
            GdipCreateStringFormat(0x1000 | 0x4000, 0, &mut format);
            GdipSetStringFormatAlign(format, 0);
            GdipCreateSolidFill(0xffffffff, &mut time_brush);
            GdipCreateSolidFill(0xebffffff, &mut detail_brush);
            let sf = self.factor * 2.;
            let mut status = 0;
            for (row, time) in [local, beijing].iter().enumerate() {
                let row_y = 3. + row as f32 * 48.;
                let clock = time_text(*time);
                let mut x = 10.;
                for c in clock.chars() {
                    let cell = if c == ':' { 7.5 } else { 14. };
                    let character = wide(&c.to_string());
                    status |= GdipDrawString(
                        g,
                        character.as_ptr(),
                        1,
                        self.time_font.0,
                        &RectF {
                            x: x * sf,
                            y: row_y * sf,
                            width: cell * sf,
                            height: 40. * sf,
                        },
                        format,
                        time_brush,
                    );
                    x += cell;
                }
                let date = wide(&date_text(*time));
                status |= GdipDrawString(
                    g,
                    date.as_ptr(),
                    (date.len() - 1) as i32,
                    self.detail_font.0,
                    &RectF {
                        x: 140. * sf,
                        y: row_y * sf,
                        width: 170. * sf,
                        height: 25. * sf,
                    },
                    format,
                    detail_brush,
                );
            }
            GdipDeleteBrush(time_brush);
            GdipDeleteBrush(detail_brush);
            GdipDeleteStringFormat(format);
            GdipDeleteGraphics(g);
            GdipDisposeImage(image);
            checked(status, "Draw clock")?;
            for y in 0..h as usize {
                for x in 0..w as usize {
                    let i = y * 2 * sw as usize + x * 2;
                    self.mask[y * w as usize + x] = (((high[i] >> 24)
                        + (high[i + 1] >> 24)
                        + (high[i + sw as usize] >> 24)
                        + (high[i + sw as usize + 1] >> 24)
                        + 2)
                        / 4) as u8;
                }
            }
            let radius = (2. * self.factor).round().max(1.) as usize;
            let blur = blur_mask(&self.mask, w as usize, h as usize, radius);
            let dark_shadow = brightness(color) > 145;
            let shadow_rgb = if dark_shadow { 0 } else { 0xffffff };
            let shadow_strength = if dark_shadow { 0.55 } else { 0.50 };
            let offset = (1.5 * self.factor).round() as usize;
            let pixels = self.surface.data_mut();
            for y in 0..h as usize {
                for x in 0..w as usize {
                    let i = y * w as usize + x;
                    let a = self.mask[i] as u32;
                    let sa = if y >= offset {
                        (blur[(y - offset) * w as usize + x] as f32 * shadow_strength) as u32
                    } else {
                        0
                    };
                    let back = sa * (255 - a) / 255;
                    let alpha = (a + back).max(1);
                    let channel = |shift: u32| -> u32 {
                        (((color >> shift) & 255) * a + ((shadow_rgb >> shift) & 255) * back) / 255
                    };
                    pixels[i] =
                        (alpha << 24) | (channel(16) << 16) | (channel(8) << 8) | channel(0);
                }
            }
            Ok(())
        }
    }
    pub fn update(&self, hwnd: Handle) -> Result<(), String> {
        unsafe {
            let mut rect = Rect::default();
            GetWindowRect(hwnd, &mut rect);
            let ok = UpdateLayeredWindow(
                hwnd,
                0,
                &Point {
                    x: rect.left,
                    y: rect.top,
                },
                &Size {
                    cx: self.surface.width,
                    cy: self.surface.height,
                },
                self.surface.dc,
                &Point { x: 0, y: 0 },
                0,
                &BlendFunction {
                    op: 0,
                    flags: 0,
                    alpha: 255,
                    format: 1,
                },
                2,
            );
            if ok == 0 {
                Err(format!("UpdateLayeredWindow: {}", GetLastError()))
            } else {
                Ok(())
            }
        }
    }
    pub fn sample_background(&mut self, hwnd: Handle) -> Option<u8> {
        unsafe {
            let mut rect = Rect::default();
            if GetWindowRect(hwnd, &mut rect) == 0 {
                return None;
            }
            let screen = GetDC(0);
            if screen == 0 {
                return None;
            }
            let ok = BitBlt(
                self.capture.dc,
                0,
                0,
                self.capture.width,
                self.capture.height,
                screen,
                rect.left,
                rect.top,
                0x00CC0020 | 0x40000000,
            );
            ReleaseDC(0, screen);
            if ok == 0 {
                return None;
            }
            GdiFlush();
            sample_pixels(
                self.capture.data(),
                self.surface.data(),
                self.capture.width as usize,
                self.capture.height as usize,
                self.factor,
            )
        }
    }
}
fn sample_pixels(data: &[u32], current: &[u32], w: usize, h: usize, factor: f32) -> Option<u8> {
    let mut samples = Vec::new();
    // Ignore our glyphs AND their shadows; transparent holes reveal the actual windows/wallpaper beneath.
    let step = (5. * factor).round().max(3.) as usize;
    let margin = (12. * factor).round().max(2.) as usize;
    for y in (margin..h.saturating_sub(margin)).step_by(step) {
        for x in (margin..w.saturating_sub(margin)).step_by(step) {
            let i = y * w + x;
            if current[i] >> 24 <= 3 {
                samples.push(brightness(data[i]));
            }
        }
    }
    if samples.len() < 12 {
        return None;
    }
    samples.sort_unstable();
    // A trimmed mean tolerates small bright wallpaper highlights without chasing single pixels.
    let trim = samples.len() / 6;
    let range = &samples[trim..samples.len() - trim];
    Some((range.iter().map(|v| *v as u32).sum::<u32>() / range.len() as u32) as u8)
}
pub fn brightness(rgb: u32) -> u8 {
    ((((rgb >> 16) & 255) * 54 + ((rgb >> 8) & 255) * 183 + (rgb & 255) * 19) >> 8) as u8
}
fn blur_mask(input: &[u8], w: usize, h: usize, r: usize) -> Vec<u8> {
    let mut a = input.to_vec();
    let mut b = vec![0; input.len()];
    // Two separable box passes approximate a small soft Gaussian; no GPU or idle animation.
    for _ in 0..2 {
        for y in 0..h {
            let mut sum = 0u32;
            for x in 0..w + r {
                if x < w {
                    sum += a[y * w + x] as u32
                }
                if x > 2 * r {
                    sum -= a[y * w + x - 2 * r - 1] as u32
                }
                if x >= r {
                    b[y * w + x - r] = (sum / (2 * r + 1) as u32) as u8
                }
            }
        }
        for x in 0..w {
            let mut sum = 0u32;
            for y in 0..h + r {
                if y < h {
                    sum += b[y * w + x] as u32
                }
                if y > 2 * r {
                    sum -= b[(y - 2 * r - 1) * w + x] as u32
                }
                if y >= r {
                    a[(y - r) * w + x] = (sum / (2 * r + 1) as u32) as u8
                }
            }
        }
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn samples_background_and_ignores_own_text() {
        let (w, h) = (288, 104);
        let mut background = vec![0x171b23; w * h];
        let mut overlay = vec![0x01000000; w * h];
        for y in 20..70 {
            for x in 20..150 {
                let i = y * w + x;
                background[i] = 0xffffff;
                overlay[i] = 0xffffffff;
            }
        }
        assert_eq!(sample_pixels(&background, &overlay, w, h, 1.), Some(26));
        for (b, o) in background.iter_mut().zip(&overlay) {
            if o >> 24 <= 3 {
                *b = 0xf2f4f7
            }
        }
        assert_eq!(sample_pixels(&background, &overlay, w, h, 1.), Some(243));
        // The smallest menu size must still provide enough samples for auto color.
        assert_eq!(
            sample_pixels(
                &vec![0xf2f4f7; 63 * 27],
                &vec![0x01000000; 63 * 27],
                63,
                27,
                0.375
            ),
            Some(243)
        );
    }
    #[test]
    fn offline_placeholder_keeps_both_timezone_rows() {
        let _gdip = GdiPlus::start().unwrap();
        let mut r = Renderer::new(96, 100).unwrap();
        r.draw(SystemTime::default(), SystemTime::default(), 0xf4f6f9)
            .unwrap();
        let pixels = r.surface.data();
        let w = r.surface.width as usize;
        let second_row = (48. * r.factor).round() as usize;
        assert!(pixels[..w * second_row].iter().any(|p| *p >> 24 > 180));
        assert!(pixels[w * second_row..].iter().any(|p| *p >> 24 > 180));
    }

    #[test]
    fn rendered_clock_has_transparency_and_both_timezone_rows() {
        let _gdip = GdiPlus::start().unwrap();
        let mut r = Renderer::new(96, 100).unwrap();
        let local = SystemTime {
            year: 2026,
            month: 10,
            day: 28,
            weekday: 3,
            hour: 14,
            minute: 3,
            second: 19,
            ..Default::default()
        };
        let beijing = SystemTime {
            year: 2026,
            month: 10,
            day: 29,
            weekday: 4,
            hour: 5,
            minute: 3,
            second: 19,
            ..Default::default()
        };
        assert_eq!(time_text(local), "14:03:19");
        assert_eq!(date_text(local), "10月28日 周3");
        assert_eq!(time_text(beijing), "05:03:19");
        assert_eq!(date_text(beijing), "10月29日 周4");
        r.draw(local, beijing, 0xf4f6f9).unwrap();
        let pixels = r.surface.data();
        let w = r.surface.width as usize;
        let h = r.surface.height as usize;
        assert!(pixels.iter().filter(|p| **p >> 24 <= 3).count() > pixels.len() / 2);
        let second_row = (48. * r.factor).round() as usize;
        assert!(pixels[..w * second_row].iter().any(|p| *p >> 24 > 200));
        assert!(pixels[w * second_row..].iter().any(|p| *p >> 24 > 180));
        let row_bounds = |start: usize, end: usize| {
            let mut left = w;
            let mut right = 0;
            for y in start..end {
                for x in 0..w {
                    if pixels[y * w + x] >> 24 > 30 {
                        left = left.min(x);
                        right = right.max(x);
                    }
                }
            }
            (left, right)
        };
        let first_bounds = row_bounds(0, second_row);
        let second_bounds = row_bounds(second_row, h);
        assert_eq!(first_bounds.0, second_bounds.0);
        assert!(first_bounds.1.abs_diff(second_bounds.1) <= 1);
        let vertical_bounds = |start: usize, end: usize, x_start: usize, x_end: usize| {
            let mut top = h;
            let mut bottom = 0;
            for y in start..end {
                for x in x_start..x_end {
                    if pixels[y * w + x] >> 24 > 40 {
                        top = top.min(y);
                        bottom = bottom.max(y);
                    }
                }
            }
            (top, bottom)
        };
        let clock_bounds = vertical_bounds(0, second_row, 0, 80);
        let date_bounds = vertical_bounds(0, second_row, 80, w);
        assert!((clock_bounds.0 + clock_bounds.1).abs_diff(date_bounds.0 + date_bounds.1) <= 1);
        assert!(pixels[..w].iter().all(|p| *p >> 24 <= 3));
        assert!(pixels[(h - 1) * w..].iter().all(|p| *p >> 24 <= 3));
    }
}
