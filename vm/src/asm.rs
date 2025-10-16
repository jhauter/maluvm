use crate::interpreter::InterpreterErrorType;
use core::fmt;
use std::{
    collections::HashMap,
    ffi::os_str::Display,
    num::{ParseIntError, TryFromIntError},
};

#[derive(Debug, Clone)]
pub enum AssembleErrorKind {
    MissingDelimiter,
    UnknownOperation,
    UnableToParseInt(ParseIntError),
    IntSize(TryFromIntError),
    MissingArgument,
    TooManyArguments,
    UnknownLabel(String),
    LabelAlreadyExists(String),
    UnexpectedRegisterId(i32),
    UnexpectedImmArgSize,
}

impl From<ParseIntError> for AssembleErrorKind {
    fn from(value: ParseIntError) -> Self {
        AssembleErrorKind::UnableToParseInt(value)
    }
}

impl From<TryFromIntError> for AssembleErrorKind {
    fn from(value: TryFromIntError) -> Self {
        AssembleErrorKind::IntSize(value)
    }
}

#[derive(Debug, Clone)]
pub struct AssembleError {
    kind: AssembleErrorKind,
    line: usize,
}
impl<'src> AssembleError {
    pub fn new(state: &Parser, kind: AssembleErrorKind) -> Self {
        AssembleError {
            kind,
            line: state.line,
        }
    }
}
const STATEMENT_SEP: char = ';';
const ENTRY_LABEL_NAME: &'static str = "__ENTRY__";
pub const BYTECODE_HEADER: [u8; 4] = [b'm', b'a', b'l', b'u'];

//TODO (joh): This has to be updated manually each time the bytecode header definiton changes.
//Make this a macro maybe
pub const CODE_START_ADDR_POS: u32 = (2 * size_of::<u32>()) as u32;
pub const CODE_START: u32 = (3 * size_of::<u32>()) as u32;

pub struct ParseOutput<'src, T> {
    word: T,
    rest: Option<&'src str>,
}

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
pub enum RawArg {
    Register(u8),
    Num(u32),
}
impl RawArg {
    pub fn from_arg_type(arg: &ArgType<'_>, parser: &Parser) -> Result<Self, AssembleError> {
        match arg {
            ArgType::AbsLabelRef(l) => Ok(RawArg::Num(parser.get_abs_label_addr(l)? as u32)),
            ArgType::OffLabelRef(l) => Ok(RawArg::Num(parser.get_off_label_addr(l)? as u32)),
            ArgType::Number(n) => Ok(RawArg::Num(*n as u32)),
            ArgType::Register(r) => Ok(RawArg::Register(*r)),
        }
    }

    pub fn size_bytes(&self) -> usize {
        match self {
            RawArg::Register(_) => size_of::<u8>(),
            RawArg::Num(_) => size_of::<u32>(),
        }
    }
    pub fn encode(&self, buffer: &mut Vec<u8>) {
        match self {
            Self::Register(r) => buffer.push(*r),
            Self::Num(n) => buffer.extend_from_slice(&n.to_le_bytes()),
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
pub struct Op<'src> {
    opcode: u8,
    arg: Option<ArgType<'src>>,
}
impl Op<'_> {
    pub fn size_bytes(&self) -> usize {
        size_of::<u8>() + self.arg.as_ref().map_or(0, |a| a.size_bytes())
    }
}
#[derive(PartialEq, Debug, Clone)]
pub struct RawOp {
    pub opcode: u8,
    pub arg: Option<RawArg>,
}
impl RawOp {
    pub fn from_op(op: &Op<'_>, parser: &Parser) -> Result<Self, AssembleError> {
        let opcode = op.opcode;

        let arg = op
            .arg
            .clone()
            .map(|a| RawArg::from_arg_type(&a, parser))
            .transpose()?;
        Ok(Self { opcode, arg })
    }

