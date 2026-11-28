use std::io::Write;
use std::process::{Command, Output, Stdio};

fn execute(args: &[&str], input: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_mipsion"))
        .args(args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let mut stdin = child.stdin.take().unwrap();

    stdin.write_all(input.as_bytes()).unwrap();

    // Close stdin to deliver EOF.
    drop(stdin);

    child.wait_with_output().unwrap()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn cli_debugger_help_option() {
    let output = execute(&["--help"], "");

    assert!(output.status.success());
    assert!(stdout(&output).contains("--debug"));
}

#[test]
fn cli_debugger_elf_launch() {
    let output = execute(&["--debug", "examples/hello.elf"], "quit\n");

    assert!(output.status.success(), "{}", stderr(&output));

    let text = stdout(&output);

    assert!(text.contains("ELF32"));

    assert!(text.contains("MIPSion Interactive Debugger"));

    assert!(text.contains("00001000  2408002A"));

    assert!(text.contains("Debugger terminated"));
}

#[test]
fn cli_debugger_raw_launch() {
    let output = execute(&["--debug", "examples/hello.bin"], "status\nquit\n");

    assert!(output.status.success(), "{}", stderr(&output));

    let text = stdout(&output);

    assert!(text.contains("Raw Binary"));

    assert!(text.contains("MIPSion Interactive Debugger"));

    assert!(text.contains("Entry:       0x00000000"));
}

#[test]
fn cli_debugger_elf_workflow() {
    let output = execute(
        &["--debug", "examples/hello.elf"],
        "break 0x1004\n\
         watch t0\n\
         continue\n\
         continue\n\
         step\n\
         regs\n\
         reset\n\
         watchpoints\n\
         quit\n",
    );

    assert!(output.status.success(), "{}", stderr(&output));

    let text = stdout(&output);

    for expected in [
        "Watchpoint hit: $t0",
        "0x0 -> 0x2A",
        "Breakpoint hit: 0x00001004",
        "$t1: 00000000 -> 0000002B",
        "Session reset",
        "$t0 = 0x0",
        "Debugger terminated",
    ] {
        assert!(
            text.contains(expected),
            "missing output: {expected}\n{text}"
        );
    }
}

#[test]
fn cli_debugger_exception() {
    let output = execute(&["--debug", "examples/hello.elf"], "continue\nquit\n");

    assert!(output.status.success());

    let text = stdout(&output);

    assert!(text.contains("Exception: 8"));
    assert!(text.contains("EPC=0x00001008"));
    assert!(text.contains("Steps: 3"));
}

#[test]
fn cli_debugger_custom_step_limit() {
    let output = execute(
        &["--debug", "examples/hello.elf", "--max-steps", "1"],
        "continue\nstatus\nquit\n",
    );

    assert!(output.status.success());

    let text = stdout(&output);

    assert!(text.contains("Step limit reached"));

    assert!(text.contains("PC:          0x00001004"));
}

#[test]
fn cli_debugger_rejects_conflicting_options() {
    let output = execute(&["--debug", "examples/hello.elf", "--trace"], "");

    assert_eq!(output.status.code(), Some(2));

    assert!(stderr(&output).contains("--debug cannot be combined"));
}

#[test]
fn cli_debugger_handles_missing_program() {
    let output = execute(&["--debug", "examples/nonexistent-program.elf"], "");

    assert!(!output.status.success());

    assert!(stderr(&output).contains("failed to read"));
}

#[test]
fn cli_normal_execution_is_unchanged() {
    let output = execute(&["examples/hello.elf"], "");

    assert!(output.status.success());

    let text = stdout(&output);

    assert!(text.contains("Execution finished"));

    assert!(!text.contains("MIPSion Interactive Debugger"));
}

#[test]
fn cli_normal_trace_is_unchanged() {
    let output = execute(&["examples/hello.elf", "--trace"], "");

    assert!(output.status.success());

    let text = stdout(&output);

    assert!(text.contains("Instruction Trace"));
    assert!(text.contains("[000001]"));
    assert!(text.contains("Execution finished"));
}

#[test]
fn cli_assembler_pads_raw_output_and_runs_delay_slot() {
    let output_path =
        std::env::temp_dir().join(format!("mipsion-assembler-{}.bin", std::process::id()));
    let path = output_path.to_str().unwrap();
    let build = execute(
        &[
            "--assemble",
            "examples/delay-slot.s",
            "-o",
            path,
            "--load-address",
            "0x1000",
        ],
        "",
    );
    assert!(build.status.success(), "{}", stderr(&build));
    let binary = std::fs::read(&output_path).unwrap();
    assert_eq!(binary.len(), 36);
    assert_eq!(&binary[..4], &[41, 0, 8, 0x24]);
    assert_eq!(&binary[32..], &[0x5a, 0, 0, 0]);

    let repeat = execute(&["--assemble", "examples/delay-slot.s", "-o", path], "");
    assert!(!repeat.status.success());
    assert_eq!(std::fs::read(&output_path).unwrap(), binary);

    let output = execute(&[path, "--load-address", "0x1000", "--dump-registers"], "");
    std::fs::remove_file(output_path).unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("$t0   0000002A"), "{text}");
    assert!(text.contains("$t2   0000005A"), "{text}");
}

#[test]
fn cli_debugger_loads_and_steps_assembly() {
    let output = execute(
        &["examples/delay-slot.s", "--source", "--debug"],
        "step\nstep\nstep\nregs\nquit\n",
    );
    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("Format:       Assembly"), "{text}");
    assert!(text.contains("$t0   0000002A"), "{text}");
    assert!(text.contains("00000010"), "{text}");
}

