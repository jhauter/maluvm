use std::str::Utf8Error;

use smallvec::SmallVec;

use crate::asm::{self, opcode::{self, StoreArgs}, DATA_START, CODE_START_ADDR_POS};

const INITAL_VALUE_STACK_SIZE: usize = 65536 / 4;
const INITAL_RETURN_STACK_SIZE: usize = 20;
const MIN_HEAP_SIZE: usize = 65536;
const MAX_GLOBALS: usize = 64;
const MAX_LOCALS: usize = 64;
const MAX_ARGS: usize = 12;

#[derive(Debug)]
pub enum InterpreterErrorType {
    IOError(std::io::Error),
    InvalidStringData(Utf8Error),
    InvalidBytecodeHeader,
    AddrOutOfBounds(u32),
    UnexpectedValStackEmpty,
    ReachedUnreachable,
    InvalidJumpAddr(u32),
    InvalidLocalId(u8),
    InvalidGlobalId(u8),
    ArgStackFull,
    UnexpectedEmptyFrameStack,

}
impl From<std::io::Error>  for InterpreterErrorType {
    fn from(value: std::io::Error) -> Self {
        Self::IOError(value)
    }
}
impl From<Utf8Error> for InterpreterErrorType {
    fn from(value: Utf8Error) -> Self {
        Self::InvalidStringData(value)
    }
}
pub struct Frame {
    pub locals: [u32; MAX_LOCALS],
    pub return_addr: u32,
}
impl Frame {
    pub fn empty() -> Self {
        Self {
            locals: [0; _],
            return_addr: CODE_START_ADDR_POS,
        }
    }
}

pub trait SyscallHandler {
    fn on_syscall(&mut self, interpreter: &mut Interpreter, syscall_id: u32, args: &[u32]) -> u32;
}

pub struct Interpreter {
    pub value_stack: Vec<u32>,
    pub return_stack: Vec<Frame>,
    pub memory: Vec<u8>,
    pub pc: u32,
    pub globals: [u32; MAX_GLOBALS],
    pub args: SmallVec<[u32; MAX_ARGS]>,
    pub start_pc_addr: u32,
    pub bytecode_len: usize,
    pub running: bool,
    pub assertion_failed: bool,
}

macro_rules! interpreter_impl_read_op {
    ($name: ident, $t: tt) => {
        pub fn $name(&self, addr: u32) -> Result<$t, InterpreterErrorType> {
            Ok($t::from_le_bytes(
                self.memory
                    .get(addr as usize..addr as usize + size_of::<$t>())
                    .ok_or(InterpreterErrorType::AddrOutOfBounds(addr))?
                    .try_into()
                    .unwrap(),
            ))
        }
    };
}
macro_rules! interpreter_impl_store {
    ($name: ident, $t: tt) => {
        pub fn $name(&mut self, addr: u32, value: $t) -> Result<(), InterpreterErrorType> {
            self.memory
                .get_mut(addr as usize..addr as usize + size_of::<$t>())
                .ok_or(InterpreterErrorType::AddrOutOfBounds(addr))?
                .copy_from_slice(&$t::to_le_bytes(value));
            Ok(())
        }
    };
}
macro_rules! do_binop {
    ($self: ident, $a: ident, $b: ident, $op: expr) => {
        let $b = $self.pop()?;
        let $a = $self.pop()?;
        $self.push($op as u32);
        $self.pc += 1;
    };
}

pub fn is_bytecode_header_valid(bytecode: &[u8]) -> Result<(), InterpreterErrorType> {
    if bytecode[0..4].eq(&[b'm', b'a', b'l', b'u']) {
        Ok(())
    } else {
        Err(InterpreterErrorType::InvalidBytecodeHeader)
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self {
            value_stack: Default::default(),
            return_stack: Default::default(),
            memory: Default::default(),
            pc: Default::default(),
            globals: [0; _],
            args: Default::default(),
            running: Default::default(),
            assertion_failed: Default::default(),
            start_pc_addr: 0,
            bytecode_len: 0,
            
        }
    }
}
impl Interpreter {
    interpreter_impl_read_op!(read_u16, u16);
    interpreter_impl_read_op!(read_u32, u32);

    interpreter_impl_read_op!(read_i16, i16);
    interpreter_impl_read_op!(read_i32, i32);

