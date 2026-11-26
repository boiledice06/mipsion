#![allow(dead_code)]

mod alu;
mod cpu;
mod decoder;
mod executor;
mod instruction;
mod isa;
mod memory;

mod cp0;
mod exception;

#[cfg(test)]
mod regression;
#[cfg(test)]
mod test_helpers;

fn main() {}
