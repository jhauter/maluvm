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
