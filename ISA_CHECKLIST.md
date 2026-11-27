# MIPSion — MIPS32 Release 2 ISA 체크리스트

기준은 MIPS Technologies의 **MD00086 Rev. 2.62** §3.1, §3.2, 개별 명령어 페이지, Appendix A다. 표의 페이지는 인쇄본 번호다. JALX의 32-bit 인코딩은 **MD00076 Rev. 2.50** p.76도 참조한다. [Volume II](https://www.cse.iitb.ac.in/~br/courses/cs305-autumn2011/labs/lab05/MD00086-2B-MIPS32BIS-AFP-02_62.pdf) · [Volume IV-a](https://www.0x04.net/~mwk/doc/mips/MD00076-2B-MIPS1632-AFP-02.50.pdf)

- **CPU 필수**는 branch-likely를 포함한다. **CP0**는 privileged 명령어이며 구현별 CP0 기능 차이는 Volume III의 규칙을 따른다. **TLB**는 표준 TLB를 채택할 때 적용된다.

- **FPU 기본**은 FPU를 채택할 때 필요한 S/D/W 프로필과 지원 명령어다. **FPU 64**는 64-bit FPU 기능을 사용하는 추가 명령어다. MFHC1/MTHC1은 기본 프로필에서도 doubleword의 상위 word에 접근한다. **CP2**, **EJTAG**, **MIPS16e**는 선택 기능이다.

- 집계 범위는 Volume II의 32-bit 명령어와 Appendix A의 JALX다. FPU format 및 compare condition마다 한 행을 집계하고, B/BAL/NOP는 Idiom으로 별도 집계한다. MIPS16e의 자체 인코딩, DSP, MT, SmartMIPS, MIPS-3D 등의 독립 ASE 명령어는 각 Volume IV의 별도 ISA 범위다. CP2 내부 연산은 구현 정의다.

- 실행 상태는 `대기`, `부분`, `구현`, `별칭`이다. `부분`은 일부 동작이 실행 가능하다는 뜻이다. 회귀 상태는 현재 검증 범위다.

- opcode는 bits 31:26이다. funct는 해당 instruction class에서 bits 5:0이 function field일 때만 표기한다. mask/pattern은 `word & mask == pattern`에 사용한다. 레지스터 조합이나 범위 제한은 실행 시점 검증 사항이다.

- branch의 목적지는 `PC + 4 + (sign_extend(offset) << 2)`이며 link 주소는 `PC + 8`이다. J/JAL/JALX는 `PC + 4`에서 상위 4비트를 가져온다. branch/jump/ERET/DERET가 branch 또는 jump delay slot에 오면 UNPREDICTABLE이다. WAIT와 PAUSE의 delay slot 제한은 각 행을 참조한다.

- 일반 load/store의 유효 주소는 `GPR[base] + sign_extend(offset)`이다. 정렬이 필요한 접근에서 halfword/word/doubleword의 주소가 각각 2/4/8바이트 경계에 맞지 않으면 Address Error다. LWL/LWR/SWL/SWR는 정렬되지 않은 주소에서 부분 word를 접근한다.

- FPU 연산에 유효하지 않은 FPR 또는 형식을 사용하면 UNPREDICTABLE이다. PS 산술과 비교는 두 lane에 독립적으로 적용된다. COP1X, L, PS는 64-bit FPU 기능이 필요하다. 일반적인 산술 결과는 FCSR rounding mode를 따른다. FPU 예외는 Cause/Enable에 따르며 Unimplemented Operation은 항상 trap이다. Coprocessor Unusable은 coprocessor 사용이 금지되었을 때 적용된다. 선택 기능 미지원 시 §3.1에 따른 Reserved Instruction 또는 Coprocessor Unusable 처리가 필요하다.

- FPU compare의 condition 하위 3비트는 비교 관계를, bit 3은 qNaN에서 Invalid Operation을 발생시킬지 지정한다. PS 비교에서 홀수 cc는 UNPREDICTABLE이다. 짝수 cc를 사용해 하위 결과를 cc, 상위 결과를 cc+1에 기록한다.

| 집계 | 수 |
| --- | ---: |
| CPU 필수 | 105 |
| CP0 / TLB | 9 / 4 |
| FPU 기본 / FPU 64 | 86 / 69 |
| CP2 / EJTAG / MIPS16e | 15 / 2 / 1 |
| 명령어 합계 | **291** |
| Idiom | 3 |

현재 실행 경로는 114/291개(39.18%)이며 CPU 필수 105개 모두에 기본 실행 경로가 존재한다. 구현 68개, 부분 구현 46개, 대기 177개이며 별칭 3개는 분모에서 제외한다. 사용자 보고 기준 회귀 테스트는 145/145 통과했다.

이는 명령어별 전체 명세 적합성이나 코드 커버리지 100%를 의미하지 않는다.

| 실행 상태 | 수 | 비율 |
| --- | ---: | ---: |
| 구현 | 68 | 23.37% |
| 부분 | 46 | 15.81% |
| 대기 | 177 | 60.82% |
| 실행 경로 합계 | 114 | 39.18% |

v0.2.0에서는 CP0 register 접근, privilege mode, exception vector, ERET, timer/software interrupt, MMU, 32-entry TLB 및 TLB refill을 지원한다.

Bare Mode는 기존 회귀 테스트와의 호환성을 위해 유지하며, TLB Mode에서 실제 virtual address translation을 수행한다. 물리 메모리 범위 검사, 완전한 CP0 명세, 중복 TLB entry 처리, cache 및 execution hazard는 후속 구현 대상이다.

## 작업 순서

- [x] Integer ALU, HI/LO, load/store, branch/jump 및 delay slot.
- [x] 기초 CP0 예외 상태, overflow, trap 및 주소 정렬 검사.
- [x] 단일 CPU LL/SC, synchronization/hazard 기본 경로, RDHWR 기본 경로.
- [x] CP0 접근 명령어, 기본 권한 모델, 예외 벡터 및 ERET.
- [x] 기본 MMU/TLB 및 TLB 접근 예외.
- [ ] 물리 메모리 범위 검사 및 cache 기능.
- [ ] FPU 및 선택 기능.

## 정의 생성

아래 표가 mnemonic, 런타임 상태, mask/pattern의 원본이다. Cargo가 `build.rs`를 실행해 `대기`, `부분`, `구현` 행의 `Operation`과 `INSTRUCTIONS`를 `OUT_DIR`에 생성한다. Decoder 등록과 실행 구현은 별개다. 동작을 작성한 뒤 해당 행의 상태를 갱신하고 회귀 테스트를 추가한다. SLL과 EHB/PAUSE/SSNOP처럼 인코딩이 겹치는 경우 구체적인 mask를 우선한다. 별칭 행은 문서에만 포함한다.

테스트에서는 `encode_r/i/j`에 인코딩 필드 값을 전달한다. `encode_j`의 target은 주소에서 추출한 26-bit 필드다. `run`은 0번 주소부터 word 수만큼 step을 실행한다. branch 및 메모리 테스트는 `load_program`과 `run_steps`에 직접 준비한 CPU와 Memory를 전달할 수 있다.

## 명령어 목록

| Mnemonic | 분류 | 실행 | 테스트 | Opcode | Funct | Mask | Pattern | Operand format | 동작 | 예외 및 제한 | 쪽 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `ABS.D` | FPU 기본 | 대기 | 대기 | `0x11` | `0x05` | `0xFFFF003F` | `0x46200005` | `fd, fs` | 절댓값 | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation | 44 |
| `ABS.PS` | FPU 64 | 대기 | 대기 | `0x11` | `0x05` | `0xFFFF003F` | `0x46C00005` | `fd, fs` | 절댓값 | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation | 44 |
| `ABS.S` | FPU 기본 | 대기 | 대기 | `0x11` | `0x05` | `0xFFFF003F` | `0x46000005` | `fd, fs` | 절댓값 | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation | 44 |
| `ADD` | CPU 필수 | 부분 | 기초 예외 경로 | `0x00` | `0x20` | `0xFC0007FF` | `0x00000020` | `rd, rs, rt` | signed 32-bit 덧셈. ADDI는 sign-extended immediate 사용 | Integer Overflow (signed 결과 범위 초과 시, 목적 register 기록 생략) | 45 |
| `ADD.D` | FPU 기본 | 대기 | 대기 | `0x11` | `0x00` | `0xFFE0003F` | `0x46200000` | `fd, fs, ft` | fs + ft | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation, Inexact, Overflow, Underflow | 46 |
| `ADD.PS` | FPU 64 | 대기 | 대기 | `0x11` | `0x00` | `0xFFE0003F` | `0x46C00000` | `fd, fs, ft` | fs + ft | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation, Inexact, Overflow, Underflow | 46 |
| `ADD.S` | FPU 기본 | 대기 | 대기 | `0x11` | `0x00` | `0xFFE0003F` | `0x46000000` | `fd, fs, ft` | fs + ft | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation, Inexact, Overflow, Underflow | 46 |
| `ADDI` | CPU 필수 | 부분 | 기초 예외 경로 | `0x08` | — | `0xFC000000` | `0x20000000` | `rt, rs, immediate` | signed 32-bit 덧셈. ADDI는 sign-extended immediate 사용 | Integer Overflow (signed 결과 범위 초과 시, 목적 register 기록 생략) | 47 |
| `ADDIU` | CPU 필수 | 구현 | 회귀 통과 | `0x09` | — | `0xFC000000` | `0x24000000` | `rt, rs, immediate` | modulo 2³² 덧셈. ADDIU는 sign-extended immediate 사용 | — | 48 |
| `ADDU` | CPU 필수 | 구현 | 회귀 통과 | `0x00` | `0x21` | `0xFC0007FF` | `0x00000021` | `rd, rs, rt` | modulo 2³² 덧셈. ADDIU는 sign-extended immediate 사용 | — | 49 |
| `ALNV.PS` | FPU 64 | 대기 | 대기 | `0x13` | `0x1E` | `0xFC00003F` | `0x4C00001E` | `fd, fs, ft, rs` | rs의 하위 주소 비트에 따라 두 PS 값의 lane을 정렬 | Coprocessor Unusable, Reserved Instruction | 50 |
| `AND` | CPU 필수 | 구현 | 회귀 통과 | `0x00` | `0x24` | `0xFC0007FF` | `0x00000024` | `rd, rs, rt` | bitwise AND | — | 52 |
| `ANDI` | CPU 필수 | 구현 | 회귀 통과 | `0x0C` | — | `0xFC000000` | `0x30000000` | `rt, rs, immediate` | zero-extended immediate와 bitwise AND | — | 53 |
| `B` | Idiom | 별칭 | — | `0x04` | — | `0xFFFF0000` | `0x10000000` | `offset` | BEQ $zero, $zero와 동일한 무조건 branch | — | 54 |
| `BAL` | Idiom | 별칭 | — | `0x01` | — | `0xFFFF0000` | `0x04110000` | `offset` | BGEZAL $zero와 동일한 branch-and-link | — | 55 |
| `BC1F` | FPU 기본 | 대기 | 대기 | `0x11` | — | `0xFFE30000` | `0x45000000` | `[cc,] offset` | CP1 조건 코드가 0이면 PC-relative branch | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation | 56 |
| `BC1FL` | FPU 기본 | 대기 | 대기 | `0x11` | — | `0xFFE30000` | `0x45020000` | `[cc,] offset` | CP1 조건 코드가 0이면 PC-relative branch. 미분기 시 delay slot annul | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation | 58 |
| `BC1T` | FPU 기본 | 대기 | 대기 | `0x11` | — | `0xFFE30000` | `0x45010000` | `[cc,] offset` | CP1 조건 코드가 1이면 PC-relative branch | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation | 60 |
| `BC1TL` | FPU 기본 | 대기 | 대기 | `0x11` | — | `0xFFE30000` | `0x45030000` | `[cc,] offset` | CP1 조건 코드가 1이면 PC-relative branch. 미분기 시 delay slot annul | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation | 62 |
| `BC2F` | CP2 | 대기 | 대기 | `0x12` | — | `0xFFE30000` | `0x49000000` | `[cc,] offset` | CP2 조건 코드가 0이면 PC-relative branch | Coprocessor Unusable, Reserved Instruction | 64 |
| `BC2FL` | CP2 | 대기 | 대기 | `0x12` | — | `0xFFE30000` | `0x49020000` | `[cc,] offset` | CP2 조건 코드가 0이면 PC-relative branch. 미분기 시 delay slot annul | Coprocessor Unusable, Reserved Instruction | 65 |
| `BC2T` | CP2 | 대기 | 대기 | `0x12` | — | `0xFFE30000` | `0x49010000` | `[cc,] offset` | CP2 조건 코드가 1이면 PC-relative branch | Coprocessor Unusable, Reserved Instruction | 66 |
| `BC2TL` | CP2 | 대기 | 대기 | `0x12` | — | `0xFFE30000` | `0x49030000` | `[cc,] offset` | CP2 조건 코드가 1이면 PC-relative branch. 미분기 시 delay slot annul | Coprocessor Unusable, Reserved Instruction | 67 |
| `BEQ` | CPU 필수 | 구현 | 회귀 통과 | `0x04` | — | `0xFC000000` | `0x10000000` | `rs, rt, offset` | rs == rt이면 PC-relative branch | — | 68 |
| `BEQL` | CPU 필수 | 구현 | 회귀 통과 | `0x14` | — | `0xFC000000` | `0x50000000` | `rs, rt, offset` | rs == rt이면 PC-relative branch. 미분기 시 delay slot annul | — | 69 |
| `BGEZ` | CPU 필수 | 구현 | 회귀 통과 | `0x01` | — | `0xFC1F0000` | `0x04010000` | `rs, offset` | signed rs >= 0이면 PC-relative branch | — | 70 |
| `BGEZAL` | CPU 필수 | 구현 | 회귀 통과 | `0x01` | — | `0xFC1F0000` | `0x04110000` | `rs, offset` | signed rs >= 0이면 PC-relative branch. $31에 PC+8 기록 | rs == $31이면 UNPREDICTABLE | 71 |
| `BGEZALL` | CPU 필수 | 구현 | 회귀 통과 | `0x01` | — | `0xFC1F0000` | `0x04130000` | `rs, offset` | signed rs >= 0이면 PC-relative branch. $31에 PC+8 기록. 미분기 시 delay slot annul | rs == $31이면 UNPREDICTABLE | 72 |
| `BGEZL` | CPU 필수 | 구현 | 회귀 통과 | `0x01` | — | `0xFC1F0000` | `0x04030000` | `rs, offset` | signed rs >= 0이면 PC-relative branch. 미분기 시 delay slot annul | — | 74 |
| `BGTZ` | CPU 필수 | 구현 | 회귀 통과 | `0x07` | — | `0xFC1F0000` | `0x1C000000` | `rs, offset` | signed rs > 0이면 PC-relative branch | — | 75 |
| `BGTZL` | CPU 필수 | 구현 | 회귀 통과 | `0x17` | — | `0xFC1F0000` | `0x5C000000` | `rs, offset` | signed rs > 0이면 PC-relative branch. 미분기 시 delay slot annul | — | 76 |
| `BLEZ` | CPU 필수 | 구현 | 회귀 통과 | `0x06` | — | `0xFC1F0000` | `0x18000000` | `rs, offset` | signed rs <= 0이면 PC-relative branch | — | 77 |
| `BLEZL` | CPU 필수 | 구현 | 회귀 통과 | `0x16` | — | `0xFC1F0000` | `0x58000000` | `rs, offset` | signed rs <= 0이면 PC-relative branch. 미분기 시 delay slot annul | — | 78 |
| `BLTZ` | CPU 필수 | 구현 | 회귀 통과 | `0x01` | — | `0xFC1F0000` | `0x04000000` | `rs, offset` | signed rs < 0이면 PC-relative branch | — | 79 |
| `BLTZAL` | CPU 필수 | 구현 | 회귀 통과 | `0x01` | — | `0xFC1F0000` | `0x04100000` | `rs, offset` | signed rs < 0이면 PC-relative branch. $31에 PC+8 기록 | rs == $31이면 UNPREDICTABLE | 80 |
| `BLTZALL` | CPU 필수 | 구현 | 회귀 통과 | `0x01` | — | `0xFC1F0000` | `0x04120000` | `rs, offset` | signed rs < 0이면 PC-relative branch. $31에 PC+8 기록. 미분기 시 delay slot annul | rs == $31이면 UNPREDICTABLE | 81 |
| `BLTZL` | CPU 필수 | 구현 | 회귀 통과 | `0x01` | — | `0xFC1F0000` | `0x04020000` | `rs, offset` | signed rs < 0이면 PC-relative branch. 미분기 시 delay slot annul | — | 83 |
| `BNE` | CPU 필수 | 구현 | 회귀 통과 | `0x05` | — | `0xFC000000` | `0x14000000` | `rs, rt, offset` | rs != rt이면 PC-relative branch | — | 84 |
| `BNEL` | CPU 필수 | 구현 | 회귀 통과 | `0x15` | — | `0xFC000000` | `0x54000000` | `rs, rt, offset` | rs != rt이면 PC-relative branch. 미분기 시 delay slot annul | — | 85 |
| `BREAK` | CPU 필수 | 부분 | 기초 예외 경로 | `0x00` | `0x0D` | `0xFC00003F` | `0x0000000D` | `[code]` | Breakpoint 예외 발생 | Breakpoint | 86 |
| `C.EQ.D` | FPU 기본 | 대기 | 대기 | `0x11` | `0x32` | `0xFFE000FF` | `0x46200032` | `[cc,] fs, ft` | FCC에 equal 비교 결과 기록. sNaN에서 Invalid Operation | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation | 87 |
| `C.EQ.PS` | FPU 64 | 대기 | 대기 | `0x11` | `0x32` | `0xFFE000FF` | `0x46C00032` | `[cc,] fs, ft` | FCC에 equal 비교 결과 기록. sNaN에서 Invalid Operation | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation | 87 |
| `C.EQ.S` | FPU 기본 | 대기 | 대기 | `0x11` | `0x32` | `0xFFE000FF` | `0x46000032` | `[cc,] fs, ft` | FCC에 equal 비교 결과 기록. sNaN에서 Invalid Operation | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation | 87 |
| `C.F.D` | FPU 기본 | 대기 | 대기 | `0x11` | `0x30` | `0xFFE000FF` | `0x46200030` | `[cc,] fs, ft` | FCC에 항상 false 비교 결과 기록. sNaN에서 Invalid Operation | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation | 87 |
| `C.F.PS` | FPU 64 | 대기 | 대기 | `0x11` | `0x30` | `0xFFE000FF` | `0x46C00030` | `[cc,] fs, ft` | FCC에 항상 false 비교 결과 기록. sNaN에서 Invalid Operation | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation | 87 |
| `C.F.S` | FPU 기본 | 대기 | 대기 | `0x11` | `0x30` | `0xFFE000FF` | `0x46000030` | `[cc,] fs, ft` | FCC에 항상 false 비교 결과 기록. sNaN에서 Invalid Operation | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation | 87 |
| `C.LE.D` | FPU 기본 | 대기 | 대기 | `0x11` | `0x3E` | `0xFFE000FF` | `0x4620003E` | `[cc,] fs, ft` | FCC에 less 또는 equal 비교 결과 기록. sNaN/qNaN에서 Invalid Operation | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation | 87 |
| `C.LE.PS` | FPU 64 | 대기 | 대기 | `0x11` | `0x3E` | `0xFFE000FF` | `0x46C0003E` | `[cc,] fs, ft` | FCC에 less 또는 equal 비교 결과 기록. sNaN/qNaN에서 Invalid Operation | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation | 87 |
| `C.LE.S` | FPU 기본 | 대기 | 대기 | `0x11` | `0x3E` | `0xFFE000FF` | `0x4600003E` | `[cc,] fs, ft` | FCC에 less 또는 equal 비교 결과 기록. sNaN/qNaN에서 Invalid Operation | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation | 87 |
| `C.LT.D` | FPU 기본 | 대기 | 대기 | `0x11` | `0x3C` | `0xFFE000FF` | `0x4620003C` | `[cc,] fs, ft` | FCC에 less 비교 결과 기록. sNaN/qNaN에서 Invalid Operation | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation | 87 |
| `C.LT.PS` | FPU 64 | 대기 | 대기 | `0x11` | `0x3C` | `0xFFE000FF` | `0x46C0003C` | `[cc,] fs, ft` | FCC에 less 비교 결과 기록. sNaN/qNaN에서 Invalid Operation | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation | 87 |
| `C.LT.S` | FPU 기본 | 대기 | 대기 | `0x11` | `0x3C` | `0xFFE000FF` | `0x4600003C` | `[cc,] fs, ft` | FCC에 less 비교 결과 기록. sNaN/qNaN에서 Invalid Operation | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation | 87 |
| `C.NGE.D` | FPU 기본 | 대기 | 대기 | `0x11` | `0x3D` | `0xFFE000FF` | `0x4620003D` | `[cc,] fs, ft` | FCC에 unordered 또는 less 비교 결과 기록. sNaN/qNaN에서 Invalid Operation | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation | 87 |
| `C.NGE.PS` | FPU 64 | 대기 | 대기 | `0x11` | `0x3D` | `0xFFE000FF` | `0x46C0003D` | `[cc,] fs, ft` | FCC에 unordered 또는 less 비교 결과 기록. sNaN/qNaN에서 Invalid Operation | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation | 87 |
| `C.NGE.S` | FPU 기본 | 대기 | 대기 | `0x11` | `0x3D` | `0xFFE000FF` | `0x4600003D` | `[cc,] fs, ft` | FCC에 unordered 또는 less 비교 결과 기록. sNaN/qNaN에서 Invalid Operation | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation | 87 |
| `C.NGL.D` | FPU 기본 | 대기 | 대기 | `0x11` | `0x3B` | `0xFFE000FF` | `0x4620003B` | `[cc,] fs, ft` | FCC에 unordered 또는 equal 비교 결과 기록. sNaN/qNaN에서 Invalid Operation | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation | 87 |
| `C.NGL.PS` | FPU 64 | 대기 | 대기 | `0x11` | `0x3B` | `0xFFE000FF` | `0x46C0003B` | `[cc,] fs, ft` | FCC에 unordered 또는 equal 비교 결과 기록. sNaN/qNaN에서 Invalid Operation | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation | 87 |
| `C.NGL.S` | FPU 기본 | 대기 | 대기 | `0x11` | `0x3B` | `0xFFE000FF` | `0x4600003B` | `[cc,] fs, ft` | FCC에 unordered 또는 equal 비교 결과 기록. sNaN/qNaN에서 Invalid Operation | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation | 87 |
| `C.NGLE.D` | FPU 기본 | 대기 | 대기 | `0x11` | `0x39` | `0xFFE000FF` | `0x46200039` | `[cc,] fs, ft` | FCC에 unordered 비교 결과 기록. sNaN/qNaN에서 Invalid Operation | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation | 87 |
| `C.NGLE.PS` | FPU 64 | 대기 | 대기 | `0x11` | `0x39` | `0xFFE000FF` | `0x46C00039` | `[cc,] fs, ft` | FCC에 unordered 비교 결과 기록. sNaN/qNaN에서 Invalid Operation | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation | 87 |
| `C.NGLE.S` | FPU 기본 | 대기 | 대기 | `0x11` | `0x39` | `0xFFE000FF` | `0x46000039` | `[cc,] fs, ft` | FCC에 unordered 비교 결과 기록. sNaN/qNaN에서 Invalid Operation | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation | 87 |
| `C.NGT.D` | FPU 기본 | 대기 | 대기 | `0x11` | `0x3F` | `0xFFE000FF` | `0x4620003F` | `[cc,] fs, ft` | FCC에 unordered 또는 less 또는 equal 비교 결과 기록. sNaN/qNaN에서 Invalid Operation | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation | 87 |
| `C.NGT.PS` | FPU 64 | 대기 | 대기 | `0x11` | `0x3F` | `0xFFE000FF` | `0x46C0003F` | `[cc,] fs, ft` | FCC에 unordered 또는 less 또는 equal 비교 결과 기록. sNaN/qNaN에서 Invalid Operation | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation | 87 |
| `C.NGT.S` | FPU 기본 | 대기 | 대기 | `0x11` | `0x3F` | `0xFFE000FF` | `0x4600003F` | `[cc,] fs, ft` | FCC에 unordered 또는 less 또는 equal 비교 결과 기록. sNaN/qNaN에서 Invalid Operation | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation | 87 |
| `C.OLE.D` | FPU 기본 | 대기 | 대기 | `0x11` | `0x36` | `0xFFE000FF` | `0x46200036` | `[cc,] fs, ft` | FCC에 less 또는 equal 비교 결과 기록. sNaN에서 Invalid Operation | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation | 87 |
| `C.OLE.PS` | FPU 64 | 대기 | 대기 | `0x11` | `0x36` | `0xFFE000FF` | `0x46C00036` | `[cc,] fs, ft` | FCC에 less 또는 equal 비교 결과 기록. sNaN에서 Invalid Operation | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation | 87 |
| `C.OLE.S` | FPU 기본 | 대기 | 대기 | `0x11` | `0x36` | `0xFFE000FF` | `0x46000036` | `[cc,] fs, ft` | FCC에 less 또는 equal 비교 결과 기록. sNaN에서 Invalid Operation | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation | 87 |
| `C.OLT.D` | FPU 기본 | 대기 | 대기 | `0x11` | `0x34` | `0xFFE000FF` | `0x46200034` | `[cc,] fs, ft` | FCC에 less 비교 결과 기록. sNaN에서 Invalid Operation | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation | 87 |
| `C.OLT.PS` | FPU 64 | 대기 | 대기 | `0x11` | `0x34` | `0xFFE000FF` | `0x46C00034` | `[cc,] fs, ft` | FCC에 less 비교 결과 기록. sNaN에서 Invalid Operation | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation | 87 |
| `C.OLT.S` | FPU 기본 | 대기 | 대기 | `0x11` | `0x34` | `0xFFE000FF` | `0x46000034` | `[cc,] fs, ft` | FCC에 less 비교 결과 기록. sNaN에서 Invalid Operation | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation | 87 |
| `C.SEQ.D` | FPU 기본 | 대기 | 대기 | `0x11` | `0x3A` | `0xFFE000FF` | `0x4620003A` | `[cc,] fs, ft` | FCC에 equal 비교 결과 기록. sNaN/qNaN에서 Invalid Operation | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation | 87 |
| `C.SEQ.PS` | FPU 64 | 대기 | 대기 | `0x11` | `0x3A` | `0xFFE000FF` | `0x46C0003A` | `[cc,] fs, ft` | FCC에 equal 비교 결과 기록. sNaN/qNaN에서 Invalid Operation | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation | 87 |
| `C.SEQ.S` | FPU 기본 | 대기 | 대기 | `0x11` | `0x3A` | `0xFFE000FF` | `0x4600003A` | `[cc,] fs, ft` | FCC에 equal 비교 결과 기록. sNaN/qNaN에서 Invalid Operation | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation | 87 |
| `C.SF.D` | FPU 기본 | 대기 | 대기 | `0x11` | `0x38` | `0xFFE000FF` | `0x46200038` | `[cc,] fs, ft` | FCC에 항상 false 비교 결과 기록. sNaN/qNaN에서 Invalid Operation | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation | 87 |
| `C.SF.PS` | FPU 64 | 대기 | 대기 | `0x11` | `0x38` | `0xFFE000FF` | `0x46C00038` | `[cc,] fs, ft` | FCC에 항상 false 비교 결과 기록. sNaN/qNaN에서 Invalid Operation | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation | 87 |
| `C.SF.S` | FPU 기본 | 대기 | 대기 | `0x11` | `0x38` | `0xFFE000FF` | `0x46000038` | `[cc,] fs, ft` | FCC에 항상 false 비교 결과 기록. sNaN/qNaN에서 Invalid Operation | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation | 87 |
| `C.UEQ.D` | FPU 기본 | 대기 | 대기 | `0x11` | `0x33` | `0xFFE000FF` | `0x46200033` | `[cc,] fs, ft` | FCC에 unordered 또는 equal 비교 결과 기록. sNaN에서 Invalid Operation | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation | 87 |
| `C.UEQ.PS` | FPU 64 | 대기 | 대기 | `0x11` | `0x33` | `0xFFE000FF` | `0x46C00033` | `[cc,] fs, ft` | FCC에 unordered 또는 equal 비교 결과 기록. sNaN에서 Invalid Operation | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation | 87 |
| `C.UEQ.S` | FPU 기본 | 대기 | 대기 | `0x11` | `0x33` | `0xFFE000FF` | `0x46000033` | `[cc,] fs, ft` | FCC에 unordered 또는 equal 비교 결과 기록. sNaN에서 Invalid Operation | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation | 87 |
| `C.ULE.D` | FPU 기본 | 대기 | 대기 | `0x11` | `0x37` | `0xFFE000FF` | `0x46200037` | `[cc,] fs, ft` | FCC에 unordered 또는 less 또는 equal 비교 결과 기록. sNaN에서 Invalid Operation | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation | 87 |
| `C.ULE.PS` | FPU 64 | 대기 | 대기 | `0x11` | `0x37` | `0xFFE000FF` | `0x46C00037` | `[cc,] fs, ft` | FCC에 unordered 또는 less 또는 equal 비교 결과 기록. sNaN에서 Invalid Operation | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation | 87 |
| `C.ULE.S` | FPU 기본 | 대기 | 대기 | `0x11` | `0x37` | `0xFFE000FF` | `0x46000037` | `[cc,] fs, ft` | FCC에 unordered 또는 less 또는 equal 비교 결과 기록. sNaN에서 Invalid Operation | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation | 87 |
| `C.ULT.D` | FPU 기본 | 대기 | 대기 | `0x11` | `0x35` | `0xFFE000FF` | `0x46200035` | `[cc,] fs, ft` | FCC에 unordered 또는 less 비교 결과 기록. sNaN에서 Invalid Operation | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation | 87 |
| `C.ULT.PS` | FPU 64 | 대기 | 대기 | `0x11` | `0x35` | `0xFFE000FF` | `0x46C00035` | `[cc,] fs, ft` | FCC에 unordered 또는 less 비교 결과 기록. sNaN에서 Invalid Operation | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation | 87 |
| `C.ULT.S` | FPU 기본 | 대기 | 대기 | `0x11` | `0x35` | `0xFFE000FF` | `0x46000035` | `[cc,] fs, ft` | FCC에 unordered 또는 less 비교 결과 기록. sNaN에서 Invalid Operation | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation | 87 |
| `C.UN.D` | FPU 기본 | 대기 | 대기 | `0x11` | `0x31` | `0xFFE000FF` | `0x46200031` | `[cc,] fs, ft` | FCC에 unordered 비교 결과 기록. sNaN에서 Invalid Operation | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation | 87 |
| `C.UN.PS` | FPU 64 | 대기 | 대기 | `0x11` | `0x31` | `0xFFE000FF` | `0x46C00031` | `[cc,] fs, ft` | FCC에 unordered 비교 결과 기록. sNaN에서 Invalid Operation | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation | 87 |
| `C.UN.S` | FPU 기본 | 대기 | 대기 | `0x11` | `0x31` | `0xFFE000FF` | `0x46000031` | `[cc,] fs, ft` | FCC에 unordered 비교 결과 기록. sNaN에서 Invalid Operation | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation | 87 |
| `CACHE` | CP0 | 대기 | 대기 | `0x2F` | — | `0xFC000000` | `0xBC000000` | `op, offset(base)` | op이 지정하는 cache 조작 | TLB Refill Exception. TLB Invalid Exception Coprocessor Unusable Exception Address Error Exception Cache Error Exception Bus Error Exception | 91 |
| `CEIL.L.D` | FPU 64 | 대기 | 대기 | `0x11` | `0x0A` | `0xFFFF003F` | `0x4620000A` | `fd, fs` | +∞ 방향 반올림으로 L signed 정수 변환 | Coprocessor Unusable, Reserved Instruction / FPU: Invalid Operation, Unimplemented Operation, Inexact, Overflow | 97 |
| `CEIL.L.S` | FPU 64 | 대기 | 대기 | `0x11` | `0x0A` | `0xFFFF003F` | `0x4600000A` | `fd, fs` | +∞ 방향 반올림으로 L signed 정수 변환 | Coprocessor Unusable, Reserved Instruction / FPU: Invalid Operation, Unimplemented Operation, Inexact, Overflow | 97 |
| `CEIL.W.D` | FPU 기본 | 대기 | 대기 | `0x11` | `0x0E` | `0xFFFF003F` | `0x4620000E` | `fd, fs` | +∞ 방향 반올림으로 W signed 정수 변환 | Coprocessor Unusable, Reserved Instruction / FPU: Invalid Operation, Unimplemented Operation, Inexact, Overflow | 98 |
| `CEIL.W.S` | FPU 기본 | 대기 | 대기 | `0x11` | `0x0E` | `0xFFFF003F` | `0x4600000E` | `fd, fs` | +∞ 방향 반올림으로 W signed 정수 변환 | Coprocessor Unusable, Reserved Instruction / FPU: Invalid Operation, Unimplemented Operation, Inexact, Overflow | 98 |
| `CFC1` | FPU 기본 | 대기 | 대기 | `0x11` | — | `0xFFE007FF` | `0x44400000` | `rt, fs` | CP1 control register → rt | Coprocessor Unusable, Reserved Instruction | 99 |
| `CFC2` | CP2 | 대기 | 대기 | `0x12` | — | `0xFFE00000` | `0x48400000` | `rt, rd` (예시) | CP2 control register → rt | Coprocessor Unusable, Reserved Instruction. Impl의 register가 없으면 UNPREDICTABLE | 100 |
| `CLO` | CPU 필수 | 구현 | 회귀 통과 | `0x1C` | `0x21` | `0xFC0007FF` | `0x70000021` | `rd, rs` | 상위부터 연속된 1의 개수 | rt와 rd 불일치 시 UNPREDICTABLE | 101 |
| `CLZ` | CPU 필수 | 구현 | 회귀 통과 | `0x1C` | `0x20` | `0xFC0007FF` | `0x70000020` | `rd, rs` | 상위부터 연속된 0의 개수 | rt와 rd 불일치 시 UNPREDICTABLE | 102 |
| `COP2` | CP2 | 대기 | 대기 | `0x12` | — | `0xFE000000` | `0x4A000000` | `func` | 구현 정의 cofun을 CP2에 전달 | Coprocessor Unusable, Reserved Instruction | 103 |
| `CTC1` | FPU 기본 | 대기 | 대기 | `0x11` | — | `0xFFE007FF` | `0x44C00000` | `rt, fs` | CP1 rt → control register | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation, Division-by-zero, Inexact, Overflow, Underflow | 104 |
| `CTC2` | CP2 | 대기 | 대기 | `0x12` | — | `0xFFE00000` | `0x48C00000` | `rt, rd` (예시) | CP2 rt → control register | Coprocessor Unusable, Reserved Instruction. Impl의 register가 없으면 UNPREDICTABLE | 106 |
| `CVT.D.L` | FPU 64 | 대기 | 대기 | `0x11` | `0x21` | `0xFFFF003F` | `0x46A00021` | `fd, fs` | FCSR rounding mode로 D 형식 변환 | Coprocessor Unusable, Reserved Instruction / FPU: Invalid Operation, Unimplemented Operation, Inexact | 107 |
| `CVT.D.S` | FPU 기본 | 대기 | 대기 | `0x11` | `0x21` | `0xFFFF003F` | `0x46000021` | `fd, fs` | FCSR rounding mode로 D 형식 변환 | Coprocessor Unusable, Reserved Instruction / FPU: Invalid Operation, Unimplemented Operation, Inexact | 107 |
| `CVT.D.W` | FPU 기본 | 대기 | 대기 | `0x11` | `0x21` | `0xFFFF003F` | `0x46800021` | `fd, fs` | FCSR rounding mode로 D 형식 변환 | Coprocessor Unusable, Reserved Instruction / FPU: Invalid Operation, Unimplemented Operation, Inexact | 107 |
| `CVT.L.D` | FPU 64 | 대기 | 대기 | `0x11` | `0x25` | `0xFFFF003F` | `0x46200025` | `fd, fs` | FCSR rounding mode로 L 형식 변환 | Coprocessor Unusable, Reserved Instruction / FPU: Invalid Operation, Unimplemented Operation, Inexact, Overflow | 108 |
| `CVT.L.S` | FPU 64 | 대기 | 대기 | `0x11` | `0x25` | `0xFFFF003F` | `0x46000025` | `fd, fs` | FCSR rounding mode로 L 형식 변환 | Coprocessor Unusable, Reserved Instruction / FPU: Invalid Operation, Unimplemented Operation, Inexact, Overflow | 108 |
| `CVT.PS.S` | FPU 64 | 대기 | 대기 | `0x11` | `0x26` | `0xFFE0003F` | `0x46000026` | `fd, fs, ft` | fs와 ft의 S 값을 PS의 상·하위 lane에 결합 | Coprocessor Unusable, Reserved Instruction / FPU: Invalid Operation, Unimplemented Operation | 109 |
| `CVT.S.D` | FPU 기본 | 대기 | 대기 | `0x11` | `0x20` | `0xFFFF003F` | `0x46200020` | `fd, fs` | FCSR rounding mode로 S 형식 변환 | Coprocessor Unusable, Reserved Instruction / FPU: Invalid Operation, Unimplemented Operation, Inexact, Overflow, Underflow | 110 |
| `CVT.S.L` | FPU 64 | 대기 | 대기 | `0x11` | `0x20` | `0xFFFF003F` | `0x46A00020` | `fd, fs` | FCSR rounding mode로 S 형식 변환 | Coprocessor Unusable, Reserved Instruction / FPU: Invalid Operation, Unimplemented Operation, Inexact, Overflow, Underflow | 110 |
| `CVT.S.PL` | FPU 64 | 대기 | 대기 | `0x11` | `0x28` | `0xFFFF003F` | `0x46C00028` | `fd, fs` | PS 하위 lane을 S로 추출 | Coprocessor Unusable, Reserved Instruction | 111 |
| `CVT.S.PU` | FPU 64 | 대기 | 대기 | `0x11` | `0x20` | `0xFFFF003F` | `0x46C00020` | `fd, fs` | PS 상위 lane을 S로 추출 | Coprocessor Unusable, Reserved Instruction / FPU: 112 | 112 |
| `CVT.S.W` | FPU 기본 | 대기 | 대기 | `0x11` | `0x20` | `0xFFFF003F` | `0x46800020` | `fd, fs` | FCSR rounding mode로 S 형식 변환 | Coprocessor Unusable, Reserved Instruction / FPU: Invalid Operation, Unimplemented Operation, Inexact, Overflow, Underflow | 110 |
| `CVT.W.D` | FPU 기본 | 대기 | 대기 | `0x11` | `0x24` | `0xFFFF003F` | `0x46200024` | `fd, fs` | FCSR rounding mode로 W 형식 변환 | Coprocessor Unusable, Reserved Instruction / FPU: Invalid Operation, Unimplemented Operation, Inexact, Overflow | 113 |
| `CVT.W.S` | FPU 기본 | 대기 | 대기 | `0x11` | `0x24` | `0xFFFF003F` | `0x46000024` | `fd, fs` | FCSR rounding mode로 W 형식 변환 | Coprocessor Unusable, Reserved Instruction / FPU: Invalid Operation, Unimplemented Operation, Inexact, Overflow | 113 |
| `DERET` | EJTAG | 대기 | 대기 | `0x10` | `0x1F` | `0xFFFFFFFF` | `0x4200001F` | — | DEPC에서 실행을 재개하고 Debug Mode 종료 | Coprocessor Unusable Exception Reserved Instruction Exception. Debug Mode 외 실행 시 UNDEFINED | 114 |
| `DI` | CP0 | 부분 | 회귀 통과 (제한 범위) | `0x10` | — | `0xFFE0FFFF` | `0x41606000` | `[rt]` | 이전 Status를 rt에 쓰고 Status.IE를 해제 | Coprocessor Unusable, Reserved Instruction (Release 1 implementations) | 115 |
| `DIV` | CPU 필수 | 구현 | 회귀 통과 | `0x00` | `0x1A` | `0xFC00FFFF` | `0x0000001A` | `rs, rt` | signed 몫을 LO, 나머지를 HI에 기록 | 0으로 나누면 HI/LO 결과 UNPREDICTABLE | 116 |
| `DIV.D` | FPU 기본 | 대기 | 대기 | `0x11` | `0x03` | `0xFFE0003F` | `0x46200003` | `fd, fs, ft` | fs ÷ ft | Coprocessor Unusable, Reserved Instruction / FPU: Inexact, Invalid Operation, Unimplemented Operation, Division-by-zero, Overflow, Underflow | 118 |
| `DIV.S` | FPU 기본 | 대기 | 대기 | `0x11` | `0x03` | `0xFFE0003F` | `0x46000003` | `fd, fs, ft` | fs ÷ ft | Coprocessor Unusable, Reserved Instruction / FPU: Inexact, Invalid Operation, Unimplemented Operation, Division-by-zero, Overflow, Underflow | 118 |
| `DIVU` | CPU 필수 | 구현 | 회귀 통과 | `0x00` | `0x1B` | `0xFC00FFFF` | `0x0000001B` | `rs, rt` | unsigned 몫을 LO, 나머지를 HI에 기록 | 0으로 나누면 HI/LO 결과 UNPREDICTABLE | 119 |
| `EHB` | CPU 필수 | 구현 | 회귀 통과 | `0x00` | `0x00` | `0xFFFFFFFF` | `0x000000C0` | — | 앞선 명령어의 execution hazard 해소 | — | 120 |
| `EI` | CP0 | 부분 | 회귀 통과 (제한 범위) | `0x10` | — | `0xFFE0FFFF` | `0x41606020` | `[rt]` | 이전 Status를 rt에 쓰고 Status.IE를 설정 | Coprocessor Unusable, Reserved Instruction (Release 1 implementations) | 121 |
| `ERET` | CP0 | 부분 | 회귀 통과 (제한 범위) | `0x10` | `0x18` | `0xFFFFFFFF` | `0x42000018` | — | ERL이면 ErrorEPC와 ERL, 그 외는 EPC와 EXL을 사용하여 exception return | Coprocessor Unusable Exception | 122 |
| `EXT` | CPU 필수 | 구현 | 회귀 통과 | `0x1F` | `0x00` | `0xFC00003F` | `0x7C000000` | `rt, rs, pos, size` | rs의 pos부터 size비트를 추출하고 zero-extend | Reserved Instruction. pos + size > 32이면 UNPREDICTABLE | 123 |
| `FLOOR.L.D` | FPU 64 | 대기 | 대기 | `0x11` | `0x0B` | `0xFFFF003F` | `0x4620000B` | `fd, fs` | −∞ 방향 반올림으로 L signed 정수 변환 | Coprocessor Unusable, Reserved Instruction / FPU: Invalid Operation, Unimplemented Operation, Inexact, Overflow | 125 |
| `FLOOR.L.S` | FPU 64 | 대기 | 대기 | `0x11` | `0x0B` | `0xFFFF003F` | `0x4600000B` | `fd, fs` | −∞ 방향 반올림으로 L signed 정수 변환 | Coprocessor Unusable, Reserved Instruction / FPU: Invalid Operation, Unimplemented Operation, Inexact, Overflow | 125 |
| `FLOOR.W.D` | FPU 기본 | 대기 | 대기 | `0x11` | `0x0F` | `0xFFFF003F` | `0x4620000F` | `fd, fs` | −∞ 방향 반올림으로 W signed 정수 변환 | Coprocessor Unusable, Reserved Instruction / FPU: Invalid Operation, Unimplemented Operation, Inexact, Overflow | 126 |
| `FLOOR.W.S` | FPU 기본 | 대기 | 대기 | `0x11` | `0x0F` | `0xFFFF003F` | `0x4600000F` | `fd, fs` | −∞ 방향 반올림으로 W signed 정수 변환 | Coprocessor Unusable, Reserved Instruction / FPU: Invalid Operation, Unimplemented Operation, Inexact, Overflow | 126 |
| `INS` | CPU 필수 | 구현 | 회귀 통과 | `0x1F` | `0x04` | `0xFC00003F` | `0x7C000004` | `rt, rs, pos, size` | rs의 하위 size비트로 rt의 pos부터 size비트를 갱신 | Reserved Instruction. pos + size > 32 또는 잘못된 msb이면 UNPREDICTABLE | 127 |
| `J` | CPU 필수 | 구현 | 회귀 통과 | `0x02` | — | `0xFC000000` | `0x08000000` | `target` | 같은 256 MiB 영역 안에서 jump | — | 129 |
| `JAL` | CPU 필수 | 구현 | 회귀 통과 | `0x03` | — | `0xFC000000` | `0x0C000000` | `target` | J와 동일한 jump 및 $31에 PC+8 기록 | — | 130 |
| `JALR` | CPU 필수 | 구현 | 회귀 통과 | `0x00` | `0x09` | `0xFC1F043F` | `0x00000009` | `[rd,] rs` | rs 주소로 jump 및 rd에 PC+8 기록 | target fetch 주소 정렬 위반 시 Address Error. rs == rd이면 UNPREDICTABLE | 131 |
| `JALR.HB` | CPU 필수 | 부분 | 회귀 통과 (제한 범위) | `0x00` | `0x09` | `0xFC1F043F` | `0x00000409` | `[rd,] rs` | JALR 및 execution/instruction hazard 해소 | target fetch 주소 정렬 위반 시 Address Error. rs == rd이면 UNPREDICTABLE | 133 |
| `JALX` | MIPS16e | 대기 | 대기 | `0x1D` | — | `0xFC000000` | `0x74000000` | `target` | JAL 방식으로 jump 및 link 후 MIPS16e 모드로 전환 | MIPS16e 미지원 시 Reserved Instruction | IV-a 76 |
| `JR` | CPU 필수 | 구현 | 회귀 통과 | `0x00` | `0x08` | `0xFC1FFC3F` | `0x00000008` | `rs` | rs 주소로 jump | target fetch 주소 정렬 위반 시 Address Error | 135 |
| `JR.HB` | CPU 필수 | 부분 | 회귀 통과 (제한 범위) | `0x00` | `0x08` | `0xFC1FFC3F` | `0x00000408` | `rs` | JR 및 execution/instruction hazard 해소 | target fetch 주소 정렬 위반 시 Address Error | 136 |
| `LB` | CPU 필수 | 부분 | 회귀 통과 (제한 범위) | `0x20` | — | `0xFC000000` | `0x80000000` | `rt, offset(base)` | byte sign-extend load | TLB Refill, TLB Invalid, Address Error, Watch | 138 |
| `LBU` | CPU 필수 | 부분 | 회귀 통과 (제한 범위) | `0x24` | — | `0xFC000000` | `0x90000000` | `rt, offset(base)` | byte zero-extend load | TLB Refill, TLB Invalid, Address Error, Watch | 139 |
| `LDC1` | FPU 기본 | 대기 | 대기 | `0x35` | — | `0xFC000000` | `0xD4000000` | `ft, offset(base)` | doubleword를 FPR에 기록 load | Coprocessor Unusable, Reserved Instruction, TLB Refill, TLB Invalid, Address Error, Watch | 140 |
| `LDC2` | CP2 | 대기 | 대기 | `0x36` | — | `0xFC000000` | `0xD8000000` | `rt, offset(base)` | doubleword를 CP2에 기록 load | Coprocessor Unusable, Reserved Instruction, TLB Refill, TLB Invalid, Address Error, Watch | 141 |
| `LDXC1` | FPU 64 | 대기 | 대기 | `0x13` | `0x01` | `0xFC00F83F` | `0x4C000001` | `fd, index(base)` | base + index의 doubleword를 FPR에 기록 load | TLB Refill, TLB Invalid, Address Error, Reserved Instruction, Coprocessor Unusable, Watch | 142 |
| `LH` | CPU 필수 | 부분 | 회귀 통과 (제한 범위) | `0x21` | — | `0xFC000000` | `0x84000000` | `rt, offset(base)` | halfword sign-extend load | TLB Refill, TLB Invalid, Bus Error, Address Error, Watch | 143 |
| `LHU` | CPU 필수 | 부분 | 회귀 통과 (제한 범위) | `0x25` | — | `0xFC000000` | `0x94000000` | `rt, offset(base)` | halfword zero-extend load | TLB Refill, TLB Invalid, Address Error, Watch | 144 |
| `LL` | CPU 필수 | 부분 | 회귀 통과 (제한 범위) | `0x30` | — | `0xFC000000` | `0xC0000000` | `rt, offset(base)` | word load 및 LL/SC reservation 설정 | TLB Refill, TLB Invalid, Address Error, Reserved Instruction, Watch. 동기화 불가 메모리 접근은 UNPREDICTABLE | 145 |
| `LUI` | CPU 필수 | 구현 | 회귀 통과 | `0x0F` | — | `0xFFE00000` | `0x3C000000` | `rt, immediate` | immediate를 상위 16비트에 기록하고 하위 16비트를 0으로 설정 | — | 146 |
| `LUXC1` | FPU 64 | 대기 | 대기 | `0x13` | `0x05` | `0xFC00F83F` | `0x4C000005` | `fd, index(base)` | base + index를 8바이트 경계로 내림한 주소의 doubleword를 FPR에 기록 load | Coprocessor Unusable, Reserved Instruction, TLB Refill, TLB Invalid, Watch | 147 |
| `LW` | CPU 필수 | 부분 | 회귀 통과 (제한 범위) | `0x23` | — | `0xFC000000` | `0x8C000000` | `rt, offset(base)` | word load | TLB Refill, TLB Invalid, Bus Error, Address Error, Watch | 148 |
| `LWC1` | FPU 기본 | 대기 | 대기 | `0x31` | — | `0xFC000000` | `0xC4000000` | `ft, offset(base)` | word를 FPR에 기록 load | TLB Refill, TLB Invalid, Address Error, Reserved Instruction, Coprocessor Unusable, Watch | 149 |
| `LWC2` | CP2 | 대기 | 대기 | `0x32` | — | `0xFC000000` | `0xC8000000` | `rt, offset(base)` | word를 CP2에 기록 load | TLB Refill, TLB Invalid, Address Error, Reserved Instruction, Coprocessor Unusable, Watch | 150 |
| `LWL` | CPU 필수 | 부분 | 회귀 통과 (제한 범위) | `0x22` | — | `0xFC000000` | `0x88000000` | `rt, offset(base)` | word의 상위 쪽 부분을 rt에 병합 load | TLB Refill, TLB Invalid, Bus Error, Address Error, Watch | 151 |
| `LWR` | CPU 필수 | 부분 | 회귀 통과 (제한 범위) | `0x26` | — | `0xFC000000` | `0x98000000` | `rt, offset(base)` | word의 하위 쪽 부분을 rt에 병합 load | TLB Refill, TLB Invalid, Bus Error, Address Error, Watch | 153 |
| `LWXC1` | FPU 64 | 대기 | 대기 | `0x13` | `0x00` | `0xFC00F83F` | `0x4C000000` | `fd, index(base)` | base + index의 word를 FPR에 기록 load | TLB Refill, TLB Invalid, Address Error, Reserved Instruction, Coprocessor Unusable, Watch | 155 |
| `MADD` | CPU 필수 | 구현 | 회귀 통과 | `0x1C` | `0x00` | `0xFC00FFFF` | `0x70000000` | `rs, rt` | signed 64-bit 곱을 HI:LO에 더함 | — | 156 |
| `MADD.D` | FPU 64 | 대기 | 대기 | `0x13` | `0x21` | `0xFC00003F` | `0x4C000021` | `fd, fr, fs, ft` | fs × ft + fr. 곱셈과 덧셈 또는 뺄셈은 각각 반올림 | Coprocessor Unusable, Reserved Instruction / FPU: Inexact, Unimplemented Operation, Invalid Operation, Overflow, Underflow | 157 |
| `MADD.PS` | FPU 64 | 대기 | 대기 | `0x13` | `0x26` | `0xFC00003F` | `0x4C000026` | `fd, fr, fs, ft` | fs × ft + fr. 곱셈과 덧셈 또는 뺄셈은 각각 반올림 | Coprocessor Unusable, Reserved Instruction / FPU: Inexact, Unimplemented Operation, Invalid Operation, Overflow, Underflow | 157 |
| `MADD.S` | FPU 64 | 대기 | 대기 | `0x13` | `0x20` | `0xFC00003F` | `0x4C000020` | `fd, fr, fs, ft` | fs × ft + fr. 곱셈과 덧셈 또는 뺄셈은 각각 반올림 | Coprocessor Unusable, Reserved Instruction / FPU: Inexact, Unimplemented Operation, Invalid Operation, Overflow, Underflow | 157 |
| `MADDU` | CPU 필수 | 구현 | 회귀 통과 | `0x1C` | `0x01` | `0xFC00FFFF` | `0x70000001` | `rs, rt` | unsigned 64-bit 곱을 HI:LO에 더함 | — | 158 |
| `MFC0` | CP0 | 부분 | 회귀 통과 (제한 범위) | `0x10` | — | `0xFFE007F8` | `0x40000000` | `rt, rd[, sel]` | CP0[rd, sel]을 rt로 이동 | Coprocessor Unusable, Reserved Instruction | 159 |
| `MFC1` | FPU 기본 | 대기 | 대기 | `0x11` | — | `0xFFE007FF` | `0x44000000` | `rt, fs` | FPR 하위 word → rt | Coprocessor Unusable, Reserved Instruction | 160 |
| `MFC2` | CP2 | 대기 | 대기 | `0x12` | — | `0xFFE00000` | `0x48000000` | `rt, rd[, sel]` (예시) | CP2 register 하위 word → rt | Coprocessor Unusable. Impl의 register가 없으면 UNPREDICTABLE | 161 |
| `MFHC1` | FPU 기본 | 대기 | 대기 | `0x11` | — | `0xFFE007FF` | `0x44600000` | `rt, fs` | FPR 상위 word → rt | Coprocessor Unusable, Reserved Instruction | 162 |
| `MFHC2` | CP2 | 대기 | 대기 | `0x12` | — | `0xFFE00000` | `0x48600000` | `rt, rd[, sel]` (예시) | CP2 register 상위 word → rt | Coprocessor Unusable, Reserved Instruction. Impl의 register가 없거나 64-bit가 아니면 UNPREDICTABLE | 163 |
| `MFHI` | CPU 필수 | 구현 | 회귀 통과 | `0x00` | `0x10` | `0xFFFF07FF` | `0x00000010` | `rd` | HI를 rd로 이동 | — | 164 |
| `MFLO` | CPU 필수 | 구현 | 회귀 통과 | `0x00` | `0x12` | `0xFFFF07FF` | `0x00000012` | `rd` | LO를 rd로 이동 | — | 165 |
| `MOV.D` | FPU 기본 | 대기 | 대기 | `0x11` | `0x06` | `0xFFFF003F` | `0x46200006` | `fd, fs` | fs를 fd로 복사 | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation | 166 |
| `MOV.PS` | FPU 64 | 대기 | 대기 | `0x11` | `0x06` | `0xFFFF003F` | `0x46C00006` | `fd, fs` | fs를 fd로 복사 | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation | 166 |
| `MOV.S` | FPU 기본 | 대기 | 대기 | `0x11` | `0x06` | `0xFFFF003F` | `0x46000006` | `fd, fs` | fs를 fd로 복사 | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation | 166 |
| `MOVF` | FPU 기본 | 대기 | 대기 | `0x00` | `0x01` | `0xFC0307FF` | `0x00000001` | `rd, rs, cc` | FCC[cc]가 0이면 rs를 rd로 이동 | Reserved Instruction, Coprocessor Unusable | 167 |
| `MOVF.D` | FPU 기본 | 대기 | 대기 | `0x11` | `0x11` | `0xFFE3003F` | `0x46200011` | `fd, fs, cc` | FCC[cc]가 0이면 fs를 fd로 복사 | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation | 168 |
| `MOVF.PS` | FPU 64 | 대기 | 대기 | `0x11` | `0x11` | `0xFFE3003F` | `0x46C00011` | `fd, fs, cc` | FCC[cc]가 0이면 fs를 fd로 복사 | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation | 168 |
| `MOVF.S` | FPU 기본 | 대기 | 대기 | `0x11` | `0x11` | `0xFFE3003F` | `0x46000011` | `fd, fs, cc` | FCC[cc]가 0이면 fs를 fd로 복사 | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation | 168 |
| `MOVN` | CPU 필수 | 구현 | 회귀 통과 | `0x00` | `0x0B` | `0xFC0007FF` | `0x0000000B` | `rd, rs, rt` | rt가 0이 아니면 rs를 rd로 이동 | — | 169 |
| `MOVN.D` | FPU 기본 | 대기 | 대기 | `0x11` | `0x13` | `0xFFE0003F` | `0x46200013` | `fd, fs, rt` | rt가 0이 아니면 fs를 fd로 복사 | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation | 170 |
| `MOVN.PS` | FPU 64 | 대기 | 대기 | `0x11` | `0x13` | `0xFFE0003F` | `0x46C00013` | `fd, fs, rt` | rt가 0이 아니면 fs를 fd로 복사 | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation | 170 |
| `MOVN.S` | FPU 기본 | 대기 | 대기 | `0x11` | `0x13` | `0xFFE0003F` | `0x46000013` | `fd, fs, rt` | rt가 0이 아니면 fs를 fd로 복사 | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation | 170 |
| `MOVT` | FPU 기본 | 대기 | 대기 | `0x00` | `0x01` | `0xFC0307FF` | `0x00010001` | `rd, rs, cc` | FCC[cc]가 1이면 rs를 rd로 이동 | Reserved Instruction, Coprocessor Unusable | 171 |
| `MOVT.D` | FPU 기본 | 대기 | 대기 | `0x11` | `0x11` | `0xFFE3003F` | `0x46210011` | `fd, fs, cc` | FCC[cc]가 1이면 fs를 fd로 복사 | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation | 172 |
| `MOVT.PS` | FPU 64 | 대기 | 대기 | `0x11` | `0x11` | `0xFFE3003F` | `0x46C10011` | `fd, fs, cc` | FCC[cc]가 1이면 fs를 fd로 복사 | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation | 172 |
| `MOVT.S` | FPU 기본 | 대기 | 대기 | `0x11` | `0x11` | `0xFFE3003F` | `0x46010011` | `fd, fs, cc` | FCC[cc]가 1이면 fs를 fd로 복사 | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation | 172 |
| `MOVZ` | CPU 필수 | 구현 | 회귀 통과 | `0x00` | `0x0A` | `0xFC0007FF` | `0x0000000A` | `rd, rs, rt` | rt가 0이면 rs를 rd로 이동 | — | 173 |
| `MOVZ.D` | FPU 기본 | 대기 | 대기 | `0x11` | `0x12` | `0xFFE0003F` | `0x46200012` | `fd, fs, rt` | rt가 0이면 fs를 fd로 복사 | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation | 174 |
| `MOVZ.PS` | FPU 64 | 대기 | 대기 | `0x11` | `0x12` | `0xFFE0003F` | `0x46C00012` | `fd, fs, rt` | rt가 0이면 fs를 fd로 복사 | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation | 174 |
| `MOVZ.S` | FPU 기본 | 대기 | 대기 | `0x11` | `0x12` | `0xFFE0003F` | `0x46000012` | `fd, fs, rt` | rt가 0이면 fs를 fd로 복사 | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation | 174 |
| `MSUB` | CPU 필수 | 구현 | 회귀 통과 | `0x1C` | `0x04` | `0xFC00FFFF` | `0x70000004` | `rs, rt` | signed 64-bit 곱을 HI:LO에서 뺌 | — | 175 |
| `MSUB.D` | FPU 64 | 대기 | 대기 | `0x13` | `0x29` | `0xFC00003F` | `0x4C000029` | `fd, fr, fs, ft` | fs × ft − fr. 곱셈과 덧셈 또는 뺄셈은 각각 반올림 | Coprocessor Unusable, Reserved Instruction / FPU: Inexact, Unimplemented Operation, Invalid Operation, Overflow, Underflow | 176 |
| `MSUB.PS` | FPU 64 | 대기 | 대기 | `0x13` | `0x2E` | `0xFC00003F` | `0x4C00002E` | `fd, fr, fs, ft` | fs × ft − fr. 곱셈과 덧셈 또는 뺄셈은 각각 반올림 | Coprocessor Unusable, Reserved Instruction / FPU: Inexact, Unimplemented Operation, Invalid Operation, Overflow, Underflow | 176 |
| `MSUB.S` | FPU 64 | 대기 | 대기 | `0x13` | `0x28` | `0xFC00003F` | `0x4C000028` | `fd, fr, fs, ft` | fs × ft − fr. 곱셈과 덧셈 또는 뺄셈은 각각 반올림 | Coprocessor Unusable, Reserved Instruction / FPU: Inexact, Unimplemented Operation, Invalid Operation, Overflow, Underflow | 176 |
| `MSUBU` | CPU 필수 | 구현 | 회귀 통과 | `0x1C` | `0x05` | `0xFC00FFFF` | `0x70000005` | `rs, rt` | unsigned 64-bit 곱을 HI:LO에서 뺌 | — | 177 |
| `MTC0` | CP0 | 부분 | 회귀 통과 (제한 범위) | `0x10` | — | `0xFFE007F8` | `0x40800000` | `rt, rd[, sel]` | rt를 CP0[rd, sel]로 이동 | Coprocessor Unusable, Reserved Instruction | 178 |
| `MTC1` | FPU 기본 | 대기 | 대기 | `0x11` | — | `0xFFE007FF` | `0x44800000` | `rt, fs` | FPR rt → 하위 word | Coprocessor Unusable | 179 |
| `MTC2` | CP2 | 대기 | 대기 | `0x12` | — | `0xFFE00000` | `0x48800000` | `rt, rd[, sel]` (예시) | CP2 register rt → 하위 word | Coprocessor Unusable, Reserved Instruction. Impl의 register가 없으면 UNPREDICTABLE | 180 |
| `MTHC1` | FPU 기본 | 대기 | 대기 | `0x11` | — | `0xFFE007FF` | `0x44E00000` | `rt, fs` | FPR rt → 상위 word | Coprocessor Unusable, Reserved Instruction | 181 |
| `MTHC2` | CP2 | 대기 | 대기 | `0x12` | — | `0xFFE00000` | `0x48E00000` | `rt, rd[, sel]` (예시) | CP2 register rt → 상위 word | Coprocessor Unusable, Reserved Instruction. Impl의 register가 없거나 64-bit가 아니면 UNPREDICTABLE | 182 |
| `MTHI` | CPU 필수 | 구현 | 회귀 통과 | `0x00` | `0x11` | `0xFC1FFFFF` | `0x00000011` | `rs` | rs를 HI로 이동 | — | 183 |
| `MTLO` | CPU 필수 | 구현 | 회귀 통과 | `0x00` | `0x13` | `0xFC1FFFFF` | `0x00000013` | `rs` | rs를 LO로 이동 | — | 184 |
| `MUL` | CPU 필수 | 구현 | 회귀 통과 | `0x1C` | `0x02` | `0xFC0007FF` | `0x70000002` | `rd, rs, rt` | signed 곱의 하위 32비트를 rd에 기록 | HI/LO 결과 UNPREDICTABLE | 185 |
| `MUL.D` | FPU 기본 | 대기 | 대기 | `0x11` | `0x02` | `0xFFE0003F` | `0x46200002` | `fd, fs, ft` | fs × ft | Coprocessor Unusable, Reserved Instruction / FPU: Inexact, Unimplemented Operation, Invalid Operation, Overflow, Underflow | 186 |
| `MUL.PS` | FPU 64 | 대기 | 대기 | `0x11` | `0x02` | `0xFFE0003F` | `0x46C00002` | `fd, fs, ft` | fs × ft | Coprocessor Unusable, Reserved Instruction / FPU: Inexact, Unimplemented Operation, Invalid Operation, Overflow, Underflow | 186 |
| `MUL.S` | FPU 기본 | 대기 | 대기 | `0x11` | `0x02` | `0xFFE0003F` | `0x46000002` | `fd, fs, ft` | fs × ft | Coprocessor Unusable, Reserved Instruction / FPU: Inexact, Unimplemented Operation, Invalid Operation, Overflow, Underflow | 186 |
| `MULT` | CPU 필수 | 구현 | 회귀 통과 | `0x00` | `0x18` | `0xFC00FFFF` | `0x00000018` | `rs, rt` | signed 64-bit 곱을 HI:LO에 기록 | — | 187 |
| `MULTU` | CPU 필수 | 구현 | 회귀 통과 | `0x00` | `0x19` | `0xFC00FFFF` | `0x00000019` | `rs, rt` | unsigned 64-bit 곱을 HI:LO에 기록 | — | 188 |
| `NEG.D` | FPU 기본 | 대기 | 대기 | `0x11` | `0x07` | `0xFFFF003F` | `0x46200007` | `fd, fs` | 부호 반전 | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation | 189 |
| `NEG.PS` | FPU 64 | 대기 | 대기 | `0x11` | `0x07` | `0xFFFF003F` | `0x46C00007` | `fd, fs` | 부호 반전 | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation | 189 |
| `NEG.S` | FPU 기본 | 대기 | 대기 | `0x11` | `0x07` | `0xFFFF003F` | `0x46000007` | `fd, fs` | 부호 반전 | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation | 189 |
| `NMADD.D` | FPU 64 | 대기 | 대기 | `0x13` | `0x31` | `0xFC00003F` | `0x4C000031` | `fd, fr, fs, ft` | −(fs × ft + fr). 곱셈과 덧셈 또는 뺄셈은 각각 반올림 | Coprocessor Unusable, Reserved Instruction / FPU: Inexact, Unimplemented Operation, Invalid Operation, Overflow, Underflow | 190 |
| `NMADD.PS` | FPU 64 | 대기 | 대기 | `0x13` | `0x36` | `0xFC00003F` | `0x4C000036` | `fd, fr, fs, ft` | −(fs × ft + fr). 곱셈과 덧셈 또는 뺄셈은 각각 반올림 | Coprocessor Unusable, Reserved Instruction / FPU: Inexact, Unimplemented Operation, Invalid Operation, Overflow, Underflow | 190 |
| `NMADD.S` | FPU 64 | 대기 | 대기 | `0x13` | `0x30` | `0xFC00003F` | `0x4C000030` | `fd, fr, fs, ft` | −(fs × ft + fr). 곱셈과 덧셈 또는 뺄셈은 각각 반올림 | Coprocessor Unusable, Reserved Instruction / FPU: Inexact, Unimplemented Operation, Invalid Operation, Overflow, Underflow | 190 |
| `NMSUB.D` | FPU 64 | 대기 | 대기 | `0x13` | `0x39` | `0xFC00003F` | `0x4C000039` | `fd, fr, fs, ft` | −(fs × ft − fr). 곱셈과 덧셈 또는 뺄셈은 각각 반올림 | Coprocessor Unusable, Reserved Instruction / FPU: Inexact, Unimplemented Operation, Invalid Operation, Overflow, Underflow | 191 |
| `NMSUB.PS` | FPU 64 | 대기 | 대기 | `0x13` | `0x3E` | `0xFC00003F` | `0x4C00003E` | `fd, fr, fs, ft` | −(fs × ft − fr). 곱셈과 덧셈 또는 뺄셈은 각각 반올림 | Coprocessor Unusable, Reserved Instruction / FPU: Inexact, Unimplemented Operation, Invalid Operation, Overflow, Underflow | 191 |
| `NMSUB.S` | FPU 64 | 대기 | 대기 | `0x13` | `0x38` | `0xFC00003F` | `0x4C000038` | `fd, fr, fs, ft` | −(fs × ft − fr). 곱셈과 덧셈 또는 뺄셈은 각각 반올림 | Coprocessor Unusable, Reserved Instruction / FPU: Inexact, Unimplemented Operation, Invalid Operation, Overflow, Underflow | 191 |
| `NOP` | Idiom | 별칭 | — | `0x00` | `0x00` | `0xFFFFFFFF` | `0x00000000` | — | SLL $zero, $zero, 0과 동일한 no-op | — | 192 |
| `NOR` | CPU 필수 | 구현 | 회귀 통과 | `0x00` | `0x27` | `0xFC0007FF` | `0x00000027` | `rd, rs, rt` | bitwise OR 결과의 bitwise NOT | — | 193 |
| `OR` | CPU 필수 | 구현 | 회귀 통과 | `0x00` | `0x25` | `0xFC0007FF` | `0x00000025` | `rd, rs, rt` | bitwise OR | — | 194 |
| `ORI` | CPU 필수 | 구현 | 회귀 통과 | `0x0D` | — | `0xFC000000` | `0x34000000` | `rt, rs, immediate` | zero-extended immediate와 bitwise OR | — | 195 |
| `PAUSE` | CPU 필수 | 부분 | 회귀 통과 (제한 범위) | `0x00` | `0x00` | `0xFFFFFFFF` | `0x00000140` | — | LLBit를 기준으로 대기. 구체적인 대기 동작은 구현 정의 | delay slot에 오면 UNPREDICTABLE | 197 |
| `PLL.PS` | FPU 64 | 대기 | 대기 | `0x11` | `0x2C` | `0xFFE0003F` | `0x46C0002C` | `fd, fs, ft` | fs 하위와 ft 하위를 PS 상·하위에 결합 | Coprocessor Unusable, Reserved Instruction | 199 |
| `PLU.PS` | FPU 64 | 대기 | 대기 | `0x11` | `0x2D` | `0xFFE0003F` | `0x46C0002D` | `fd, fs, ft` | fs 하위와 ft 상위를 PS 상·하위에 결합 | Coprocessor Unusable, Reserved Instruction | 200 |
| `PREF` | CPU 필수 | 구현 | 회귀 통과 | `0x33` | — | `0xFC000000` | `0xCC000000` | `hint,offset(base)` | base + sign-extended offset 주소에 대한 cache prefetch hint | Bus Error, Cache Error. TLB 및 주소 관련 예외 억제 | 201 |
| `PREFX` | FPU 64 | 대기 | 대기 | `0x13` | `0x0F` | `0xFC0007FF` | `0x4C00000F` | `hint, index(base)` | base + index 주소에 대한 cache prefetch hint | Coprocessor Unusable, Reserved Instruction, Bus Error, Cache Error | 204 |
| `PUL.PS` | FPU 64 | 대기 | 대기 | `0x11` | `0x2E` | `0xFFE0003F` | `0x46C0002E` | `fd, fs, ft` | fs 상위와 ft 하위를 PS 상·하위에 결합 | Coprocessor Unusable, Reserved Instruction | 205 |
| `PUU.PS` | FPU 64 | 대기 | 대기 | `0x11` | `0x2F` | `0xFFE0003F` | `0x46C0002F` | `fd, fs, ft` | fs 상위와 ft 상위를 PS 상·하위에 결합 | Coprocessor Unusable, Reserved Instruction | 206 |
| `RDHWR` | CPU 필수 | 부분 | 회귀 통과 (제한 범위) | `0x1F` | `0x3B` | `0xFFE007FF` | `0x7C00003B` | `rt, rd` | hardware register rd를 rt로 이동 | Reserved Instruction. HWREna 권한 부족 또는 지원하지 않는 register 번호 | 207 |
| `RDPGPR` | CP0 | 대기 | 대기 | `0x10` | — | `0xFFE007FF` | `0x41400000` | `rd, rt` | 이전 shadow GPR set의 rt를 현재 rd로 이동 | Coprocessor Unusable, Reserved Instruction | 209 |
| `RECIP.D` | FPU 64 | 대기 | 대기 | `0x11` | `0x15` | `0xFFFF003F` | `0x46200015` | `fd, fs` | 1/fs 근사 | Coprocessor Unusable, Reserved Instruction / FPU: Inexact, Division-by-zero, Unimplemented Op, Invalid Op, Overflow, Underflow | 210 |
| `RECIP.S` | FPU 64 | 대기 | 대기 | `0x11` | `0x15` | `0xFFFF003F` | `0x46000015` | `fd, fs` | 1/fs 근사 | Coprocessor Unusable, Reserved Instruction / FPU: Inexact, Division-by-zero, Unimplemented Op, Invalid Op, Overflow, Underflow | 210 |
| `ROTR` | CPU 필수 | 구현 | 회귀 통과 | `0x00` | `0x02` | `0xFFE0003F` | `0x00200002` | `rd, rt, sa` | right rotate by sa | Reserved Instruction | 211 |
| `ROTRV` | CPU 필수 | 구현 | 회귀 통과 | `0x00` | `0x06` | `0xFC0007FF` | `0x00000046` | `rd, rt, rs` | right rotate by rs[4:0] | Reserved Instruction | 212 |
| `ROUND.L.D` | FPU 64 | 대기 | 대기 | `0x11` | `0x08` | `0xFFFF003F` | `0x46200008` | `fd, fs` | nearest ties-to-even 반올림으로 L signed 정수 변환 | Coprocessor Unusable, Reserved Instruction / FPU: Inexact, Unimplemented Operation, Invalid Operation, Overflow | 213 |
| `ROUND.L.S` | FPU 64 | 대기 | 대기 | `0x11` | `0x08` | `0xFFFF003F` | `0x46000008` | `fd, fs` | nearest ties-to-even 반올림으로 L signed 정수 변환 | Coprocessor Unusable, Reserved Instruction / FPU: Inexact, Unimplemented Operation, Invalid Operation, Overflow | 213 |
| `ROUND.W.D` | FPU 기본 | 대기 | 대기 | `0x11` | `0x0C` | `0xFFFF003F` | `0x4620000C` | `fd, fs` | nearest ties-to-even 반올림으로 W signed 정수 변환 | Coprocessor Unusable, Reserved Instruction / FPU: Inexact, Unimplemented Operation, Invalid Operation, Overflow | 214 |
| `ROUND.W.S` | FPU 기본 | 대기 | 대기 | `0x11` | `0x0C` | `0xFFFF003F` | `0x4600000C` | `fd, fs` | nearest ties-to-even 반올림으로 W signed 정수 변환 | Coprocessor Unusable, Reserved Instruction / FPU: Inexact, Unimplemented Operation, Invalid Operation, Overflow | 214 |
| `RSQRT.D` | FPU 64 | 대기 | 대기 | `0x11` | `0x16` | `0xFFFF003F` | `0x46200016` | `fd, fs` | 1/√fs 근사 | Coprocessor Unusable, Reserved Instruction / FPU: Inexact, Division-by-zero, Unimplemented Operation, Invalid Operation, Overflow, Underflow | 215 |
| `RSQRT.S` | FPU 64 | 대기 | 대기 | `0x11` | `0x16` | `0xFFFF003F` | `0x46000016` | `fd, fs` | 1/√fs 근사 | Coprocessor Unusable, Reserved Instruction / FPU: Inexact, Division-by-zero, Unimplemented Operation, Invalid Operation, Overflow, Underflow | 215 |
| `SB` | CPU 필수 | 부분 | 회귀 통과 (제한 범위) | `0x28` | — | `0xFC000000` | `0xA0000000` | `rt, offset(base)` | 하위 byte store | TLB Refill, TLB Invalid, TLB Modified, Bus Error, Address Error, Watch | 216 |
| `SC` | CPU 필수 | 부분 | 회귀 통과 (제한 범위) | `0x38` | — | `0xFC000000` | `0xE0000000` | `rt, offset(base)` | reservation이 유효하면 word store. 성공 여부를 rt에 1 또는 0으로 기록 | TLB Refill, TLB Invalid, TLB Modified, Address Error, Watch. 선행 LL 및 동일 주소 조건 위반은 UNPREDICTABLE | 217 |
| `SDBBP` | EJTAG | 대기 | 대기 | `0x1C` | `0x3F` | `0xFC00003F` | `0x7000003F` | `code` | Debug Breakpoint 발생. Debug Mode에서는 Debug Mode Breakpoint | Debug Breakpoint Exception Debug Mode Breakpoint Exception | 219 |
| `SDC1` | FPU 기본 | 대기 | 대기 | `0x3D` | — | `0xFC000000` | `0xF4000000` | `ft, offset(base)` | FPR의 doubleword store | Coprocessor Unusable, Reserved Instruction, TLB Refill, TLB Invalid, TLB Modified, Address Error, Watch | 220 |
| `SDC2` | CP2 | 대기 | 대기 | `0x3E` | — | `0xFC000000` | `0xF8000000` | `rt, offset(base)` | CP2의 doubleword store | Coprocessor Unusable, Reserved Instruction, TLB Refill, TLB Invalid, TLB Modified, Address Error, Watch | 221 |
| `SDXC1` | FPU 64 | 대기 | 대기 | `0x13` | `0x09` | `0xFC0007FF` | `0x4C000009` | `fs, index(base)` | base + index에 FPR doubleword store | TLB Refill, TLB Invalid, TLB Modified, Coprocessor Unusable, Address Error, Reserved Instruction, Watch | 222 |
| `SEB` | CPU 필수 | 구현 | 회귀 통과 | `0x1F` | `0x20` | `0xFFE007FF` | `0x7C000420` | `rd, rt` | rt의 하위 8비트를 sign-extend | Reserved Instruction | 223 |
| `SEH` | CPU 필수 | 구현 | 회귀 통과 | `0x1F` | `0x20` | `0xFFE007FF` | `0x7C000620` | `rd, rt` | rt의 하위 16비트를 sign-extend | Reserved Instruction | 224 |
| `SH` | CPU 필수 | 부분 | 회귀 통과 (제한 범위) | `0x29` | — | `0xFC000000` | `0xA4000000` | `rt, offset(base)` | 하위 halfword store | TLB Refill, TLB Invalid, TLB Modified, Address Error, Watch | 225 |
| `SLL` | CPU 필수 | 구현 | 회귀 통과 | `0x00` | `0x00` | `0xFFE0003F` | `0x00000000` | `rd, rt, sa` | logical left shift by sa | — | 226 |
| `SLLV` | CPU 필수 | 구현 | 회귀 통과 | `0x00` | `0x04` | `0xFC0007FF` | `0x00000004` | `rd, rt, rs` | logical left shift by rs[4:0] | — | 227 |
| `SLT` | CPU 필수 | 구현 | 회귀 통과 | `0x00` | `0x2A` | `0xFC0007FF` | `0x0000002A` | `rd, rs, rt` | 작으면 1, 아니면 0. U는 unsigned 비교. immediate는 U 여부와 관계없이 sign-extend | — | 228 |
| `SLTI` | CPU 필수 | 구현 | 회귀 통과 | `0x0A` | — | `0xFC000000` | `0x28000000` | `rt, rs, immediate` | 작으면 1, 아니면 0. U는 unsigned 비교. immediate는 U 여부와 관계없이 sign-extend | — | 229 |
| `SLTIU` | CPU 필수 | 구현 | 회귀 통과 | `0x0B` | — | `0xFC000000` | `0x2C000000` | `rt, rs, immediate` | 작으면 1, 아니면 0. U는 unsigned 비교. immediate는 U 여부와 관계없이 sign-extend | — | 230 |
| `SLTU` | CPU 필수 | 구현 | 회귀 통과 | `0x00` | `0x2B` | `0xFC0007FF` | `0x0000002B` | `rd, rs, rt` | 작으면 1, 아니면 0. U는 unsigned 비교. immediate는 U 여부와 관계없이 sign-extend | — | 231 |
| `SQRT.D` | FPU 기본 | 대기 | 대기 | `0x11` | `0x04` | `0xFFFF003F` | `0x46200004` | `fd, fs` | 제곱근 | Coprocessor Unusable, Reserved Instruction / FPU: Invalid Operation, Inexact, Unimplemented Operation | 232 |
| `SQRT.S` | FPU 기본 | 대기 | 대기 | `0x11` | `0x04` | `0xFFFF003F` | `0x46000004` | `fd, fs` | 제곱근 | Coprocessor Unusable, Reserved Instruction / FPU: Invalid Operation, Inexact, Unimplemented Operation | 232 |
| `SRA` | CPU 필수 | 구현 | 회귀 통과 | `0x00` | `0x03` | `0xFFE0003F` | `0x00000003` | `rd, rt, sa` | arithmetic right shift by sa | — | 233 |
| `SRAV` | CPU 필수 | 구현 | 회귀 통과 | `0x00` | `0x07` | `0xFC0007FF` | `0x00000007` | `rd, rt, rs` | arithmetic right shift by rs[4:0] | — | 234 |
| `SRL` | CPU 필수 | 구현 | 회귀 통과 | `0x00` | `0x02` | `0xFFE0003F` | `0x00000002` | `rd, rt, sa` | logical right shift by sa | — | 235 |
| `SRLV` | CPU 필수 | 구현 | 회귀 통과 | `0x00` | `0x06` | `0xFC0007FF` | `0x00000006` | `rd, rt, rs` | logical right shift by rs[4:0] | — | 236 |
| `SSNOP` | CPU 필수 | 구현 | 회귀 통과 | `0x00` | `0x00` | `0xFFFFFFFF` | `0x00000040` | — | superscalar issue 경계용 no-op | — | 237 |
| `SUB` | CPU 필수 | 부분 | 기초 예외 경로 | `0x00` | `0x22` | `0xFC0007FF` | `0x00000022` | `rd, rs, rt` | signed 32-bit 뺄셈 | Integer Overflow (signed 결과 범위 초과 시, 목적 register 기록 생략) | 238 |
| `SUB.D` | FPU 기본 | 대기 | 대기 | `0x11` | `0x01` | `0xFFE0003F` | `0x46200001` | `fd, fs, ft` | fs − ft | Coprocessor Unusable, Reserved Instruction / FPU: Inexact, Overflow, Underflow, Invalid Operation, Unimplemented Operation | 239 |
| `SUB.PS` | FPU 64 | 대기 | 대기 | `0x11` | `0x01` | `0xFFE0003F` | `0x46C00001` | `fd, fs, ft` | fs − ft | Coprocessor Unusable, Reserved Instruction / FPU: Inexact, Overflow, Underflow, Invalid Operation, Unimplemented Operation | 239 |
| `SUB.S` | FPU 기본 | 대기 | 대기 | `0x11` | `0x01` | `0xFFE0003F` | `0x46000001` | `fd, fs, ft` | fs − ft | Coprocessor Unusable, Reserved Instruction / FPU: Inexact, Overflow, Underflow, Invalid Operation, Unimplemented Operation | 239 |
| `SUBU` | CPU 필수 | 구현 | 회귀 통과 | `0x00` | `0x23` | `0xFC0007FF` | `0x00000023` | `rd, rs, rt` | modulo 2³² 뺄셈 | — | 240 |
| `SUXC1` | FPU 64 | 대기 | 대기 | `0x13` | `0x0D` | `0xFC0007FF` | `0x4C00000D` | `fs, index(base)` | base + index를 8바이트 경계로 내림한 주소에 FPR doubleword store | Coprocessor Unusable, Reserved Instruction, TLB Refill, TLB Invalid, TLB Modified, Watch | 241 |
| `SW` | CPU 필수 | 부분 | 회귀 통과 (제한 범위) | `0x2B` | — | `0xFC000000` | `0xAC000000` | `rt, offset(base)` | word store | TLB Refill, TLB Invalid, TLB Modified, Address Error, Watch | 242 |
| `SWC1` | FPU 기본 | 대기 | 대기 | `0x39` | — | `0xFC000000` | `0xE4000000` | `ft, offset(base)` | FPR의 word store | Coprocessor Unusable, Reserved Instruction, TLB Refill, TLB Invalid, TLB Modified, Address Error, Watch | 243 |
| `SWC2` | CP2 | 대기 | 대기 | `0x3A` | — | `0xFC000000` | `0xE8000000` | `rt, offset(base)` | CP2의 word store | Coprocessor Unusable, Reserved Instruction, TLB Refill, TLB Invalid, TLB Modified, Address Error, Watch | 244 |
| `SWL` | CPU 필수 | 부분 | 회귀 통과 (제한 범위) | `0x2A` | — | `0xFC000000` | `0xA8000000` | `rt, offset(base)` | rt 상위 쪽의 부분 word store | TLB Refill, TLB Invalid, TLB Modified, Bus Error, Address Error , Watch | 245 |
| `SWR` | CPU 필수 | 부분 | 회귀 통과 (제한 범위) | `0x2E` | — | `0xFC000000` | `0xB8000000` | `rt, offset(base)` | rt 하위 쪽의 부분 word store | TLB Refill, TLB Invalid, TLB Modified, Bus Error, Address Error, Watch | 247 |
| `SWXC1` | FPU 64 | 대기 | 대기 | `0x13` | `0x08` | `0xFC0007FF` | `0x4C000008` | `fs, index(base)` | base + index에 FPR word store | TLB Refill, TLB Invalid, TLB Modified, Address Error, Reserved Instruction, Coprocessor Unusable, Watch | 249 |
| `SYNC` | CPU 필수 | 부분 | 회귀 통과 (제한 범위) | `0x00` | `0x0F` | `0xFFFFF83F` | `0x0000000F` | `[stype]` | stype에 따른 메모리 접근 ordering/completion barrier | — | 250 |
| `SYNCI` | CPU 필수 | 부분 | 회귀 통과 (제한 범위) | `0x01` | — | `0xFC1F0000` | `0x041F0000` | `offset(base)` | 주소에 해당하는 cache line의 instruction stream 동기화 | Reserved Instruction Exception (Release 1 implementations only) TLB Refill Exception TLB Invalid Exception Address Error Exception Cache Error Exception Bus Error Exception | 255 |
| `SYSCALL` | CPU 필수 | 부분 | 기초 예외 경로 | `0x00` | `0x0C` | `0xFC00003F` | `0x0000000C` | `[code]` | System Call 예외 발생 | System Call | 257 |
| `TEQ` | CPU 필수 | 부분 | 기초 예외 경로 | `0x00` | `0x34` | `0xFC00003F` | `0x00000034` | `rs, rt` | rt와 비교해 같으면 Trap 발생 | Trap | 258 |
| `TEQI` | CPU 필수 | 부분 | 기초 예외 경로 | `0x01` | — | `0xFC1F0000` | `0x040C0000` | `rs, immediate` | sign-extended immediate와 비교해 같으면 Trap 발생 | Trap | 259 |
| `TGE` | CPU 필수 | 부분 | 기초 예외 경로 | `0x00` | `0x30` | `0xFC00003F` | `0x00000030` | `rs, rt` | rt와 비교해 signed rs가 비교값 이상이면 Trap 발생 | Trap | 260 |
| `TGEI` | CPU 필수 | 부분 | 기초 예외 경로 | `0x01` | — | `0xFC1F0000` | `0x04080000` | `rs, immediate` | sign-extended immediate와 비교해 signed rs가 비교값 이상이면 Trap 발생 | Trap | 261 |
| `TGEIU` | CPU 필수 | 부분 | 기초 예외 경로 | `0x01` | — | `0xFC1F0000` | `0x04090000` | `rs, immediate` | sign-extended immediate와 비교해 unsigned rs가 비교값 이상이면 Trap 발생 | Trap | 262 |
| `TGEU` | CPU 필수 | 부분 | 기초 예외 경로 | `0x00` | `0x31` | `0xFC00003F` | `0x00000031` | `rs, rt` | rt와 비교해 unsigned rs가 비교값 이상이면 Trap 발생 | Trap | 263 |
| `TLBP` | TLB | 부분 | 회귀 통과 (제한 범위) | `0x10` | `0x08` | `0xFFFFFFFF` | `0x42000008` | — | EntryHi와 일치하는 TLB 엔트리를 찾아 Index 갱신 | Coprocessor Unusable Machine Check | 264 |
| `TLBR` | TLB | 부분 | 회귀 통과 (제한 범위) | `0x10` | `0x01` | `0xFFFFFFFF` | `0x42000001` | — | Index의 TLB 엔트리를 CP0 레지스터로 읽음 | Coprocessor Unusable Machine Check. Index가 범위 밖이면 UNDEFINED | 265 |
| `TLBWI` | TLB | 부분 | 회귀 통과 (제한 범위) | `0x10` | `0x02` | `0xFFFFFFFF` | `0x42000002` | — | CP0의 TLB 데이터를 Index 엔트리에 기록 | Coprocessor Unusable Machine Check. Index가 범위 밖이면 UNDEFINED | 267 |
| `TLBWR` | TLB | 부분 | 회귀 통과 (제한 범위) | `0x10` | `0x06` | `0xFFFFFFFF` | `0x42000006` | — | CP0의 TLB 데이터를 Random 엔트리에 기록 | Coprocessor Unusable Machine Check | 269 |
| `TLT` | CPU 필수 | 부분 | 기초 예외 경로 | `0x00` | `0x32` | `0xFC00003F` | `0x00000032` | `rs, rt` | rt와 비교해 signed rs가 비교값 미만이면 Trap 발생 | Trap | 270 |
| `TLTI` | CPU 필수 | 부분 | 기초 예외 경로 | `0x01` | — | `0xFC1F0000` | `0x040A0000` | `rs, immediate` | sign-extended immediate와 비교해 signed rs가 비교값 미만이면 Trap 발생 | Trap | 271 |
| `TLTIU` | CPU 필수 | 부분 | 기초 예외 경로 | `0x01` | — | `0xFC1F0000` | `0x040B0000` | `rs, immediate` | sign-extended immediate와 비교해 unsigned rs가 비교값 미만이면 Trap 발생 | Trap | 272 |
| `TLTU` | CPU 필수 | 부분 | 기초 예외 경로 | `0x00` | `0x33` | `0xFC00003F` | `0x00000033` | `rs, rt` | rt와 비교해 unsigned rs가 비교값 미만이면 Trap 발생 | Trap | 273 |
| `TNE` | CPU 필수 | 부분 | 기초 예외 경로 | `0x00` | `0x36` | `0xFC00003F` | `0x00000036` | `rs, rt` | rt와 비교해 다르면 Trap 발생 | Trap | 274 |
| `TNEI` | CPU 필수 | 부분 | 기초 예외 경로 | `0x01` | — | `0xFC1F0000` | `0x040E0000` | `rs, immediate` | sign-extended immediate와 비교해 다르면 Trap 발생 | Trap | 275 |
| `TRUNC.L.D` | FPU 64 | 대기 | 대기 | `0x11` | `0x09` | `0xFFFF003F` | `0x46200009` | `fd, fs` | 0 방향 반올림으로 L signed 정수 변환 | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation, Overflow, Inexact | 276 |
| `TRUNC.L.S` | FPU 64 | 대기 | 대기 | `0x11` | `0x09` | `0xFFFF003F` | `0x46000009` | `fd, fs` | 0 방향 반올림으로 L signed 정수 변환 | Coprocessor Unusable, Reserved Instruction / FPU: Unimplemented Operation, Invalid Operation, Overflow, Inexact | 276 |
| `TRUNC.W.D` | FPU 기본 | 대기 | 대기 | `0x11` | `0x0D` | `0xFFFF003F` | `0x4620000D` | `fd, fs` | 0 방향 반올림으로 W signed 정수 변환 | Coprocessor Unusable, Reserved Instruction / FPU: Inexact, Invalid Operation, Overflow, Unimplemented Operation | 277 |
| `TRUNC.W.S` | FPU 기본 | 대기 | 대기 | `0x11` | `0x0D` | `0xFFFF003F` | `0x4600000D` | `fd, fs` | 0 방향 반올림으로 W signed 정수 변환 | Coprocessor Unusable, Reserved Instruction / FPU: Inexact, Invalid Operation, Overflow, Unimplemented Operation | 277 |
| `WAIT` | CP0 | 대기 | 대기 | `0x10` | `0x20` | `0xFE00003F` | `0x42000020` | — | 구현에서 지원하는 wake 이벤트까지 대기 가능 | CP0 접근 금지 시 Coprocessor Unusable. delay slot에서는 UNDEFINED | 278 |
| `WRPGPR` | CP0 | 대기 | 대기 | `0x10` | — | `0xFFE007FF` | `0x41C00000` | `rd, rt` | 현재 rt를 이전 shadow GPR set의 rd로 이동 | Coprocessor Unusable, Reserved Instruction | 279 |
| `WSBH` | CPU 필수 | 구현 | 회귀 통과 | `0x1F` | `0x20` | `0xFFE007FF` | `0x7C0000A0` | `rd, rt` | 각 halfword 내부의 두 byte를 교환 | Reserved Instruction | 280 |
| `XOR` | CPU 필수 | 구현 | 회귀 통과 | `0x00` | `0x26` | `0xFC0007FF` | `0x00000026` | `rd, rs, rt` | bitwise XOR | — | 281 |
| `XORI` | CPU 필수 | 구현 | 회귀 통과 | `0x0E` | — | `0xFC000000` | `0x38000000` | `rt, rs, immediate` | zero-extended immediate와 bitwise XOR | — | 282 |
