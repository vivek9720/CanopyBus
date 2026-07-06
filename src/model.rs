use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DeviceId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ZoneId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceClass {
    Gateway,
    ClimateSensor,
    MoistureProbe,
    Valve,
    NutrientPump,
    LightingRail,
    Camera,
    Unknown(u8),
}

impl DeviceClass {
    pub fn from_code(code: u8) -> Self {
        match code {
            0 => Self::Gateway,
            1 => Self::ClimateSensor,
            2 => Self::MoistureProbe,
            3 => Self::Valve,
            4 => Self::NutrientPump,
            5 => Self::LightingRail,
            6 => Self::Camera,
            other => Self::Unknown(other),
        }
    }

    pub fn as_code(self) -> u8 {
        match self {
            Self::Gateway => 0,
            Self::ClimateSensor => 1,
            Self::MoistureProbe => 2,
            Self::Valve => 3,
            Self::NutrientPump => 4,
            Self::LightingRail => 5,
            Self::Camera => 6,
            Self::Unknown(code) => code,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Device {
    pub id: DeviceId,
    pub alias: String,
    pub class: DeviceClass,
    pub zone: ZoneId,
    pub crop: String,
    pub firmware: u16,
    pub channels: Vec<String>,
    pub calibration: Vec<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkKind {
    Parent,
    Neighbor,
    Backup,
    Controls,
    Observes,
    Unknown(u8),
}

impl LinkKind {
    pub fn from_code(code: u8) -> Self {
        match code {
            0 => Self::Parent,
            1 => Self::Neighbor,
            2 => Self::Backup,
            3 => Self::Controls,
            4 => Self::Observes,
            other => Self::Unknown(other),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Link {
    pub from: DeviceId,
    pub to: DeviceId,
    pub kind: LinkKind,
    pub weight: i16,
}

#[derive(Debug, Clone)]
pub struct ScheduleWindow {
    pub zone: ZoneId,
    pub start_minute: u16,
    pub end_minute: u16,
    pub target: i32,
    pub recurrence: u8,
    pub label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleKind {
    Temperature,
    Humidity,
    Co2,
    Moisture,
    Ph,
    Ec,
    Flow,
    Light,
    Unknown(u8),
}

impl SampleKind {
    pub fn from_code(code: u8) -> Self {
        match code {
            0 => Self::Temperature,
            1 => Self::Humidity,
            2 => Self::Co2,
            3 => Self::Moisture,
            4 => Self::Ph,
            5 => Self::Ec,
            6 => Self::Flow,
            7 => Self::Light,
            other => Self::Unknown(other),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SampleRow {
    pub timestamp_delta: u32,
    pub values: Vec<i64>,
}

#[derive(Debug, Clone)]
pub struct SamplePage {
    pub device: DeviceId,
    pub kind: SampleKind,
    pub base_timestamp: u64,
    pub scale: i16,
    pub rows: Vec<SampleRow>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JournalKind {
    Boot,
    Alarm,
    OperatorAck,
    ConfigChange,
    RuleAction,
    SampleGap,
    Unknown(u8),
}

impl JournalKind {
    pub fn from_code(code: u8) -> Self {
        match code {
            0 => Self::Boot,
            1 => Self::Alarm,
            2 => Self::OperatorAck,
            3 => Self::ConfigChange,
            4 => Self::RuleAction,
            5 => Self::SampleGap,
            other => Self::Unknown(other),
        }
    }
}

#[derive(Debug, Clone)]
pub struct JournalEntry {
    pub timestamp: u64,
    pub kind: JournalKind,
    pub device: DeviceId,
    pub severity: u8,
    pub message: String,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct RuleSet {
    pub name: String,
    pub bytecode: Vec<u8>,
    pub labels: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct FacilityManifest {
    pub facility_id: String,
    pub site_name: String,
    pub created_at: u64,
    pub devices: Vec<Device>,
    pub properties: BTreeMap<String, String>,
}

impl FacilityManifest {
    pub fn empty() -> Self {
        Self {
            facility_id: String::new(),
            site_name: String::new(),
            created_at: 0,
            devices: Vec::new(),
            properties: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CanopyArchive {
    pub version: u8,
    pub flags: u16,
    pub manifest: FacilityManifest,
    pub links: Vec<Link>,
    pub windows: Vec<ScheduleWindow>,
    pub pages: Vec<SamplePage>,
    pub journals: Vec<JournalEntry>,
    pub rules: Vec<RuleSet>,
    pub notes: Vec<String>,
}

impl CanopyArchive {
    pub fn new(version: u8) -> Self {
        Self {
            version,
            flags: 0,
            manifest: FacilityManifest::empty(),
            links: Vec::new(),
            windows: Vec::new(),
            pages: Vec::new(),
            journals: Vec::new(),
            rules: Vec::new(),
            notes: Vec::new(),
        }
    }

    pub fn devices(&self) -> &[Device] {
        &self.manifest.devices
    }

    pub fn device_by_id(&self, id: DeviceId) -> Option<&Device> {
        self.manifest.devices.iter().find(|d| d.id == id)
    }

    pub fn merge(&mut self, other: CanopyArchive) {
        if self.manifest.facility_id.is_empty() {
            self.manifest = other.manifest;
        } else {
            self.manifest.devices.extend(other.manifest.devices);
            self.manifest.properties.extend(other.manifest.properties);
        }
        self.links.extend(other.links);
        self.windows.extend(other.windows);
        self.pages.extend(other.pages);
        self.journals.extend(other.journals);
        self.rules.extend(other.rules);
        self.notes.extend(other.notes);
    }
}
