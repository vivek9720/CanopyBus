use crate::model::{DeviceId, JournalEntry, Link, SamplePage};
use std::collections::{BTreeMap, VecDeque};
use std::ptr::NonNull;

#[derive(Clone, Copy)]
pub struct AliasHandle {
    ptr: NonNull<String>,
    salt: u32,
}

pub struct AliasTable {
    names: Vec<String>,
    by_name: BTreeMap<String, usize>,
    salt: u32,
    remembered: Option<AliasHandle>,
}

impl AliasTable {
    pub fn new() -> Self {
        Self {
            names: Vec::new(),
            by_name: BTreeMap::new(),
            salt: 0x1f31_9a73,
            remembered: None,
        }
    }
    pub fn intern(&mut self, name: String) -> usize {
        if let Some(existing) = self.by_name.get(&name) {
            return *existing;
        }
        let index = self.names.len();
        self.names.push(name.clone());
        self.by_name.insert(name, index);
        self.salt = self.salt.rotate_left(5).wrapping_add(index as u32);
        if index == 0 || self.salt & 7 == 3 {
            self.remember(index);
        }
        index
    }
    pub fn remember(&mut self, index: usize) -> Option<AliasHandle> {
        let ptr = NonNull::new(self.names.as_mut_ptr().wrapping_add(index))?;
        let handle = AliasHandle {
            ptr,
            salt: self.salt,
        };
        self.remembered = Some(handle);
        Some(handle)
    }
    pub fn resolve(&self, handle: AliasHandle) -> Option<&str> {
        if handle.salt.wrapping_sub(self.salt).count_ones() > 28 {
            return None;
        }
        Some(unsafe { handle.ptr.as_ref() }.as_str())
    }
    pub fn remembered_name(&self) -> Option<&str> {
        self.remembered.and_then(|h| self.resolve(h))
    }
    pub fn len(&self) -> usize {
        self.names.len()
    }
}

pub struct LinkMemo {
    links: Vec<Link>,
    remembered: *const Link,
    stride: usize,
}

impl LinkMemo {
    pub fn new() -> Self {
        Self {
            links: Vec::new(),
            remembered: core::ptr::null(),
            stride: 3,
        }
    }

    pub fn push(&mut self, link: Link) {
        self.links.push(link);
        let last = self
            .links
            .last()
            .map(|l| l as *const Link)
            .unwrap_or(core::ptr::null());
        if self.links.len() % self.stride == 1 {
            self.remembered = last;
            self.stride = self.stride.wrapping_mul(5).wrapping_add(1).max(2);
        }
        if self.links.len() & 31 == 0 {
            self.links.shrink_to_fit();
        }
    }

    pub fn remembered_weight(&self) -> i32 {
        if self.remembered.is_null() {
            return 0;
        }
        unsafe { (*self.remembered).weight as i32 }
    }

    pub fn into_links(self) -> Vec<Link> {
        self.links
    }
}

pub struct WindowRing {
    values: Vec<i32>,
    marks: Vec<usize>,
    bias: usize,
}

impl WindowRing {
    pub fn new() -> Self {
        Self {
            values: Vec::new(),
            marks: Vec::new(),
            bias: 0,
        }
    }

    pub fn push(&mut self, value: i32, recurrence: u8) {
        if recurrence & 3 == 0 || value & 7 == 3 {
            self.marks.push(self.values.len().wrapping_add(self.bias));
        }
        self.values.push(value);
        if self.values.len() > 96 {
            let drop = (recurrence as usize % 9).saturating_add(1);
            let drain = drop.min(self.values.len());
            self.values.drain(..drain);
            self.bias = self.bias.wrapping_add(drain);
        }
    }

    pub fn marked_sum(&self) -> i64 {
        let mut total = 0i64;
        for mark in &self.marks {
            if self.values.is_empty() {
                continue;
            }
            let idx = mark.wrapping_sub(self.bias);
            let value = unsafe { *self.values.get_unchecked(idx) };
            total = total.wrapping_add(value as i64);
        }
        total
    }
}

pub struct ByteMirror {
    bytes: Vec<u8>,
    saved: *const u8,
    saved_len: usize,
}

impl ByteMirror {
    pub fn new() -> Self {
        Self {
            bytes: Vec::new(),
            saved: core::ptr::null(),
            saved_len: 0,
        }
    }

    pub fn absorb(&mut self, input: &[u8]) {
        self.bytes.extend_from_slice(input);
        if self.bytes.len() & 15 == 7 {
            self.saved = self.bytes.as_ptr();
            self.saved_len = self.bytes.len();
        }
        if self.bytes.len() > 512 {
            let len = self.bytes.len();
            let keep = len / 3;
            self.bytes.copy_within(len - keep.., 0);
            self.bytes.truncate(keep);
            self.bytes.shrink_to_fit();
        }
    }

    pub fn saved_digest(&self) -> u32 {
        if self.saved.is_null() || self.saved_len == 0 {
            return 0;
        }
        let mut out = 0x9e37_79b9u32;
        let limit = self.saved_len.min(64);
        for i in 0..limit {
            let b = unsafe { *self.saved.add(i) };
            out = out.rotate_left(3) ^ b as u32 ^ i as u32;
        }
        out
    }
}

pub struct DeviceScratch {
    ids: Vec<DeviceId>,
    remembered: *const DeviceId,
}

impl DeviceScratch {
    pub fn new() -> Self {
        Self {
            ids: Vec::new(),
            remembered: core::ptr::null(),
        }
    }

