use crate::win::{wide, SystemTime};
use std::{
    ffi::c_void,
    ptr::{null, null_mut},
    time::Instant,
};

const TIME_API_HOST: &str = "timeapi.io";
const TIME_API_PATH: &str = "/api/Time/current/zone?timeZone=Asia%2FShanghai";
const MAX_RESPONSE_BYTES: usize = 8 * 1024;
const SYNC_INTERVAL_SECONDS: u64 = 15 * 60;
const RETRY_INTERVAL_SECONDS: u64 = 30;

pub struct OnlineClock {
    unix_millis: i64,
    synced_at: Instant,
}

impl OnlineClock {
    pub fn new(unix_millis: i64) -> Self {
        Self {
            unix_millis,
            synced_at: Instant::now(),
        }
    }

    pub fn now(&self) -> SystemTime {
        let elapsed = self.synced_at.elapsed().as_millis() as i64;
        from_unix_millis(self.unix_millis + elapsed)
    }
}

pub fn sync_interval() -> std::time::Duration {
    std::time::Duration::from_secs(SYNC_INTERVAL_SECONDS)
}

pub fn retry_interval() -> std::time::Duration {
    std::time::Duration::from_secs(RETRY_INTERVAL_SECONDS)
}

pub fn fetch_beijing_time() -> Result<i64, String> {
    let request_started = Instant::now();
    let body = https_get_beijing_time()?;
    let received_at = Instant::now();
    let mut unix_millis = parse_api_response(&body)?;
    // The service samples its clock during the request. Half the round trip
    // approximates the time at which that sample was taken.
    unix_millis += received_at.duration_since(request_started).as_millis() as i64 / 2;
    Ok(unix_millis)
}

fn https_get_beijing_time() -> Result<String, String> {
    unsafe {
        let agent = wide("ShikeClock/1.1");
        let session = InternetHandle(WinHttpOpen(agent.as_ptr(), 0, null(), null(), 0));
        if session.0.is_null() {
            return Err("无法建立网络时间连接".into());
        }
        if WinHttpSetTimeouts(session.0, 3000, 3000, 3000, 3000) == 0 {
            return Err("无法设置网络时间连接超时".into());
        }

        let host = wide(TIME_API_HOST);
        let connection = InternetHandle(WinHttpConnect(session.0, host.as_ptr(), 443, 0));
        if connection.0.is_null() {
            return Err("无法连接网络时间服务".into());
        }

        let method = wide("GET");
        let path = wide(TIME_API_PATH);
        let request = InternetHandle(WinHttpOpenRequest(
            connection.0,
            method.as_ptr(),
            path.as_ptr(),
            null(),
            null(),
            null(),
            0x0080_0000,
        ));
        if request.0.is_null() {
            return Err("无法创建网络时间请求".into());
        }
        if WinHttpSendRequest(request.0, null(), 0, null_mut(), 0, 0, 0) == 0
            || WinHttpReceiveResponse(request.0, null_mut()) == 0
        {
            return Err("网络时间请求失败，请检查网络连接".into());
        }

        let mut status = 0u32;
        let mut status_size = std::mem::size_of::<u32>() as u32;
        if WinHttpQueryHeaders(
            request.0,
            0x2000_0013,
            null(),
            (&mut status as *mut u32).cast(),
            &mut status_size,
            null_mut(),
        ) == 0
            || status != 200
        {
            return Err(format!("网络时间服务返回 HTTP {status}"));
        }

        let mut response = Vec::new();
        loop {
            let mut available = 0u32;
            if WinHttpQueryDataAvailable(request.0, &mut available) == 0 {
                return Err("读取网络时间响应失败".into());
            }
            if available == 0 {
                break;
            }
            let next_len = response.len() + available as usize;
            if next_len > MAX_RESPONSE_BYTES {
                return Err("网络时间响应过长".into());
            }
            let previous_len = response.len();
            response.resize(next_len, 0);
            let mut read = 0u32;
            if WinHttpReadData(
                request.0,
                response[previous_len..].as_mut_ptr().cast(),
                available,
                &mut read,
            ) == 0
            {
                return Err("读取网络时间响应失败".into());
            }
            response.truncate(previous_len + read as usize);
        }
        String::from_utf8(response).map_err(|_| "网络时间服务返回了无效文本".into())
    }
}

fn parse_api_response(body: &str) -> Result<i64, String> {
    if json_string(body, "timeZone") != Some("Asia/Shanghai") {
        return Err("网络时间服务未返回北京时间".into());
    }
    let year = json_number(body, "year").ok_or("网络时间响应缺少年份")?;
    let month = json_number(body, "month").ok_or("网络时间响应缺少月份")?;
    let day = json_number(body, "day").ok_or("网络时间响应缺少日期")?;
    let hour = json_number(body, "hour").ok_or("网络时间响应缺少小时")?;
    let minute = json_number(body, "minute").ok_or("网络时间响应缺少分钟")?;
    let second = json_number(body, "seconds").ok_or("网络时间响应缺少秒数")?;
    let millis = json_number(body, "milliSeconds").unwrap_or(0);

    if !(1601..=9999).contains(&year)
        || !(1..=12).contains(&month)
        || day < 1
        || day > days_in_month(year, month)
        || hour > 23
        || minute > 59
        || second > 59
        || millis > 999
    {
        return Err("网络时间服务返回了无效日期".into());
    }

    let days = days_from_civil(year, month, day);
    Ok(days * 86_400_000
        + i64::from(hour) * 3_600_000
        + i64::from(minute) * 60_000
        + i64::from(second) * 1_000
        + i64::from(millis))
}

