use mipsion::{assembler, cli, cpu, elf, inspection, loader, memory, runtime, trace};
use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    use cli::{CliAction, parse_args};

    let action = match parse_args(std::env::args().skip(1)) {
        Ok(action) => action,
        Err(error) => {
            eprintln!("Error: {error}");
            eprintln!("Use --help for usage information.");
            return ExitCode::from(2);
        }
    };

    let result = match action {
        CliAction::Help => {
            print!("{}", cli::HELP);
            return ExitCode::SUCCESS;
        }
        CliAction::Version => {
            println!("MIPSion {}", env!("CARGO_PKG_VERSION"));
            return ExitCode::SUCCESS;
        }
        CliAction::Run(config) => run_program(config, false),
        CliAction::RunSource(config) => run_program(config, true),
        CliAction::Assemble {
            source,
            output,
            load_address,
        } => write_assembly(&source, &output, load_address),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn read_assembly(path: &Path, load_address: u32) -> Result<Vec<u8>, String> {
    let source = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let mut bytes = assembler::assemble(&source, load_address)
        .map_err(|error| format!("{}:{error}", path.display()))?
        .bytes;
    if bytes.is_empty() {
        return Err(format!(
            "{}: assembly produced an empty image",
            path.display()
        ));
    }
    bytes.resize(bytes.len().next_multiple_of(4), 0);
    Ok(bytes)
}

fn write_assembly(source: &Path, output: &Path, load_address: u32) -> Result<(), String> {
    use std::io::Write;

    let bytes = read_assembly(source, load_address)?;
    let mut file = std::fs::File::create_new(output)
        .map_err(|error| format!("failed to create {}: {error}", output.display()))?;
    if let Err(error) = file.write_all(&bytes) {
        drop(file);
        let detail = match std::fs::remove_file(output) {
            Ok(()) => String::new(),
            Err(cleanup) => format!(" (failed to remove partial file: {cleanup})"),
        };
        return Err(format!(
            "failed to write {}: {error}{detail}",
            output.display()
        ));
    }
    Ok(())
}

fn run_program(config: cli::CliConfig, source: bool) -> Result<(), String> {
    use cpu::Cpu;
    use loader::load_raw;
    use memory::Memory;
    use runtime::{ExecutionConfig, run, run_with_trace};

    const MAX_MEMORY_SIZE: usize = 256 * 1024 * 1024;

    if config.memory_size > MAX_MEMORY_SIZE {
        return Err("memory size exceeds 256 MiB limit".into());
    }

    let bytes = if source {
        read_assembly(&config.program, config.load_address)?
    } else {
        std::fs::read(&config.program)
            .map_err(|error| format!("failed to read {}: {error}", config.program.display()))?
    };

    let mut memory = Memory::new(config.memory_size);
    let mut cpu = Cpu::new();

    if let Some((address, size)) = config.dump_memory
        && !memory.contains_range(address, size)
    {
        return Err("memory dump range is out of bounds".into());
    }

    let is_elf = !source && bytes.starts_with(b"\x7FELF");

    let (entry, format) = if is_elf {
        let result = elf::load_elf(&mut memory, &bytes)
            .map_err(|error| format!("failed to load ELF: {error:?}"))?;

        cpu.write_reg(29, result.stack_pointer);

        (result.entry, "ELF32")
    } else {
        load_raw(&mut memory, config.load_address, &bytes)
            .map_err(|error| format!("failed to load program: {error:?}"))?;

        (config.entry, if source { "Assembly" } else { "Raw Binary" })
    };

    cpu.write_pc(entry);

    println!("MIPSion {}", env!("CARGO_PKG_VERSION"));
    println!("Program:      {}", config.program.display());
    println!("Program size: {} bytes", bytes.len());
    println!("Memory size:  {} bytes", config.memory_size);
    println!("Format:       {format}");

    if !is_elf {
        println!("Load address: {:#010X}", config.load_address);
    }

    println!("Entry point:  {entry:#010X}");

    if is_elf {
        println!("Stack:        {:#010X}", cpu.read_reg(29));
    }

    if config.debug {
        use mipsion::debugger::{DebuggerSession, run_repl};

        let mut session = DebuggerSession::new(cpu, memory);

        let stdin = std::io::stdin();
        let stdout = std::io::stdout();

        return run_repl(&mut session, stdin.lock(), stdout.lock(), config.max_steps)
            .map_err(|error| format!("debugger I/O error: {error}"));
    }

    let execution_config = ExecutionConfig {
        max_steps: config.max_steps,
        stop_on_exception: config.stop_on_exception,
    };

    let result = if config.trace {
        println!();
        println!("Instruction Trace");
        println!("-----------------");

        run_with_trace(&mut cpu, &mut memory, execution_config, |event| {
            print!("{}", trace::format_trace(event));
        })
    } else {
        run(&mut cpu, &mut memory, execution_config)
    };

    println!();
    println!("Execution finished");
    println!("Steps:        {}", result.steps);
    println!("Final PC:     {:#010X}", result.final_pc);
    println!("Stop reason:  {:?}", result.reason);

    if config.dump_registers {
        println!();
        print!("{}", inspection::dump_registers(&cpu));
    }

    if config.dump_cp0 {
        println!();
        print!("{}", inspection::dump_cp0(&cpu));
    }

    if config.dump_fpu {
        println!();
        print!("{}", inspection::dump_fpu(&cpu));
    }

    if let Some((address, size)) = config.dump_memory {
        println!();

        print!("{}", inspection::dump_memory(&memory, address, size)?);
    }

    Ok(())
}