    pub fn encode(&self, dest: &mut Vec<u8>) {
        dest.push(self.opcode);
        if let Some(arg) = self.arg.as_ref() {
            arg.encode(dest);
        }
    }

    pub fn size_bytes(&self) -> usize {
        size_of::<u8>() + self.arg.as_ref().map_or(0, |a| a.size_bytes())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Label<'src> {
    pub name: &'src str,
    pub position: usize,
}

impl<'src> std::fmt::Display for Label<'src> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {:#05x}", self.name, self.position)
    }
}

pub struct BytecodeInfo {
    pub code_size_bytes: u32,
    pub instruction_count: u32,
    pub code_start_offset: u32,
}

impl BytecodeInfo {
    //NOTE(joh): Maybe use a packed struct?
    pub const fn total_header_size() -> usize {
        3 * size_of::<u32>() + size_of_val(&BYTECODE_HEADER)
    }
    pub fn total_size(&self) -> usize {
        self.code_size_bytes as usize + Self::total_header_size()
    }

    fn to_bytecode(&self) -> Vec<u8> {
        let mut buffer = Vec::with_capacity(self.total_size());
        buffer.extend_from_slice(&BYTECODE_HEADER);

        buffer.extend_from_slice(&(self.code_size_bytes).to_le_bytes());
        buffer.extend_from_slice(&(self.instruction_count).to_le_bytes());
        buffer.extend_from_slice(&(self.code_start_offset).to_le_bytes());

        buffer
    }
}

macro_rules! impl_parse_num {
    ($fn_name: ident, $type: ty) => {
        pub fn $fn_name(&self, str: &str) -> Result<$type, AssembleError> {
            if str.len() == 1 {
                Ok(<$type>::from_str_radix(str, 10)
                    .map_err(|e| AssembleError::new(self, e.into()))?)
            } else {
                match &str[0..2] {
                    "0x" => Ok(<$type>::from_str_radix(&str[2..], 16)
                        .map_err(|e| AssembleError::new(self, e.into()))?),
                    "0b" => Ok(<$type>::from_str_radix(&str[2..], 2)
                        .map_err(|e| AssembleError::new(self, e.into()))?),
                    _ => Ok(<$type>::from_str_radix(str, 10)
                        .map_err(|e| AssembleError::new(self, e.into()))?),
                }
            }
        }
    };
}

#[derive(Debug, Eq, PartialEq, PartialOrd, Clone, Copy)]
pub struct LabelId(usize);

impl fmt::Display for LabelId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub struct Parser {
    op_count: usize,
    op_size_bytes: usize,
    line: usize,

    labels: HashMap<String, u32>,
}

pub struct ParseResult {
    pub code: Box<[u8]>, 
    pub labels: Box<[(String, u32)]>,
}

impl<'src> Parser {
    impl_parse_num!(parse_u32, u32);
    impl_parse_num!(parse_i32, i32);
    impl_parse_num!(parse_u8, u8);

    pub fn new() -> Self {
        Self {
            line: 0,
            op_count: 0,
            op_size_bytes: 0,
            labels: HashMap::new(),
        }
    }

    pub fn parse(code: &'src str) -> Result<ParseResult, AssembleError> {
        let mut parser = Self::new();

        let elems = parser.parse_elems(code)?;
        let ops = parser.parse_ops(&elems)?;
        let mut labels: Vec<(String, u32)> = parser.labels.iter()
            .map(|(k, v)| (k.to_string(), *v))
            .collect::<Vec<_>>();

        labels.sort_by(|(_, v1), (_, v2)| v1.cmp(v2)); 
            
        let res = ParseResult {
            code: parser.as_bytecode(&ops),
            labels: labels.into_boxed_slice(),
        };
        Ok(res)
    }

