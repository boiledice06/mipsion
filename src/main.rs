#![allow(dead_code)]

mod alu;
mod cpu;
mod decoder;
mod executor;
mod fpu;
mod instruction;
mod isa;
mod loader;
mod memory;
mod mmu;
mod runtime;
mod tlb;

mod cli;

mod cp0;
mod cp1;
mod exception;

#[cfg(test)]
mod regression;
#[cfg(test)]
mod test_helpers;

fn main() -> std::process::ExitCode {
    use cli::{CliAction, parse_args};
    use std::process::ExitCode;

    let action = match parse_args(std::env::args().skip(1)) {
        Ok(action) => action,

        Err(error) => {
            eprintln!("Error: {error}");
            eprintln!("Use --help for usage information.");

            return ExitCode::from(2);
        }
    };

    match action {
        CliAction::Help => {
            print!("{}", cli::HELP);
            ExitCode::SUCCESS
        }

        CliAction::Version => {
            println!("MIPSion {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }

        CliAction::Run(config) => match run_program(config) {
            Ok(()) => ExitCode::SUCCESS,

            Err(error) => {
                eprintln!("Error: {error}");
                ExitCode::FAILURE
            }
        },
    }
}

fn run_program(config: cli::CliConfig) -> Result<(), String> {
    use cpu::Cpu;
    use loader::load_raw;
    use memory::Memory;
    use runtime::{ExecutionConfig, run};

    const MAX_MEMORY_SIZE: usize = 256 * 1024 * 1024;

    if config.memory_size > MAX_MEMORY_SIZE {
        return Err("memory size exceeds 256 MiB limit".into());
    }

    let bytes = std::fs::read(&config.program)
        .map_err(|error| format!("failed to read {}: {error}", config.program.display()))?;

    let mut memory = Memory::new(config.memory_size);
    let mut cpu = Cpu::new();

    load_raw(&mut memory, config.load_address, &bytes)
        .map_err(|error| format!("failed to load program: {error:?}"))?;

    cpu.write_pc(config.entry);

    println!("MIPSion {}", env!("CARGO_PKG_VERSION"));
    println!("Program:      {}", config.program.display());
    println!("Program size: {} bytes", bytes.len());
    println!("Memory size:  {} bytes", config.memory_size);
    println!("Load address: {:#010X}", config.load_address);
    println!("Entry point:  {:#010X}", config.entry);

    let result = run(
        &mut cpu,
        &mut memory,
        ExecutionConfig {
            max_steps: config.max_steps,
            stop_on_exception: config.stop_on_exception,
        },
    );

    println!();
    println!("Execution finished");
    println!("Steps:        {}", result.steps);
    println!("Final PC:     {:#010X}", result.final_pc);
    println!("Stop reason:  {:?}", result.reason);

    Ok(())
}
