//! The system's sound: volume and device of the speakers and of the microphone, as the sound
//! server (PipeWire or PulseAudio) reports them, through `pactl`.

use std::process::Command;

use serde::Serialize;
use serde_json::Value;

use crate::wine::host_env;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Device {
    /// What `pactl` calls it, and what selects it.
    pub name: String,
    /// What the user reads (`Built-in Audio Analog Stereo`).
    pub description: String,
}

/// One direction of the sound: playing (speakers) or recording (microphone).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Side {
    pub devices: Vec<Device>,
    /// Name of the device in use.
    pub default: String,
    /// 0 to 100 (above 100 is possible on the system, shown as 100).
    pub volume: u8,
    pub muted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Status {
    pub output: Side,
    pub input: Side,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Output,
    Input,
}

impl Kind {
    pub fn parse(text: &str) -> Result<Self, String> {
        match text {
            "output" => Ok(Self::Output),
            "input" => Ok(Self::Input),
            other => Err(format!("unknown audio kind `{other}`")),
        }
    }

    fn list(self) -> &'static str {
        match self {
            Self::Output => "sinks",
            Self::Input => "sources",
        }
    }

    fn word(self) -> &'static str {
        match self {
            Self::Output => "sink",
            Self::Input => "source",
        }
    }
}

fn pactl(args: &[&str]) -> Result<String, String> {
    let output = host_env(&mut Command::new("pactl"))
        .args(args)
        .output()
        .map_err(|e| format!("pactl is not available: {e}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// The devices of a `pactl --format=json list sinks|sources` answer, microphones' own
/// "monitor" sources (what the speakers play) left out, and the volume of the one in `default`.
fn parse(json: &str, default: &str) -> Result<Side, String> {
    let list: Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let items = list.as_array().ok_or("unexpected pactl answer")?;
    let mut devices = Vec::new();
    let mut volume = 0;
    let mut muted = false;
    for item in items {
        let Some(name) = item.get("name").and_then(Value::as_str) else {
            continue;
        };
        if name.ends_with(".monitor") {
            continue;
        }
        let description = item
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or(name);
        devices.push(Device {
            name: name.to_string(),
            description: description.to_string(),
        });
        if name == default {
            muted = item.get("mute").and_then(Value::as_bool).unwrap_or(false);
            volume = item
                .get("volume")
                .and_then(Value::as_object)
                .and_then(|channels| channels.values().next())
                .and_then(|channel| channel.get("value_percent"))
                .and_then(Value::as_str)
                .and_then(|text| text.trim_end_matches('%').trim().parse::<u32>().ok())
                .map_or(0, |percent| percent.min(100) as u8);
        }
    }
    Ok(Side {
        devices,
        default: default.to_string(),
        volume,
        muted,
    })
}

fn side(kind: Kind) -> Result<Side, String> {
    let default = pactl(&[&format!("get-default-{}", kind.word())])?;
    let json = pactl(&["--format=json", "list", kind.list()])?;
    parse(&json, &default)
}

pub fn status() -> Result<Status, String> {
    Ok(Status {
        output: side(Kind::Output)?,
        input: side(Kind::Input)?,
    })
}

pub fn set_volume(kind: Kind, percent: u8) -> Result<(), String> {
    let target = format!("@DEFAULT_{}@", kind.word().to_uppercase());
    pactl(&[
        &format!("set-{}-volume", kind.word()),
        &target,
        &format!("{}%", percent.min(100)),
    ])
    .map(|_| ())
}

pub fn set_muted(kind: Kind, muted: bool) -> Result<(), String> {
    let target = format!("@DEFAULT_{}@", kind.word().to_uppercase());
    pactl(&[
        &format!("set-{}-mute", kind.word()),
        &target,
        if muted { "1" } else { "0" },
    ])
    .map(|_| ())
}

pub fn set_device(kind: Kind, name: &str) -> Result<(), String> {
    pactl(&[&format!("set-default-{}", kind.word()), name]).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SINKS: &str = r#"[
      {"index":56,"state":"SUSPENDED","name":"alsa_output.pci.analog-stereo","description":"Built-in Audio Analog Stereo",
       "mute":false,"volume":{"front-left":{"value":42598,"value_percent":"65%","db":"-11.23 dB"},"front-right":{"value":42598,"value_percent":"65%","db":"-11.23 dB"}}},
      {"index":57,"name":"bluez_output.AA.1","description":"Headphones","mute":true,
       "volume":{"mono":{"value":70000,"value_percent":"107%","db":"1 dB"}}}
    ]"#;

    #[test]
    fn the_volume_and_mute_of_the_default_device_are_read() {
        let side = parse(SINKS, "alsa_output.pci.analog-stereo").unwrap();
        assert_eq!((side.volume, side.muted), (65, false));
        assert_eq!(side.devices.len(), 2);
        assert_eq!(side.devices[0].description, "Built-in Audio Analog Stereo");
        // Above 100 % reads as 100, and a muted device says so.
        let side = parse(SINKS, "bluez_output.AA.1").unwrap();
        assert_eq!((side.volume, side.muted), (100, true));
    }

    #[test]
    fn microphones_do_not_list_what_the_speakers_play() {
        let sources = r#"[
          {"name":"alsa_output.pci.analog-stereo.monitor","description":"Monitor of Built-in Audio"},
          {"name":"alsa_input.pci.analog-stereo","description":"Built-in Audio Analog Stereo","mute":false,
           "volume":{"front-left":{"value":30000,"value_percent":"46%"}}}
        ]"#;
        let side = parse(sources, "alsa_input.pci.analog-stereo").unwrap();
        assert_eq!(side.devices.len(), 1);
        assert_eq!(side.volume, 46);
    }

    #[test]
    fn a_default_that_is_not_listed_reads_as_silent_without_failing() {
        let side = parse(SINKS, "gone").unwrap();
        assert_eq!((side.volume, side.muted), (0, false));
        assert!(parse("not json", "x").is_err());
        assert_eq!(Kind::parse("output"), Ok(Kind::Output));
        assert!(Kind::parse("loud").is_err());
    }
}
