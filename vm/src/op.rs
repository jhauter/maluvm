use crate::asm::ArgType;

#[allow(non_upper_case_globals)]
pub mod opcode {
    pub const Nop: u8 = 0x01;
    pub const Unreachable: u8 = 0x02;
    pub const Drop: u8 = 0x03;
    pub const Const: u8 = 0x04;
    pub const Jmp: u8 = 0x05;
    pub const JmpIf: u8 = 0x06;
    pub const Branch: u8 = 0x07;
    pub const BranchIf: u8 = 0x08;
    pub const LocalGet: u8 = 0x09;
    pub const LocalSet: u8 = 0x0a;
    pub const LocalTee: u8 = 0x0b;
    pub const GlobalGet: u8 = 0x0c;
    pub const GlobalSet: u8 = 0x0e;
    pub const GlobalTee: u8 = 0x0f;
    pub const Eq: u8 = 0x10;
    pub const Eqz: u8 = 0x11;
    pub const Add: u8 = 0x12;
    pub const Sub: u8 = 0x13;
    pub const Divs: u8 = 0x14;
    pub const Divu: u8 = 0x15;
    pub const Mul: u8 = 0x16;
    pub const Neg: u8 = 0x17;
    pub const Gt: u8 = 0x18;
    pub const Lt: u8 = 0x19;
    pub const Ge: u8 = 0x1a;
    pub const Le: u8 = 0x1b;
    pub const Shiftr: u8 = 0x1c;
    pub const Shiftl: u8 = 0x1d;
    pub const And: u8 = 0x1e;
    pub const Or: u8 = 0x1f;
    pub const Xor: u8 = 0x20;
    pub const Call: u8 = 0x21;
    pub const Return: u8 = 0x22;
    pub const Store8: u8 = 0x23;
    pub const Store16: u8 = 0x24;
    pub const Store32: u8 = 0x25;
    pub const Load8u: u8 = 0x26;
    pub const Load8s: u8 = 0x27;
    pub const Load16s: u8 = 0x28;
    pub const Load16u: u8 = 0x29;
    pub const Load32s: u8 = 0x2a;
    pub const Load32u: u8 = 0x2b;
    pub const Extend8_32s: u8 = 0x2c;
    pub const Extend16_32s: u8 = 0x2d;
    pub const Extend8_32u: u8 = 0x2e;
    pub const Extend16_32u: u8 = 0x2f;
    pub const End: u8 = 0x30;
    pub const PushArg: u8 = 0x31;
    pub const DbgAssert: u8 = 0x32;

    pub struct StoreArgs {
        pub addr: u32,
        pub value: u32,
    }
}

#[derive(PartialEq, Debug, Clone)]
pub enum Op<'src> {
    Nop,
    Unreachable,
    Drop,
    Const(ArgType<'src>),
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
    Store8(ArgType<'src>),
    Store16(ArgType<'src>),
    Store32(ArgType<'src>),
    Load8u(ArgType<'src>),
    Load8s(ArgType<'src>),
    Load16s(ArgType<'src>),
    Load16u(ArgType<'src>),
    Load32s(ArgType<'src>),
    Load32u(ArgType<'src>),

    Extend8_32s,
    Extend16_32s,
    Extend8_32u,
    Extend16_32u,
    PushArg,
    DbgAssert,
    End,
}

impl<'src> std::fmt::Display for Op<'src> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Op::Nop => write!(f, "nop"),
            Op::Unreachable => write!(f, "unreachable"),
            Op::Drop => write!(f, "drop"),
            Op::Const(i) => write!(f, "const {}", i),
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
            Op::Store8(off) => write!(f, "store_8 {}", off),
            Op::Store16(off) => write!(f, "store_16 {}", off),
            Op::Store32(off) => write!(f, "store_32 {}", off),
            Op::Load8u(off) => write!(f, "load_8_u {}", off),
            Op::Load8s(off) => write!(f, "load_8_s {}", off),
            Op::Load16s(off) => write!(f, "load_16_s {}", off),
            Op::Load16u(off) => write!(f, "load_16_u {}", off),
            Op::Load32s(off) => write!(f, "load_32_s {}", off),
            Op::Load32u(off) => write!(f, "load_32_u {}", off),
            Op::Extend8_32s => write!(f, "extend_8_32_s"),
            Op::Extend16_32s => write!(f, "extend_16_32_s"),
            Op::Extend8_32u => write!(f, "extend_8_32_u"),
            Op::Extend16_32u => write!(f, "extend_16_32_u"),
            Op::And => write!(f, "and"),
            Op::Or => write!(f, "or"),
            Op::Xor => write!(f, "xor"),
            Op::End => write!(f, "end"),
            Op::PushArg => write!(f, "push_arg"),
            Op::DbgAssert => write!(f, "dbg_assert"),
        }
    }
}

impl<'src> Op<'src> {
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
            Op::DbgAssert => 0x32,
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
