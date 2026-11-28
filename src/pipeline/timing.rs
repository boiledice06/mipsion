use super::trace::{CacheEvent, CachePort};
use crate::cache::{AccessResult, Cache, CacheAccess, CacheConfig};
use crate::memory::Memory;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheModelConfig {
    pub instruction: CacheConfig,
    pub data: CacheConfig,
    /// Extra pipeline cycles for a miss. Hits incur no extra cycles.
    pub miss_latency: u32,
}

impl Default for CacheModelConfig {
    fn default() -> Self {
        Self {
            instruction: CacheConfig::default(),
            data: CacheConfig::default(),
            miss_latency: 8,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PipelineOptions {
    pub caches: Option<CacheModelConfig>,
    pub forwarding: bool,
}

pub(super) struct CacheTiming {
    pub instruction: Cache,
    pub data: Cache,
    miss_latency: u32,
}

impl CacheTiming {
    pub fn new(config: CacheModelConfig) -> Self {
        Self {
            instruction: Cache::new(config.instruction),
            data: Cache::new(config.data),
            miss_latency: config.miss_latency,
        }
    }

    pub fn access(
        &mut self,
        memory: &Memory,
        port: CachePort,
        kind: CacheAccess,
        address: u32,
        width: usize,
    ) -> (u32, CacheEvent) {
        let cache = match port {
            CachePort::Instruction => &mut self.instruction,
            CachePort::Data => &mut self.data,
        };
        let (value, result) = cache.access(memory, address, width, kind);
        let latency = if matches!(result, AccessResult::Miss { .. }) {
            self.miss_latency
        } else {
            0
        };
        (
            value,
            CacheEvent {
                port,
                kind,
                address,
                result,
                latency,
            },
        )
    }
}

#[derive(Clone, Copy)]
pub(super) struct Pending<T> {
    pub payload: T,
    pub remaining: u32,
}

impl<T> Pending<T> {
    pub fn tick(&mut self) {
        self.remaining = self.remaining.saturating_sub(1);
    }
}