    interpreter_impl_store!(store_u16, u16);
    interpreter_impl_store!(store_u32, u32);

    interpreter_impl_store!(store_i16, i16);
    interpreter_impl_store!(store_i32, i32);

    pub fn read_str(&mut self, addr: u32, len: u32) -> Result<&str, InterpreterErrorType> {
        //TODO: Kommuniziere dass addr + länge out of bounds ist
        let slice = self.memory.get(addr as usize .. addr as usize + len as usize)
            .ok_or(InterpreterErrorType::AddrOutOfBounds(addr))?;
            
        Ok(str::from_utf8(slice)?)
    } 

    pub fn from_bytecode(bytecode: &[u8]) -> Result<Self, InterpreterErrorType> {
        is_bytecode_header_valid(bytecode)?;

        let mut interpreter = Interpreter::default(); 
        interpreter.memory = vec![0; MIN_HEAP_SIZE + bytecode.len()];
        interpreter.init_memory(bytecode);
        interpreter.return_stack.push(Frame::empty());
        let start_code_addr = interpreter.read_u32(CODE_START_ADDR_POS)?;
        interpreter.start_pc_addr = start_code_addr;
        interpreter.bytecode_len = bytecode.len();

        interpreter.pc = start_code_addr;
        println!("code start addr: {}", interpreter.pc);

        Ok(interpreter)
    }

    pub fn init_memory(&mut self, bytecode: &[u8]) {
        self.memory[..bytecode.len() - 4].copy_from_slice(&bytecode[4..]);
    }

    pub fn reset_all(&mut self, bytecode: &[u8]) -> Result<(), InterpreterErrorType> {
        is_bytecode_header_valid(bytecode)?;

        self.value_stack.clear();
        self.return_stack.clear();
        self.memory.fill(0);
        self.globals.fill(0);
        self.running = false;
        self.args.clear();
        self.assertion_failed = false;
        
        self.init_memory(bytecode);
        self.return_stack.push(Frame::empty());

        let start_code_addr = self.read_u32(CODE_START_ADDR_POS)?;
        self.pc = start_code_addr;
        self.start_pc_addr = self.pc;
        self.bytecode_len = bytecode.len();

        println!("code start addr: {}", self.pc);

        Ok(())
    }

    pub fn inital_bytecode(&self) -> &[u8] {
        &self.memory[DATA_START as usize .. DATA_START as usize + self.bytecode_len as usize]
    }

    pub fn reset_pc(&mut self) {
        self.pc = self.start_pc_addr;
    }

    fn read_u8(&self, addr: u32) -> Result<u8, InterpreterErrorType> {
        self.memory
            .get(addr as usize)
            .ok_or(InterpreterErrorType::AddrOutOfBounds(addr))
            .cloned()
    }

    fn store_u8(&mut self, addr: u32, value: u8) -> Result<(), InterpreterErrorType> {
        *self
            .memory
            .get_mut(addr as usize)
            .ok_or(InterpreterErrorType::AddrOutOfBounds(addr))? = value;
        Ok(())
    }

    fn push(&mut self, val: u32) {
        println!("push {val}");
        self.value_stack.push(val);
    }

    fn pop(&mut self) -> Result<u32, InterpreterErrorType> {
        let val = self
            .value_stack
            .pop()
            .ok_or(InterpreterErrorType::UnexpectedValStackEmpty)?;
        println!("pop {val}");
        Ok(val)
    }

    fn pop_bool(&mut self) -> Result<bool, InterpreterErrorType> {
        match self.pop()? {
            0x00 => Ok(false),
            _ => Ok(true),
        }
    }

    fn peek(&self) -> Result<u32, InterpreterErrorType> {
        self.value_stack
            .last()
            .ok_or(InterpreterErrorType::UnexpectedValStackEmpty)
            .copied()
    }

    pub fn try_jump_to(&mut self, addr: u32) -> Result<(), InterpreterErrorType> {
        if addr >= self.memory.len() as u32 {
            Err(InterpreterErrorType::InvalidJumpAddr(addr))
        } else {
            self.pc = addr;
            Ok(())
        }
    }

    pub fn exec_jmp(&mut self) -> Result<(), InterpreterErrorType> {
        println!("jmp!");
        let addr = self.pop()?;
        self.try_jump_to(addr)
    }

