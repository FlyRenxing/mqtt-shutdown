use std::os::windows::process::CommandExt;
use std::process::Command;

use crate::config::APP_TITLE;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PowerAction {
    Shutdown { seconds: u32 },
    Reboot { seconds: u32 },
    Abort,
}

pub fn apply(action: PowerAction) -> Result<(), String> {
    let output = match action {
        PowerAction::Shutdown { seconds } => run(&[
            "/s",
            "/t",
            &seconds.to_string(),
            "/c",
            APP_TITLE,
        ])?,
        PowerAction::Reboot { seconds } => run(&[
            "/r",
            "/t",
            &seconds.to_string(),
            "/c",
            APP_TITLE,
        ])?,
        PowerAction::Abort => run(&["/a"])?,
    };

    if output.status.success() {
        return Ok(());
    }

    let code = output.status.code().unwrap_or(-1);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let detail = first_line(&stderr).or_else(|| first_line(&stdout));

    if action == PowerAction::Abort && (code == 1116 || code == 1190) {
        return Ok(());
    }

    match detail {
        Some(text) => Err(format!("shutdown 失败 ({code}): {text}")),
        None => Err(format!("shutdown 失败，退出码 {code}")),
    }
}

fn run(args: &[&str]) -> Result<std::process::Output, String> {
    Command::new("shutdown")
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|err| format!("无法启动 shutdown.exe: {err}"))
}

fn first_line(text: &str) -> Option<String> {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_string)
}

pub fn parse_command(payload: &str, default_delay: u32) -> Option<PowerAction> {
    let raw = payload.trim();
    if raw.is_empty() {
        return None;
    }

    let lower = raw.to_ascii_lowercase();
    let (verb, delay) = split_verb_delay(&lower, default_delay);

    match verb {
        "off" | "shutdown" | "关机" | "關閉" | "关闭" => {
            Some(PowerAction::Shutdown { seconds: delay })
        }
        "reboot" | "restart" | "重启" | "重啟" => Some(PowerAction::Reboot { seconds: delay }),
        "on" | "cancel" | "abort" | "取消" => Some(PowerAction::Abort),
        _ => None,
    }
}

fn split_verb_delay(lower: &str, default_delay: u32) -> (&str, u32) {
    if let Some((verb, rest)) = lower.split_once('#') {
        let delay = rest
            .split(|c: char| !c.is_ascii_digit())
            .find(|s| !s.is_empty())
            .and_then(|s| s.parse().ok())
            .unwrap_or(default_delay);
        (verb.trim(), delay)
    } else {
        (lower.trim(), default_delay)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_switch_payloads() {
        assert_eq!(
            parse_command("off", 30),
            Some(PowerAction::Shutdown { seconds: 30 })
        );
        assert_eq!(
            parse_command("off#10", 30),
            Some(PowerAction::Shutdown { seconds: 10 })
        );
        assert_eq!(parse_command("on", 30), Some(PowerAction::Abort));
        assert_eq!(
            parse_command("reboot#5", 30),
            Some(PowerAction::Reboot { seconds: 5 })
        );
        assert_eq!(
            parse_command("关机", 15),
            Some(PowerAction::Shutdown { seconds: 15 })
        );
        assert_eq!(parse_command("hello", 30), None);
    }
}
