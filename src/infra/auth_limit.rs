use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const WINDOW: Duration = Duration::from_secs(15 * 60);
const LOCKOUT: Duration = Duration::from_secs(15 * 60);
const MAX_FAILS: u32 = 8;

#[derive(Debug, Default)]
struct Entry {
    fails: Vec<Instant>,
    locked_until: Option<Instant>,
}

#[derive(Debug, Default)]
pub struct LoginLimiter {
    inner: Mutex<HashMap<String, Entry>>,
}

impl LoginLimiter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn key(client: &str, username: &str) -> String {
        let user = username.trim().to_ascii_lowercase();
        format!("{client}\0{user}")
    }

    pub fn is_blocked(&self, key: &str) -> bool {
        let mut map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let now = Instant::now();
        Self::cleanup_entry(map.entry(key.to_string()).or_default(), now);
        map.get(key)
            .and_then(|e| e.locked_until)
            .is_some_and(|until| until > now)
    }

    pub fn record_failure(&self, key: &str) {
        let mut map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let now = Instant::now();
        let entry = map.entry(key.to_string()).or_default();
        Self::cleanup_entry(entry, now);
        entry.fails.push(now);
        if entry.fails.len() as u32 >= MAX_FAILS {
            entry.locked_until = Some(now + LOCKOUT);
            entry.fails.clear();
        }
    }

    pub fn clear(&self, key: &str) {
        let mut map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        map.remove(key);
    }

    fn cleanup_entry(entry: &mut Entry, now: Instant) {
        if entry.locked_until.is_some_and(|until| until <= now) {
            entry.locked_until = None;
        }
        entry.fails.retain(|t| now.duration_since(*t) < WINDOW);
    }
}
