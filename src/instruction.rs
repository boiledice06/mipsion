pub struct RawInstruction {
    word: u32,
}

impl RawInstruction {
    pub fn new(word: u32) -> Self {
        Self { word }
    }

    fn extract(&self, shift: u32, width: u32) -> u32 {
        (self.word >> shift) & ((1u32 << width) - 1)
    }

    pub fn opcode(&self) -> u8 {
        self.extract(26, 6) as u8
    }

    pub fn rs(&self) -> u8 {
        self.extract(21, 5) as u8
    }

    pub fn rt(&self) -> u8 {
        self.extract(16, 5) as u8
    }

    pub fn rd(&self) -> u8 {
        self.extract(11, 5) as u8
    }

    pub fn fs(&self) -> u8 {
        self.extract(11, 5) as u8
    }

    pub fn ft(&self) -> u8 {
        self.extract(16, 5) as u8
    }

    pub fn shamt(&self) -> u8 {
        self.extract(6, 5) as u8
    }

    pub fn funct(&self) -> u8 {
        self.extract(0, 6) as u8
    }

    pub fn sel(&self) -> u8 {
        self.extract(0, 3) as u8
    }

    pub fn immediate(&self) -> u16 {
        self.extract(0, 16) as u16
    }

    pub fn target(&self) -> u32 {
        self.extract(0, 26)
    }
}

pub enum Instruction {
    RType {
        opcode: u8,
        rs: u8,
        rt: u8,
        rd: u8,
        shamt: u8,
        funct: u8,
    },
    IType {
        opcode: u8,
        rs: u8,
        rt: u8,
        immediate: u16,
    },
    JType {
        opcode: u8,
        target: u32,
    },
}

// impl Instruction {
//     pub fn decode(word: u32) -> Self {
//         Self
//     }
// }
