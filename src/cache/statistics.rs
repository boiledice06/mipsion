#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CacheStats {
    pub reads: u64,
    pub writes: u64,
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub invalidations: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheAccess {
    Read,
    Write,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessResult {
    Hit,
    /// The evicted physical line base, if every way was occupied.
    Miss {
        evicted: Option<u32>,
    },
}
