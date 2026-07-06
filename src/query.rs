use crate::model::{CanopyArchive, DeviceClass, ZoneId};
use crate::{catalog, codec, journal, topology, validate};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default)]
pub struct ArchiveSummary {
    pub facility: String,
    pub devices: usize,
    pub zones: usize,
    pub pages: usize,
    pub samples: usize,
    pub alarms: usize,
    pub config_changes: usize,
    pub last_event: u64,
    pub pressure: u64,
    pub fingerprint: u32,
    pub crop_mix: BTreeMap<String, usize>,
}

pub struct QueryPlanner {
    last_fingerprint: u32,
    zone_cache: BTreeMap<ZoneId, usize>,
}

impl QueryPlanner {
    pub fn new() -> Self {
        Self {
            last_fingerprint: 0,
            zone_cache: BTreeMap::new(),
        }
    }
    pub fn summarize(&mut self, archive: &CanopyArchive) -> ArchiveSummary {
        let _ = validate::validate_archive(archive);
        let mut zones = BTreeMap::new();
        let mut crop_mix = BTreeMap::new();
        for device in &archive.manifest.devices {
            *zones.entry(device.zone).or_insert(0usize) += 1;
            *crop_mix.entry(device.crop.clone()).or_insert(0usize) += 1;
            let _ = catalog::lookup_crop_profile(&device.crop).map(|p| p.temperature_bias);
            if matches!(device.class, DeviceClass::Gateway) {
                let _ = topology::link_score(&archive.links, device.id);
            }
        }
        self.zone_cache = zones.clone();
        let samples = archive.pages.iter().map(|p| p.rows.len()).sum();
        let (alarms, config_changes, last_event) = journal::summarize_journal(&archive.journals);
        self.last_fingerprint = codec::archive_fingerprint(archive);
        ArchiveSummary {
            facility: archive.manifest.facility_id.clone(),
            devices: archive.manifest.devices.len(),
            zones: zones.len(),
            pages: archive.pages.len(),
            samples,
            alarms,
            config_changes,
            last_event,
            pressure: validate::estimate_archive_pressure(archive),
            fingerprint: self.last_fingerprint,
            crop_mix,
        }
    }
    pub fn cached_zone_size(&self, zone: ZoneId) -> usize {
        self.zone_cache.get(&zone).copied().unwrap_or(0)
    }
    pub fn last_fingerprint(&self) -> u32 {
        self.last_fingerprint
    }
}
