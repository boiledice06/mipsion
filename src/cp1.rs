const FCSR_FCC_MASK: u32 = 0xFE80_0000;
const FCSR_CAUSE_MASK: u32 = 0x0003_F000;
const FCSR_ENABLE_MASK: u32 = 0x0000_0F80;
const FCSR_FLAGS_MASK: u32 = 0x0000_007C;
const FCSR_FS_MASK: u32 = 1 << 24;
const FCSR_RM_MASK: u32 = 0b11;

const FCSR_WRITE_MASK: u32 = FCSR_FCC_MASK
    | FCSR_CAUSE_MASK
    | FCSR_ENABLE_MASK
    | FCSR_FLAGS_MASK
    | FCSR_FS_MASK
    | FCSR_RM_MASK;

pub struct Cp1 {
    fpr: [u64; 32],
    fir: u32,
    fcsr: u32,
}

impl Cp1 {
    pub fn new() -> Self {
        Self {
            fpr: [0; 32],
            fir: 0,
            fcsr: 0,
        }
    }

    pub fn read_raw(&self, index: usize) -> u64 {
        self.fpr[index]
    }

    pub fn write_raw(&mut self, index: usize, value: u64) {
        self.fpr[index] = value;
    }

    pub fn read_word(&self, index: usize) -> u32 {
        self.fpr[index] as u32
    }

    pub fn write_word(&mut self, index: usize, value: u32) {
        self.fpr[index] = (self.fpr[index] & 0xFFFF_FFFF_0000_0000) | u64::from(value);
    }

    pub fn valid_double_register(&self, index: usize) -> bool {
        index < 32 && index & 1 == 0
    }

    pub fn read_high_word(&self, index: usize) -> Option<u32> {
        if !self.valid_double_register(index) {
            return None;
        }

        Some(self.read_word(index + 1))
    }

    pub fn write_high_word(&mut self, index: usize, value: u32) -> bool {
        if !self.valid_double_register(index) {
            return false;
        }

        self.write_word(index + 1, value);

        true
    }

    pub fn read_doubleword(&self, index: usize) -> Option<u64> {
        if !self.valid_double_register(index) {
            return None;
        }

        let low = u64::from(self.read_word(index));
        let high = u64::from(self.read_word(index + 1));

        Some((high << 32) | low)
    }

    pub fn write_doubleword(&mut self, index: usize, value: u64) -> bool {
        if !self.valid_double_register(index) {
            return false;
        }

        self.write_word(index, value as u32);
        self.write_word(index + 1, (value >> 32) as u32);

        true
    }

    pub fn read_fir(&self) -> u32 {
        self.fir
    }

    pub fn read_fcsr(&self) -> u32 {
        self.fcsr
    }

    pub fn read_control(&self, index: u8) -> Option<u32> {
        match index {
            // FIR
            0 => Some(self.fir),

            // FCCR
            25 => {
                let fcc0 = (self.fcsr >> 23) & 1;
                let fcc1_7 = (self.fcsr >> 24) & 0xFE;

                Some(fcc1_7 | fcc0)
            }

            // FEXR
            26 => Some(self.fcsr & (FCSR_CAUSE_MASK | FCSR_FLAGS_MASK)),

            // FENR
            28 => {
                let fs = ((self.fcsr >> 24) & 1) << 2;

                Some((self.fcsr & (FCSR_ENABLE_MASK | FCSR_RM_MASK)) | fs)
            }

            // FCSR
            31 => Some(self.fcsr),

            _ => None,
        }
    }

    pub fn write_control(&mut self, index: u8, value: u32) -> bool {
        match index {
            // FCCR
            25 => {
                let fcc0 = (value & 1) << 23;
                let fcc1_7 = (value & 0xFE) << 24;

                self.fcsr = (self.fcsr & !FCSR_FCC_MASK) | fcc0 | fcc1_7;
            }

            // FEXR
            26 => {
                let mask = FCSR_CAUSE_MASK | FCSR_FLAGS_MASK;

                self.fcsr = (self.fcsr & !mask) | (value & mask);
            }

            // FENR
            28 => {
                let mask = FCSR_ENABLE_MASK | FCSR_FS_MASK | FCSR_RM_MASK;

                let fs = ((value >> 2) & 1) << 24;

                self.fcsr = (self.fcsr & !mask) | (value & (FCSR_ENABLE_MASK | FCSR_RM_MASK)) | fs;
            }

            // FCSR
            31 => {
                self.fcsr = (self.fcsr & !FCSR_WRITE_MASK) | (value & FCSR_WRITE_MASK);
            }

            // FIR 및 미지원 제어 레지스터
            _ => return false,
        }

        true
    }

    pub fn exception_requested(&self) -> bool {
        let cause = (self.fcsr >> 12) & 0x3F;
        let enables = (self.fcsr >> 7) & 0x1F;

        let unimplemented = cause & 0x20 != 0;
        let enabled_exception = cause & enables != 0;

        unimplemented || enabled_exception
    }

    pub fn rounding_mode(&self) -> u32 {
        self.fcsr & FCSR_RM_MASK
    }

    pub fn apply_fp_exceptions(&mut self, exceptions: u32) -> bool {
        let exceptions = exceptions & 0x3F;

        // Cause: 이번 연산에서 발생한 예외
        self.fcsr = (self.fcsr & !FCSR_CAUSE_MASK) | (exceptions << 12);

        let enables = (self.fcsr >> 7) & 0x1F;

        let trap = (exceptions & enables != 0) || (exceptions & 0x20 != 0);

        // Flag: 비활성화된 예외의 누적 기록
        if !trap {
            self.fcsr |= (exceptions & 0x1F) << 2;
        }

        trap
    }
}