fn json_number(body: &str, key: &str) -> Option<u32> {
    let key = format!("\"{key}\"");
    let rest = body.get(body.find(&key)? + key.len()..)?;
    let value = rest.get(rest.find(':')? + 1..)?.trim_start();
    let digits: String = value.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        None
    } else {
        digits.parse().ok()
    }
}

fn json_string<'a>(body: &'a str, key: &str) -> Option<&'a str> {
    let key = format!("\"{key}\"");
    let rest = body.get(body.find(&key)? + key.len()..)?;
    let value = rest.get(rest.find(':')? + 1..)?.trim_start();
    let value = value.strip_prefix('"')?;
    value.get(..value.find('"')?)
}

#[allow(clippy::manual_is_multiple_of)]
fn is_leap_year(year: u32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        2 if is_leap_year(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        _ => 0,
    }
}

fn days_from_civil(year: u32, month: u32, day: u32) -> i64 {
    let mut year = i64::from(year);
    let month = i64::from(month);
    year -= i64::from(month <= 2);
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let shifted_month = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * shifted_month + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

fn civil_from_days(days: i64) -> (u32, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    (year as u32, month as u32, day as u32)
}

fn from_unix_millis(unix_millis: i64) -> SystemTime {
    let seconds = unix_millis.div_euclid(1_000);
    let days = seconds.div_euclid(86_400);
    let second_of_day = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    SystemTime {
        year: year as u16,
        month: month as u16,
        day: day as u16,
        weekday: (days + 4).rem_euclid(7) as u16,
        hour: (second_of_day / 3_600) as u16,
        minute: ((second_of_day % 3_600) / 60) as u16,
        second: (second_of_day % 60) as u16,
        millis: unix_millis.rem_euclid(1_000) as u16,
    }
}

struct InternetHandle(*mut c_void);

impl Drop for InternetHandle {
    fn drop(&mut self) {
        unsafe {
            if !self.0.is_null() {
                WinHttpCloseHandle(self.0);
            }
        }
    }
}

#[link(name = "winhttp")]
extern "system" {
    fn WinHttpOpen(
        agent: *const u16,
        access_type: u32,
        proxy_name: *const u16,
        proxy_bypass: *const u16,
        flags: u32,
    ) -> *mut c_void;
    fn WinHttpConnect(
        session: *mut c_void,
        server: *const u16,
        port: u16,
        reserved: u32,
    ) -> *mut c_void;
    fn WinHttpOpenRequest(
        connection: *mut c_void,
        method: *const u16,
        path: *const u16,
        version: *const u16,
        referrer: *const u16,
        accept_types: *const *const u16,
        flags: u32,
    ) -> *mut c_void;
    fn WinHttpSetTimeouts(
        handle: *mut c_void,
        resolve_timeout: i32,
        connect_timeout: i32,
        send_timeout: i32,
        receive_timeout: i32,
    ) -> i32;
    fn WinHttpSendRequest(
        request: *mut c_void,
        headers: *const u16,
        headers_length: u32,
        optional: *mut c_void,
        optional_length: u32,
        total_length: u32,
        context: usize,
    ) -> i32;
    fn WinHttpReceiveResponse(request: *mut c_void, reserved: *mut c_void) -> i32;
    fn WinHttpQueryHeaders(
        request: *mut c_void,
        info_level: u32,
        name: *const u16,
        buffer: *mut c_void,
        buffer_length: *mut u32,
        index: *mut u32,
    ) -> i32;
    fn WinHttpQueryDataAvailable(request: *mut c_void, available: *mut u32) -> i32;
    fn WinHttpReadData(
        request: *mut c_void,
        buffer: *mut c_void,
        bytes_to_read: u32,
        bytes_read: *mut u32,
    ) -> i32;
    fn WinHttpCloseHandle(handle: *mut c_void) -> i32;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_beijing_time_api_response() {
        let json = r#"{"year":2026,"month":10,"day":20,"hour":9,"minute":54,"seconds":49,"milliSeconds":283,"timeZone":"Asia/Shanghai"}"#;
        let millis = parse_api_response(json).unwrap();
        let time = from_unix_millis(millis);
        assert_eq!(
            (
                time.year,
                time.month,
                time.day,
                time.weekday,
                time.hour,
                time.minute,
                time.second
            ),
            (2026, 10, 20, 2, 9, 54, 49)
        );
        assert_eq!(time.millis, 283);
    }

    #[test]
    fn online_clock_rolls_over_midnight_without_system_clock() {
        let before_midnight = parse_api_response(
            r#"{"year":2026,"month":12,"day":31,"hour":23,"minute":59,"seconds":59,"milliSeconds":0,"timeZone":"Asia/Shanghai"}"#,
        )
        .unwrap();
        let after_one_second = from_unix_millis(before_midnight + 1_000);
        assert_eq!(
            (
                after_one_second.year,
                after_one_second.month,
                after_one_second.day
            ),
            (2027, 1, 1)
        );
        assert_eq!(
            (
                after_one_second.hour,
                after_one_second.minute,
                after_one_second.second
            ),
            (0, 0, 0)
        );
    }

    #[test]
    fn rejects_invalid_dates_and_times() {
        assert!(parse_api_response(
            r#"{"year":2025,"month":2,"day":29,"hour":12,"minute":0,"seconds":0,"timeZone":"Asia/Shanghai"}"#
        )
        .is_err());
        assert!(parse_api_response(
            r#"{"year":2026,"month":1,"day":1,"hour":24,"minute":0,"seconds":0,"timeZone":"Asia/Shanghai"}"#
        )
        .is_err());
        assert!(parse_api_response(
            r#"{"year":2026,"month":10,"day":20,"hour":9,"minute":54,"seconds":49,"timeZone":"Asia/Singapore"}"#
        )
        .is_err());
    }
}