    pub fn exec_branch(&mut self) -> Result<(), InterpreterErrorType> {
        let addr = self.pop()? + self.pc;
        self.try_jump_to(addr)
    }

    pub fn current_frame(&self) -> &Frame {
        self.return_stack.last().unwrap()
    }
    pub fn current_frame_mut(&mut self) -> &mut Frame {
        self.return_stack.last_mut().unwrap()
    }

    pub fn read_imm_u8(&self, offset: u32) -> Result<u8, InterpreterErrorType> {
        let addr = self.pc + offset;
        self.read_u8(addr)
    }

    pub fn read_imm_u32(&self, offset: u32) -> Result<u32, InterpreterErrorType> {
        let addr = self.pc + offset;
        self.read_u32(addr)
    }

    pub fn read_store_args(&mut self) -> Result<StoreArgs, InterpreterErrorType> {
        let offset = self.read_imm_u32(1)?;
        let value = self.pop()?;
        let addr = self.pop()? + offset;

        Ok(StoreArgs { addr, value })
    }

    pub fn read_local(&self, id_arg_offset: u32) -> Result<u32, InterpreterErrorType> {
        let id = self.read_imm_u8(id_arg_offset)?;

        self.current_frame()
            .locals
            .get(id as usize)
            .ok_or(InterpreterErrorType::InvalidLocalId(id as u8))
            .copied()
    }

    fn read_global(&self, id_arg_offset: u32) -> Result<u32, InterpreterErrorType> {
        let id = self.read_imm_u8(id_arg_offset)?;
        self.globals
            .get(id as usize)
            .ok_or(InterpreterErrorType::InvalidLocalId(id as u8))
            .copied()
    }

    fn set_local(&mut self, id_arg_offset: u32, value: u32) -> Result<u32, InterpreterErrorType> {
        let id = self.read_imm_u8(id_arg_offset)?;
        *self
            .current_frame_mut()
            .locals
            .get_mut(id as usize)
            .ok_or(InterpreterErrorType::InvalidLocalId(id as u8))? = value;
        Ok(value)
    }

    fn set_global(&mut self, id_arg_offset: u32, value: u32) -> Result<u32, InterpreterErrorType> {
        let id = self.read_imm_u8(id_arg_offset)?;
        *self
            .globals
            .get_mut(id as usize)
            .ok_or(InterpreterErrorType::InvalidGlobalId(id as u8))? = value;
        Ok(value)
    }

    pub fn create_frame(&mut self) {
        self.return_stack.push(Frame::empty());
        let frame = self.return_stack.last_mut().unwrap();
        //TODO: (joh): Check here if pc + 1 might be out of bounds?
        frame.return_addr = self.pc + 1;

        frame.locals[..self.args.len()].copy_from_slice(&self.args);
    }

