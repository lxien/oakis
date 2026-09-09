use chrono::{DateTime, Local, NaiveDateTime, TimeZone, Utc};

const LOCAL_FMT: &str = "%Y-%m-%dT%H:%M";
const UTC_FMT: &str = "%Y-%m-%dT%H:%M:%SZ";
const DISPLAY_FMT: &str = "%Y-%m-%d %H:%M";

pub fn now_local_input() -> String {
    Local::now().format(LOCAL_FMT).to_string()
}

pub fn to_local_input(stored: Option<&str>) -> String {
    let Some(raw) = stored.map(str::trim).filter(|s| !s.is_empty()) else {
        return now_local_input();
    };
    if let Some(dt) = parse_stored_utc(raw) {
        return dt.with_timezone(&Local).format(LOCAL_FMT).to_string();
    }
    now_local_input()
}

pub fn format_local(stored: &str) -> String {
    let raw = stored.trim();
    if raw.is_empty() {
        return String::new();
    }
    if let Some(dt) = parse_to_local(raw) {
        return dt.format(DISPLAY_FMT).to_string();
    }
    raw.trim_end_matches('Z')
        .replace('T', " ")
        .chars()
        .take(16)
        .collect()
}

pub fn parse_to_local(raw: &str) -> Option<chrono::DateTime<Local>> {
    parse_stored_utc(raw).map(|dt| dt.with_timezone(&Local))
}

fn parse_stored_utc(raw: &str) -> Option<DateTime<Utc>> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(raw) {
        return Some(dt.with_timezone(&Utc));
    }
    let trimmed = raw.trim_end_matches('Z');
    if let Ok(naive) = NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%dT%H:%M:%S") {
        return Some(Utc.from_utc_datetime(&naive));
    }
    if let Ok(naive) = NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%dT%H:%M") {
        return Some(Utc.from_utc_datetime(&naive));
    }
    if let Ok(naive) = NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%d %H:%M:%S") {
        return Some(Utc.from_utc_datetime(&naive));
    }
    if let Ok(naive) = NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%d %H:%M") {
        return Some(Utc.from_utc_datetime(&naive));
    }
    None
}

pub fn resolve_published_input(raw: &str) -> String {
    parse_local_input(raw).unwrap_or_else(|| Utc::now().format(UTC_FMT).to_string())
}

fn parse_local_input(raw: &str) -> Option<String> {
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }
    let naive = NaiveDateTime::parse_from_str(s, LOCAL_FMT)
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S"))
        .ok()?;
    let local = Local.from_local_datetime(&naive).single()?;
    Some(local.with_timezone(&Utc).format(UTC_FMT).to_string())
}

pub fn form_local_or(stored_utc: &str, raw_input: &str) -> String {
    let t = raw_input.trim();
    if !t.is_empty() && (t.contains('T') || t.len() >= 16) {
        if NaiveDateTime::parse_from_str(t, LOCAL_FMT).is_ok()
            || NaiveDateTime::parse_from_str(t, "%Y-%m-%dT%H:%M:%S").is_ok()
        {
            return t.chars().take(16).collect();
        }
    }
    to_local_input(Some(stored_utc))
}
