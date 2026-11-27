#![allow(dead_code)]

pub mod cli;
pub mod cpu;
pub mod isa;
pub mod loader;
pub mod memory;
pub mod runtime;

// Compatibility re-exports
pub use cpu::{alu, cp0, cp1, exception, executor, fpu};
pub use isa::{decoder, instruction};
pub use loader::elf;
pub use memory::{mmu, tlb};
pub use runtime::{inspection, trace};

#[cfg(test)]
mod tests;

#[cfg(test)]
pub(crate) use tests::helpers as test_helpers;
