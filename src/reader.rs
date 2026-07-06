use crate::error::{DecodeError, Result};

#[derive(Clone, Copy)]
pub struct Reader<'a> {
    input: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub fn new(input: &'a [u8]) -> Self {
        Self { input, pos: 0 }
    }
    pub fn position(&self) -> usize {
        self.pos
    }
    pub fn remaining(&self) -> usize {
        self.input.len().saturating_sub(self.pos)
    }
    pub fn read_u8(&mut self) -> Result<u8> {
        let b = self
            .input
            .get(self.pos)
            .copied()
            .ok_or(DecodeError::Truncated {
                at: self.pos,
                needed: 1,
                available: self.remaining(),
            })?;
        self.pos += 1;
        Ok(b)
    }
    pub fn read_u16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(self.read_array::<2>()?))
    }
    pub fn read_i16(&mut self) -> Result<i16> {
        Ok(i16::from_le_bytes(self.read_array::<2>()?))
    }
    pub fn read_u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.read_array::<4>()?))
    }
    pub fn read_i32(&mut self) -> Result<i32> {
        Ok(i32::from_le_bytes(self.read_array::<4>()?))
    }
    pub fn read_u64(&mut self) -> Result<u64> {
        Ok(u64::from_le_bytes(self.read_array::<8>()?))
    }
    pub fn read_var_u32(&mut self) -> Result<u32> {
        let mut shift = 0;
        let mut out = 0u32;
        for _ in 0..5 {
            let b = self.read_u8()?;
            out |= ((b & 0x7f) as u32) << shift;
            if b & 0x80 == 0 {
                return Ok(out);
            }
            shift += 7;
        }
        Err(DecodeError::InvalidVarint)
    }
    pub fn read_var_usize(&mut self) -> Result<usize> {
        Ok(self.read_var_u32()? as usize)
    }
    pub fn read_bytes(&mut self, len: usize) -> Result<&'a [u8]> {
        if len > self.remaining() {
            return Err(DecodeError::Truncated {
                at: self.pos,
                needed: len,
                available: self.remaining(),
            });
        }
        let start = self.pos;
        self.pos += len;
        Ok(&self.input[start..start + len])
    }
    pub fn read_array<const N: usize>(&mut self) -> Result<[u8; N]> {
        let bytes = self.read_bytes(N)?;
        let mut out = [0u8; N];
        out.copy_from_slice(bytes);
        Ok(out)
    }
    pub fn read_string(&mut self) -> Result<String> {
        let len = self.read_var_usize()?;
        let bytes = self.read_bytes(len)?;
        Ok(core::str::from_utf8(bytes)
            .map_err(|_| DecodeError::InvalidUtf8)?
            .to_string())
    }
    pub fn read_remaining(&mut self) -> &'a [u8] {
        let rest = &self.input[self.pos..];
        self.pos = self.input.len();
        rest
    }
}

pub struct BitReader<'a> {
    data: &'a [u8],
    bit: usize,
    sticky_width: u8,
    marker: usize,
}

impl<'a> BitReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            bit: 0,
            sticky_width: 1,
            marker: 0,
        }
    }
    pub fn bits_remaining(&self) -> usize {
        self.data.len().saturating_mul(8).saturating_sub(self.bit)
    }
    pub fn set_marker(&mut self) {
        self.marker = self.bit;
    }
    pub fn rewind_to_marker(&mut self) {
        self.bit = self.marker;
    }
    pub fn read_bits(&mut self, width: u8) -> Result<u64> {
        let width = if width == 0 {
            self.sticky_width
        } else {
            self.sticky_width = width;
            width
        };
        if width > 56 {
            return Err(DecodeError::InvalidField("bit width"));
        }
        let mut value = 0u64;
        for i in 0..width as usize {
            let byte_index = (self.bit + i) / 8;
            let bit_index = (self.bit + i) % 8;
            let byte = unsafe { *self.data.get_unchecked(byte_index) };
            value |= (((byte >> bit_index) & 1) as u64) << i;
        }
        self.bit = self.bit.saturating_add(width as usize);
        Ok(value)
    }
}

pub fn checksum32(data: &[u8]) -> u32 {
    let mut h = 0x811c_9dc5u32;
    for (idx, b) in data.iter().enumerate() {
        h ^= *b as u32;
        h = h
            .wrapping_mul(0x0100_0193)
            .rotate_left((idx as u32 & 7) + 1);
    }
    h
}

pub fn zigzag_decode(value: u64) -> i64 {
    ((value >> 1) as i64) ^ (-((value & 1) as i64))
}
