use crate::{cache::CacheConfig, pipeline::PipelineOptions};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliConfig {
    pub program: PathBuf,
    pub load_address: u32,
    pub entry: u32,
    pub memory_size: usize,
    pub max_steps: u64,
    pub stop_on_exception: bool,
    pub dump_registers: bool,
    pub dump_cp0: bool,
    pub dump_fpu: bool,
    pub dump_memory: Option<(u32, usize)>,
    pub trace: bool,
    pub debug: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliAction {
    Run(CliConfig),
    RunSource(CliConfig),
    RunPipeline {
        config: CliConfig,
        source: bool,
        max_cycles: u64,
    },
    RunTimedPipeline {
        config: CliConfig,
        source: bool,
        max_cycles: u64,
        options: PipelineOptions,
    },
    Assemble {
        source: PathBuf,
        output: PathBuf,
        load_address: u32,
    },
    Help,
    Version,
}

pub const HELP: &str = "\
MIPSion - MIPS32 Release 2 Emulator

Usage:
    mipsion <program> [options]
    mipsion --assemble <source> -o <output> [--load-address <addr>]

Options:
    --load-address <addr>      Program load address
    --entry <addr>             Initial program counter
    --memory-size <bytes>      Memory size (default: 16777216)
    --max-steps <count>        Maximum steps (default: 1000000)
    --continue-on-exception    Continue execution after exceptions
    -h, --help                 Show this help message
    -V, --version              Show version
    --dump-registers           Display general-purpose registers
    --dump-cp0                 Display system control registers
    --dump-fpu                 Display floating-point registers
    --dump-memory <addr> <size>
                               Display physical memory contents
    --trace                    Trace instruction execution
    --source                   Read the program as UTF-8 assembly
    --pipeline                 Use the five-stage pipeline engine
    --max-cycles <count>       Pipeline cycle limit (default: 5000000)
    --forwarding               Forward register results in pipeline mode
    --cache                    Enable instruction and data cache timing
    --icache <bytes:line:ways> Instruction cache geometry (enables caches)
    --dcache <bytes:line:ways> Data cache geometry (enables caches)
    --memory-latency <cycles>  Extra cycles per miss (enables caches, default: 8)
    --debug                    Start interactive debugger

Numbers may be decimal or hexadecimal (0x prefix).
";

pub(crate) fn parse_number(value: &str) -> Result<u64, String> {
    let result = if let Some(hex) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        u64::from_str_radix(hex, 16)
    } else {
        value.parse::<u64>()
    };

    result.map_err(|_| format!("invalid number: {value}"))
}

pub(crate) fn parse_address(value: &str) -> Result<u32, String> {
    u32::try_from(parse_number(value)?).map_err(|_| format!("address out of range: {value}"))
}

fn next_value(args: &mut impl Iterator<Item = String>, option: &str) -> Result<String, String> {
    args.next()
        .ok_or_else(|| format!("missing value for {option}"))
}

