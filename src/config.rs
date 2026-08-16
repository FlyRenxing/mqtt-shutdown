use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const APP_TITLE: &str = "MQTT关机";
pub const APP_ID: &str = "MqttShutdown";
pub const REPO_URL: &str = "https://github.com/FlyRenxing/mqtt-shutdown";
pub const DEFAULT_DELAY_SECS: u32 = 30;
pub const MIN_DELAY_SECS: f64 = 0.0;
pub const MAX_DELAY_SECS: f64 = 600.0;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_client_id")]
    pub client_id: String,
    #[serde(default = "default_topic")]
    pub topic: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_delay")]
    pub delay_secs: u32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            client_id: default_client_id(),
            topic: default_topic(),
            username: String::new(),
            password: String::new(),
            enabled: true,
            delay_secs: DEFAULT_DELAY_SECS,
        }
    }
}

impl AppConfig {
    pub fn load() -> Self {
        let path = settings_path();
        let Ok(text) = fs::read_to_string(&path) else {
            return Self::default();
        };
        serde_json::from_str(&text).unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), String> {
        let path = settings_path();
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(|err| format!("无法创建配置目录: {err}"))?;
        }
        let text = serde_json::to_string_pretty(self)
            .map_err(|err| format!("无法序列化配置: {err}"))?;
        fs::write(&path, text).map_err(|err| format!("无法写入配置: {err}"))
    }

    pub fn endpoint(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    pub fn clamp_delay(&self) -> u32 {
        self.delay_secs
            .min(MAX_DELAY_SECS as u32)
            .max(MIN_DELAY_SECS as u32)
    }
}

pub fn data_dir() -> PathBuf {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    base.join(APP_ID)
}

pub fn settings_path() -> PathBuf {
    data_dir().join("settings.json")
}

pub fn icon_path() -> Option<PathBuf> {
    let dest = data_dir().join("app.ico");
    if fs::create_dir_all(data_dir()).is_err() {
        return bundled_icon();
    }
    if fs::write(&dest, include_bytes!("../assets/app.ico")).is_ok() {
        return Some(dest);
    }
    bundled_icon()
}

fn bundled_icon() -> Option<PathBuf> {
    let dev = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/app.ico");
    dev.exists().then_some(dev)
}

pub fn default_host_hint() -> &'static str {
    "例如 mqtt.bemfa.com"
}

pub fn open_repo() {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let _ = std::process::Command::new("explorer.exe")
        .arg(REPO_URL)
        .creation_flags(CREATE_NO_WINDOW)
        .spawn();
}

fn default_host() -> String {
    String::new()
}

fn default_port() -> u16 {
    1883
}

fn default_client_id() -> String {
    String::new()
}

fn default_topic() -> String {
    String::new()
}

fn default_true() -> bool {
    true
}

fn default_delay() -> u32 {
    DEFAULT_DELAY_SECS
}
