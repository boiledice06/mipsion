use super::trace::Redirect;
use crate::exception::Exception;
use crate::isa::Operation;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Fault {
    pub exception: Exception,
    pub bad_address: Option<u32>,
}

impl Fault {
    pub fn new(exception: Exception, bad_address: Option<u32>) -> Self {
        Self {
            exception,
            bad_address,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Action {
    Write(u8, u32),
    Store(Store),
    Control {
        redirect: Redirect,
        link: Option<(u8, u32)>,
    },
    Fault(Fault),
    Unsupported(Option<&'static str>),
}

impl Action {
    pub fn register_write(self) -> Option<(u8, u32)> {
        match self {
            Self::Write(register, value) if register != 0 => Some((register, value)),
            Self::Control { link, .. } => link,
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Store {
    pub address: u32,
    pub width: Width,
    pub value: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Width {
    Byte,
    Half,
    Word,
}

impl Width {
    pub fn bytes(self) -> usize {
        match self {
            Self::Byte => 1,
            Self::Half => 2,
            Self::Word => 4,
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct Instruction {
    pub pc: u32,
    pub physical: Option<u32>,
    pub delay_slot: bool,
    pub word: Result<u32, Fault>,
}

#[derive(Clone, Copy)]
pub(super) struct Decoded {
    pub instruction: Instruction,
    pub operation: &'static Operation,
    pub control: bool,
    pub sources: [Option<u8>; 2],
    pub destination: Option<u8>,
}

#[derive(Clone, Copy)]
pub(super) enum Issued {
    Operation(Decoded, [u32; 2]),
    Ready(Instruction, Action),
}

impl Issued {
    pub fn instruction(self) -> Instruction {
        match self {
            Self::Operation(decoded, _) => decoded.instruction,
            Self::Ready(instruction, _) => instruction,
        }
    }

    pub fn destination(self) -> Option<u8> {
        match self {
            Self::Operation(decoded, _) => decoded.destination,
            Self::Ready(_, _) => None,
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct Executed {
    pub instruction: Instruction,
    pub effect: Effect,
}

#[derive(Clone, Copy)]
pub(super) enum Effect {
    Ready(Action),
    Load {
        destination: u8,
        address: u32,
        width: Width,
        signed: bool,
    },
    Store(Store),
}

impl Effect {
    pub fn destination(self) -> Option<u8> {
        match self {
            Self::Ready(action) => action.register_write().map(|(register, _)| register),
            Self::Load { destination, .. } if destination != 0 => Some(destination),
            _ => None,
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct Prepared {
    pub instruction: Instruction,
    pub action: Action,
}
