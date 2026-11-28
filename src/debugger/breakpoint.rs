use std::collections::BTreeSet;

#[derive(Debug, Default)]
pub struct Breakpoints {
    addresses: BTreeSet<u32>,
}

impl Breakpoints {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, address: u32) -> Result<bool, String> {
        if address & 3 != 0 {
            return Err(format!("unaligned breakpoint address: 0x{address:08X}"));
        }

        Ok(self.addresses.insert(address))
    }

    pub fn remove(&mut self, address: u32) -> bool {
        self.addresses.remove(&address)
    }

    pub fn contains(&self, address: u32) -> bool {
        self.addresses.contains(&address)
    }

    pub fn clear(&mut self) {
        self.addresses.clear();
    }

    pub fn len(&self) -> usize {
        self.addresses.len()
    }

    pub fn is_empty(&self) -> bool {
        self.addresses.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = u32> + '_ {
        self.addresses.iter().copied()
    }
}
