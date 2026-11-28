use mipsion::pipeline::PipelineOptions;
use mipsion::{assembler, cli, cpu, elf, inspection, loader, memory, runtime, trace};
use std::path::Path;
use std::process::ExitCode;

const MAX_SOURCE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_PROGRAM_BYTES: u64 = 256 * 1024 * 1024;

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
        CliAction::Run(config) => run_program(config, false, None),
        CliAction::RunSource(config) => run_program(config, true, None),
        CliAction::RunPipeline {
            config,
            source,
            max_cycles,
        } => run_program(
            config,
            source,
            Some((max_cycles, PipelineOptions::default())),
        ),
        CliAction::RunTimedPipeline {
            config,
            source,
            max_cycles,
            options,
        } => run_program(config, source, Some((max_cycles, options))),
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

fn read_input(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    use std::io::Read;

    let read_error = |error| format!("failed to read {}: {error}", path.display());
    let limit_error = || {
        format!(
            "{}: input exceeds {} MiB limit",
            path.display(),
            limit / (1024 * 1024)
        )
    };
    let file = std::fs::File::open(path).map_err(read_error)?;
    if file.metadata().map_err(read_error)?.len() > limit {
        return Err(limit_error());
    }
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(read_error)?;
    if bytes.len() as u64 > limit {
        return Err(limit_error());
    }
    Ok(bytes)
}

fn read_assembly(path: &Path, load_address: u32) -> Result<Vec<u8>, String> {
    let source = String::from_utf8(read_input(path, MAX_SOURCE_BYTES)?)
        .map_err(|error| format!("failed to read {} as UTF-8: {error}", path.display()))?;
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

fn run_program(
    config: cli::CliConfig,
    source: bool,
    timing: Option<(u64, PipelineOptions)>,
) -> Result<(), String> {
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
        read_input(&config.program, MAX_PROGRAM_BYTES)?
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

    if let Some((max_cycles, options)) = timing {
        return run_pipeline(config, cpu, memory, max_cycles, options);
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

    print_dumps(&config, &cpu, &memory)
}

fn run_pipeline(
    config: cli::CliConfig,
    cpu: cpu::Cpu,
    memory: memory::Memory,
    max_cycles: u64,
    options: PipelineOptions,
) -> Result<(), String> {
    use mipsion::pipeline::{Pipeline, Writeback, format_cycle, format_statistics};
    let mut pipeline = Pipeline::with_options(cpu, memory, options);
    if config.debug {
        let stdin = std::io::stdin();
        let stdout = std::io::stdout();
        return mipsion::debugger::run_pipeline_repl(
            pipeline,
            stdin.lock(),
            stdout.lock(),
            max_cycles,
        )
        .map_err(|error| format!("debugger I/O error: {error}"));
    }
    let reason = loop {
        if pipeline.retired() >= config.max_steps {
            break "Retirement limit reached".into();
        }
        if pipeline.cycles() >= max_cycles {
            break "Cycle limit reached".into();
        }
        let Some(event) = pipeline.cycle() else {
            break "Halted".into();
        };
        if config.trace {
            println!("{}", format_cycle(&event));
        }
        match event.writeback {
            Some(Writeback::Unsupported { pc, word, mnemonic }) => {
                return Err(format!(
                    "unsupported pipeline instruction {} ({word:#010X}) at {pc:#010X}",
                    mnemonic.unwrap_or("unknown")
                ));
            }
            Some(Writeback::Exception {
                exception,
                epc,
                in_delay_slot,
                ..
            }) if config.stop_on_exception => {
                break format!("{exception:?} at EPC {epc:#010X} (delay slot: {in_delay_slot})");
            }
            _ => {}
        }
    };
    println!();
    println!("Pipeline execution finished");
    print!("{}", format_statistics(pipeline.statistics()));
    println!("Final PC:     {:#010X}", pipeline.cpu().read_pc());
    println!("Stop reason:  {reason}");
    print_dumps(&config, pipeline.cpu(), pipeline.memory())
}

fn print_dumps(
    config: &cli::CliConfig,
    cpu: &cpu::Cpu,
    memory: &memory::Memory,
) -> Result<(), String> {
    if config.dump_registers {
        println!();
        print!("{}", inspection::dump_registers(cpu));
    }

    if config.dump_cp0 {
        println!();
        print!("{}", inspection::dump_cp0(cpu));
    }

    if config.dump_fpu {
        println!();
        print!("{}", inspection::dump_fpu(cpu));
    }

    if let Some((address, size)) = config.dump_memory {
        println!();

        print!("{}", inspection::dump_memory(memory, address, size)?);
    }

    Ok(())
}
