#[derive(PartialEq, PartialOrd, Debug, Clone)]
pub enum Op {
    Nop,
    Unreachable,
    Drop,
    Const(i32),
    Jmp,
    JmpIf,
    Branch,
    BranchIf,
    LocalGet(u8),
    LocalSet(u8),
    LocalTee(u8),
    GlobalGet(u8),
    GlobalSet(u8),
    GlobalTee(u8),
    Eq,
    Eqz,
    Add,
    Sub,
    Divs,
    Divu,
    Mul,
    Neg,
    Gt,
    Lt,
    Ge,
    Le,
    And,
    Or,
    Xor,
    Shiftr,
    Shiftl,
    Call,
    Return,
    Store8(u32),
    Store16(u32),
    Store32(u32),

    Load8u(u32),
    Load8s(u32),
    Load16s(u32),
    Load16u(u32),
    Load32s(u32),
    Load32u(u32),

    Extend8_32s,
    Extend16_32s,
    Extend8_32u,
    Extend16_32u,
    PushArg,
    End,
}

impl std::fmt::Display for Op {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Op::Nop => write!(f, "nop"),
            Op::Unreachable => write!(f, "unreachable"),
            Op::Drop => write!(f, "drop"),
            Op::Const(i) => write!(f, "const {:x}", i),
            Op::Jmp => write!(f, "jmp"),
            Op::JmpIf => write!(f, "jmp_if"),
            Op::Branch => write!(f, "branch"),
            Op::BranchIf => write!(f, "branch_if"),
            Op::LocalGet(i) => write!(f, "local_get {:x}", i),
            Op::LocalSet(i) => write!(f, "local_set {:x}", i),
            Op::LocalTee(i) => write!(f, "local_tee {:x}", i),
            Op::GlobalGet(i) => write!(f, "global_get {:x}", i),
            Op::GlobalSet(i) => write!(f, "global_set {:x}", i),
            Op::GlobalTee(i) => write!(f, "global_tee {:x}", i),
            Op::Eq => write!(f, "eq"),
            Op::Eqz => write!(f, "eqz"),
            Op::Add => write!(f, "add"),
            Op::Sub => write!(f, "sub"),
            Op::Divs => write!(f, "div_s"),
            Op::Divu => write!(f, "div_u"),
            Op::Mul => write!(f, "mul"),
            Op::Neg => write!(f, "neg"),
            Op::Gt => write!(f, "gt"),
            Op::Lt => write!(f, "lt"),
            Op::Ge => write!(f, "ge"),
            Op::Le => write!(f, "le"),
            Op::Shiftr => write!(f, "shiftr"),
            Op::Shiftl => write!(f, "shiftl"),
            Op::Call => write!(f, "call"),
            Op::Return => write!(f, "return"),
            Op::Store8(off) => write!(f, "store_8 {:x}", off),
            Op::Store16(off) => write!(f, "store_16 {:x}", off),
            Op::Store32(off) => write!(f, "store_32 {:x}", off),
            Op::Load8u(off) => write!(f, "load_8_u {:x}", off),
            Op::Load8s(off) => write!(f, "load_8_s {:x}", off),
            Op::Load16s(off) => write!(f, "load_16_s {:x}", off),
            Op::Load16u(off) => write!(f, "load_16_u {:x}", off),
            Op::Load32s(off) => write!(f, "load_32_s {:x}", off),
            Op::Load32u(off) => write!(f, "load_32_u {:x}", off),
            Op::Extend8_32s => write!(f, "extend_8_32_s"),
            Op::Extend16_32s => write!(f, "extend_16_32_s"),
            Op::Extend8_32u => write!(f, "extend_8_32_u"),
            Op::Extend16_32u => write!(f, "extend_16_32_u"),
            Op::And => write!(f, "and"),
            Op::Or => write!(f, "or"),
            Op::Xor => write!(f, "xor"),
            Op::End => write!(f, "end"),
            Op::PushArg => write!(f, "push_arg"),
        }
    }
}

impl Op {
    pub fn repr(&self) -> u8 {
        match self {
            Op::Nop => 0x01,
            Op::Unreachable => 0x02,
            Op::Drop => 0x03,
            Op::Const(_) => 0x04,
            Op::Jmp => 0x05,
            Op::JmpIf => 0x06,
            Op::Branch => 0x07,
            Op::BranchIf => 0x08,
            Op::LocalGet(_) => 0x09,
            Op::LocalSet(_) => 0x0a,
            Op::LocalTee(_) => 0x0b,
            Op::GlobalGet(_) => 0x0c,
            Op::GlobalSet(_) => 0x0e,
            Op::GlobalTee(_) => 0x0f,
            Op::Eq => 0x10,
            Op::Eqz => 0x11,
            Op::Add => 0x12,
            Op::Sub => 0x13,
            Op::Divs => 0x14,
            Op::Divu => 0x15,
            Op::Mul => 0x16,
            Op::Neg => 0x17,
            Op::Gt => 0x18,
            Op::Lt => 0x19,
            Op::Ge => 0x1a,
            Op::Le => 0x1b,
            Op::Shiftr => 0x1c,
            Op::Shiftl => 0x1d,
            Op::And => 0x1e,
            Op::Or => 0x1f,
            Op::Xor => 0x20,
            Op::Call => 0x21,
            Op::Return => 0x22,
            Op::Store8(_) => 0x23,
            Op::Store16(_) => 0x24,
            Op::Store32(_) => 0x25,
            Op::Load8u(_) => 0x26,
            Op::Load8s(_) => 0x27,
            Op::Load16s(_) => 0x28,
            Op::Load16u(_) => 0x29,
            Op::Load32s(_) => 0x2a,
            Op::Load32u(_) => 0x2b,
            Op::Extend8_32s => 0x2c,
            Op::Extend16_32s => 0x2d,
            Op::Extend8_32u => 0x2e,
            Op::Extend16_32u => 0x2f,
            Op::End => 0x30,
            Op::PushArg => 0x31,
        }
    }
    pub fn size_bytes(&self) -> u8 {
        match self {
            Op::LocalGet(_)
            | Op::LocalSet(_)
            | Op::LocalTee(_)
            | Op::GlobalGet(_)
            | Op::GlobalSet(_)
            | Op::GlobalTee(_) => 2,
            Op::Const(_)
            | Op::Store8(_)
            | Op::Store16(_)
            | Op::Store32(_)
            | Op::Load8u(_)
            | Op::Load8s(_)
            | Op::Load16s(_)
            | Op::Load16u(_)
            | Op::Load32s(_)
            | Op::Load32u(_) => 5,
            _ => 1,
        }
    }
}
