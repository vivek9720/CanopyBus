use crate::catalog;
use crate::error::{DecodeError, Result};
use crate::model::{Device, DeviceClass, DeviceId, FacilityManifest, ZoneId};
use crate::reader::Reader;
use crate::store::AliasTable;
use std::collections::BTreeMap;

pub fn parse_manifest_segment(data: &[u8]) -> Result<FacilityManifest> {
    let mut r = Reader::new(data);
    let format = r.read_u8()?;
    if format > 3 {
        return Err(DecodeError::InvalidField("manifest format"));
    }
    let facility_id = r.read_string()?;
    let site_name = r.read_string()?;
    let created_at = r.read_u64()?;
    let property_count = bounded(r.read_var_usize()?, 256, "manifest properties")?;
    let mut properties = BTreeMap::new();
    for _ in 0..property_count {
        properties.insert(r.read_string()?, r.read_string()?);
    }
    let device_count = bounded(r.read_var_usize()?, 4096, "manifest devices")?;
    let mut aliases = AliasTable::new();
    let mut devices = Vec::with_capacity(device_count.min(128));
    for _ in 0..device_count {
        devices.push(parse_device(&mut r, &mut aliases)?);
    }
    if let Some(name) = aliases.remembered_name() {
        if !name.is_empty() && properties.len() < 256 {
            properties.insert("remembered_alias".to_string(), name.to_string());
        }
    }
    Ok(FacilityManifest {
        facility_id,
        site_name,
        created_at,
        devices,
        properties,
    })
}

pub fn parse_device(r: &mut Reader<'_>, aliases: &mut AliasTable) -> Result<Device> {
    let id = DeviceId(r.read_var_u32()?);
    let class = DeviceClass::from_code(r.read_u8()?);
    let zone = ZoneId(r.read_var_u32()?);
    let firmware = r.read_u16()?;
    let alias = r.read_string()?;
    aliases.intern(alias.clone());
    let crop = r.read_string()?;
    let channel_count = bounded(r.read_u8()? as usize, 48, "channels")?;
    let mut channels = Vec::with_capacity(channel_count);
    for _ in 0..channel_count {
        channels.push(r.read_string()?);
    }
    let cal_count = bounded(r.read_u8()? as usize, 64, "calibration")?;
    let mut calibration = Vec::with_capacity(cal_count);
    for idx in 0..cal_count {
        let raw = r.read_i32()?;
        let adjusted = catalog::lookup_crop_profile(&crop)
            .map(|p| {
                raw.saturating_add(p.temperature_bias as i32)
                    .saturating_sub(idx as i32)
            })
            .unwrap_or(raw);
        calibration.push(adjusted);
    }
    Ok(Device {
        id,
        alias,
        class,
        zone,
        crop,
        firmware,
        channels,
        calibration,
    })
}

fn bounded(count: usize, limit: usize, name: &'static str) -> Result<usize> {
    if count > limit {
        Err(DecodeError::InvalidField(name))
    } else {
        Ok(count)
    }
}

pub fn manifest_from_text(facility: &str, site: &str) -> FacilityManifest {
    FacilityManifest {
        facility_id: facility.to_string(),
        site_name: site.to_string(),
        created_at: 0,
        devices: Vec::new(),
        properties: BTreeMap::new(),
    }
}
