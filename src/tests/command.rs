use crate::debugger::{Command, MemoryWidth, WatchTarget, parse_command};

#[test]
fn command_empty() {
    assert_eq!(parse_command(""), Ok(Command::Empty));

    assert_eq!(parse_command("   "), Ok(Command::Empty));
}

#[test]
fn command_help() {
    for input in ["help", "h"] {
        assert_eq!(parse_command(input), Ok(Command::Help));
    }
}

#[test]
fn command_step() {
    for input in ["step", "s"] {
        assert_eq!(parse_command(input), Ok(Command::Step));
    }
}

#[test]
fn command_continue() {
    assert_eq!(parse_command("c"), Ok(Command::Continue(None)));
}

#[test]
fn command_continue_count() {
    assert_eq!(
        parse_command("continue 100"),
        Ok(Command::Continue(Some(100)))
    );

    assert_eq!(parse_command("c 0x10"), Ok(Command::Continue(Some(16))));
}

#[test]
fn command_reset() {
    assert_eq!(parse_command("r"), Ok(Command::Reset));
}

#[test]
fn command_inspection() {
    for (input, expected) in [
        ("regs", Command::Registers),
        ("cp0", Command::Cp0),
        ("fpu", Command::Fpu),
        ("pc", Command::Pc),
    ] {
        assert_eq!(parse_command(input), Ok(expected));
    }
}

#[test]
fn command_disassemble_default() {
    assert_eq!(
        parse_command("d"),
        Ok(Command::Disassemble {
            address: None,
            count: 8,
        })
    );
}

#[test]
fn command_disassemble_address() {
    assert_eq!(
        parse_command("d 0x100"),
        Ok(Command::Disassemble {
            address: Some(0x100),
            count: 8,
        })
    );
}

#[test]
fn command_disassemble_count() {
    assert_eq!(
        parse_command("d 0x100 16"),
        Ok(Command::Disassemble {
            address: Some(0x100),
            count: 16,
        })
    );
}

#[test]
fn command_disassemble_invalid_count() {
    for input in ["d 0 0", "d 0 257", "d 0 -1"] {
        assert!(parse_command(input).is_err());
    }
}

#[test]
fn command_memory_dump() {
    assert_eq!(
        parse_command("m 0x200 32"),
        Ok(Command::Memory {
            address: 0x200,
            size: 32,
        })
    );
}

#[test]
fn command_breakpoint() {
    assert_eq!(parse_command("b 0x100"), Ok(Command::Break(0x100)));

    assert_eq!(parse_command("delete 0x100"), Ok(Command::Delete(0x100)));
}

#[test]
fn command_watch_register() {
    for input in ["watch $t0", "watch t0", "w $8", "w 8"] {
        assert_eq!(
            parse_command(input),
            Ok(Command::Watch(WatchTarget::Register(8)))
        );
    }
}

#[test]
fn command_watch_special_registers() {
    assert_eq!(parse_command("w hi"), Ok(Command::Watch(WatchTarget::Hi)));

    assert_eq!(parse_command("w lo"), Ok(Command::Watch(WatchTarget::Lo)));

    assert_eq!(
        parse_command("w s8"),
        Ok(Command::Watch(WatchTarget::Register(30)))
    );
}

#[test]
fn command_watch_memory() {
    for (width, expected) in [
        ("1", MemoryWidth::Byte),
        ("2", MemoryWidth::Halfword),
        ("4", MemoryWidth::Word),
        ("8", MemoryWidth::Doubleword),
    ] {
        let input = format!("wm 0x200 {width}");

        assert_eq!(
            parse_command(&input),
            Ok(Command::Watch(WatchTarget::Memory {
                address: 0x200,
                width: expected,
            }))
        );
    }
}

#[test]
fn command_invalid_watchpoint() {
    for input in [
        "watch 32",
        "watch invalid",
        "watchmem 0x200 3",
        "watchmem 0x200 16",
    ] {
        assert!(parse_command(input).is_err());
    }
}

#[test]
fn command_remove_watchpoint() {
    assert_eq!(
        parse_command("unwatch t0"),
        Ok(Command::Unwatch(WatchTarget::Register(8)))
    );

    assert_eq!(
        parse_command("unwatchmem 0x200 4"),
        Ok(Command::Unwatch(WatchTarget::Memory {
            address: 0x200,
            width: MemoryWidth::Word,
        }))
    );
}

#[test]
fn command_invalid_arguments() {
    for input in [
        "unknown",
        "step 1",
        "break",
        "break 0x100000000",
        "mem 0",
        "mem 0 0",
        "mem 0 65537",
        "continue -1",
        "quit extra",
    ] {
        assert!(parse_command(input).is_err());
    }
}

#[test]
fn command_quit() {
    for input in ["quit", "q", "exit"] {
        assert_eq!(parse_command(input), Ok(Command::Quit));
    }
}
