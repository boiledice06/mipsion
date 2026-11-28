//! Physically indexed and tagged, write-through, write-allocate cache.

mod config;
mod line;
mod replacement;
mod statistics;

pub use config::{AddressParts, CacheConfig, ConfigError};
pub use line::LineState;
pub use statistics::{AccessResult, CacheAccess, CacheStats};

use crate::memory::Memory;
use line::Line;
use replacement::least_recently_used;
use std::collections::BTreeMap;

pub struct Cache {
    config: CacheConfig,
    // Allocate backing storage only for sets and ways that have been touched.
    sets: BTreeMap<u32, Vec<Line>>,
    time: u64,
    stats: CacheStats,
}

impl Cache {
    pub fn new(config: CacheConfig) -> Self {
        Self {
            config,
            sets: BTreeMap::new(),
            time: 0,
            stats: CacheStats::default(),
        }
    }

    pub fn config(&self) -> CacheConfig {
        self.config
    }
    pub fn stats(&self) -> CacheStats {
        self.stats
    }

    pub fn line(&self, set: u32, way: u32) -> Option<LineState> {
        if set >= self.config.sets() || way >= self.config.ways() {
            return None;
        }
        Some(
            self.sets
                .get(&set)
                .map_or_else(LineState::default, |lines| lines[way as usize].state),
        )
    }

    /// Start one validated, single-line access. Width is between one and four bytes.
    pub(crate) fn access(
        &mut self,
        memory: &Memory,
        address: u32,
        width: usize,
        kind: CacheAccess,
    ) -> (u32, AccessResult) {
        let parts = self.config.address(address);
        debug_assert!(memory.contains_range(address, width));
        debug_assert!(
            (1..=4).contains(&width)
                && parts.offset as usize + width <= self.config.line_size() as usize
        );
        match kind {
            CacheAccess::Read => self.stats.reads += 1,
            CacheAccess::Write => self.stats.writes += 1,
        }
        self.time += 1;
        let lines = self
            .sets
            .entry(parts.set)
            .or_insert_with(|| (0..self.config.ways()).map(|_| Line::default()).collect());
        let (index, result) = if let Some(index) = lines
            .iter()
            .position(|line| line.state.valid && line.state.tag == parts.tag)
        {
            self.stats.hits += 1;
            (index, AccessResult::Hit)
        } else {
            self.stats.misses += 1;
            let index = least_recently_used(lines);
            let line = &mut lines[index];
            let evicted = line
                .state
                .valid
                .then(|| self.config.line_base(line.state.tag, parts.set));
            if evicted.is_some() {
                self.stats.evictions += 1;
            }
            line.state = LineState {
                valid: true,
                dirty: false,
                tag: parts.tag,
            };
            line.bytes.resize(self.config.line_size() as usize, 0);
            // A physical memory tail can end partway through a cache line.
            let available = (memory.size() - parts.line_base as usize).min(line.bytes.len());
            for (offset, byte) in line.bytes.iter_mut().enumerate() {
                *byte = if offset < available {
                    memory.read_byte(parts.line_base + offset as u32)
                } else {
                    0
                };
            }
            (index, AccessResult::Miss { evicted })
        };
        let line = &mut lines[index];
        line.last_used = self.time;
        let start = parts.offset as usize;
        let mut value = [0; 4];
        value[..width].copy_from_slice(&line.bytes[start..start + width]);
        (u32::from_le_bytes(value), result)
    }

    /// Retire a validated store into backing memory and any resident data line.
    pub(crate) fn write_through(&mut self, memory: &mut Memory, address: u32, bytes: &[u8]) {
        let parts = self.config.address(address);
        if let Some(line) = self.sets.get_mut(&parts.set).and_then(|lines| {
            lines
                .iter_mut()
                .find(|line| line.state.valid && line.state.tag == parts.tag)
        }) {
            let start = parts.offset as usize;
            line.bytes[start..start + bytes.len()].copy_from_slice(bytes);
        }
        for (offset, &byte) in bytes.iter().enumerate() {
            memory.write_byte(address + offset as u32, byte);
        }
    }

    pub(crate) fn invalidate(&mut self, address: u32) -> Option<u32> {
        let parts = self.config.address(address);
        let line = self
            .sets
            .get_mut(&parts.set)?
            .iter_mut()
            .find(|line| line.state.valid && line.state.tag == parts.tag)?;
        line.state.valid = false;
        self.stats.invalidations += 1;
        Some(parts.line_base)
    }
}

#[cfg(test)]
mod tests;
