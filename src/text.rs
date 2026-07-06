use crate::error::{DecodeError, Result};
use crate::manifest;
use crate::model::{
    CanopyArchive, Device, DeviceClass, DeviceId, JournalEntry, JournalKind, Link, LinkKind,
    RuleSet, SampleKind, SamplePage, SampleRow, ScheduleWindow, ZoneId,
};
use crate::{rules, validate};

pub fn parse_text_archive(data: &[u8]) -> Result<CanopyArchive> {
    let text = core::str::from_utf8(data).map_err(|_| DecodeError::InvalidUtf8)?;
    let mut archive = CanopyArchive::new(1);
    archive.manifest = manifest::manifest_from_text("text-import", "unnamed-site");
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let Some(kind) = parts.next() else {
            continue;
        };
        match kind {
            "facility" => {
                archive.manifest.facility_id = parts.next().unwrap_or("site").to_string();
                archive.manifest.site_name = parts.collect::<Vec<_>>().join(" ");
            }
            "property" => {
                let key = parts.next().unwrap_or("key").to_string();
                archive
                    .manifest
                    .properties
                    .insert(key, parts.collect::<Vec<_>>().join(" "));
            }
            "device" => archive
                .manifest
                .devices
                .push(parse_device_line(parts.collect())?),
            "link" => archive.links.push(parse_link_line(parts.collect())?),
            "window" => archive.windows.push(parse_window_line(parts.collect())?),
            "sample" => archive.pages.push(parse_sample_line(parts.collect())?),
            "journal" => archive.journals.push(parse_journal_line(parts.collect())?),
            "rule" => archive.rules.push(parse_rule_line(parts.collect())?),
            "note" => archive.notes.push(parts.collect::<Vec<_>>().join(" ")),
            other => return Err(DecodeError::InvalidText(other.to_string())),
        }
    }
    if !archive.rules.is_empty() {
        let _ = rules::evaluate_rules(&archive);
    }
    let _ = validate::validate_archive(&archive);
    Ok(archive)
}

fn parse_device_line(fields: Vec<&str>) -> Result<Device> {
    if fields.len() < 5 {
        return Err(DecodeError::InvalidField("device text"));
    }
    let class = match fields[1] {
        "gateway" => DeviceClass::Gateway,
        "climate" => DeviceClass::ClimateSensor,
        "moisture" => DeviceClass::MoistureProbe,
        "valve" => DeviceClass::Valve,
        "pump" => DeviceClass::NutrientPump,
        "light" => DeviceClass::LightingRail,
        "camera" => DeviceClass::Camera,
        _ => DeviceClass::Unknown(hash_name(fields[1]) as u8),
    };
    let mut channels = Vec::new();
    let mut calibration = Vec::new();
    for extra in fields.iter().skip(5) {
        if let Some(value) = extra.strip_prefix("cal=") {
            calibration.push(value.parse::<i32>().unwrap_or(0));
        } else {
            channels.push((*extra).to_string());
        }
    }
    Ok(Device {
        id: DeviceId(parse_u32(fields[0])?),
        class,
        zone: ZoneId(parse_u32(fields[2])?),
        alias: fields[3].to_string(),
        crop: fields[4].to_string(),
        firmware: 1,
        channels,
        calibration,
    })
}

fn parse_link_line(fields: Vec<&str>) -> Result<Link> {
    if fields.len() < 4 {
        return Err(DecodeError::InvalidField("link text"));
    }
    let kind = match fields[2] {
        "parent" => LinkKind::Parent,
        "neighbor" => LinkKind::Neighbor,
        "backup" => LinkKind::Backup,
        "controls" => LinkKind::Controls,
        "observes" => LinkKind::Observes,
        _ => LinkKind::Unknown(hash_name(fields[2]) as u8),
    };
    Ok(Link {
        from: DeviceId(parse_u32(fields[0])?),
        to: DeviceId(parse_u32(fields[1])?),
        kind,
        weight: fields[3].parse::<i16>().unwrap_or(1),
    })
}

fn parse_window_line(fields: Vec<&str>) -> Result<ScheduleWindow> {
    if fields.len() < 5 {
        return Err(DecodeError::InvalidField("window text"));
    }
    Ok(ScheduleWindow {
        zone: ZoneId(parse_u32(fields[0])?),
        start_minute: fields[1].parse::<u16>().unwrap_or(0) % 1440,
        end_minute: fields[2].parse::<u16>().unwrap_or(0) % 1440,
        target: fields[3].parse::<i32>().unwrap_or(0),
        recurrence: fields[4].parse::<u8>().unwrap_or(0),
        label: fields.get(5).copied().unwrap_or("window").to_string(),
    })
}

fn parse_sample_line(fields: Vec<&str>) -> Result<SamplePage> {
    if fields.len() < 4 {
        return Err(DecodeError::InvalidField("sample text"));
    }
    let kind = match fields[1] {
        "temp" => SampleKind::Temperature,
        "humidity" => SampleKind::Humidity,
        "co2" => SampleKind::Co2,
        "moisture" => SampleKind::Moisture,
        "ph" => SampleKind::Ph,
        "ec" => SampleKind::Ec,
        "flow" => SampleKind::Flow,
        "light" => SampleKind::Light,
        _ => SampleKind::Unknown(hash_name(fields[1]) as u8),
    };
    let rows = fields
        .iter()
        .skip(3)
        .enumerate()
        .map(|(idx, item)| SampleRow {
            timestamp_delta: idx as u32 * 60,
            values: item
                .split(',')
                .map(|v| v.parse::<i64>().unwrap_or(0))
                .collect(),
        })
        .collect();
    Ok(SamplePage {
        device: DeviceId(parse_u32(fields[0])?),
        kind,
        base_timestamp: fields[2].parse::<u64>().unwrap_or(0),
        scale: 1,
        rows,
    })
}

fn parse_journal_line(fields: Vec<&str>) -> Result<JournalEntry> {
    if fields.len() < 5 {
        return Err(DecodeError::InvalidField("journal text"));
    }
    let kind = match fields[1] {
        "boot" => JournalKind::Boot,
        "alarm" => JournalKind::Alarm,
        "ack" => JournalKind::OperatorAck,
        "config" => JournalKind::ConfigChange,
        "rule" => JournalKind::RuleAction,
        "gap" => JournalKind::SampleGap,
        _ => JournalKind::Unknown(hash_name(fields[1]) as u8),
    };
    Ok(JournalEntry {
        timestamp: fields[0].parse::<u64>().unwrap_or(0),
        kind,
        device: DeviceId(parse_u32(fields[2])?),
        severity: fields[3].parse::<u8>().unwrap_or(0),
        message: fields[4..].join(" "),
        payload: Vec::new(),
    })
}

fn parse_rule_line(fields: Vec<&str>) -> Result<RuleSet> {
    if fields.len() < 2 {
        return Err(DecodeError::InvalidField("rule text"));
    }
    let mut bytecode = Vec::new();
    for token in fields.iter().skip(1) {
        if let Ok(value) = u8::from_str_radix(token.trim_start_matches("0x"), 16) {
            bytecode.push(value);
        }
    }
    Ok(RuleSet {
        name: fields[0].to_string(),
        bytecode,
        labels: Vec::new(),
    })
}

fn parse_u32(s: &str) -> Result<u32> {
    s.parse::<u32>()
        .map_err(|_| DecodeError::InvalidField("number"))
}
fn hash_name(s: &str) -> u32 {
    s.as_bytes()
        .iter()
        .fold(2166136261u32, |h, b| (h ^ *b as u32).wrapping_mul(16777619))
}
