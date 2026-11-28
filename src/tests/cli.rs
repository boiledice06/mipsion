use super::*;

#[test]
fn cli_default_configuration() {
    use crate::cli::CliAction;

    let CliAction::Run(config) = parse_cli(&["program.bin"]).unwrap() else {
        panic!("expected Run action");
    };

    assert_eq!(config.program.to_str(), Some("program.bin"));
    assert_eq!(config.load_address, 0);
    assert_eq!(config.entry, 0);
    assert_eq!(config.memory_size, 16 * 1024 * 1024);
    assert_eq!(config.max_steps, 1_000_000);
    assert!(config.stop_on_exception);
}

#[test]
fn cli_custom_configuration() {
    use crate::cli::CliAction;

    let CliAction::Run(config) = parse_cli(&[
        "program.bin",
        "--load-address",
        "0x100",
        "--entry",
        "0x104",
        "--memory-size",
        "4096",
        "--max-steps",
        "500",
        "--continue-on-exception",
    ])
    .unwrap() else {
        panic!("expected Run action");
    };

    assert_eq!(config.load_address, 256);
    assert_eq!(config.entry, 260);
    assert_eq!(config.memory_size, 4096);
    assert_eq!(config.max_steps, 500);
    assert!(!config.stop_on_exception);
}

#[test]
fn cli_entry_defaults_to_load_address() {
    use crate::cli::CliAction;

    let CliAction::Run(config) = parse_cli(&["program.bin", "--load-address", "0x100"]).unwrap()
    else {
        panic!("expected Run action");
    };

    assert_eq!(config.entry, 256);
}

#[test]
fn cli_decimal_and_hexadecimal_numbers() {
    use crate::cli::CliAction;

    let CliAction::Run(config) = parse_cli(&[
        "program.bin",
        "--load-address",
        "256",
        "--memory-size",
        "0X1000",
        "--max-steps",
        "0x10",
    ])
    .unwrap() else {
        panic!("expected Run action");
    };

    assert_eq!(config.load_address, 256);
    assert_eq!(config.memory_size, 4096);
    assert_eq!(config.max_steps, 16);
}

#[test]
fn cli_help_action() {
    use crate::cli::CliAction;

    assert_eq!(parse_cli(&["--help"]), Ok(CliAction::Help));
    assert_eq!(parse_cli(&["-h"]), Ok(CliAction::Help));
}

#[test]
fn cli_version_action() {
    use crate::cli::CliAction;

    assert_eq!(parse_cli(&["--version"]), Ok(CliAction::Version));

    assert_eq!(parse_cli(&["-V"]), Ok(CliAction::Version));
}

#[test]
fn cli_missing_program() {
    assert!(parse_cli(&[]).is_err());
}

#[test]
fn cli_unknown_option() {
    assert!(parse_cli(&["program.bin", "--unknown"]).is_err());
}

#[test]
fn cli_missing_option_value() {
    assert!(parse_cli(&["program.bin", "--max-steps"]).is_err());
}

#[test]
fn cli_invalid_number() {
    for value in ["abc", "-1", "0xZZ", "0x"] {
        assert!(parse_cli(&["program.bin", "--max-steps", value,]).is_err());
    }
}

#[test]
fn cli_address_overflow() {
    assert!(parse_cli(&["program.bin", "--load-address", "0x100000000",]).is_err());
}

#[test]
fn cli_unaligned_address() {
    for option in ["--load-address", "--entry"] {
        assert!(parse_cli(&["program.bin", option, "3"]).is_err());
    }
}

#[test]
fn cli_zero_memory_size() {
    assert!(parse_cli(&["program.bin", "--memory-size", "0",]).is_err());
}

#[test]
fn cli_multiple_program_paths() {
    assert!(parse_cli(&["first.bin", "second.bin"]).is_err());
}

#[test]
fn cli_inspection_defaults() {
    use crate::cli::CliAction;

    let CliAction::Run(config) = parse_cli(&["program.bin"]).unwrap() else {
        panic!("expected Run action");
    };

    assert!(!config.dump_registers);
    assert!(!config.dump_cp0);
    assert!(!config.dump_fpu);
    assert_eq!(config.dump_memory, None);
}

#[test]
fn cli_inspection_options() {
    use crate::cli::CliAction;

    let CliAction::Run(config) = parse_cli(&[
        "program.bin",
        "--dump-registers",
        "--dump-cp0",
        "--dump-fpu",
        "--dump-memory",
        "0x100",
        "32",
    ])
    .unwrap() else {
        panic!("expected Run action");
    };

    assert!(config.dump_registers);
    assert!(config.dump_cp0);
    assert!(config.dump_fpu);
    assert_eq!(config.dump_memory, Some((256, 32)));
}

#[test]
fn cli_dump_memory_missing_arguments() {
    assert!(parse_cli(&["program.bin", "--dump-memory",]).is_err());

    assert!(parse_cli(&["program.bin", "--dump-memory", "0",]).is_err());
}

#[test]
fn cli_dump_memory_invalid_size() {
    for size in ["0", "-1", "65537", "abc"] {
        assert!(parse_cli(&["program.bin", "--dump-memory", "0", size,]).is_err());
    }
}

#[test]
fn cli_trace_default_disabled() {
    use crate::cli::CliAction;

    let CliAction::Run(config) = parse_cli(&["program.bin"]).unwrap() else {
        panic!("expected Run action");
    };

    assert!(!config.trace);
}

#[test]
fn cli_trace_enabled() {
    use crate::cli::CliAction;

    let CliAction::Run(config) = parse_cli(&["program.bin", "--trace"]).unwrap() else {
        panic!("expected Run action");
    };

    assert!(config.trace);
}

#[test]
fn cli_debug_default_disabled() {
    use crate::cli::CliAction;

    let CliAction::Run(config) = parse_cli(&["program.bin"]).unwrap() else {
        panic!("expected Run action");
    };

    assert!(!config.debug);
}

#[test]
fn cli_debug_enabled() {
    use crate::cli::CliAction;

    let CliAction::Run(config) = parse_cli(&["--debug", "program.bin"]).unwrap() else {
        panic!("expected Run action");
    };

    assert!(config.debug);
}

#[test]
fn cli_debug_with_execution_limit() {
    use crate::cli::CliAction;

    let CliAction::Run(config) =
        parse_cli(&["program.bin", "--debug", "--max-steps", "100"]).unwrap()
    else {
        panic!("expected Run action");
    };

    assert!(config.debug);
    assert_eq!(config.max_steps, 100);
}

#[test]
fn cli_debug_conflicting_options() {
    for option in [
        "--trace",
        "--dump-registers",
        "--dump-cp0",
        "--dump-fpu",
        "--continue-on-exception",
    ] {
        assert!(parse_cli(&["program.bin", "--debug", option,]).is_err());
    }

    assert!(parse_cli(&["program.bin", "--debug", "--dump-memory", "0", "4",]).is_err());
}