    pub fn try_push_label(&mut self, name: &str, position: u32) -> Result<LabelId, AssembleError> {
        match self.labels.get(name) {
            Some(_) => Err(AssembleError::new(
                self,
                AssembleErrorKind::LabelAlreadyExists(name.to_string()),
            )),
            None => {
                let id = self.labels.len();
                _ = self.labels.insert(name.to_string(), position);

                Ok(LabelId(id))
            }
        }
    }

    pub fn parse_elems(&mut self, code: &'src str) -> Result<Box<[Elem<'src>]>, AssembleError> {
        let mut rest = Some(code);
        let mut elems = Vec::new();

        loop {
            match rest {
                Some(r) => match r.chars().next() {
                    Some('\n') | Some('\r') | Some(' ') => rest = self.skip_whitespace(r),

                    Some('#') => {
                        //TODO: Make this more consistent
                        let statement = self.slice_until(&r[1..], ';')?;
                        let arg = self.parse_arg(statement.word)?;
                        self.op_size_bytes += 5;

                        elems.push(Elem::Const(arg));
                        self.op_count += 1;
                        rest = statement.rest;
                    }

                    Some('*') => {
                        let arg = r.chars().next();
                    }
                    Some(':') => {
                        let (label, label_rest) = self.parse_label(&r[1..])?;

                        let id = self.try_push_label(label.name, label.position as u32)?;
                        elems.push(Elem::Label(id));
                        rest = label_rest;
                    }
                    Some(_) => {
                        let (op, op_rest) = self.parse_op(r)?;
                        self.op_size_bytes += op.size_bytes() as usize;

                        elems.push(Elem::Op(op));
                        self.op_count += 1;

                        rest = op_rest;
                    }
                    None => break,
                },
                None => break,
            }
        }
        Ok(elems.into())
    }

