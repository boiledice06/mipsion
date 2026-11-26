use crate::cp0::TLB_SIZE;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TlbEntry {
    pub entry_hi: u32,
    pub entry_lo0: u32,
    pub entry_lo1: u32,
    pub page_mask: u32,
}

impl TlbEntry {
    pub fn global(&self) -> bool {
        self.entry_lo0 & 1 != 0 && self.entry_lo1 & 1 != 0
    }

    pub fn asid(&self) -> u8 {
        (self.entry_hi & 0xFF) as u8
    }

    pub fn matches(&self, address: u32, asid: u8) -> bool {
        // VPN2 및 PageMask 비교
        let mask = !(self.page_mask | 0x1FFF);

        let vpn_match = (self.entry_hi & mask) == (address & mask);

        let asid_match = self.global() || self.asid() == asid;

        vpn_match && asid_match
    }
}

pub struct Tlb {
    entries: [Option<TlbEntry>; TLB_SIZE],
}

impl Tlb {
    pub fn new() -> Self {
        Self {
            entries: [None; TLB_SIZE],
        }
    }

    pub fn read(&self, index: usize) -> Option<TlbEntry> {
        self.entries.get(index).copied().flatten()
    }

    pub fn write(&mut self, index: usize, entry: TlbEntry) {
        self.entries[index] = Some(entry);
    }

    pub fn probe(&self, address: u32, asid: u8) -> Option<usize> {
        self.entries
            .iter()
            .position(|entry| entry.is_some_and(|e| e.matches(address, asid)))
    }
}
