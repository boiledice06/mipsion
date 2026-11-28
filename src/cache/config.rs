use std::fmt;

const MAX_CAPACITY: u32 = 16 * 1024 * 1024;
const MAX_WAYS: u32 = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheConfig {
    capacity: u32,
    line_size: u32,
    ways: u32,
    sets: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddressParts {
    pub tag: u32,
    pub set: u32,
    pub offset: u32,
    pub line_base: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfigError(&'static str);

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for ConfigError {}

impl CacheConfig {
    pub fn new(capacity: u32, line_size: u32, ways: u32) -> Result<Self, ConfigError> {
        if capacity > MAX_CAPACITY {
            return Err(ConfigError("cache capacity must not exceed 16 MiB"));
        }
        if line_size < 4 || !line_size.is_power_of_two() {
            return Err(ConfigError(
                "cache line size must be a power of two and at least four bytes",
            ));
        }
        if !(1..=MAX_WAYS).contains(&ways) {
            return Err(ConfigError(
                "cache associativity must be between 1 and 16 ways",
            ));
        }
        let Some(set_size) = line_size.checked_mul(ways) else {
            return Err(ConfigError("cache set size overflows"));
        };
        let sets = capacity / set_size;
        if !capacity.is_multiple_of(set_size) || !sets.is_power_of_two() {
            return Err(ConfigError(
                "cache capacity must contain a nonzero power-of-two number of complete sets",
            ));
        }
        Ok(Self {
            capacity,
            line_size,
            ways,
            sets,
        })
    }

    pub fn capacity(self) -> u32 {
        self.capacity
    }
    pub fn line_size(self) -> u32 {
        self.line_size
    }
    pub fn ways(self) -> u32 {
        self.ways
    }
    pub fn sets(self) -> u32 {
        self.sets
    }

    pub fn address(self, physical: u32) -> AddressParts {
        let line = physical / self.line_size;
        let offset = physical & (self.line_size - 1);
        AddressParts {
            tag: line / self.sets,
            set: line & (self.sets - 1),
            offset,
            line_base: physical - offset,
        }
    }

    pub(super) fn line_base(self, tag: u32, set: u32) -> u32 {
        (tag * self.sets + set) * self.line_size
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self::new(1024, 16, 2).expect("valid default geometry")
    }
}