    pub fn exec_next_op(&mut self, syscall_handler: &mut impl SyscallHandler) -> Result<(), InterpreterErrorType> {
        let op = self.read_u8(self.pc)?;
        println!("op: {:0x}", op);
        match op {
            opcode::Nop => Ok(self.pc += 1),
            opcode::End => {
                self.running = false;
                Ok(())
            }
            opcode::Unreachable => Err(InterpreterErrorType::ReachedUnreachable),
            opcode::Drop => {
                _ = self.pop()?;
                self.pc += 1;
                Ok(())
            }
            opcode::Const => {
                let arg = self.read_i32(self.pc + 1)?;
                self.push(arg as u32);
                self.pc += 1_u32 + size_of::<i32>() as u32;
                Ok(())
            }
            opcode::Jmp => {
                println!("jmp");
                Ok(_ = self.exec_jmp()?)
            }
            opcode::JmpIf => {
                println!("jmp if");
                let addr = self.pop()?;

                if self.pop_bool()? {
                    println!("addr: {}", addr);
                    self.try_jump_to(addr)?;
                } else {
                    self.pc += 1;
                }

                Ok(())
            }
            opcode::Branch => Ok(_ = self.exec_branch()),
            opcode::BranchIf => {
                let addr = self.pop()? + self.pc;
                if self.pop_bool()? {
                    self.try_jump_to(addr)?;
                } else {
                    self.pc += 1;
                }
                Ok(())
            }

            opcode::LocalGet => {
                println!("local get");
                self.push(self.read_local(1)?);
                self.pc += 2;
                Ok(())
            }
            opcode::LocalSet => {
                println!("local set");
                let val = self.pop()?;
                self.set_local(1, val)?;
                self.pc += 2;
                Ok(())
            }
            opcode::LocalTee => {
                println!("local tee");
                let val = self.peek()?;
                self.set_local(1, val)?;
                self.pc += 2;
                Ok(())
            }
            opcode::GlobalGet => {
                println!("global get");
                let global = self.read_global(1)?;
                println!("globals {:?}", self.globals);
                self.push(global);
                self.pc += 2;
                Ok(())
            }
            opcode::GlobalSet => {
                println!("global set");
                let val = self.pop()?;
                _ = self.set_global(1, val)?;
                self.pc += 2;
                Ok(())
            }
            opcode::GlobalTee => {
                println!("global tee");
                let val = self.peek()?;
                self.set_global(1, val)?;
                self.pc += 2;
                Ok(())
            }
            opcode::Add => {
                println!("add");
                do_binop!(self, a, b, a.wrapping_add(b));
                Ok(())
            }
            opcode::Sub => {
                do_binop!(self, a, b, a.wrapping_sub(b));
                Ok(())
            }
            opcode::Mul => {
                do_binop!(self, a, b, a * b);
                Ok(())
            }
            opcode::Divu => {
                do_binop!(self, a, b, a / b);
                Ok(())
            }
            opcode::Divs => {
                do_binop!(self, a, b, a as i32 / b as i32);
                Ok(())
            }
            opcode::Lt => {
                do_binop!(self, a, b, a < b);
                Ok(())
            }
            opcode::Gt => {
                do_binop!(self, a, b, a > b);
                Ok(())
            }
            opcode::Ge => {
                do_binop!(self, a, b, a >= b);
                Ok(())
            }
            opcode::Le => {
                do_binop!(self, a, b, a <= b);
                Ok(())
            }
            opcode::And => {
                do_binop!(self, a, b, a & b);
                Ok(())
            }
            opcode::Or => {
                do_binop!(self, a, b, a | b);
                Ok(())
            }
            opcode::Xor => {
                do_binop!(self, a, b, a ^ b);
                Ok(())
            }

            opcode::Shiftl => {
                do_binop!(self, a, b, a.wrapping_shl(b));
                Ok(())
            }
            opcode::Shiftr => {
                do_binop!(self, a, b, a >> b);
                Ok(())
            }

            opcode::Store8 => {
                let args = self.read_store_args()?;
                self.store_u8(args.addr, args.value as u8)?;
                self.pc += 5;
                Ok(())
            }

            opcode::Store16 => {
                let args = self.read_store_args()?;
                self.store_u16(args.addr, args.value as u16)?;
                self.pc += 5;
                Ok(())
            }

            opcode::Store32 => {
                let args = self.read_store_args()?;
                self.store_u32(args.addr, args.value)?;
                self.pc += 5;
                Ok(())
            }

            opcode::Load8u => {
                let offset = self.read_imm_u32(1)?;
                let addr = offset + self.pop()?;

                self.push(self.read_u8(addr)? as u32);
                self.pc += 5;
                Ok(())
            }
            opcode::Load16u => {
                let offset = self.read_imm_u32(1)?;
                let addr = offset + self.pop()?;
                self.push(self.read_u16(addr)? as u32);
                self.pc += 5;
                Ok(())
            }

            opcode::Load32u => {
                let offset = self.read_imm_u32(1)?;
                let addr = offset + self.pop()?;
                self.push(self.read_u32(addr)?);
                self.pc += 5;
                Ok(())
            }

            opcode::PushArg => {
                if self.args.len() >= MAX_ARGS {
                    Err(InterpreterErrorType::ArgStackFull)
                } else {
                    let arg = self.pop()?;
                    self.args.push(arg);
                    self.pc += 1;
                    Ok(())
                }
            }
            opcode::Call => {
                let addr = self.pop()?;
                if addr >= self.memory.len() as u32 {
                    Err(InterpreterErrorType::InvalidJumpAddr(addr))
                } else {
                    self.create_frame();
                    self.pc = addr;
                    self.args.clear();

                    Ok(())
                }
            }

            opcode::Return => {
                let last_frame = self
                    .return_stack
                    .pop()
                    .ok_or(InterpreterErrorType::UnexpectedEmptyFrameStack)?;
                match last_frame.return_addr {
                    0 => {
                        self.running = false;
                        Ok(())
                    }
                    addr => {
                        self.pc = addr;
                        Ok(())
                    }
                }
            }
            opcode::DbgAssert => {
                let cond = self.pop_bool()?;
                match cond {
                    true => self.pc += 1,
                    false => {
                        println!("Assertion failed at: 0x{:4x}", self.pc);
                        self.running = false;
                        self.assertion_failed = true;
                    }
                }
                Ok(())
            }
            opcode::Syscall => {
                let id = self.pop()?;
                let args = self.args.clone(); 
                let ret = syscall_handler.on_syscall(self, id, args.as_slice());       
                self.args.clear(); 

                self.push(ret);
                Ok(())
            }
            _ => todo!(),
        }
    }

