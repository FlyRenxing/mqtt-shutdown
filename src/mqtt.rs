use std::sync::Mutex;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;

use rumqttc::{Client, Event, Incoming, LastWill, MqttOptions, QoS};

use crate::config::AppConfig;
use crate::power::{self, PowerAction};
use crate::state::{self, ConnState, Pending};

pub struct MqttSession {
    client: Client,
}

impl MqttSession {
    fn stop(self) {
        let _ = self.client.disconnect();
    }
}

static SESSION: Mutex<Option<MqttSession>> = Mutex::new(None);

pub fn start(config: AppConfig) {
    stop();
    if config.host.is_empty() || config.client_id.is_empty() || config.topic.is_empty() {
        state::set_conn(ConnState::Disconnected);
        state::push_log("请先在「连接」页填写服务器、Client ID 和主题。");
        return;
    }
    let session = spawn(config);
    *SESSION.lock().expect("mqtt session") = Some(session);
}

pub fn stop() {
    if let Some(session) = SESSION.lock().expect("mqtt session").take() {
        session.stop();
    }
}

fn spawn(config: AppConfig) -> MqttSession {
    let mut options = MqttOptions::new(&config.client_id, &config.host, config.port);
    options.set_keep_alive(Duration::from_secs(60));
    options.set_clean_session(true);
    if !config.username.is_empty() {
        options.set_credentials(&config.username, &config.password);
    }
    options.set_last_will(LastWill::new(
        format!("{}/up", config.topic),
        "offline",
        QoS::AtLeastOnce,
        false,
    ));

    let (client, mut connection) = Client::new(options, 32);
    let worker = client.clone();
    let host = config.host.clone();
    let port = config.port;
    let topic = config.topic.clone();

    state::set_conn(ConnState::Connecting);
    state::push_log(format!("正在连接 {host}:{port} …"));

    thread::Builder::new()
        .name("mqtt-worker".into())
        .spawn(move || {
            for notification in connection.iter() {
                match notification {
                    Ok(Event::Incoming(Incoming::ConnAck(_))) => {
                        state::set_conn(ConnState::Connected);
                        if let Err(err) = worker.subscribe(&topic, QoS::AtLeastOnce) {
                            state::push_log(format!("订阅失败: {err}"));
                            continue;
                        }
                        let _ = worker.publish(
                            format!("{topic}/up"),
                            QoS::AtLeastOnce,
                            false,
                            "on",
                        );
                        state::push_log(format!("已连接并订阅 {topic}"));
                    }
                    Ok(Event::Incoming(Incoming::Publish(publish))) => {
                        let payload = String::from_utf8_lossy(&publish.payload);
                        handle_payload(&payload);
                    }
                    Ok(Event::Incoming(Incoming::Disconnect)) => {
                        state::set_conn(ConnState::Disconnected);
                        state::push_log("服务器断开连接");
                    }
                    Err(err) => {
                        state::set_conn(ConnState::Reconnecting);
                        state::push_log(format!("连接出错，将自动重试: {err}"));
                    }
                    _ => {}
                }
            }
            state::set_conn(ConnState::Disconnected);
            state::push_log("MQTT 线程已退出");
        })
        .expect("spawn MQTT thread");

    MqttSession { client }
}

fn handle_payload(payload: &str) {
    let text = payload.trim();
    state::push_log(format!("收到指令: {text}"));

    let delay = state::store()
        .settings
        .delay_secs
        .load(Ordering::Relaxed);
    let Some(action) = power::parse_command(text, delay) else {
        state::push_log("未识别的指令，已忽略");
        return;
    };

    if !matches!(action, PowerAction::Abort)
        && !state::store().settings.enabled.load(Ordering::Relaxed)
    {
        state::push_log("远程控制已关闭，指令被忽略");
        return;
    }

    apply_action(action);
}

pub fn apply_action(action: PowerAction) {
    match power::apply(action) {
        Ok(()) => match action {
            PowerAction::Abort => {
                state::set_pending(Pending::None);
                state::push_log("已取消关机 / 重启");
            }
            PowerAction::Shutdown { seconds } => {
                state::set_pending(Pending::Shutdown {
                    deadline: std::time::Instant::now() + Duration::from_secs(seconds as u64),
                });
                state::push_log(format!("将在 {seconds} 秒后关机"));
            }
            PowerAction::Reboot { seconds } => {
                state::set_pending(Pending::Reboot {
                    deadline: std::time::Instant::now() + Duration::from_secs(seconds as u64),
                });
                state::push_log(format!("将在 {seconds} 秒后重启"));
            }
        },
        Err(err) => state::push_log(err),
    }
}
