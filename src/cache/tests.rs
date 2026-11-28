use super::{AccessResult, AddressParts, Cache, CacheAccess, CacheConfig, CacheStats, LineState};
use crate::memory::Memory;

#[test]
fn decomposes_physical_tag_set_offset_and_checks_geometry() {
    let config = CacheConfig::new(128, 16, 2).unwrap();
    assert_eq!(config.capacity(), 128);
    assert_eq!(config.ways(), 2);
    assert_eq!(config.sets(), 4);
    assert_eq!(
        config.address(0x1234),
        AddressParts {
            tag: 0x48,
            set: 3,
            offset: 4,
            line_base: 0x1230
        }
    );
    assert_eq!(config.address(u32::MAX).line_base, 0xffff_fff0);
    assert!(CacheConfig::new(96, 16, 3).is_ok());
    for (capacity, line, ways) in [
        (0, 4, 1),
        (100, 16, 1),
        (64, 0, 1),
        (64, 2, 1),
        (64, 12, 1),
        (64, 16, 0),
        (64, 16, 17),
        (96, 16, 2),
        (64, 1 << 31, 16),
    ] {
        assert!(CacheConfig::new(capacity, line, ways).is_err());
    }
}

#[test]
fn direct_mapping_distinguishes_offsets_and_conflict_evictions() {
    let mut memory = Memory::new(96);
    memory.write_word(4, 0x4433_2211);
    let mut cache = Cache::new(CacheConfig::new(32, 16, 1).unwrap());
    assert_eq!(cache.line(0, 0), Some(LineState::default()));
    assert_eq!(cache.line(0, 1), None);
    assert_eq!(
        cache.access(&memory, 4, 4, CacheAccess::Read),
        (0x4433_2211, AccessResult::Miss { evicted: None })
    );
    assert_eq!(
        cache.access(&memory, 5, 1, CacheAccess::Read),
        (0x22, AccessResult::Hit)
    );
    assert_eq!(
        cache.access(&memory, 36, 4, CacheAccess::Read).1,
        AccessResult::Miss { evicted: Some(0) }
    );
    assert_eq!(
        cache.stats(),
        CacheStats {
            reads: 3,
            hits: 1,
            misses: 2,
            evictions: 1,
            ..CacheStats::default()
        }
    );
}

#[test]
fn associative_replacement_uses_the_least_recent_way() {
    let memory = Memory::new(96);
    let mut cache = Cache::new(CacheConfig::new(16, 8, 2).unwrap());
    let mut access = |address| cache.access(&memory, address, 4, CacheAccess::Read).1;
    assert_eq!(access(0), AccessResult::Miss { evicted: None });
    assert_eq!(access(8), AccessResult::Miss { evicted: None });
    assert_eq!(access(4), AccessResult::Hit);
    assert_eq!(access(16), AccessResult::Miss { evicted: Some(8) });
    assert_eq!(access(8), AccessResult::Miss { evicted: Some(0) });
}

#[test]
fn write_through_keeps_a_clean_line_and_updates_backing_memory() {
    let mut memory = Memory::new(96);
    let mut cache = Cache::new(CacheConfig::new(8, 8, 1).unwrap());
    assert_eq!(
        cache.access(&memory, 2, 2, CacheAccess::Write).1,
        AccessResult::Miss { evicted: None }
    );
    cache.write_through(&mut memory, 2, &[0xab, 0xcd]);
    assert_eq!(
        cache.access(&memory, 0, 4, CacheAccess::Read),
        (0xcdab_0000, AccessResult::Hit)
    );
    assert_eq!(memory.read_halfword(2), 0xcdab);
    assert_eq!(
        cache.line(0, 0),
        Some(LineState {
            valid: true,
            dirty: false,
            tag: 0
        })
    );
    assert_eq!(
        cache.access(&memory, 8, 4, CacheAccess::Read).1,
        AccessResult::Miss { evicted: Some(0) }
    );
    assert_eq!(memory.read_halfword(2), 0xcdab);
}

#[test]
fn invalidate_and_refill_at_a_partial_memory_tail() {
    let mut memory = Memory::new(19);
    let mut cache = Cache::new(CacheConfig::new(32, 16, 1).unwrap());
    memory.write_byte(18, 17);
    assert_eq!(cache.access(&memory, 18, 1, CacheAccess::Read).0, 17);
    memory.write_byte(18, 22);
    assert_eq!(cache.invalidate(18), Some(16));
    assert_eq!(cache.invalidate(18), None);
    assert_eq!(cache.stats().invalidations, 1);
    assert_eq!(
        cache.access(&memory, 18, 1, CacheAccess::Read),
        (22, AccessResult::Miss { evicted: None })
    );
}

#[test]
fn cache_address_decomposition_inverts_at_geometry_boundaries() {
    for (capacity, line_size, ways) in [
        (4, 4, 1),
        (96, 16, 3),
        (256, 32, 2),
        (16 * 1024 * 1024, 4096, 16),
    ] {
        let config = CacheConfig::new(capacity, line_size, ways).unwrap();
        for physical in [
            0,
            line_size - 1,
            line_size,
            capacity - 1,
            capacity,
            u32::MAX,
        ] {
            let parts = config.address(physical);
            let line_number = physical / line_size;
            assert_eq!(parts.set, line_number % config.sets());
            assert_eq!(parts.tag, line_number / config.sets());
            assert_eq!(parts.offset, physical % line_size);
            assert_eq!(config.line_base(parts.tag, parts.set), parts.line_base);
            assert_eq!(parts.line_base + parts.offset, physical);
        }
    }
}

#[test]
fn invalid_way_precedes_lru_even_if_its_stamp_is_newer() {
    let memory = Memory::new(64);
    let mut cache = Cache::new(CacheConfig::new(24, 8, 3).unwrap());
    for address in [0, 8, 16] {
        assert_eq!(
            cache.access(&memory, address, 1, CacheAccess::Read).1,
            AccessResult::Miss { evicted: None }
        );
    }
    assert_eq!(cache.invalidate(16), Some(16));
    assert_eq!(
        cache.access(&memory, 24, 1, CacheAccess::Read).1,
        AccessResult::Miss { evicted: None }
    );
    assert_eq!(cache.line(0, 2).unwrap().tag, 3);
    assert_eq!(
        cache.access(&memory, 32, 1, CacheAccess::Read).1,
        AccessResult::Miss { evicted: Some(0) }
    );
    assert_eq!(cache.stats().evictions, 1);
    assert_eq!(cache.stats().invalidations, 1);
}
