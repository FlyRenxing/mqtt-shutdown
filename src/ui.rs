use std::sync::atomic::Ordering;
use std::time::Duration;

use windows_reactor::*;

use crate::autostart;
use crate::config::{
    APP_TITLE, AppConfig, MAX_DELAY_SECS, MIN_DELAY_SECS, REPO_URL, default_host_hint,
    settings_path,
};
use crate::mqtt;
use crate::power::PowerAction;
use crate::state::{self, ConnState, LogEntry, Pending, UiBridge};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Home,
    Control,
    Connect,
    Settings,
}

impl Page {
    fn tag(self) -> &'static str {
        match self {
            Self::Home => "home",
            Self::Control => "control",
            Self::Connect => "connect",
            Self::Settings => "prefs",
        }
    }

    fn from_tag(tag: &str) -> Self {
        match tag {
            "control" => Self::Control,
            "connect" => Self::Connect,
            "prefs" => Self::Settings,
            _ => Self::Home,
        }
    }
}

#[derive(Clone, PartialEq)]
struct HomeProps {
    conn: ConnState,
    logs: Vec<LogEntry>,
    pending: Pending,
    remaining: u32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct ControlProps {
    pending: Pending,
    remaining: u32,
}

pub fn app(cx: &mut RenderCx) -> Element {
    let (page, set_page) = cx.use_state(Page::Home);
    let (conn, set_conn) = cx.use_async_state(ConnState::Connecting);
    let (logs, set_logs) = cx.use_async_state(Vec::<LogEntry>::new());
    let (pending, set_pending) = cx.use_async_state(Pending::None);
    let (_tick, bump_tick) = cx.use_reducer(0_u32);
    let start_hidden = crate::tray::take_hidden_flag();

    cx.use_effect_with_cleanup((), {
        let set_conn = set_conn.clone();
        let set_logs = set_logs.clone();
        let set_pending = set_pending.clone();
        move || {
            state::register_ui(UiBridge {
                conn: set_conn,
                logs: set_logs,
                pending: set_pending,
            });
            mqtt::start(state::config_snapshot());
            Some(|| {
                state::unregister_ui();
            })
        }
    });

    cx.use_effect_with_cleanup((), {
        move || {
            let attached = std::rc::Rc::new(std::cell::Cell::new(false));
            let hidden = start_hidden;
            let timer = DispatcherTimer::new(Duration::from_millis(50), {
                let attached = attached.clone();
                move || {
                    if attached.get() {
                        return;
                    }
                    if crate::tray::attach_main_window() {
                        if hidden {
                            crate::tray::hide_to_tray();
                        }
                        attached.set(true);
                    }
                }
            })
            .ok();
            Some(move || {
                if let Some(timer) = timer {
                    let _ = timer.stop();
                }
            })
        }
    });

    cx.use_effect_with_cleanup((), {
        let bump_tick = bump_tick.clone();
        move || {
            let timer = DispatcherTimer::new(Duration::from_millis(250), move || {
                bump_tick.call(|n| n.wrapping_add(1));
            })
            .ok();
            Some(move || {
                if let Some(timer) = timer {
                    let _ = timer.stop();
                }
            })
        }
    });

    let remaining = pending.remaining_secs();
    let content = match page {
        Page::Home => component(
            home_page,
            HomeProps {
                conn,
                logs: logs.clone(),
                pending,
                remaining,
            },
        ),
        Page::Control => component(control_page, ControlProps { pending, remaining }),
        Page::Connect => component(connect_page, ()),
        Page::Settings => component(settings_page, ()),
    };

    let nav = NavigationView::new(
        [
            NavViewItem::new("主页").tag("home").icon(Symbol::Home),
            NavViewItem::new("电源").tag("control").icon(Symbol::Remote),
            NavViewItem::new("连接").tag("connect").icon(Symbol::Globe),
            NavViewItem::new("设置").tag("prefs").icon(Symbol::Setting),
        ],
        content,
    )
    .selected_tag(page.tag())
    .on_selection_changed(move |tag: String| set_page.call(Page::from_tag(&tag)))
    .pane_display_mode(NavigationViewPaneDisplayMode::Left)
    .pane_title(APP_TITLE)
    .open_pane_length(228.0)
    .settings_visible(false)
    .back_button_visible(false)
    .pane_toggle_button_visible(true);

    let title_bar = TitleBar::new(APP_TITLE)
        .subtitle("远程电源")
        .pane_toggle_button_visible(false)
        .back_button_visible(false);

    grid((title_bar.grid_row(0), nav.grid_row(1)))
        .rows([GridLength::Auto, GridLength::Star(1.0)])
        .columns([GridLength::Star(1.0)])
        .into()
}

fn home_page(props: &HomeProps, _cx: &mut RenderCx) -> Element {
    let config = state::config_snapshot();
    let remaining = props.remaining;
    let pending_open = props.pending != Pending::None;
    let pending_kind = props.pending.kind_label().unwrap_or("关机");

    let status_title = props.conn.label();
    let status_desc = if props.conn.is_ok() {
        format!("{}  ·  主题 {}", config.endpoint(), config.topic)
    } else {
        format!("正在连接 {}", config.endpoint())
    };

    let log_items: Vec<Element> = if props.logs.is_empty() {
        vec![caption("连接成功后，收到的指令会显示在这里。")
            .opacity(0.7)
            .wrap()
            .into()]
    } else {
        props.logs.iter()
            .rev()
            .take(16)
            .map(|entry| {
                caption(format!("{}    {}", entry.time, entry.text))
                    .wrap()
                    .into()
            })
            .collect()
    };

    page_frame(
        "主页",
        "通过 MQTT 远程关闭或重启这台电脑。关闭窗口后程序会留在托盘继续运行。",
        vstack((
            InfoBar::new(format!("即将{pending_kind}"))
                .message(format!(
                    "系统将在 {remaining} 秒后{pending_kind}。可在「电源」页或下发取消指令中止。"
                ))
                .warning()
                .is_closable(false)
                .is_open(pending_open),
            status_card(status_title, &status_desc, props.conn),
            section_label("最近活动"),
            card(
                scroll_view(vstack(log_items).spacing(6.0))
                    .min_height(180.0)
                    .max_height(280.0),
            ),
            section_label("指令"),
            card(
                caption("off / 关机    ·    on / 取消    ·    reboot / 重启    ·    off#10 表示 10 秒后关机")
                    .opacity(0.78)
                    .wrap(),
            ),
        ))
        .spacing(12.0),
    )
}

fn control_page(props: &ControlProps, cx: &mut RenderCx) -> Element {
    let config = state::config_snapshot();
    let (enabled, set_enabled) = cx.use_state(config.enabled);
    let (delay, set_delay) = cx.use_state(config.clamp_delay() as f64);
    let (confirm, set_confirm) = cx.use_state(false);
    let pending = props.pending;

    let delay_secs = delay.clamp(MIN_DELAY_SECS, MAX_DELAY_SECS).round() as u32;
    state::store()
        .settings
        .enabled
        .store(enabled, Ordering::Relaxed);
    state::store()
        .settings
        .delay_secs
        .store(delay_secs, Ordering::Relaxed);

    let on_toggle = {
        let set_enabled = set_enabled.clone();
        move |value: bool| {
            set_enabled.call(value);
            let mut next = state::config_snapshot();
            next.enabled = value;
            let _ = state::replace_config(next);
        }
    };
    let on_delay = {
        let set_delay = set_delay.clone();
        move |value: f64| {
            let value = value.clamp(MIN_DELAY_SECS, MAX_DELAY_SECS);
            set_delay.call(value);
            let mut next = state::config_snapshot();
            next.delay_secs = value.round() as u32;
            let _ = state::replace_config(next);
        }
    };

    let remaining = props.remaining;
    let pending_open = pending != Pending::None;
    let pending_kind = pending.kind_label().unwrap_or("关机");

    page_frame(
        "电源",
        "远程指令到达时，按这里的开关和倒计时执行。",
        vstack((
            InfoBar::new(format!("即将{pending_kind}"))
                .message(format!("还剩 {remaining} 秒。点取消可中止。"))
                .warning()
                .is_closable(false)
                .is_open(pending_open),
            settings_card(
                "允许远程关机",
                "打开后，收到关机或重启指令会执行。关闭后只连着服务器，不会被远程关掉或重启；取消指令仍然有效。",
                trailing_switch(enabled, on_toggle),
            ),
            settings_card(
                "默认倒计时",
                "未指定秒数的 off / reboot 会使用这个值。范围 0–600 秒。",
                NumberBox::new(delay_secs as f64)
                    .range(MIN_DELAY_SECS, MAX_DELAY_SECS)
                    .on_value_changed(on_delay)
                    .width(140.0),
            ),
            section_label("本机测试"),
            card(
                vstack((
                    caption("测试走同一条关机路径，请先保存未完成的工作。")
                        .opacity(0.72)
                        .wrap(),
                    hstack((
                        button("测试关机").accent().on_click({
                            let set_confirm = set_confirm.clone();
                            move || set_confirm.call(true)
                        }),
                        button("取消关机").on_click(|| mqtt::apply_action(PowerAction::Abort)),
                    ))
                    .spacing(8.0),
                ))
                .spacing(12.0),
            ),
            ContentDialog::new("测试关机？")
                .content(format!(
                    "将在 {delay_secs} 秒后关闭这台电脑，与远程指令相同。"
                ))
                .primary_button_text("关机")
                .close_button_text("返回")
                .is_open(confirm)
                .on_closed({
                    let set_confirm = set_confirm.clone();
                    move |result: ContentDialogResult| {
                        set_confirm.call(false);
                        if result == ContentDialogResult::Primary {
                            mqtt::apply_action(PowerAction::Shutdown {
                                seconds: delay_secs,
                            });
                        }
                    }
                }),
        ))
        .spacing(8.0),
    )
}

fn connect_page(_: &(), cx: &mut RenderCx) -> Element {
    let saved = state::config_snapshot();
    let (host, set_host) = cx.use_state(saved.host.clone());
    let (port, set_port) = cx.use_state(saved.port as f64);
    let (client_id, set_client_id) = cx.use_state(saved.client_id.clone());
    let (topic, set_topic) = cx.use_state(saved.topic.clone());
    let (username, set_username) = cx.use_state(saved.username.clone());
    let (password, set_password) = cx.use_state(saved.password.clone());
    let (status, set_status) = cx.use_state(String::new());

    let draft = AppConfig {
        host: host.trim().to_string(),
        port: port.round().clamp(1.0, 65535.0) as u16,
        client_id: client_id.trim().to_string(),
        topic: topic.trim().to_string(),
        username: username.trim().to_string(),
        password: password.clone(),
        enabled: saved.enabled,
        delay_secs: saved.delay_secs,
    };
    let dirty = draft.host != saved.host
        || draft.port != saved.port
        || draft.client_id != saved.client_id
        || draft.topic != saved.topic
        || draft.username != saved.username
        || draft.password != saved.password;

    let save = {
        let draft = draft.clone();
        let set_status = set_status.clone();
        move || {
            if draft.host.is_empty() || draft.client_id.is_empty() || draft.topic.is_empty() {
                set_status.call("服务器、Client ID 和主题不能为空。".into());
                return;
            }
            match state::replace_config(draft.clone()) {
                Ok(()) => {
                    mqtt::start(draft.clone());
                    set_status.call("已保存，正在重新连接。".into());
                }
                Err(err) => set_status.call(err),
            }
        }
    };

    page_frame(
        "连接",
        "任意 MQTT 服务器都可以。巴法云把私钥当作 Client ID，一般不用填用户名和密码。",
        vstack((
            card(
                vstack((
                    grid((
                        TextBox::new(host)
                            .header("服务器")
                            .placeholder_text(default_host_hint())
                            .on_text_changed(set_host)
                            .grid_column(0)
                            .min_width(0.0),
                        TextBox::new((port.round().clamp(1.0, 65535.0) as u16).to_string())
                            .header("端口")
                            .placeholder_text("1883")
                            .on_text_changed({
                                let set_port = set_port.clone();
                                move |value: String| {
                                    let digits: String =
                                        value.chars().filter(|c| c.is_ascii_digit()).collect();
                                    if let Ok(parsed) = digits.parse::<f64>() {
                                        set_port.call(parsed.clamp(1.0, 65535.0));
                                    }
                                }
                            })
                            .grid_column(1)
                            .width(120.0),
                    ))
                    .columns([GridLength::Star(1.0), GridLength::Auto])
                    .column_spacing(12.0),
                    TextBox::new(client_id)
                        .header("Client ID")
                        .placeholder_text("设备私钥或客户端标识")
                        .on_text_changed(set_client_id),
                    TextBox::new(topic)
                        .header("主题")
                        .placeholder_text("订阅主题")
                        .on_text_changed(set_topic),
                    TextBox::new(username)
                        .header("用户名（可选）")
                        .on_text_changed(set_username),
                    PasswordBox::new()
                        .value(password)
                        .header("密码（可选）")
                        .on_password_changed(set_password),
                ))
                .spacing(12.0),
            ),
            hstack((
                button("保存并连接")
                    .accent()
                    .enabled(dirty)
                    .on_click(save),
                caption(if status.is_empty() && dirty {
                    "有未保存的更改".into()
                } else {
                    status
                })
                .opacity(0.72)
                .vertical_alignment(VerticalAlignment::Center),
            ))
            .spacing(12.0),
        ))
        .spacing(12.0),
    )
}

fn settings_page(_: &(), cx: &mut RenderCx) -> Element {
    let (autostart_on, set_autostart) = cx.use_state(autostart::is_enabled());
    let (message, set_message) = cx.use_state(String::new());

    let on_autostart = {
        let set_autostart = set_autostart.clone();
        let set_message = set_message.clone();
        move |value: bool| {
            match autostart::set_enabled(value) {
                Ok(()) => {
                    set_autostart.call(value);
                    set_message.call(String::new());
                }
                Err(err) => set_message.call(err),
            }
        }
    };

    page_frame(
        "设置",
        "开机启动和窗口行为。",
        vstack((
            settings_card(
                "开机时启动",
                "登录 Windows 后在托盘运行，不弹出主窗口。",
                trailing_switch(autostart_on, on_autostart),
            ),
            settings_card(
                "关闭窗口",
                "点击关闭时最小化到托盘，远程关机继续有效。在托盘图标上右键可退出。",
                caption("已启用").opacity(0.72),
            ),
            if message.is_empty() {
                Element::Empty
            } else {
                InfoBar::new("无法更改开机启动")
                    .message(message)
                    .error()
                    .is_closable(false)
                    .into()
            },
            section_label("关于"),
            card(
                vstack((
                    body_strong("MQTT关机"),
                    caption("用 MQTT 指令关闭或重启这台 Windows 电脑。").opacity(0.72),
                    caption(REPO_URL).opacity(0.72).wrap(),
                    button("打开项目主页").on_click(crate::config::open_repo),
                    caption(format!("配置文件  {}", settings_path().display()))
                        .opacity(0.6)
                        .wrap(),
                ))
                .spacing(6.0),
            ),
        ))
        .spacing(8.0),
    )
}

fn page_frame(heading: &str, subtitle: &str, body: impl Into<Element>) -> Element {
    let content = vstack((
        text_block(heading).font_size(28.0).bold(),
        text_block(subtitle)
            .foreground(ThemeRef::SecondaryText)
            .horizontal_alignment(HorizontalAlignment::Left)
            .max_width(800.0)
            .wrap(),
        body.into(),
    ))
    .spacing(16.0)
    .margin(Thickness {
        left: 36.0,
        top: 24.0,
        right: 36.0,
        bottom: 36.0,
    })
    .horizontal_alignment(HorizontalAlignment::Stretch);

    scroll_viewer(content).into()
}

fn section_label(text: &str) -> Element {
    body_strong(text)
        .margin(Thickness {
            left: 4.0,
            top: 8.0,
            right: 0.0,
            bottom: 0.0,
        })
        .into()
}

fn card(child: impl Into<Element>) -> Element {
    border(child.into())
        .background(ThemeRef::CardBackground)
        .border_brush(ThemeRef::CardStroke)
        .border_thickness(Thickness::uniform(1.0))
        .corner_radius(8.0)
        .padding(Thickness::uniform(16.0))
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .into()
}

fn trailing_switch(is_on: bool, on_toggled: impl IntoCallback<bool>) -> ToggleSwitch {
    ToggleSwitch::new(is_on)
        .on_content("")
        .off_content("")
        .min_width(0.0)
        .on_toggled(on_toggled)
}

fn settings_card(title_text: &str, desc: &str, trailing: impl Into<Element>) -> Element {
    card(
        grid((
            vstack((
                text_block(title_text).font_size(14.0).semibold(),
                text_block(desc)
                    .font_size(12.0)
                    .foreground(ThemeRef::SecondaryText)
                    .wrap(),
            ))
            .spacing(4.0)
            .min_width(0.0)
            .grid_column(0)
            .vertical_alignment(VerticalAlignment::Center),
            hstack((trailing.into(),))
                .grid_column(1)
                .vertical_alignment(VerticalAlignment::Center)
                .horizontal_alignment(HorizontalAlignment::Right),
        ))
        .columns([GridLength::Star(1.0), GridLength::Auto])
        .column_spacing(16.0),
    )
}

fn status_card(title_text: &str, desc: &str, conn: ConnState) -> Element {
    let color = match conn {
        ConnState::Connected => Color::rgb(16, 137, 62),
        ConnState::Connecting | ConnState::Reconnecting => Color::rgb(196, 140, 12),
        ConnState::Disconnected => Color::rgb(196, 43, 28),
    };
    card(
        hstack((
            text_block("●").foreground(color).font_size(18.0),
            vstack((body_strong(title_text), caption(desc).opacity(0.72).wrap())).spacing(4.0),
        ))
        .spacing(12.0)
        .vertical_alignment(VerticalAlignment::Center),
    )
}
