use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU32};
use std::sync::OnceLock;
use std::time::Instant;

use windows_reactor::AsyncSetState;

use crate::config::AppConfig;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnState {
    Connecting,
    Connected,
    Reconnecting,
    Disconnected,
}

impl ConnState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Connecting => "正在连接",
            Self::Connected => "已连接",
            Self::Reconnecting => "正在重连",
            Self::Disconnected => "已断开",
        }
    }

    pub fn is_ok(self) -> bool {
        matches!(self, Self::Connected)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pending {
    None,
    Shutdown { deadline: Instant },
    Reboot { deadline: Instant },
}

impl Pending {
    pub fn deadline(self) -> Option<Instant> {
        match self {
            Self::None => None,
            Self::Shutdown { deadline } | Self::Reboot { deadline } => Some(deadline),
        }
    }

    pub fn kind_label(self) -> Option<&'static str> {
        match self {
            Self::None => None,
            Self::Shutdown { .. } => Some("关机"),
            Self::Reboot { .. } => Some("重启"),
        }
    }

    pub fn remaining_secs(self) -> u32 {
        let Some(deadline) = self.deadline() else {
            return 0;
        };
        deadline.saturating_duration_since(Instant::now()).as_secs() as u32
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogEntry {
    pub time: String,
    pub text: String,
}

pub struct Settings {
    pub enabled: AtomicBool,
    pub delay_secs: AtomicU32,
}

pub struct UiBridge {
    pub conn: AsyncSetState<ConnState>,
    pub logs: AsyncSetState<Vec<LogEntry>>,
    pub pending: AsyncSetState<Pending>,
}

pub struct AppStore {
    pub config: Mutex<AppConfig>,
    pub settings: Settings,
    pub conn: Mutex<ConnState>,
    pub logs: Mutex<Vec<LogEntry>>,
    pub pending: Mutex<Pending>,
    bridge: Mutex<Option<UiBridge>>,
}

impl AppStore {
    fn new() -> Self {
        let config = AppConfig::load();
        Self {
            settings: Settings {
                enabled: AtomicBool::new(config.enabled),
                delay_secs: AtomicU32::new(config.clamp_delay()),
            },
            config: Mutex::new(config),
            conn: Mutex::new(ConnState::Connecting),
            logs: Mutex::new(Vec::new()),
            pending: Mutex::new(Pending::None),
            bridge: Mutex::new(None),
        }
    }
}

pub fn store() -> &'static AppStore {
    static STORE: OnceLock<AppStore> = OnceLock::new();
    STORE.get_or_init(AppStore::new)
}

pub fn config_snapshot() -> AppConfig {
    store().config.lock().expect("config").clone()
}

pub fn replace_config(next: AppConfig) -> Result<(), String> {
    next.save()?;
    let store = store();
    store
        .settings
        .enabled
        .store(next.enabled, std::sync::atomic::Ordering::Relaxed);
    store
        .settings
        .delay_secs
        .store(next.clamp_delay(), std::sync::atomic::Ordering::Relaxed);
    *store.config.lock().expect("config") = next;
    Ok(())
}

pub fn register_ui(bridge: UiBridge) {
    {
        let conn = *store().conn.lock().expect("conn");
        let logs = store().logs.lock().expect("logs").clone();
        let pending = *store().pending.lock().expect("pending");
        bridge.conn.call(conn);
        bridge.logs.call(logs);
        bridge.pending.call(pending);
    }
    *store().bridge.lock().expect("bridge") = Some(bridge);
}

pub fn unregister_ui() {
    *store().bridge.lock().expect("bridge") = None;
}

pub fn set_conn(state: ConnState) {
    *store().conn.lock().expect("conn") = state;
    if let Some(bridge) = store().bridge.lock().expect("bridge").as_ref() {
        bridge.conn.call(state);
    }
}

pub fn set_pending(pending: Pending) {
    *store().pending.lock().expect("pending") = pending;
    if let Some(bridge) = store().bridge.lock().expect("bridge").as_ref() {
        bridge.pending.call(pending);
    }
}

pub fn push_log(text: impl Into<String>) {
    let entry = LogEntry {
        time: now_hms(),
        text: text.into(),
    };
    let snapshot = {
        let mut logs = store().logs.lock().expect("logs");
        logs.push(entry);
        const MAX: usize = 40;
        if logs.len() > MAX {
            let drain = logs.len() - MAX;
            logs.drain(0..drain);
        }
        logs.clone()
    };
    if let Some(bridge) = store().bridge.lock().expect("bridge").as_ref() {
        bridge.logs.call(snapshot);
    }
}

pub fn now_hms() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let local = now.saturating_add(8 * 3600);
    let secs = local % 86400;
    format!(
        "{:02}:{:02}:{:02}",
        secs / 3600,
        (secs % 3600) / 60,
        secs % 60
    )
}