    pub fn parse_ops(&self, elems: &[Elem<'src>]) -> Result<Box<[RawOp]>, AssembleError> {
        let mut ops = Vec::with_capacity(self.op_count);

        for elem in elems {
            match elem {
                Elem::Op(op) => ops.push(RawOp::from_op(op, self)?),
                Elem::Label(_) => {}
                Elem::Const(arg_type) => ops.push(RawOp {
                    opcode: opcode::Const,
                    arg: Some(RawArg::from_arg_type(arg_type, self)?),
                }),
            }
        }

        Ok(ops.into_boxed_slice())
    }

    pub fn try_get_label(&self, id: &'src str) -> Result<u32, AssembleError> {
        self.labels
            .get(id)
            .ok_or(AssembleError::new(
                self,
                AssembleErrorKind::UnknownLabel(id.to_string()),
            ))
            .copied()
    }

    pub fn get_abs_label_addr(&self, name: &'src str) -> Result<i32, AssembleError> {
        let label = self.try_get_label(name)?;
        Ok(label as i32 + CODE_START as i32)
    }

    pub fn get_off_label_addr(&self, name: &'src str) -> Result<i32, AssembleError> {
        let label = self.try_get_label(name)?;
        Ok(((self.op_size_bytes as isize) - (label as isize)) as i32)
    }

    pub fn get_bytecode_info(&self) -> BytecodeInfo {
        let code_start_offset =
            self.labels.get(ENTRY_LABEL_NAME).copied().unwrap_or(0) as u32 + CODE_START;

        BytecodeInfo {
            code_size_bytes: self.op_size_bytes as u32,
            instruction_count: self.op_count as u32,
            code_start_offset,
        }
    }

    pub fn as_bytecode(&self, ops: &'src [RawOp]) -> Box<[u8]> {
        let info = self.get_bytecode_info();
        let mut buffer = info.to_bytecode();

        ops.iter().for_each(|o| o.encode(&mut buffer));

        buffer.into_boxed_slice()
    }

    pub fn slice_until(
        &self,
        rest: &'src str,
        delim: char,
    ) -> Result<ParseOutput<'src, &'src str>, AssembleError> {
        let end = rest
            .char_indices()
            .find(|(_, s)| *s == delim)
            .ok_or(AssembleError::new(
                self,
                AssembleErrorKind::MissingDelimiter,
            ))?
            .0;

        let next_rest = rest.get((end + 1)..);
        Ok(ParseOutput {
            word: &rest[..end],
            rest: next_rest,
        })
    }

    pub fn skip_whitespace(&mut self, rest: &'src str) -> Option<&'src str> {
        for (i, c) in rest.char_indices() {
            match c {
                '\r' | ' ' => {}
                '\n' => self.line += 1,
                _ => return Some(&rest[i..]),
            }
        }
        None
    }

    pub fn parse_arg(&self, s: &'src str) -> Result<ArgType<'src>, AssembleError> {
        match s.chars().next().unwrap() {
            '@' => Ok(ArgType::AbsLabelRef(&s[1..])),
            '.' => Ok(ArgType::OffLabelRef(&s[1..])),
            _ => {
                let num = self.parse_i32(s)?;
                Ok(ArgType::Number(num))
            }
        }
    }
    pub fn arg_register(
        &self,
        args: &mut impl Iterator<Item = &'src str>,
    ) -> Result<ArgType<'src>, AssembleError> {
        let s = args
            .next()
            .ok_or(AssembleError::new(self, AssembleErrorKind::MissingArgument))?;
        match self.parse_arg(s)? {
            ArgType::Number(num) => match num {
                n @ 0..255 => Ok(ArgType::Register(n as u8)),
                num => Err(AssembleError::new(
                    self,
                    AssembleErrorKind::UnexpectedRegisterId(num),
                )),
            },
            _ => Err(AssembleError::new(
                self,
                AssembleErrorKind::UnexpectedImmArgSize,
            )),
        }
    }

    pub fn arg_const(
        &self,
        args: &mut impl Iterator<Item = &'src str>,
    ) -> Result<ArgType<'src>, AssembleError> {
        let s = args
            .next()
            .ok_or(AssembleError::new(self, AssembleErrorKind::MissingArgument))?;
        Ok(self.parse_arg(s)?)
    }

    pub fn parse_op(&self, s: &'src str) -> Result<(Op<'src>, Option<&'src str>), AssembleError> {
        let statement = self.slice_until(s, ';')?;
        let mut op_str = iter_op_args(statement.word);
        let op_name = op_str.next().unwrap();
        let (opcode, arg): (u8, Option<ArgType<'src>>) = match op_name {
            "nop" => Ok((opcode::Nop, None)),
            "unreachable" => Ok((opcode::Unreachable, None)),
            "drop" => Ok((opcode::Drop, None)),
            "const" => Ok((opcode::Const, Some(self.arg_const(&mut op_str)?))),
            "jmp" => Ok((opcode::Jmp, None)),
            "jmp_if" => Ok((opcode::JmpIf, None)),
            "branch" => Ok((opcode::Branch, None)),
            "branch_if" => Ok((opcode::BranchIf, None)),
            "local_get" => Ok((opcode::LocalGet, Some(self.arg_register(&mut op_str)?))),
            "local_set" => Ok((opcode::LocalSet, Some(self.arg_register(&mut op_str)?))),
            "local_tee" => Ok((opcode::LocalTee, Some(self.arg_register(&mut op_str)?))),
            "global_get" => Ok((opcode::GlobalGet, Some(self.arg_register(&mut op_str)?))),
            "global_set" => Ok((opcode::GlobalSet, Some(self.arg_register(&mut op_str)?))),
            "global_tee" => Ok((opcode::GlobalTee, Some(self.arg_register(&mut op_str)?))),
            "eq" => Ok((opcode::Eq, None)),
            "eqz" => Ok((opcode::Eqz, None)),
            "add" => Ok((opcode::Add, None)),
            "sub" => Ok((opcode::Sub, None)),
            "div_s" => Ok((opcode::Divs, None)),
            "div_u" => Ok((opcode::Divu, None)),
            "mul" => Ok((opcode::Mul, None)),
            "neg" => Ok((opcode::Neg, None)),
            "gt" => Ok((opcode::Gt, None)),
            "lt" => Ok((opcode::Lt, None)),
            "ge" => Ok((opcode::Ge, None)),
            "le" => Ok((opcode::Le, None)),
            "shiftr" => Ok((opcode::Shiftr, None)),
            "shiftl" => Ok((opcode::Shiftl, None)),
            "call" => Ok((opcode::Call, None)),
            "return" => Ok((opcode::Return, None)),
            "store_8" => Ok((opcode::Store8, Some(self.arg_const(&mut op_str)?))),
            "store_16" => Ok((opcode::Store16, Some(self.arg_const(&mut op_str)?))),
            "store_32" => Ok((opcode::Store32, Some(self.arg_const(&mut op_str)?))),
            "load_8_u" => Ok((opcode::Load8u, Some(self.arg_const(&mut op_str)?))),
            "load_8_s" => Ok((opcode::Load8s, Some(self.arg_const(&mut op_str)?))),
            "load_16_s" => Ok((opcode::Load16s, Some(self.arg_const(&mut op_str)?))),
            "load_16_u" => Ok((opcode::Load16u, Some(self.arg_const(&mut op_str)?))),
            "load_32_s" => Ok((opcode::Load32s, Some(self.arg_const(&mut op_str)?))),
            "load_32_u" => Ok((opcode::Load32u, Some(self.arg_const(&mut op_str)?))),
            "extend_8_32_s" => Ok((opcode::Extend16_32s, None)),
            "extend_16_32_s" => Ok((opcode::Extend16_32s, None)),
            "extend_8_32_u" => Ok((opcode::Extend8_32u, None)),
            "extend_16_32_u" => Ok((opcode::Extend16_32u, None)),
            "end" => Ok((opcode::End, None)),
            "push_arg" => Ok((opcode::PushArg, None)),
            "dbg_assert" => Ok((opcode::DbgAssert, None)),
            _ => Err(AssembleError::new(
                self,
                AssembleErrorKind::UnknownOperation,
            )),
        }?;
        match op_str.next() {
            Some(_) => Err(AssembleError::new(
                self,
                AssembleErrorKind::TooManyArguments,
            )),
            None => Ok((Op { opcode, arg }, statement.rest)),
        }
    }

    pub fn parse_label(
        &self,
        s: &'src str,
    ) -> Result<(Label<'src>, Option<&'src str>), AssembleError> {
        let label = self.slice_until(s, ':')?;
        let position = self.op_size_bytes;
        Ok((
            Label {
                name: label.word,
                position,
            },
            label.rest,
        ))
    }
}

pub fn iter_op_args(str: &str) -> impl Iterator<Item = &str> {
    str.split_whitespace()
}

#[derive(PartialEq, Debug, Clone)]
pub enum ArgType<'src> {
    AbsLabelRef(&'src str),
    OffLabelRef(&'src str),
    Number(i32),
    Register(u8),
}

impl<'src> ArgType<'src> {
    pub fn size_bytes(&self) -> usize {
        match self {
            ArgType::AbsLabelRef(_) | ArgType::OffLabelRef(_) | ArgType::Number(_) => {
                size_of::<u32>()
            }
            ArgType::Register(_) => size_of::<u8>(),
        }
    }
}
impl<'src> fmt::Display for ArgType<'src> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArgType::AbsLabelRef(label) => write!(f, "@{label}"),
            ArgType::OffLabelRef(label) => write!(f, ".{label}"),
            ArgType::Number(num) => write!(f, "{num}"),
            ArgType::Register(num) => write!(f, "{num}"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Elem<'src> {
    Op(Op<'src>),
    Const(ArgType<'src>),
    Label(LabelId),
}

#[cfg(test)]
mod tests {

    use super::*;
    macro_rules! assert_ops_eq {
        ($code: expr, $expected: expr) => {
            let mut parser = Parser::new();

            let elems = parser.parse_elems($code).unwrap();
            let ops: &[RawOp] = &parser.parse_ops(&elems).unwrap();
            let expected: &[RawOp] = $expected;
            assert_eq!(ops, expected)
        };
    }

    macro_rules! reg {
        ($num: expr) => {
            ArgType::Register($num)
        };
    }
    macro_rules! raw_reg {
        ($num: expr) => {
            RawArg::Register($num)
        };
    }

    macro_rules! raw_num {
        ($num: expr) => {
            RawArg::Num($num)
        };
    }

    macro_rules! op {
        ($opcode: ident, $arg: expr) => {
            Op {
                opcode: opcode::$opcode,
                arg: Some($arg),
            }
        };
        ($opcode: ident) => {
            Op {
                opcode: opcode::$opcode,
                arg: None,
            }
        };
    }

    macro_rules! raw_op {
        ($opcode: ident, $arg: expr) => {
            RawOp {
                opcode: opcode::$opcode,
                arg: Some($arg),
            }
        };
        ($opcode: ident) => {
            RawOp {
                opcode: opcode::$opcode,
                arg: None,
            }
        };
    }

    #[test]
    fn parse_number() {
        let s = Parser::new();

        assert_eq!(s.parse_i32("0xFA").unwrap(), 250);
        assert_eq!(s.parse_i32("0x-7D0").unwrap(), -2000);
        assert_eq!(s.parse_i32("500").unwrap(), 500);
        assert_eq!(s.parse_i32("+9876").unwrap(), 9876);
    }

    #[test]
    fn parse_single_op() {
        let s = Parser::new();
        assert_eq!(s.parse_op("nop;").unwrap().0, op!(Nop));
        assert_eq!(
            s.parse_op("local_get 5;").unwrap().0,
            op!(LocalGet, reg!(5))
        );
        assert_eq!(
            s.parse_op("local_set 0xA;").unwrap().0,
            op!(LocalSet, reg!(10))
        );
    }

    #[test]
    fn parse_multiple() {
        let code = "
            nop;


                    local_get 5; local_set 0xA;
            nop;
        ";
        assert_ops_eq!(
            code,
            &[
                raw_op!(Nop),
                raw_op!(LocalGet, raw_reg!(5)),
                raw_op!(LocalSet, raw_reg!(0xA)),
                raw_op!(Nop)
            ]
        );
    }

    /*
    #[test]
    fn parse_with_labels() {
        let code = "
            :blub:
            nop;
            nop;
            nop;

            :label:
            const @label;
            #@blub; #.label;
            #100;
        ";
        assert_ops_eq!(
            code,
            &[
                raw_op!(Nop),
                raw_op!(Nop),
                raw_op!(Nop),
                raw_op!(Const, raw_num!(3))
                Op::Const(ArgType::AbsLabelRef("blub")),
                Op::Const(ArgType::OffLabelRef("label")),
                Op::Const(ArgType::Number(100))
            ]
        );
    }
    */
    #[test]
    fn test_bytecode() {
        let code = "
            nop;
            nop;
            const 5;
            add;
        ";
        let mut parser = Parser::new();

        let elems = parser.parse_elems(code).unwrap();
        let ops = &parser.parse_ops(&elems).unwrap();

        let buffer = parser.as_bytecode(ops);

        assert_eq!(&buffer[0..4], &[b'm', b'a', b'l', b'u']);

        let size_bytes = u32::from_le_bytes(buffer[4..8].try_into().unwrap());
        let op_count = u32::from_le_bytes(buffer[8..12].try_into().unwrap());
        let expected_size = ops.iter().fold(0, |acc, op| acc + op.size_bytes() as u32);

        assert_eq!(op_count, 4);
        assert_eq!(size_bytes, expected_size);
    }
}
