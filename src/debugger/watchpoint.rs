use std::collections::BTreeMap;

use crate::cpu::Cpu;
use crate::memory::Memory;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemoryWidth {
    Byte,
    Halfword,
    Word,
    Doubleword,
}

impl MemoryWidth {
    pub fn bytes(self) -> usize {
        match self {
            Self::Byte => 1,
            Self::Halfword => 2,
            Self::Word => 4,
            Self::Doubleword => 8,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WatchTarget {
    Register(usize),
    Hi,
    Lo,

    Memory { address: u32, width: MemoryWidth },
}

impl WatchTarget {
    pub fn read(self, cpu: &Cpu, memory: &Memory) -> Result<u64, String> {
        match self {
            Self::Register(index) => {
                if index >= 32 {
                    return Err(format!("invalid register index: {index}"));
                }

                Ok(u64::from(cpu.read_reg(index)))
            }

            Self::Hi => Ok(u64::from(cpu.read_hi())),

            Self::Lo => Ok(u64::from(cpu.read_lo())),

            Self::Memory { address, width } => {
                if !memory.contains_range(address, width.bytes()) {
                    return Err(format!(
                        "watchpoint memory range out of bounds: \
                         0x{address:08X}"
                    ));
                }

                let value = match width {
                    MemoryWidth::Byte => u64::from(memory.read_byte(address)),

                    MemoryWidth::Halfword => u64::from(memory.read_halfword(address)),

                    MemoryWidth::Word => u64::from(memory.read_word(address)),

                    MemoryWidth::Doubleword => memory.read_doubleword(address),
                };

                Ok(value)
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WatchpointHit {
    pub target: WatchTarget,
    pub before: u64,
    pub after: u64,
}

#[derive(Debug, Default)]
pub struct Watchpoints {
    values: BTreeMap<WatchTarget, u64>,
}

impl Watchpoints {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, target: WatchTarget, cpu: &Cpu, memory: &Memory) -> Result<bool, String> {
        if self.values.contains_key(&target) {
            return Ok(false);
        }

        let value = target.read(cpu, memory)?;

        self.values.insert(target, value);

        Ok(true)
    }

    pub fn remove(&mut self, target: WatchTarget) -> bool {
        self.values.remove(&target).is_some()
    }

    pub fn contains(&self, target: WatchTarget) -> bool {
        self.values.contains_key(&target)
    }

    pub fn clear(&mut self) {
        self.values.clear();
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (WatchTarget, u64)> + '_ {
        self.values.iter().map(|(&target, &value)| (target, value))
    }

    pub fn detect_changes(&mut self, cpu: &Cpu, memory: &Memory) -> Vec<WatchpointHit> {
        let mut hits = Vec::new();

        for (&target, previous) in &mut self.values {
            let Ok(current) = target.read(cpu, memory) else {
                continue;
            };

            if current != *previous {
                hits.push(WatchpointHit {
                    target,
                    before: *previous,
                    after: current,
                });

                *previous = current;
            }
        }

        hits
    }

    pub fn refresh(&mut self, cpu: &Cpu, memory: &Memory) {
        for (&target, previous) in &mut self.values {
            if let Ok(current) = target.read(cpu, memory) {
                *previous = current;
            }
        }
    }
}
