use crate::win::Rect;
use std::{fs, path::PathBuf};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ColorMode {
    Auto,
    Light,
    Dark,
    Custom,
}
#[derive(Clone, Debug)]
pub struct Settings {
    pub x: i32,
    pub y: i32,
    pub scale: u32,
    pub mode: ColorMode,
    pub custom: u32,
    pub topmost: bool,
    pub placed: bool,
    pub autostart: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            scale: 100,
            mode: ColorMode::Auto,
            custom: 0x86E5D1,
            topmost: true,
            placed: false,
            autostart: true,
        }
    }
}
impl Settings {
    pub fn path() -> PathBuf {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("ShikeClock")
            .join("settings.ini")
    }
    pub fn parse(text: &str) -> Self {
        let mut s = Self::default();
        for line in text.lines() {
            if let Some((k, v)) = line.split_once('=') {
                match k.trim() {
                    "x" => s.x = v.trim().parse().unwrap_or(s.x),
                    "y" => s.y = v.trim().parse().unwrap_or(s.y),
                    "scale" => {
                        let n = v.trim().parse().unwrap_or(100);
                        s.scale = if [75, 100, 125, 150, 200].contains(&n) {
                            n
                        } else {
                            100
                        };
                    }
                    "color" => {
                        s.mode = match v.trim() {
                            "light" => ColorMode::Light,
                            "dark" => ColorMode::Dark,
                            "custom" => ColorMode::Custom,
                            _ => ColorMode::Auto,
                        }
                    }
                    "custom" => {
                        s.custom = u32::from_str_radix(v.trim(), 16).unwrap_or(s.custom) & 0xffffff
                    }
                    "topmost" => s.topmost = v.trim() != "false",
                    "placed" => s.placed = v.trim() == "true",
                    "autostart" => s.autostart = v.trim() == "true",
                    _ => {}
                }
            }
        }
        s
    }
    pub fn load() -> Self {
        fs::read_to_string(Self::path())
            .map(|v| Self::parse(&v))
            .unwrap_or_default()
    }
    pub fn save(&self) -> std::io::Result<()> {
        let path = Self::path();
        fs::create_dir_all(path.parent().unwrap())?;
        let mode = match self.mode {
            ColorMode::Auto => "auto",
            ColorMode::Light => "light",
            ColorMode::Dark => "dark",
            ColorMode::Custom => "custom",
        };
        let text =
            format!(
            "x={}\ny={}\nscale={}\ncolor={}\ncustom={:06X}\ntopmost={}\nplaced={}\nautostart={}\n",
            self.x, self.y, self.scale, mode, self.custom, self.topmost, self.placed, self.autostart
        );
        let temp = path.with_extension("tmp");
        fs::write(&temp, text)?;
        // Same-directory rename replaces atomically on Windows; no partial settings file.
        fs::rename(temp, path)
    }
}
pub fn clamp_position(x: i32, y: i32, w: i32, h: i32, area: Rect) -> (i32, i32) {
    (
        x.clamp(area.left, (area.right - w).max(area.left)),
        y.clamp(area.top, (area.bottom - h).max(area.top)),
    )
}
pub fn weekday(day: u16) -> u16 {
    if day == 0 {
        7
    } else {
        day
    }
}
pub fn auto_light(previous: bool, brightness: u8) -> bool {
    if previous {
        brightness < 128
    } else {
        brightness < 112
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn corrupt_settings_remain_safe() {
        let s = Settings::parse("scale=999999\nx=NaN\ncustom=BADHEX\ncolor=unknown");
        assert_eq!(s.scale, 100);
        assert_eq!(s.x, 0);
        assert_eq!(s.mode, ColorMode::Auto);
    }
    #[test]
    fn negative_monitor_is_supported() {
        assert_eq!(
            clamp_position(
                -3000,
                1800,
                300,
                120,
                Rect {
                    left: -1920,
                    top: 0,
                    right: 0,
                    bottom: 1080
                }
            ),
            (-1920, 960)
        );
    }
    #[test]
    fn sunday_is_seven() {
        assert_eq!(weekday(0), 7);
        assert_eq!(weekday(1), 1);
        assert_eq!(weekday(6), 6);
    }
    #[test]
    fn background_hysteresis() {
        assert!(auto_light(true, 120));
        assert!(!auto_light(false, 120));
        assert!(!auto_light(true, 230));
        assert!(auto_light(false, 40));
    }
}