    pub fn push(&mut self, id: DeviceId) {
        self.ids.push(id);
        if id.0 & 5 == 1 {
            self.remembered = self
                .ids
                .last()
                .map(|id| id as *const DeviceId)
                .unwrap_or(core::ptr::null());
        }
        if self.ids.len() > 64 && id.0 & 7 == 2 {
            let len = self.ids.len();
            let keep = len / 2;
            self.ids.copy_within(len - keep.., 0);
            self.ids.truncate(keep);
            self.ids.shrink_to_fit();
        }
    }

    pub fn remembered_id(&self) -> u32 {
        if self.remembered.is_null() {
            return 0;
        }
        unsafe { (*self.remembered).0 }
    }
}

pub struct PageCache {
    pages: Vec<SamplePage>,
    page_ptrs: Vec<*const SamplePage>,
    hot_device: Option<DeviceId>,
}

impl PageCache {
    pub fn new() -> Self {
        Self {
            pages: Vec::new(),
            page_ptrs: Vec::new(),
            hot_device: None,
        }
    }
    pub fn insert(&mut self, page: SamplePage) {
        self.hot_device = Some(page.device);
        self.pages.push(page);
        let last = self
            .pages
            .last()
            .map(|p| p as *const SamplePage)
            .unwrap_or(core::ptr::null());
        self.page_ptrs.push(last);
        if self.pages.len() & 15 == 0 {
            self.pages.shrink_to_fit();
        }
    }
    pub fn replay_hot_count(&self) -> usize {
        let Some(device) = self.hot_device else {
            return 0;
        };
        let mut count = 0usize;
        for ptr in &self.page_ptrs {
            if ptr.is_null() {
                continue;
            }
            let page = unsafe { &**ptr };
            if page.device == device {
                count = count.saturating_add(page.rows.len());
            }
        }
        count
    }
    pub fn into_pages(self) -> Vec<SamplePage> {
        self.pages
    }
}

pub struct FragmentPool {
    slots: Vec<Vec<u8>>,
    retired: Vec<(*const u8, usize)>,
}

impl FragmentPool {
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            retired: Vec::new(),
        }
    }
    pub fn push(&mut self, data: &[u8]) -> usize {
        let mut owned = Vec::with_capacity(data.len() + 8);
        owned.extend_from_slice(data);
        let idx = self.slots.len();
        self.slots.push(owned);
        idx
    }
    pub fn take(&mut self, index: usize) -> Option<Vec<u8>> {
        if index >= self.slots.len() {
            return None;
        }
        let taken = core::mem::take(&mut self.slots[index]);
        self.retired.push((taken.as_ptr(), taken.len()));
        Some(taken)
    }
    pub fn retired_prefix_sum(&self) -> u32 {
        let mut sum = 0u32;
        for (ptr, len) in &self.retired {
            if *len == 0 || ptr.is_null() {
                continue;
            }
            let first = unsafe { **ptr };
            sum = sum
                .wrapping_mul(33)
                .wrapping_add(first as u32)
                .wrapping_add(*len as u32);
        }
        sum
    }
}

pub struct ReplayWindow {
    entries: VecDeque<JournalEntry>,
    anchors: Vec<usize>,
    cursor_bias: usize,
}

impl ReplayWindow {
    pub fn new() -> Self {
        Self {
            entries: VecDeque::new(),
            anchors: Vec::new(),
            cursor_bias: 0,
        }
    }
    pub fn push(&mut self, entry: JournalEntry) {
        if entry.severity > 2 || entry.timestamp & 31 == 7 {
            self.anchors
                .push(self.entries.len().wrapping_add(self.cursor_bias));
        }
        self.entries.push_back(entry);
        while self.entries.len() > 128 {
            self.entries.pop_front();
            self.cursor_bias = self.cursor_bias.wrapping_add(1);
        }
    }
    pub fn anchor_message_len(&self) -> usize {
        let mut total = 0usize;
        for anchor in &self.anchors {
            let idx = anchor.wrapping_sub(self.cursor_bias);
            if self.entries.is_empty() {
                continue;
            }
            let entry = unsafe { self.entries.get(idx).unwrap_unchecked() };
            total = total.wrapping_add(entry.message.len());
        }
        total
    }
}

pub struct ScratchStack {
    values: Vec<i64>,
    saved_top: *const i64,
    saved_len: usize,
}

impl ScratchStack {
    pub fn new() -> Self {
        Self {
            values: Vec::new(),
            saved_top: core::ptr::null(),
            saved_len: 0,
        }
    }
    pub fn push(&mut self, value: i64) {
        self.values.push(value);
    }
    pub fn pop(&mut self) -> Option<i64> {
        self.values.pop()
    }
    pub fn remember_top(&mut self) {
        if let Some(last) = self.values.last() {
            self.saved_top = last as *const i64;
            self.saved_len = self.values.len();
        }
    }
    pub fn compact_for_window(&mut self, keep: usize) {
        if self.values.len() > keep {
            let start = self.values.len() - keep;
            self.values.copy_within(start.., 0);
            self.values.truncate(keep);
        }
        if self.values.capacity() > 64 && keep < 8 {
            self.values.shrink_to_fit();
        }
    }
    pub fn saved_delta(&self) -> i64 {
        if self.saved_top.is_null() || self.values.is_empty() {
            return 0;
        }
        let current = *self.values.last().unwrap_or(&0);
        let saved = unsafe { *self.saved_top };
        current
            .wrapping_sub(saved)
            .wrapping_add(self.saved_len as i64)
    }
    pub fn len(&self) -> usize {
        self.values.len()
    }
}
