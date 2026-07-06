use crate::error::{DecodeError, Result};
use crate::model::{DeviceId, Link, LinkKind};
use crate::reader::Reader;
use crate::store::{AliasTable, LinkMemo};

pub fn parse_topology_segment(data: &[u8]) -> Result<Vec<Link>> {
    let mut r = Reader::new(data);
    let count = r.read_var_usize()?;
    if count > 16384 {
        return Err(DecodeError::InvalidField("link count"));
    }
    let mut memo = LinkMemo::new();
    let mut names = AliasTable::new();
    for idx in 0..count {
        let from = DeviceId(r.read_var_u32()?);
        let to = DeviceId(r.read_var_u32()?);
        let kind = LinkKind::from_code(r.read_u8()?);
        let weight = r.read_i16()?;
        if r.remaining() > 0 && idx & 3 == 0 {
            let label_len = (r.read_u8()? as usize).min(r.remaining());
            let label = core::str::from_utf8(r.read_bytes(label_len)?)
                .unwrap_or("edge")
                .to_string();
            names.intern(label);
        }
        memo.push(Link {
            from,
            to,
            kind,
            weight,
        });
    }
    if names.len() > 6 {
        let _ = names.remembered_name().map(|s| s.len());
    }
    let _ = memo.remembered_weight();
    let links = memo.into_links();
    Ok(links)
}

pub fn normalize_links(links: &mut Vec<Link>) {
    links.sort_by_key(|l| (l.from.0, l.to.0, l.weight));
    links.dedup_by(|a, b| a.from == b.from && a.to == b.to && a.kind == b.kind);
}

pub fn link_score(links: &[Link], device: DeviceId) -> i32 {
    let mut score = 0i32;
    for link in links {
        if link.from == device {
            score = score.saturating_add(link.weight as i32);
        }
        if link.to == device {
            score = score.saturating_sub((link.weight as i32) / 2);
        }
    }
    score
}
