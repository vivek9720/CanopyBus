use crate::model::{CanopyArchive, Device, DeviceClass, LinkKind};
use crate::reader::checksum32;

pub fn write_text_archive(archive: &CanopyArchive) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "facility {} {}\n",
        archive.manifest.facility_id, archive.manifest.site_name
    ));
    for (key, value) in &archive.manifest.properties {
        out.push_str(&format!("property {} {}\n", key, value));
    }
    for device in &archive.manifest.devices {
        out.push_str(&format!(
            "device {} {} {} {} {}\n",
            device.id.0,
            class_name(device.class),
            device.zone.0,
            device.alias,
            device.crop
        ));
    }
    for link in &archive.links {
        out.push_str(&format!(
            "link {} {} {} {}\n",
            link.from.0,
            link.to.0,
            link_name(link.kind),
            link.weight
        ));
    }
    for window in &archive.windows {
        out.push_str(&format!(
            "window {} {} {} {} {} {}\n",
            window.zone.0,
            window.start_minute,
            window.end_minute,
            window.target,
            window.recurrence,
            window.label
        ));
    }
    out
}

pub fn archive_fingerprint(archive: &CanopyArchive) -> u32 {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(archive.manifest.facility_id.as_bytes());
    bytes.extend_from_slice(archive.manifest.site_name.as_bytes());
    for device in &archive.manifest.devices {
        write_device_fingerprint(device, &mut bytes);
    }
    for link in &archive.links {
        bytes.extend_from_slice(&link.from.0.to_le_bytes());
        bytes.extend_from_slice(&link.to.0.to_le_bytes());
        bytes.extend_from_slice(&link.weight.to_le_bytes());
    }
    checksum32(&bytes)
}

fn write_device_fingerprint(device: &Device, out: &mut Vec<u8>) {
    out.extend_from_slice(&device.id.0.to_le_bytes());
    out.push(device.class.as_code());
    out.extend_from_slice(&device.zone.0.to_le_bytes());
    out.extend_from_slice(device.alias.as_bytes());
    out.push(0);
    out.extend_from_slice(device.crop.as_bytes());
}

fn class_name(class: DeviceClass) -> &'static str {
    match class {
        DeviceClass::Gateway => "gateway",
        DeviceClass::ClimateSensor => "climate",
        DeviceClass::MoistureProbe => "moisture",
        DeviceClass::Valve => "valve",
        DeviceClass::NutrientPump => "pump",
        DeviceClass::LightingRail => "light",
        DeviceClass::Camera => "camera",
        DeviceClass::Unknown(_) => "unknown",
    }
}

fn link_name(kind: LinkKind) -> &'static str {
    match kind {
        LinkKind::Parent => "parent",
        LinkKind::Neighbor => "neighbor",
        LinkKind::Backup => "backup",
        LinkKind::Controls => "controls",
        LinkKind::Observes => "observes",
        LinkKind::Unknown(_) => "unknown",
    }
}
