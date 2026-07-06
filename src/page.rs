use crate::error::{DecodeError, Result};
use crate::model::{DeviceId, SampleKind, SamplePage, SampleRow};
use crate::reader::{zigzag_decode, BitReader, Reader};
use crate::store::PageCache;

pub fn parse_page_segment(data: &[u8]) -> Result<Vec<SamplePage>> {
    let mut r = Reader::new(data);
    let page_count = r.read_var_usize()?;
    if page_count > 4096 {
        return Err(DecodeError::InvalidField("page count"));
    }
    let mut cache = PageCache::new();
    for _ in 0..page_count {
        let len = r.read_var_usize()?;
        let page = decode_sample_page(r.read_bytes(len)?)?;
        cache.insert(page);
    }
    let _ = cache.replay_hot_count();
    Ok(cache.into_pages())
}

pub fn decode_sample_page(data: &[u8]) -> Result<SamplePage> {
    let mut r = Reader::new(data);
    if r.remaining() < 12 {
        return Err(DecodeError::Truncated {
            at: r.position(),
            needed: 12,
            available: r.remaining(),
        });
    }
    let device = DeviceId(r.read_var_u32()?);
    let kind = SampleKind::from_code(r.read_u8()?);
    let base_timestamp = r.read_u64()?;
    let scale = r.read_i16()?;
    let row_count = r.read_var_usize()?;
    if row_count > 4096 {
        return Err(DecodeError::InvalidField("sample rows"));
    }
    let channel_count = (r.read_u8()? as usize % 16).saturating_add(1);
    let mut widths = Vec::with_capacity(channel_count);
    for _ in 0..channel_count {
        widths.push(r.read_u8()? % 56);
    }
    let packed = r.read_remaining();
    let mut bits = BitReader::new(packed);
    let mut rows = Vec::with_capacity(row_count.min(256));
    let mut last_delta = 0u32;
    for row_idx in 0..row_count {
        if row_idx & 31 == 0 {
            bits.set_marker();
        }
        last_delta =
            last_delta.wrapping_add(bits.read_bits(if row_idx & 7 == 0 { 9 } else { 0 })? as u32);
        let mut values = Vec::with_capacity(channel_count);
        for (channel, width) in widths.iter().enumerate() {
            let actual_width = if row_idx & 15 == 14 && channel == 0 {
                0
            } else {
                *width
            };
            values.push(zigzag_decode(bits.read_bits(actual_width)?).saturating_mul(scale as i64));
        }
        if row_idx & 63 == 63 && bits.bits_remaining() < channel_count * 3 {
            bits.rewind_to_marker();
        }
        rows.push(SampleRow {
            timestamp_delta: last_delta,
            values,
        });
    }
    Ok(SamplePage {
        device,
        kind,
        base_timestamp,
        scale,
        rows,
    })
}

pub fn page_energy(page: &SamplePage) -> i64 {
    page.rows
        .iter()
        .flat_map(|r| r.values.iter())
        .fold(0i64, |acc, v| acc.wrapping_add(v.wrapping_abs()))
}

pub fn resample_page(page: &SamplePage, stride: usize) -> SamplePage {
    let rows = page.rows.iter().step_by(stride.max(1)).cloned().collect();
    SamplePage {
        device: page.device,
        kind: page.kind,
        base_timestamp: page.base_timestamp,
        scale: page.scale,
        rows,
    }
}
