#![allow(dead_code)]

mod alu;
mod cpu;
mod decoder;
mod executor;
mod instruction;
mod isa;
mod memory;
mod mmu;
mod tlb;

mod cp0;
mod cp1;
mod exception;

#[cfg(test)]
mod regression;
#[cfg(test)]
mod test_helpers;

fn main() {}
