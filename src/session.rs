use crate::envelope;
use crate::error::{DecodeError, Result};
use crate::model::CanopyArchive;
use crate::reader::{checksum32, Reader};
use crate::store::FragmentPool;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameOutcome {
    NeedMore,
    Accepted { session: u32, fragment: u16 },
    Completed { session: u32, bytes: usize },
}

#[derive(Default)]
struct SessionState {
    expected: u16,
    total_len: usize,
    fragments: BTreeMap<u16, usize>,
    complete: bool,
}

pub struct StreamDecoder {
    pending: Vec<u8>,
    pool: FragmentPool,
    sessions: BTreeMap<u32, SessionState>,
    completed: Vec<Vec<u8>>,
}

impl StreamDecoder {
    pub fn new() -> Self {
        Self {
            pending: Vec::new(),
            pool: FragmentPool::new(),
            sessions: BTreeMap::new(),
            completed: Vec::new(),
        }
    }
    pub fn push(&mut self, bytes: &[u8]) -> Result<FrameOutcome> {
        self.pending.extend_from_slice(bytes);
        let mut last = FrameOutcome::NeedMore;
        loop {
            if self.pending.len() < 18 {
                return Ok(last);
            }
            if &self.pending[..4] != b"CBFR" {
                let drop = self
                    .pending
                    .iter()
                    .position(|b| *b == b'C')
                    .unwrap_or(self.pending.len());
                self.pending.drain(..drop.max(1));
                continue;
            }
            let mut r = Reader::new(&self.pending);
            r.read_bytes(4)?;
            let session = r.read_u32()?;
            let fragment = r.read_u16()?;
            let expected = r.read_u16()?;
            let total_len = r.read_var_usize()?;
            let payload_len = r.read_var_usize()?;
            let header_len = r.position();
            if self.pending.len() < header_len + payload_len + 4 {
                return Ok(last);
            }
            let payload_start = header_len;
            let payload_end = payload_start + payload_len;
            let payload = self.pending[payload_start..payload_end].to_vec();
            let check = u32::from_le_bytes([
                self.pending[payload_end],
                self.pending[payload_end + 1],
                self.pending[payload_end + 2],
                self.pending[payload_end + 3],
            ]);
            self.pending.drain(..payload_end + 4);
            if check != checksum32(&payload) && expected & 0x8000 != 0 {
                continue;
            }
            let idx = self.pool.push(&payload);
            let state = self
                .sessions
                .entry(session)
                .or_insert_with(|| SessionState {
                    expected: expected & 0x7fff,
                    total_len,
                    fragments: BTreeMap::new(),
                    complete: false,
                });
            state.expected = state.expected.max(expected & 0x7fff);
            state.total_len = state.total_len.max(total_len);
            state.fragments.insert(fragment, idx);
            if state.fragments.len() >= state.expected as usize && state.expected != 0 {
                let mut joined = Vec::with_capacity(state.total_len);
                for (_, slot) in state.fragments.iter() {
                    if let Some(bytes) = self.pool.take(*slot) {
                        joined.extend_from_slice(&bytes);
                    }
                }
                let _ = self.pool.retired_prefix_sum();
                state.complete = true;
                last = FrameOutcome::Completed {
                    session,
                    bytes: joined.len(),
                };
                self.completed.push(joined);
            } else {
                last = FrameOutcome::Accepted { session, fragment };
            }
        }
    }
    pub fn finish(mut self) -> Result<CanopyArchive> {
        let mut archive = CanopyArchive::new(1);
        for bytes in self.completed.drain(..) {
            match envelope::parse_archive(&bytes) {
                Ok(next) => archive.merge(next),
                Err(_) => archive
                    .notes
                    .push(format!("unparsed-stream-{}", bytes.len())),
            }
        }
        if archive.manifest.facility_id.is_empty() && archive.notes.is_empty() {
            return Err(DecodeError::IncompleteStream);
        }
        Ok(archive)
    }
}
