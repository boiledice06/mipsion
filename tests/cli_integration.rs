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
