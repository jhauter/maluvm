use crate::{interpreter::InterpreterErrorType, op::Op};
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

#[derive(Debug, Clone)]
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
        pub fn $fn_name(&self, str: &'src str) -> Result<$type, AssembleError> {
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

pub struct Parser<'src> {
    op_count: usize,
    op_size_bytes: usize,
    line: usize,
    labels: HashMap<&'src str, usize>,
}

impl<'src, 'bump> Parser<'src> {
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
    
    pub fn parse(code: &'src str) -> Result<Box<[u8]>, AssembleError> {
        let mut parser = Self::new(); 

        let elems = parser.parse_elems(code)?;
        let ops = parser.parse_ops(&elems)?;
        
        parser.as_bytecode(&ops)
    }
    pub fn parse_elems(&mut self, code: &'src str) -> Result<Box<[Elem<'src>]>, AssembleError> {
        let mut rest = Some(code);
        let mut elems = Vec::new();


        loop {
            match rest {
                Some(r) => match r.chars().next() {
                    Some('\n') | Some('\r') | Some(' ') => rest = self.skip_whitespace(r),

                    Some('#') => {
                        let statement = self.slice_until(&r[1..], ';')?;
                        let arg = self.parse_arg(statement.word)?;

                        let op = Op::Const(arg);
                        self.op_size_bytes += op.size_bytes() as usize;
                        elems.push(Elem::Op(op));
                        self.op_count += 1;
                        rest = statement.rest;
                    }

                    Some(':') => {
                        let (label, label_rest) = self.parse_label(&r[1..])?;

                        println!("label: {}", label.name);
                        self.labels.insert(label.name, label.position);
                        elems.push(Elem::Label(label));
                        rest = label_rest;
                    }

                    Some(_) => {
                        let op_res = self.parse_op(r)?;
                        let op = op_res.0;
                        self.op_size_bytes += op.size_bytes() as usize;
                        elems.push(Elem::Op(op));
                        self.op_count += 1;

                        rest = op_res.1;
                    }
                    None => break,
                },
                None => break,
            }
        }
        Ok(elems.into())
    }

    pub fn parse_ops(
        &mut self,
        elems: &[Elem<'src>],
    ) -> Result<Box<[Op<'src>]>, AssembleError> {
        let mut ops = Vec::with_capacity(self.op_count);

        for elem in elems {
            match elem {
                Elem::Op(op) => ops.push(op.clone()),
                Elem::Label(_) => {}
            }
        }

        Ok(ops.into_boxed_slice())
    }

    pub fn encode_op(&self, op: &'src Op<'src>, dest: &mut Vec<u8>) -> Result<(), AssembleError> {
        dest.push(op.repr());
        match op {
            Op::LocalGet(num)
            | Op::LocalSet(num)
            | Op::LocalTee(num)
            | Op::GlobalGet(num)
            | Op::GlobalSet(num)
            | Op::GlobalTee(num) => dest.push(*num),

            Op::Store8(arg)
            | Op::Store16(arg)
            | Op::Store32(arg)
            | Op::Load8u(arg)
            | Op::Load8s(arg)
            | Op::Load16s(arg)
            | Op::Load16u(arg)
            | Op::Load32s(arg)
            | Op::Load32u(arg)
            | Op::Const(arg) => {
                let num = arg.get_numeric(self)?;
                dest.extend_from_slice(&num.to_le_bytes());
            }
            _ => {}
        }

        Ok(())
    }

    pub fn try_get_label(&self, name: &'src str) -> Result<usize, AssembleError> {
        self.labels
            .get(name)
            .ok_or(AssembleError::new(
                self,
                AssembleErrorKind::UnknownLabel(name.to_string()),
            ))
            .copied()
    }
    pub fn get_bytecode_info(&self) -> BytecodeInfo {
        let code_size_bytes = self.op_size_bytes as u32;
        let instruction_count = self.op_count as u32;
        println!("header size {}", BytecodeInfo::total_header_size());
        let code_start_offset = self.labels.get(ENTRY_LABEL_NAME).copied().unwrap_or(0) as u32 + CODE_START;
        println!("code start offset: {}", code_start_offset);
        BytecodeInfo {
            code_size_bytes,
            instruction_count,
            code_start_offset,
        }
    }

    pub fn as_bytecode(&self, ops: &'src [Op<'src>]) -> Result<Box<[u8]>, AssembleError> {
        let info = self.get_bytecode_info();
        let mut buffer = info.to_bytecode();

        ops.iter()
            .try_for_each(|o| self.encode_op(o, &mut buffer))?;

        Ok(buffer.into_boxed_slice())
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
    ) -> Result<u8, AssembleError> {
        let s = args
            .next()
            .ok_or(AssembleError::new(self, AssembleErrorKind::MissingArgument))?;
        match self.parse_arg(s)? {
            ArgType::Number(num) => match num {
                0..255 => Ok(num as u8),
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
    pub fn parse_op(
        &self,
        s: &'src str,
    ) -> Result<(Op<'src>, Option<&'src str>), AssembleError> {
        let statement = self.slice_until(s, ';')?;
        let mut op_str = iter_op_args(statement.word);
        let op_name = op_str.next().unwrap();
        println!("name: {}", op_name);
        let op = match op_name {
            "nop" => Ok(Op::Nop),
            "unreachable" => Ok(Op::Unreachable),
            "drop" => Ok(Op::Drop),
            "const" => Ok(Op::Const(self.arg_const(&mut op_str)?)),
            "jmp" => Ok(Op::Jmp),
            "jmp_if" => Ok(Op::JmpIf),
            "branch" => Ok(Op::Branch),
            "branch_if" => Ok(Op::BranchIf),
            "local_get" => Ok(Op::LocalGet(self.arg_register(&mut op_str)?)),
            "local_set" => Ok(Op::LocalSet(self.arg_register(&mut op_str)?)),
            "local_tee" => Ok(Op::LocalTee(self.arg_register(&mut op_str)?)),
            "global_get" => Ok(Op::GlobalGet(self.arg_register(&mut op_str)?)),
            "global_set" => Ok(Op::GlobalSet(self.arg_register(&mut op_str)?)),
            "global_tee" => Ok(Op::GlobalTee(self.arg_register(&mut op_str)?)),
            "eq" => Ok(Op::Eq),
            "eqz" => Ok(Op::Eqz),
            "add" => Ok(Op::Add),
            "sub" => Ok(Op::Sub),
            "div_s" => Ok(Op::Divs),
            "div_u" => Ok(Op::Divu),
            "mul" => Ok(Op::Mul),
            "neg" => Ok(Op::Neg),
            "gt" => Ok(Op::Gt),
            "lt" => Ok(Op::Lt),
            "ge" => Ok(Op::Ge),
            "le" => Ok(Op::Le),
            "shiftr" => Ok(Op::Shiftr),
            "shiftl" => Ok(Op::Shiftl),
            "call" => Ok(Op::Call),
            "return" => Ok(Op::Return),
            "store_8" => Ok(Op::Store8(self.arg_const(&mut op_str)?)),
            "store_16" => Ok(Op::Store16(self.arg_const(&mut op_str)?)),
            "store_32" => Ok(Op::Store32(self.arg_const(&mut op_str)?)),
            "load_8_u" => Ok(Op::Load8u(self.arg_const(&mut op_str)?)),
            "load_8_s" => Ok(Op::Load8s(self.arg_const(&mut op_str)?)),
            "load_16_s" => Ok(Op::Load16s(self.arg_const(&mut op_str)?)),
            "load_16_u" => Ok(Op::Load16u(self.arg_const(&mut op_str)?)),
            "load_32_s" => Ok(Op::Load32s(self.arg_const(&mut op_str)?)),
            "load_32_u" => Ok(Op::Load32u(self.arg_const(&mut op_str)?)),
            "extend_8_32_s" => Ok(Op::Extend8_32s),
            "extend_16_32_s" => Ok(Op::Extend16_32s),
            "extend_8_32_u" => Ok(Op::Extend8_32u),
            "extend_16_32_u" => Ok(Op::Extend16_32u),
            "end" => Ok(Op::End),
            "push_arg" => Ok(Op::PushArg),
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
            None => Ok((op, statement.rest)),
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

#[derive(Eq, PartialEq, Debug, Clone)]
pub enum ArgType<'src> {
    AbsLabelRef(&'src str),
    OffLabelRef(&'src str),
    Number(i32),
}

impl<'src> ArgType<'src> {
    pub fn get_numeric(&self, state: &Parser<'src>) -> Result<i32, AssembleError> {
        match self {
            ArgType::AbsLabelRef(label) => {
                let label = state.try_get_label(label)?;
                Ok(label as i32 + CODE_START as i32)
            }
            ArgType::OffLabelRef(label) => {
                let label = state.try_get_label(label)?;
                Ok(((state.op_size_bytes as isize) - (label as isize)) as i32)
            }

            ArgType::Number(num) => Ok(*num),
        }
    }
}
impl<'src> fmt::Display for ArgType<'src> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArgType::AbsLabelRef(label) => write!(f, "@{label}"),
            ArgType::OffLabelRef(label) => write!(f, ".{label}"),
            ArgType::Number(num) => write!(f, "{num}"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Elem<'src> {
    Op(Op<'src>),
    Label(Label<'src>),
}

#[cfg(test)]
mod tests {

    use super::*;
    macro_rules! assert_ops_eq {
         ($code: expr, $expected: expr) => {
        let mut parser = Parser::new(); 

        let elems = parser.parse_elems($code).unwrap();
        let ops: &[Op<'_>] = &parser.parse_ops(&elems).unwrap();
        let expected: &[Op<'_>]=  $expected; 

        assert_eq!(&ops, &expected)     
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
        assert_eq!(s.parse_op("nop;").unwrap().0, Op::Nop);
        assert_eq!(s.parse_op("local_get 5;").unwrap().0, Op::LocalGet(5));
        assert_eq!(s.parse_op("local_set 0xA;").unwrap().0, Op::LocalSet(10));
    }

    #[test]
    fn parse_multiple() {
        let code = "
            nop;


                    local_get 5; local_set 0xA;
            nop;
        ";
        assert_ops_eq!(code, &[Op::Nop, Op::LocalGet(5), Op::LocalSet(0xA), Op::Nop]);
    }

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
        assert_ops_eq!(code, &[
            Op::Nop, 
            Op::Nop,  
            Op::Nop, 
            Op::Const(ArgType::AbsLabelRef("label")), 
            Op::Const(ArgType::AbsLabelRef("blub")), 
            Op::Const(ArgType::OffLabelRef("label")),
            Op::Const(ArgType::Number(100))]);
    }

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
        let ops: &[Op<'_>] = &parser.parse_ops(&elems).unwrap();

        let buffer = parser.as_bytecode(ops).unwrap();

        assert_eq!(&buffer[0..4], &[b'm', b'a', b'l', b'u']);

        let size_bytes = u32::from_le_bytes(buffer[4..8].try_into().unwrap());
        let op_count = u32::from_le_bytes(buffer[8..12].try_into().unwrap());
        let expected_size = ops
            .iter()
            .fold(0, |acc, op| acc + op.size_bytes() as u32);

        assert_eq!(op_count, 4);
        assert_eq!(size_bytes, expected_size);
    }
}
