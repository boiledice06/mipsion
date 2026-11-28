use crate::cli::{parse_address, parse_number};
use crate::pipeline::{Pipeline, Writeback, format_cycle};
use crate::runtime::inspection::{dump_cp0, dump_memory, dump_registers};
use std::io::{self, BufRead, Write};

const HELP: &str = "cycle [count]  Advance and display pipeline cycles\n\
regs, cp0      Inspect committed registers\n\
pc             Inspect committed PC and next PC\n\
mem addr size  Inspect physical memory\n\
reset          Reset CPU, memory and pipeline\n\
quit           Exit the debugger";

pub fn run_pipeline_repl(
    mut pipeline: Pipeline,
    mut input: impl BufRead,
    mut output: impl Write,
    max_cycles: u64,
) -> io::Result<()> {
    let initial_cpu = pipeline.cpu().clone();
    let initial_memory = pipeline.memory().clone();
    writeln!(output, "MIPSion Pipeline Debugger")?;
    writeln!(output, "Type 'help' for available commands.")?;
    let mut line = String::new();
    loop {
        write!(output, "(pipeline) ")?;
        output.flush()?;
        line.clear();
        if input.read_line(&mut line)? == 0 {
            break;
        }
        let words: Vec<_> = line.split_whitespace().collect();
        match words.as_slice() {
            [] => {}
            ["help" | "h"] => writeln!(output, "{HELP}")?,
            ["quit" | "q" | "exit"] => break,
            ["regs"] => write!(output, "{}", dump_registers(pipeline.cpu()))?,
            ["cp0"] => write!(output, "{}", dump_cp0(pipeline.cpu()))?,
            ["pc"] => writeln!(
                output,
                "PC {:#010X}  Next PC {:#010X}",
                pipeline.cpu().read_pc(),
                pipeline.cpu().read_next_pc()
            )?,
            ["mem", addr, size] => {
                match parse_address(addr).and_then(|addr| {
                    let size = parse_number(size)?;
                    if !(1..=65536).contains(&size) {
                        return Err("size must be between 1 and 65536".into());
                    }
                    dump_memory(pipeline.memory(), addr, size as usize)
                }) {
                    Ok(text) => write!(output, "{text}")?,
                    Err(error) => writeln!(output, "Error: {error}")?,
                }
            }
            ["reset"] => {
                pipeline = Pipeline::new(initial_cpu.clone(), initial_memory.clone());
                writeln!(output, "Pipeline reset")?;
            }
            ["cycle" | "step" | "s", rest @ ..] if rest.len() <= 1 => {
                let count = match rest.first().map_or(Ok(1), |value| parse_number(value)) {
                    Ok(count) => count,
                    Err(error) => {
                        writeln!(output, "Error: {error}")?;
                        continue;
                    }
                };
                for _ in 0..count {
                    if pipeline.cycles() >= max_cycles {
                        writeln!(output, "Cycle limit reached")?;
                        break;
                    }
                    let Some(event) = pipeline.cycle() else {
                        writeln!(output, "Pipeline stopped. Use reset to restart.")?;
                        break;
                    };
                    writeln!(output, "{}", format_cycle(&event))?;
                    if matches!(
                        event.writeback,
                        Some(Writeback::Exception { .. } | Writeback::Unsupported { .. })
                    ) {
                        break;
                    }
                }
            }
            _ => writeln!(output, "Error: unknown command or invalid arguments")?,
        }
    }
    writeln!(output, "Pipeline debugger terminated")
}
