# The taken branch executes its delay slot and skips the next word.
.text
    addiu $t0, $zero, 41
    b finished
    addiu $t0, $t0, 1
    addiu $t0, $zero, 99
finished:
    la $t1, marker
    lbu $t2, 0($t1)
    syscall
.data
marker:
    .byte 0x5a
