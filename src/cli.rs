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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliAction {
    Run(CliConfig),
    Help,
    Version,
}

pub const HELP: &str = "\
MIPSion - MIPS32 Release 2 Emulator

Usage:
    mipsion <program> [options]

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

Numbers may be decimal or hexadecimal (0x prefix).
";

fn parse_number(value: &str) -> Result<u64, String> {
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

fn parse_address(value: &str) -> Result<u32, String> {
    u32::try_from(parse_number(value)?).map_err(|_| format!("address out of range: {value}"))
}

fn next_value(args: &mut impl Iterator<Item = String>, option: &str) -> Result<String, String> {
    args.next()
        .ok_or_else(|| format!("missing value for {option}"))
}

pub fn parse_args(args: impl IntoIterator<Item = String>) -> Result<CliAction, String> {
    let mut args = args.into_iter();

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

    Ok(CliAction::Run(CliConfig {
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
    }))
}
