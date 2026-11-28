use crate::isa::disassembler::{disassemble, format_disassembly};

#[test]
fn disassemble_nop() {
    assert_eq!(disassemble(0, 0), "nop");
}

#[test]
fn disassemble_add() {
    assert_eq!(disassemble(0x0109_5020, 0), "add $t2, $t0, $t1");
}

#[test]
fn disassemble_addiu() {
    assert_eq!(disassemble(0x2408_002A, 0), "addiu $t0, $zero, 42");
}

#[test]
fn disassemble_negative_immediate() {
    assert_eq!(disassemble(0x250A_FFFF, 0), "addiu $t2, $t0, -1");
}

#[test]
fn disassemble_logical_immediate() {
    assert_eq!(disassemble(0x3109_FFFF, 0), "andi $t1, $t0, 0xFFFF");
}

#[test]
fn disassemble_lui() {
    assert_eq!(disassemble(0x3C08_1234, 0), "lui $t0, 0x1234");
}

#[test]
fn disassemble_shift() {
    assert_eq!(disassemble(0x0008_4880, 0), "sll $t1, $t0, 2");
}

#[test]
fn disassemble_load() {
    assert_eq!(disassemble(0x8FA8_0004, 0), "lw $t0, 4($sp)");
}

#[test]
fn disassemble_store_negative_offset() {
    assert_eq!(disassemble(0xAFA8_FFFC, 0), "sw $t0, -4($sp)");
}

#[test]
fn disassemble_forward_branch() {
    assert_eq!(
        disassemble(0x1109_0003, 0x0040_0000),
        "beq $t0, $t1, 0x00400010"
    );
}

#[test]
fn disassemble_backward_branch() {
    assert_eq!(
        disassemble(0x1109_FFFC, 0x0040_000C),
        "beq $t0, $t1, 0x00400000"
    );
}

#[test]
fn disassemble_branch_address_wrap() {
    assert_eq!(
        disassemble(0x1109_0001, 0xFFFF_FFFC),
        "beq $t0, $t1, 0x00000004"
    );
}

#[test]
fn disassemble_jump() {
    assert_eq!(disassemble(0x0810_0000, 0x0040_0000), "j 0x00400000");
}

#[test]
fn disassemble_jump_region_boundary() {
    assert_eq!(disassemble(0x0800_0000, 0x0FFF_FFFC), "j 0x10000000");
}

#[test]
fn disassemble_jump_register() {
    assert_eq!(disassemble(0x03E0_0008, 0), "jr $ra");
}

#[test]
fn disassemble_syscall() {
    assert_eq!(disassemble(0x0000_000C, 0), "syscall");
}

#[test]
fn disassemble_cp0_transfer() {
    assert_eq!(disassemble(0x4008_2002, 0), "mfc0 $t0, $4, 2");
}

#[test]
fn disassemble_fpu_arithmetic() {
    assert_eq!(disassemble(0x4602_0800, 0), "add.s $f0, $f1, $f2");
}

#[test]
fn disassemble_unknown_word() {
    assert_eq!(disassemble(0xFFFF_FFFF, 0), ".word 0xFFFFFFFF");
}

#[test]
fn disassemble_formatted_line() {
    assert_eq!(
        format_disassembly(0x2408_002A, 0x0040_0000),
        "00400000  2408002A  addiu $t0, $zero, 42"
    );
}