pub fn parse_args(args: impl IntoIterator<Item = String>) -> Result<CliAction, String> {
    let mut args = args.into_iter().peekable();
    if args.peek().is_some_and(|arg| arg == "--assemble") {
        args.next();
        return parse_assembly(args);
    }

    let mut program = None;
    let mut load_address = 0;
    let mut entry = None;
    let mut memory_size = 16 * 1024 * 1024;
    let mut max_steps = 1_000_000;
    let mut stop_on_exception = true;
    let mut dump_registers = false;
    let mut dump_cp0 = false;
    let mut dump_fpu = false;
    let mut dump_memory = None;
    let mut trace = false;

    let mut debug = false;
    let mut source = false;
    let mut pipeline = false;
    let mut max_cycles = None;
    let mut timing = PipelineOptions::default();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                return Ok(CliAction::Help);
            }

            "-V" | "--version" => {
                return Ok(CliAction::Version);
            }

            "--load-address" => {
                load_address = parse_address(&next_value(&mut args, &arg)?)?;
            }

            "--entry" => {
                entry = Some(parse_address(&next_value(&mut args, &arg)?)?);
            }

            "--memory-size" => {
                let value = next_value(&mut args, &arg)?;

                memory_size = usize::try_from(parse_number(&value)?)
                    .map_err(|_| format!("memory size out of range: {value}"))?;
            }

            "--max-steps" => {
                max_steps = parse_number(&next_value(&mut args, &arg)?)?;
            }

            "--continue-on-exception" => {
                stop_on_exception = false;
            }

            "--dump-registers" => {
                dump_registers = true;
            }

            "--dump-cp0" => {
                dump_cp0 = true;
            }

            "--dump-fpu" => {
                dump_fpu = true;
            }

            "--dump-memory" => {
                let address = parse_address(&next_value(&mut args, &arg)?)?;

                let value = next_value(&mut args, &arg)?;

                let size = usize::try_from(parse_number(&value)?)
                    .map_err(|_| format!("dump size out of range: {value}"))?;

                if size == 0 {
                    return Err("dump size must be greater than zero".into());
                }

                if size > 65536 {
                    return Err("memory dump exceeds 64 KiB limit".into());
                }

                dump_memory = Some((address, size));
            }

            "--trace" => {
                trace = true;
            }

            "--pipeline" => pipeline = true,

            "--forwarding" => timing.forwarding = true,

            "--cache" => {
                timing.caches.get_or_insert_default();
            }

            "--icache" | "--dcache" => {
                let value = next_value(&mut args, &arg)?;
                let geometry = parse_geometry(&value, &arg)?;
                let caches = timing.caches.get_or_insert_default();
                if arg == "--icache" {
                    caches.instruction = geometry;
                } else {
                    caches.data = geometry;
                }
            }

            "--memory-latency" => {
                let value = next_value(&mut args, &arg)?;
                let latency = u32::try_from(parse_number(&value)?)
                    .map_err(|_| format!("memory latency out of range: {value}"))?;
                timing.caches.get_or_insert_default().miss_latency = latency;
            }

            "--max-cycles" => max_cycles = Some(parse_number(&next_value(&mut args, &arg)?)?),

            "--source" => {
                source = true;
            }

            "--debug" => {
                debug = true;
            }

            _ if arg.starts_with('-') => {
                return Err(format!("unknown option: {arg}"));
            }

            _ => {
                if program.is_some() {
                    return Err(format!("unexpected argument: {arg}"));
                }

                program = Some(PathBuf::from(arg));
            }
        }
    }

    let program = program.ok_or_else(|| "missing program path".to_string())?;

    if memory_size == 0 {
        return Err("memory size must be greater than zero".into());
    }

    if load_address & 3 != 0 {
        return Err("load address must be word-aligned".into());
    }

    let entry = entry.unwrap_or(load_address);

    if entry & 3 != 0 {
        return Err("entry point must be word-aligned".into());
    }

    if debug
        && (trace
            || !stop_on_exception
            || dump_registers
            || dump_cp0
            || dump_fpu
            || dump_memory.is_some())
    {
        return Err("--debug cannot be combined with trace, \
            dump options or --continue-on-exception"
            .into());
    }

    let config = CliConfig {
        program,
        load_address,
        entry,
        memory_size,
        max_steps,
        stop_on_exception,
        dump_registers,
        dump_cp0,
        dump_fpu,
        dump_memory,
        trace,
        debug,
    };
    if max_cycles.is_some() && !pipeline {
        return Err("--max-cycles requires --pipeline".into());
    }
    if timing != PipelineOptions::default() && !pipeline {
        return Err("cache and forwarding options require --pipeline".into());
    }
    Ok(if timing != PipelineOptions::default() {
        CliAction::RunTimedPipeline {
            config,
            source,
            max_cycles: max_cycles.unwrap_or(5_000_000),
            options: timing,
        }
    } else if pipeline {
        CliAction::RunPipeline {
            config,
            source,
            max_cycles: max_cycles.unwrap_or(5_000_000),
        }
    } else if source {
        CliAction::RunSource(config)
    } else {
        CliAction::Run(config)
    })
}

fn parse_geometry(value: &str, flag: &str) -> Result<CacheConfig, String> {
    let mut parts = value.split(':');
    let mut next = || {
        let number = parts
            .next()
            .ok_or_else(|| format!("{flag} requires bytes:line:ways"))?;
        u32::try_from(parse_number(number)?)
            .map_err(|_| format!("{flag} value out of range: {number}"))
    };
    let capacity = next()?;
    let line = next()?;
    let ways = next()?;
    if parts.next().is_some() {
        return Err(format!("{flag} requires bytes:line:ways"));
    }
    CacheConfig::new(capacity, line, ways).map_err(|error| format!("{flag}: {error}"))
}

fn parse_assembly(mut args: impl Iterator<Item = String>) -> Result<CliAction, String> {
    let mut source = None;
    let mut output = None;
    let mut load_address = 0;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => return Ok(CliAction::Help),
            "-o" | "--output" if output.is_none() => {
                output = Some(PathBuf::from(next_value(&mut args, &arg)?));
            }
            "--load-address" => load_address = parse_address(&next_value(&mut args, &arg)?)?,
            _ if arg.starts_with('-') => {
                return Err(format!("unknown or repeated assembler option: {arg}"));
            }
            _ if source.is_none() => source = Some(PathBuf::from(arg)),
            _ => return Err(format!("unexpected assembler argument: {arg}")),
        }
    }
    if load_address & 3 != 0 {
        return Err("load address must be word-aligned".into());
    }
    Ok(CliAction::Assemble {
        source: source.ok_or("missing assembly source path")?,
        output: output.ok_or("missing assembly output path (-o)")?,
        load_address,
    })
}
