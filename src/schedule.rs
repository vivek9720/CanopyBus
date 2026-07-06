use crate::calibration;
use crate::error::{DecodeError, Result};
use crate::model::{ScheduleWindow, ZoneId};
use crate::reader::Reader;
use crate::store::WindowRing;

pub fn parse_schedule_segment(data: &[u8]) -> Result<Vec<ScheduleWindow>> {
    let mut r = Reader::new(data);
    let count = r.read_var_usize()?;
    if count > 8192 {
        return Err(DecodeError::InvalidField("schedule count"));
    }
    let mut out = Vec::with_capacity(count.min(512));
    let mut ring = WindowRing::new();
    for idx in 0..count {
        let zone = ZoneId(r.read_var_u32()?);
        let start_minute = r.read_u16()? % 1440;
        let duration = r.read_u16()? % 1440;
        let target = r.read_i32()?;
        let recurrence = r.read_u8()?;
        let label = if r.remaining() > 0 {
            r.read_string().unwrap_or_else(|_| format!("window-{idx}"))
        } else {
            format!("window-{idx}")
        };
        let recipe = calibration::recipe_for_zone(zone.0 as usize, recurrence);
        let adjusted_target = target.saturating_add(recipe.temperature_offset as i32);
        ring.push(adjusted_target, recurrence);
        out.push(ScheduleWindow {
            zone,
            start_minute,
            end_minute: start_minute.wrapping_add(duration) % 1440,
            target: adjusted_target,
            recurrence,
            label,
        });
    }
    let _ = ring.marked_sum();
    Ok(out)
}

pub fn active_window(
    windows: &[ScheduleWindow],
    zone: ZoneId,
    minute: u16,
) -> Option<&ScheduleWindow> {
    windows.iter().find(|w| {
        w.zone == zone
            && if w.start_minute <= w.end_minute {
                minute >= w.start_minute && minute <= w.end_minute
            } else {
                minute >= w.start_minute || minute <= w.end_minute
            }
    })
}
