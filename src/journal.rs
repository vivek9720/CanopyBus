use crate::error::{DecodeError, Result};
use crate::model::{DeviceId, JournalEntry, JournalKind};
use crate::reader::Reader;
use crate::store::{ByteMirror, ReplayWindow};

pub fn parse_journal_segment(data: &[u8]) -> Result<Vec<JournalEntry>> {
    let mut r = Reader::new(data);
    let count = r.read_var_usize()?;
    if count > 16384 {
        return Err(DecodeError::InvalidField("journal count"));
    }
    let mut out = Vec::with_capacity(count.min(512));
    let mut window = ReplayWindow::new();
    let mut mirror = ByteMirror::new();
    let mut base = 0u64;
    for idx in 0..count {
        base = base.wrapping_add(r.read_var_u32()? as u64);
        let kind = JournalKind::from_code(r.read_u8()?);
        let device = DeviceId(r.read_var_u32()?);
        let severity = r.read_u8()? % 8;
        let message = if r.remaining() > 0 {
            r.read_string().unwrap_or_else(|_| format!("event-{idx}"))
        } else {
            format!("event-{idx}")
        };
        let payload_len = if r.remaining() > 0 {
            (r.read_u8()? as usize).min(r.remaining())
        } else {
            0
        };
        mirror.absorb(message.as_bytes());
        let payload = r.read_bytes(payload_len)?.to_vec();
        mirror.absorb(&payload);
        let entry = JournalEntry {
            timestamp: base,
            kind,
            device,
            severity,
            message,
            payload,
        };
        window.push(entry.clone());
        out.push(entry);
    }
    let _ = window.anchor_message_len();
    let _ = mirror.saved_digest();
    Ok(out)
}

pub fn summarize_journal(entries: &[JournalEntry]) -> (usize, usize, u64) {
    let mut alarms = 0usize;
    let mut changes = 0usize;
    let mut last = 0u64;
    for entry in entries {
        last = last.max(entry.timestamp);
        match entry.kind {
            JournalKind::Alarm => alarms += 1,
            JournalKind::ConfigChange => changes += 1,
            _ => {}
        }
    }
    (alarms, changes, last)
}
