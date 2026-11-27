pub struct Memory {
    data: Vec<u8>,
}

impl Memory {
    pub fn new(size: usize) -> Self {
        Self {
            data: vec![0; size],
        }
    }

    pub fn read_byte(&self, address: u32) -> u8 {
        self.data[address as usize]
    }

    pub fn write_byte(&mut self, address: u32, value: u8) {
        self.data[address as usize] = value;
    }

    pub fn read_halfword(&self, address: u32) -> u16 {
        let low = self.read_byte(address);
        let high = self.read_byte(address + 1);

        (low as u16) | ((high as u16) << 8)
    }

    pub fn write_halfword(&mut self, address: u32, value: u16) {
        let low = (value & 0xFF) as u8;
        let high = (value >> 8) as u8;

        self.write_byte(address, low);
        self.write_byte(address + 1, high);
    }

    pub fn read_word(&self, address: u32) -> u32 {
        let low = self.read_halfword(address);
        let high = self.read_halfword(address + 2);

        (low as u32) | ((high as u32) << 16)
    }

    pub fn write_word(&mut self, address: u32, value: u32) {
        let low = (value & 0xFFFF) as u16;
        let high = (value >> 16) as u16;
        self.write_halfword(address, low);
        self.write_halfword(address + 2, high);
    }

    pub fn read_doubleword(&self, address: u32) -> u64 {
        let low = self.read_word(address);
        let high = self.read_word(address + 4);

        u64::from(low) | (u64::from(high) << 32)
    }

    pub fn write_doubleword(&mut self, address: u32, value: u64) {
        let low = value as u32;
        let high = (value >> 32) as u32;

        self.write_word(address, low);
        self.write_word(address + 4, high);
    }

    pub fn read_word_left(&self, address: u32, value: u32) -> u32 {
        let base = address & !3;
        let offset = address & 3;
        let mut result = value;

        for i in 0..=offset {
            let shift = (3 - offset + i) * 8;
            let byte = self.read_byte(base + i) as u32;

            result &= !(0xFFu32 << shift);
            result |= byte << shift;
        }

        result
    }

    pub fn read_word_right(&self, address: u32, value: u32) -> u32 {
        let base = address & !3;
        let offset = address & 3;
        let mut result = value;

        for i in offset..4 {
            let shift = (i - offset) * 8;
            let byte = self.read_byte(base + i) as u32;

            result &= !(0xFFu32 << shift);
            result |= byte << shift;
        }

        result
    }

    pub fn write_word_left(&mut self, address: u32, value: u32) {
        let base = address & !3;
        let offset = address & 3;

        for i in 0..=offset {
            let shift = (3 - offset + i) * 8;
            let byte = (value >> shift) as u8;

            self.write_byte(base + i, byte);
        }
    }

    pub fn write_word_right(&mut self, address: u32, value: u32) {
        let base = address & !3;
        let offset = address & 3;

        for i in offset..4 {
            let shift = (i - offset) * 8;
            let byte = (value >> shift) as u8;

            self.write_byte(base + i, byte);
        }
    }
}
