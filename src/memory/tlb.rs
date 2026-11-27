use crate::cp0::TLB_SIZE;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlbFault {
    Miss,
    Invalid,
    Modified,
    InvalidPageMask,
}

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

    pub fn page_size(&self) -> Result<u32, TlbFault> {
        match self.page_mask {
            0x0000_0000 => Ok(4 * 1024),
            0x0000_6000 => Ok(16 * 1024),
            0x0001_E000 => Ok(64 * 1024),
            0x0007_E000 => Ok(256 * 1024),
            0x001F_E000 => Ok(1024 * 1024),
            0x007F_E000 => Ok(4 * 1024 * 1024),
            0x01FF_E000 => Ok(16 * 1024 * 1024),
            0x07FF_E000 => Ok(64 * 1024 * 1024),
            0x1FFF_E000 => Ok(256 * 1024 * 1024),

            _ => Err(TlbFault::InvalidPageMask),
        }
    }

    pub fn translate(&self, address: u32, store: bool) -> Result<u32, TlbFault> {
        let page_size = self.page_size()?;

        // Even / Odd Page 선택
        let entry_lo = if address & page_size == 0 {
            self.entry_lo0
        } else {
            self.entry_lo1
        };

        // Valid Bit 검사
        if entry_lo & (1 << 1) == 0 {
            return Err(TlbFault::Invalid);
        }

        // Dirty Bit 검사
        if store && entry_lo & (1 << 2) == 0 {
            return Err(TlbFault::Modified);
        }

        let offset_mask = page_size - 1;

        // PFN 추출 및 Page 경계 정렬
        let pfn = (entry_lo >> 6) & 0x000F_FFFF;
        let physical_base = (pfn << 12) & !offset_mask;

        let offset = address & offset_mask;

        Ok(physical_base | offset)
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

    pub fn translate(&self, address: u32, asid: u8, store: bool) -> Result<u32, TlbFault> {
        let index = self.probe(address, asid).ok_or(TlbFault::Miss)?;

        let entry = self.read(index).ok_or(TlbFault::Miss)?;

        entry.translate(address, store)
    }
}

impl Default for Tlb {
    fn default() -> Self {
        Self::new()
    }
}
