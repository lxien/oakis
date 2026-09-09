use std::net::IpAddr;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};

#[derive(Clone, Debug)]
pub struct Config {
    pub host: IpAddr,
    pub port: u16,

    pub database_url: String,

    pub login_path: String,

    pub data_dir: PathBuf,

    pub upload_dir: PathBuf,

    pub session_secure: bool,

    pub public_origin: Option<String>,

    pub trust_proxy: bool,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let host: IpAddr = env_or("OAKIS_HOST", "0.0.0.0")
            .parse()
            .context("invalid OAKIS_HOST")?;
        let port: u16 = env_or("OAKIS_PORT", "8080")
            .parse()
            .context("invalid OAKIS_PORT")?;

        let data_dir = PathBuf::from(env_or("OAKIS_DATA_DIR", "data"));

        let upload_dir = std::env::var("OAKIS_UPLOAD_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| data_dir.join("uploads"));

        let database_url = std::env::var("OAKIS_DATABASE_URL").unwrap_or_else(|_| {
            let db_path = data_dir.join("oakis.db");

            let path = db_path.to_string_lossy().replace('\\', "/");
            format!("sqlite:{path}?mode=rwc")
        });

        let login_path = normalize_login_path(&env_or("OAKIS_LOGIN_PATH", "/login"))?;

        let session_secure = matches!(
            env_or("OAKIS_SESSION_SECURE", "false")
                .to_ascii_lowercase()
                .as_str(),
            "1" | "true" | "yes" | "on"
        );

        let public_origin = std::env::var("OAKIS_PUBLIC_ORIGIN")
            .ok()
            .map(|s| s.trim().trim_end_matches('/').to_string())
            .filter(|s| !s.is_empty());
        if let Some(origin) = &public_origin {
            if !(origin.starts_with("http://") || origin.starts_with("https://")) {
                bail!("OAKIS_PUBLIC_ORIGIN 必须以 http:// 或 https:// 开头");
            }
        }

        let trust_proxy = matches!(
            env_or("OAKIS_TRUST_PROXY", "false")
                .to_ascii_lowercase()
                .as_str(),
            "1" | "true" | "yes" | "on"
        );

        Ok(Self {
            host,
            port,
            database_url,
            login_path,
            data_dir,
            upload_dir,
            session_secure,
            public_origin,
            trust_proxy,
        })
    }

    pub fn media_dir(&self) -> PathBuf {
        self.upload_dir.join("media")
    }

    pub fn posts_dir(&self) -> PathBuf {
        self.upload_dir.join("posts")
    }

    pub fn site_dir(&self) -> PathBuf {
        self.upload_dir.join("site")
    }

    pub fn ensure_dirs(&self) -> Result<()> {
        std::fs::create_dir_all(&self.data_dir)
            .with_context(|| format!("无法创建数据目录 {}", self.data_dir.display()))?;
        for dir in [self.media_dir(), self.posts_dir(), self.site_dir()] {
            std::fs::create_dir_all(&dir)
                .with_context(|| format!("无法创建上传目录 {}", dir.display()))?;
        }
        Ok(())
    }
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

fn normalize_login_path(raw: &str) -> Result<String> {
    let path = raw.trim();
    if path.is_empty() {
        bail!("OAKIS_LOGIN_PATH 不能为空");
    }
    if !path.starts_with('/') {
        bail!("OAKIS_LOGIN_PATH 必须以 / 开头，例如 /login");
    }
    if path == "/" {
        bail!("OAKIS_LOGIN_PATH 不能是 /");
    }
    if path.ends_with('/') {
        bail!("OAKIS_LOGIN_PATH 不要以 / 结尾");
    }
    if path.contains("//") || path.contains('?') || path.contains('#') {
        bail!("OAKIS_LOGIN_PATH 格式无效");
    }
    if !path
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '/' || c == '-' || c == '_')
    {
        bail!("OAKIS_LOGIN_PATH 仅允许字母、数字、/、-、_");
    }

    let reserved = [
        "/install",
        "/static",
        "/uploads",
        "/posts",
        "/admin",
        "/logout",
        "/search",
        "/categories",
        "/tags",
        "/sparks",
        "/spark",
        "/p",
    ];
    if reserved
        .iter()
        .any(|p| path == *p || path.starts_with(&format!("{p}/")))
    {
        bail!("OAKIS_LOGIN_PATH 不能占用保留路径: {path}");
    }

    Ok(path.to_string())
}
