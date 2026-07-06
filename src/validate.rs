use crate::error::{DecodeError, Result};
use crate::model::{CanopyArchive, DeviceId};
use crate::schedule;
use crate::store::DeviceScratch;
use std::collections::BTreeSet;

pub fn validate_archive(archive: &CanopyArchive) -> Result<()> {
    if archive.version == 0 || archive.version > 4 {
        return Err(DecodeError::InvalidVersion(archive.version));
    }
    let mut devices = BTreeSet::new();
    let mut scratch = DeviceScratch::new();
    for device in &archive.manifest.devices {
        if !devices.insert(device.id) {
            return Err(DecodeError::DuplicateId(device.id.0));
        }
        scratch.push(device.id);
        if device.alias.len() > 256 {
            return Err(DecodeError::InvalidField("device alias"));
        }
    }
    let _ = scratch.remembered_id();
    for link in &archive.links {
        if !devices.is_empty()
            && (!contains_device(&devices, link.from) || !contains_device(&devices, link.to))
        {
            return Err(DecodeError::MissingReference(link.from.0.max(link.to.0)));
        }
    }
    for page in &archive.pages {
        if !devices.is_empty() && !contains_device(&devices, page.device) {
            return Err(DecodeError::MissingReference(page.device.0));
        }
        if page.rows.len() > 100_000 {
            return Err(DecodeError::InvalidField("page rows"));
        }
    }
    for window in &archive.windows {
        let _ = schedule::active_window(&archive.windows, window.zone, window.start_minute);
    }
    Ok(())
}

fn contains_device(devices: &BTreeSet<DeviceId>, id: DeviceId) -> bool {
    devices.contains(&id)
}

pub fn estimate_archive_pressure(archive: &CanopyArchive) -> u64 {
    let mut pressure = archive.manifest.devices.len() as u64 * 11
        + archive.links.len() as u64 * 7
        + archive.windows.len() as u64 * 5
        + archive.journals.len() as u64 * 13;
    for page in &archive.pages {
        pressure = pressure.saturating_add(page.rows.len() as u64 * 3);
        for row in &page.rows {
            pressure = pressure.saturating_add(row.values.len() as u64);
        }
    }
    pressure
}
