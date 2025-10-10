use smallvec::SmallVec;

use crate::{
    asm::{self, CODE_START_ADDR_POS},
    interpreter::opcode::StoreArgs,
};

const INITAL_VALUE_STACK_SIZE: usize = 65536 / 4;
const INITAL_RETURN_STACK_SIZE: usize = 20;
const MIN_HEAP_SIZE: usize = 65536;
const MAX_GLOBALS: usize = 64;
const MAX_LOCALS: usize = 64;
const MAX_ARGS: usize = 12;

#[derive(Debug, Clone)]
pub enum InterpreterErrorType {
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
pub struct Frame {
    locals: [u32; MAX_LOCALS],
    return_addr: u32,
}
impl Frame {
    pub fn empty() -> Self {
        Self {
            locals: [0; _],
            return_addr: CODE_START_ADDR_POS,
        }
    }
}
pub struct Interpreter {
    value_stack: Vec<u32>,
    return_stack: Vec<Frame>,
    memory: Vec<u8>,
    pc: u32,
    globals: [u32; MAX_GLOBALS],
    args: SmallVec<[u32; MAX_ARGS]>,
    running: bool,
    assertion_failed: bool,
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
impl Interpreter {
    interpreter_impl_read_op!(read_u16, u16);
    interpreter_impl_read_op!(read_u32, u32);

    interpreter_impl_read_op!(read_i16, i16);
    interpreter_impl_read_op!(read_i32, i32);

    interpreter_impl_store!(store_u16, u16);
    interpreter_impl_store!(store_u32, u32);

    interpreter_impl_store!(store_i16, i16);
    interpreter_impl_store!(store_i32, i32);

    pub fn from_bytecode(bytecode: &[u8]) -> Result<Self, InterpreterErrorType> {
        is_bytecode_header_valid(bytecode)?;

        let mut interpreter = Interpreter {
            value_stack: Vec::with_capacity(INITAL_VALUE_STACK_SIZE),
            return_stack: Vec::with_capacity(INITAL_RETURN_STACK_SIZE),
            memory: vec![0; bytecode.len() + MIN_HEAP_SIZE],
            pc: 0,
            globals: [0; _],
            running: false,
            args: SmallVec::new(),
            assertion_failed: false,
        };

        interpreter.init_memory(bytecode);
        interpreter.return_stack.push(Frame::empty());

        let start_code_addr = interpreter.read_u32(CODE_START_ADDR_POS)?;
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
        self.memory.clear();
        self.globals.fill(0); 
        self.running = false;
        self.args.clear();
        self.assertion_failed = false;
        
        self.init_memory(bytecode);
        self.return_stack.push(Frame::empty());

        let start_code_addr = self.read_u32(CODE_START_ADDR_POS)?;
        self.pc = start_code_addr;
        println!("code start addr: {}", self.pc);

        Ok(())
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

    pub fn exec_next_op(&mut self) -> Result<(), InterpreterErrorType> {
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
            opcode::Jmp => Ok(_ = self.exec_jmp()?),
            opcode::JmpIf => {
                let addr = self.pop()?;

                if self.pop_bool()? {
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
                do_binop!(self, a, b, a + b);
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

            opcode::Extend8_32s => {
                let d = self.pop()? as i8 as i32 as u32;
                self.push(d); //?
                self.pc += 1;
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
                println!("call");
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
                        println!("Assertion failed at: {:5x}", self.pc);
                        self.running = false;
                        self.assertion_failed = true;
                    }
                }
                Ok(())
            }
            _ => todo!(),
        }
    }

    pub fn run(&mut self) -> Result<&[u32], InterpreterErrorType> {
        self.running = true;
        loop {
            if !self.running {
                break;
            }
            self.exec_next_op()?;
        }
        Ok(&self.value_stack)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asm;

    macro_rules! assert_code_result {
        ($code: expr, $expected: expr) => {
            let bytecode = asm::Parser::parse($code).unwrap();
            assert!(bytecode.len() > 0);
            let mut interpreter = Interpreter::from_bytecode(&bytecode).unwrap();

            let result = interpreter.run().unwrap();
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
            #1; local_get 0; add; 
            local_tee 0; #5; ge;
            #@end; jmp_if;
            #@loop; jmp; 

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
