const INITAL_VALUE_STACK_SIZE: usize = 65536 / 4;
const INITAL_RETURN_STACK_SIZE: usize = 20;
const MIN_HEAP_SIZE: usize = 65536;
const CODE_START_ADDR: u32 = 8;
#[derive(Debug, Clone)]
pub enum InterpreterErrorType {
    InvalidBytecodeHeader,
    AddrOutOfScope(u32),
    UnexpectedValStackEmpty,
    ReachedUnreachable,
    InvalidJumpAddr(u32),
    InvalidLocalId(u8),
}
pub struct Frame {
    locals: [u32; 64],
    return_addr: u32,
}
impl Frame {
    pub fn empty() -> Self {
        Self {
            locals: [0; 64],
            return_addr: CODE_START_ADDR,
        }
    }
}
pub struct Interpreter {
    value_stack: Vec<u32>,
    return_stack: Vec<Frame>,
    memory: Vec<u8>,
    pc: u32,
    globals: [u32; 64],
    running: bool,
}
pub fn is_bytecode_header_valid(bytecode: &[u8]) -> Result<(), InterpreterErrorType> {
    if bytecode[0..2].eq(&[b'm', b'a']) {
        Ok(())
    } else {
        Err(InterpreterErrorType::InvalidBytecodeHeader)
    }
}

macro_rules! interpreter_impl_read_op {
    ($name: ident, $t: tt) => {
        pub fn $name(&mut self, addr: u32) -> Result<$t, InterpreterErrorType> {
            Ok($t::from_le_bytes(
                self.memory
                    .get(addr as usize..addr as usize + size_of::<$t>())
                    .ok_or(InterpreterErrorType::AddrOutOfScope(addr))?
                    .try_into()
                    .unwrap(),
            ))
        }
    };
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
}
impl Interpreter {
    interpreter_impl_read_op!(read_u16, u16);
    interpreter_impl_read_op!(read_u32, u32);

    interpreter_impl_read_op!(read_i16, i16);
    interpreter_impl_read_op!(read_i32, i32);

    pub fn from_bytecode(bytecode: &[u8]) -> Result<Self, InterpreterErrorType> {
        is_bytecode_header_valid(bytecode)?;

        let mut interpreter = Interpreter {
            value_stack: Vec::with_capacity(INITAL_VALUE_STACK_SIZE),
            return_stack: Vec::with_capacity(INITAL_RETURN_STACK_SIZE),
            memory: Vec::with_capacity(bytecode.len() + MIN_HEAP_SIZE),
            pc: CODE_START_ADDR,
            globals: [0; 64],
            running: false,
        };
        interpreter.init_memory(bytecode);
        interpreter.return_stack.push(Frame::empty());

        Ok(interpreter)
    }

    pub fn init_memory(&mut self, bytecode: &[u8]) {
        self.memory.extend_from_slice(&bytecode[2..])
    }

    fn read_u8(&self, addr: u32) -> Result<u8, InterpreterErrorType> {
        self.memory
            .get(addr as usize)
            .ok_or(InterpreterErrorType::AddrOutOfScope(addr))
            .cloned()
    }

    fn push(&mut self, val: u32) {
        self.value_stack.push(val);
    }

    fn pop(&mut self) -> Result<u32, InterpreterErrorType> {
        self.value_stack
            .pop()
            .ok_or(InterpreterErrorType::UnexpectedValStackEmpty)
    }
    fn peek(&self) -> Result<u32, InterpreterErrorType> {
        self.value_stack
            .last()
            .ok_or(InterpreterErrorType::UnexpectedValStackEmpty)
            .copied()
    }

    fn pop_bool(&mut self) -> Result<bool, InterpreterErrorType> {
        match self.pop()? {
            0x00 => Ok(false),
            _ => Ok(true),
        }
    }

    pub fn exec_jmp(&mut self) -> Result<(), InterpreterErrorType> {
        let addr = self.pop()?;
        if addr >= self.memory.len() as u32 {
            Err(InterpreterErrorType::InvalidJumpAddr(addr))
        } else {
            self.pc = addr;
            Ok(())
        }
    }

    pub fn exec_branch(&mut self) -> Result<(), InterpreterErrorType> {
        let addr = self.pop()? + self.pc;
        if addr >= self.memory.len() as u32 {
            Err(InterpreterErrorType::InvalidJumpAddr(addr))
        } else {
            self.pc = addr;
            Ok(())
        }
    }

    pub fn current_frame(&self) -> &Frame {
        self.return_stack.last().unwrap()
    }
    pub fn current_frame_mut(&mut self) -> &mut Frame {
        self.return_stack.last_mut().unwrap()
    }

    pub fn read_local(&self, id_arg_offset: u32) -> Result<u32, InterpreterErrorType> {
        let id = self.pc + id_arg_offset;
        self.current_frame()
            .locals
            .get(id as usize)
            .ok_or(InterpreterErrorType::InvalidLocalId(id as u8))
            .copied()
    }

    pub fn set_local(
        &mut self,
        id_arg_offset: u32,
        value: u32,
    ) -> Result<u32, InterpreterErrorType> {
        let id = self.pc + id_arg_offset;
        *self
            .current_frame_mut()
            .locals
            .get_mut(id as usize)
            .ok_or(InterpreterErrorType::InvalidLocalId(id as u8))? = value;
        Ok(value)
    }

    pub fn exec_next_op(&mut self) -> Result<(), InterpreterErrorType> {
        match self.read_u8(self.pc)? {
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
                if self.pop_bool()? {
                    self.exec_jmp()?;
                } else {
                    self.pc += 1;
                }

                Ok(())
            }
            opcode::Branch => Ok(_ = self.exec_branch()),
            opcode::BranchIf => {
                if self.pop_bool()? {
                    self.exec_branch()?;
                } else {
                    self.pc += 1;
                }

                Ok(())
            }
            opcode::LocalGet => {
                self.push(self.read_local(1)?);
                self.pc += 2;
                Ok(())
            }
            opcode::LocalSet => {
                let val = self.pop()?;
                self.set_local(2, val)?;
                self.pc += 2;
                Ok(())
            }
            opcode::LocalTee => {
                let val = self.peek()?;
                self.set_local(2, val)?;
                self.pc += 2;
                Ok(())
            }
            opcode::Add => {
                let a = self.pop()?;
                let b = self.pop()?;
                self.push(a + b);
                self.pc += 1;
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

    #[test]
    fn hello_world() {
        let code = "
            #1; #1; add;
            end;
        ";
        let bytecode = asm::parse(code).unwrap().as_bytecode();
        assert!(bytecode.len() > 0);
        let result = Interpreter::from_bytecode(&bytecode)
            .unwrap()
            .run()
            .unwrap()[0];
        assert_eq!(result, 2);
    }
}
