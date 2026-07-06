use core::fmt;

pub type Result<T> = core::result::Result<T, DecodeError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    Truncated {
        at: usize,
        needed: usize,
        available: usize,
    },
    BadMagic,
    InvalidVersion(u8),
    InvalidVarint,
    InvalidUtf8,
    SegmentTooLarge {
        segment: u8,
        len: usize,
    },
    UnknownSegment(u8),
    InvalidField(&'static str),
    InvalidText(String),
    InvalidOpcode(u8),
    DepthLimit,
    ArithmeticOverflow,
    DuplicateId(u32),
    MissingReference(u32),
    IncompleteStream,
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecodeError::Truncated {
                at,
                needed,
                available,
            } => write!(f, "truncated at {at}: need {needed}, have {available}"),
            DecodeError::BadMagic => write!(f, "bad canopybus magic"),
            DecodeError::InvalidVersion(v) => write!(f, "unsupported version {v}"),
            DecodeError::InvalidVarint => write!(f, "invalid varint"),
            DecodeError::InvalidUtf8 => write!(f, "invalid utf-8"),
            DecodeError::SegmentTooLarge { segment, len } => {
                write!(f, "segment {segment} too large: {len}")
            }
            DecodeError::UnknownSegment(t) => write!(f, "unknown segment type {t}"),
            DecodeError::InvalidField(name) => write!(f, "invalid field {name}"),
            DecodeError::InvalidText(line) => write!(f, "invalid text line: {line}"),
            DecodeError::InvalidOpcode(op) => write!(f, "invalid opcode {op}"),
            DecodeError::DepthLimit => write!(f, "depth limit"),
            DecodeError::ArithmeticOverflow => write!(f, "arithmetic overflow"),
            DecodeError::DuplicateId(id) => write!(f, "duplicate id {id}"),
            DecodeError::MissingReference(id) => write!(f, "missing reference {id}"),
            DecodeError::IncompleteStream => write!(f, "incomplete stream"),
        }
    }
}

impl std::error::Error for DecodeError {}
