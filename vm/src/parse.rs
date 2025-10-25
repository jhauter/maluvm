use byteorder::{LittleEndian, ReadBytesExt};
use core::fmt;
use std::io::{ErrorKind, Read};

use crate::asm::{opcode, RawArg, RawOp};

#[derive(Debug, Clone)]
pub enum MaybeRawOp {
    Op(RawOp),
    Unknown(u8)
}
pub fn try_parse_ops_from_bytecode(reader: &mut impl Read) -> impl Iterator<Item = Result<MaybeRawOp, std::io::Error>> {
    (0..).map_while(|_| {
        match try_parse_op(reader) {
            Err(e) if e.kind() == ErrorKind::UnexpectedEof => None,
            v @ _ => Some(v)
        }
    })   
}

impl RawArg {
    pub fn decode_num(reader: &mut impl Read) -> Result<Self, std::io::Error> {
        Ok(Self::Num(reader.read_u32::<LittleEndian>()?))
    }
    pub fn decode_register(reader: &mut impl Read) -> Result<Self, std::io::Error> {
        Ok(Self::Register(reader.read_u8()?))
    }
} 

macro_rules! make_op {
    ($code: expr, $op: expr, Num) => {
        let arg = RawArg::decode_num($code)?;
        Ok(MaybeRawOp::Op(RawOp {opcode: $op, arg: Some(arg)}))
        
    };
    ($code: expr, $op: expr, Register) => {
        let arg = RawArg::decode_register($code)?;
        Ok(MaybeRawOp::Op(RawOp {opcode: $op, arg: Some(arg)}))
    };
    ($op: expr) => {
        Ok(MaybeRawOp::Op(RawOp {opcode: $op, arg: None}))
    }
     
}
pub fn try_parse_op(reader: &mut impl Read) -> Result<MaybeRawOp, std::io::Error> {
    let opcode = reader.read_u8()?;
    match opcode {
          opcode::Nop 
        | opcode::Unreachable
        | opcode::Drop 
        | opcode::Jmp
        | opcode::JmpIf
        | opcode::Branch 
        | opcode::BranchIf
        | opcode::Eq..=opcode::Return
        | opcode::End..=opcode::DbgAssert
        => make_op!(opcode),
        opcode::LocalGet..=opcode::GlobalTee => {
            make_op! {reader, opcode, Register}
        },
        opcode::Const 
        | opcode::Store8..=opcode::Load32u => {
            make_op! {reader, opcode, Num}
        }
        _ => Ok(MaybeRawOp::Unknown(opcode))
    }   

}
