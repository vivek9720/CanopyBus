use crate::error::{DecodeError, Result};
use crate::model::CanopyArchive;
use crate::reader::{checksum32, Reader};
use crate::session::StreamDecoder;
use crate::{journal, manifest, page, rules, schedule, text, topology};

const MAGIC: &[u8; 4] = b"CNPY";
const STREAM_MAGIC: &[u8; 4] = b"CBFR";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentKind {
    Manifest = 1,
    Topology = 2,
    Schedule = 3,
    Page = 4,
    Rules = 5,
    Journal = 6,
    Note = 7,
}

impl SegmentKind {
    fn from_byte(byte: u8) -> Result<Self> {
        match byte {
            1 => Ok(Self::Manifest),
            2 => Ok(Self::Topology),
            3 => Ok(Self::Schedule),
            4 => Ok(Self::Page),
            5 => Ok(Self::Rules),
            6 => Ok(Self::Journal),
            7 => Ok(Self::Note),
            other => Err(DecodeError::UnknownSegment(other)),
        }
    }
}

pub fn parse_archive(data: &[u8]) -> Result<CanopyArchive> {
    if data.starts_with(MAGIC) {
        parse_binary_archive(data)
    } else if data.starts_with(STREAM_MAGIC) {
        parse_stream_archive(data)
    } else {
        text::parse_text_archive(data)
    }
}

pub fn parse_binary_archive(data: &[u8]) -> Result<CanopyArchive> {
    let mut r = Reader::new(data);
    if r.read_bytes(4)? != MAGIC {
        return Err(DecodeError::BadMagic);
    }
    let version = r.read_u8()?;
    if version == 0 || version > 4 {
        return Err(DecodeError::InvalidVersion(version));
    }
    let flags = r.read_u16()?;
    let segment_count = r.read_var_usize()?;
    if segment_count > 4096 {
        return Err(DecodeError::InvalidField("segment count"));
    }
    let mut archive = CanopyArchive::new(version);
    archive.flags = flags;
    for _ in 0..segment_count {
        let kind_byte = r.read_u8()?;
        let kind = SegmentKind::from_byte(kind_byte)?;
        let segment_flags = r.read_u16()?;
        let len = r.read_var_usize()?;
        if len > 8 * 1024 * 1024 {
            return Err(DecodeError::SegmentTooLarge {
                segment: kind_byte,
                len,
            });
        }
        let payload = r.read_bytes(len)?;
        if segment_flags & 1 != 0 {
            let expected = r.read_u32()?;
            if checksum32(payload) != expected && flags & 0x8000 != 0 {
                return Err(DecodeError::InvalidField("segment checksum"));
            }
        }
        apply_segment(&mut archive, kind, payload)?;
    }
    Ok(archive)
}

pub fn parse_stream_archive(data: &[u8]) -> Result<CanopyArchive> {
    let mut decoder = StreamDecoder::new();
    let mut pos = 0usize;
    while pos < data.len() {
        let end = (pos + 64).min(data.len());
        decoder.push(&data[pos..end])?;
        pos = end;
    }
    decoder.finish()
}

pub fn apply_segment(archive: &mut CanopyArchive, kind: SegmentKind, payload: &[u8]) -> Result<()> {
    match kind {
        SegmentKind::Manifest => archive.manifest = manifest::parse_manifest_segment(payload)?,
        SegmentKind::Topology => archive
            .links
            .extend(topology::parse_topology_segment(payload)?),
        SegmentKind::Schedule => archive
            .windows
            .extend(schedule::parse_schedule_segment(payload)?),
        SegmentKind::Page => archive.pages.extend(page::parse_page_segment(payload)?),
        SegmentKind::Rules => archive.rules.extend(rules::parse_rules_segment(payload)?),
        SegmentKind::Journal => archive
            .journals
            .extend(journal::parse_journal_segment(payload)?),
        SegmentKind::Note => archive.notes.push(
            core::str::from_utf8(payload)
                .map_err(|_| DecodeError::InvalidUtf8)?
                .to_string(),
        ),
    }
    Ok(())
}