#[test]
fn cli_assembler_reports_the_input_path_and_source_location() {
    let output = execute(&["--source", "tests/fixtures/invalid-assembly.s"], "");
    assert_eq!(output.status.code(), Some(1));
    let text = stderr(&output);
    assert!(text.contains("invalid-assembly.s:2:21"), "{text}");
    assert!(text.contains("outside -32768..=32767"), "{text}");
}

#[test]
fn cli_pipeline_runs_source_and_traces_redirect_and_writeback() {
    let output = execute(
        &[
            "--pipeline",
            "--source",
            "examples/delay-slot.s",
            "--trace",
            "--dump-registers",
        ],
        "",
    );
    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    for expected in [
        "[cycle 1] IF=00000000",
        "redirect 00000004 -> 00000010",
        "retire 00000008",
        "Syscall at EPC 0x0000001C",
        "$t0   0000002A",
        "$t2   0000005A",
        "Retired:      6",
    ] {
        assert!(text.contains(expected), "{expected} missing from {text}");
    }
}

#[test]
fn cli_pipeline_debugger_cycles_and_resets() {
    let output = execute(
        &[
            "--pipeline",
            "--debug",
            "examples/hello.bin",
            "--max-cycles",
            "2",
        ],
        "cycle 3\nreset\ncycle\npc\nquit\n",
    );
    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("MIPSion Pipeline Debugger"), "{text}");
    assert!(text.contains("Cycle limit reached"), "{text}");
    assert!(text.contains("Pipeline reset"), "{text}");
    assert_eq!(text.matches("[cycle 1]").count(), 2, "{text}");
    assert!(text.contains("PC 0x00000000  Next PC 0x00000004"), "{text}");
}

#[test]
fn cli_pipeline_honors_retirement_limit_and_preserves_functional_options() {
    let output = execute(
        &["--pipeline", "examples/hello.elf", "--max-steps", "1"],
        "",
    );
    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("Retirement limit reached"), "{text}");
    assert!(text.contains("Retired:      1"), "{text}");
    let invalid = execute(&["examples/hello.elf", "--max-cycles", "1"], "");
    assert_eq!(invalid.status.code(), Some(2));
    assert!(stderr(&invalid).contains("--max-cycles requires --pipeline"));
}

#[test]
fn cli_pipeline_reports_unsupported_operation() {
    let output = execute(
        &[
            "--pipeline",
            "--source",
            "tests/fixtures/unsupported-pipeline.s",
        ],
        "",
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr(&output)
            .contains("unsupported pipeline instruction mfc0 (0x40086000) at 0x00000000")
    );
}