    pub fn run(&mut self, syscall_handler: &mut impl SyscallHandler) -> Result<&[u32], InterpreterErrorType> {
        self.running = true;
        loop {
            if !self.running {
                break;
            }
            self.exec_next_op(syscall_handler)?;
        }
        Ok(&self.value_stack)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asm;

    struct DummySyscallHandler();
    impl SyscallHandler for DummySyscallHandler {
        fn on_syscall(&mut self, _: &mut Interpreter, _: u32, _: &[u32]) -> u32 {
            return 0
        }
    }
    macro_rules! assert_code_result {
        ($code: expr, $expected: expr) => {
            let bytecode = asm::Parser::parse($code).unwrap();
            assert!(bytecode.code.len() > 0);
            let mut interpreter = Interpreter::from_bytecode(&bytecode.code).unwrap();

            let result = interpreter.run(&mut DummySyscallHandler()).unwrap();
            assert_eq!(result, $expected);
        };
    }

    #[test]
    fn hello_world_add_numbers() {
        let code = "
            #1; #1; add;
            end;
        ";
        assert_code_result!(code, &[2]);
    }

    #[test]
    fn globals_locals() {
        let code = "
            #100; global_set 0;
            #2; global_set 1; 

            global_get 0;
            global_get 1;
            add;
            
            #5; local_set 0;
            #2; local_set 1;

            local_get 0;
            local_get 1;

            add;
            end;
        ";
        assert_code_result!(code, &[102, 7]);
    }
    #[test]
    fn load_store() {
        let code = "
            #100; 
            #5;
            store_32 0;

            #100;
            load_32_u 0;
            
            end;    
        ";
        assert_code_result!(code, &[5]);
    }
    #[test]
    fn call_function_with_params() {
        let code = "
            :func1: 
            local_get 0;
            local_get 1;
            add;
            return; 
             
            :__ENTRY__:
            #1; push_arg;
            #2; push_arg;
            #@func1;
            call; 
            end;
             
        ";
        assert_code_result!(code, &[3]);
    }

    #[test]
    fn simple_if_else() {
        let code = "
            #1; #2; gt;
            #@if; jmp_if; 
            :else: #0x25; end;
            :if: unreachable;
        ";

        assert_code_result!(code, &[0x25]);
    }

    #[test]
    fn simple_loop() {
        let code = "
            :loop:
            #1; 
            local_get 0; 
            add; 
            local_tee 0; 
            #5; 
            ge;
            #@end; 
            jmp_if;
            #@loop; 
            jmp; 
            :end:
            local_get 0;
            end;
        ";
        assert_code_result!(code, &[5]);
    }

    #[test]
    fn recursion() {
        let code = "
            #0; push_arg;
            #@fn; call;
            end;
            :fn:
            #1; local_get 0; add;
            local_tee 0; #5; lt;
            #@fn_rec; jmp_if;

            local_get 0;
            return; 

            :fn_rec:
            local_get 0; push_arg;
            #@fn; call;
            return;
        ";
        assert_code_result!(code, &[5]);
    }

    #[test]
    fn assertions() {
        let code = "
            #1; #2; add; #1; gt;
            dbg_assert;

            #10;
            #5; #2; lt;
            dbg_assert; 
            unreachable;
        ";
        assert_code_result!(code, &[10]);
    }
}
