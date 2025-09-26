use crate::op::Op;

pub enum Statement {
    Op(Op),
} 

pub struct State {
    res: Vec<u8>,
}

#[derive(Debug, Clone)]
pub enum AssembleErrorKind {

}

#[derive(Debug, Clone)]
pub struct AssembleError {
    kind: AssembleErrorKind,
    line: usize,
     
}
pub fn assemble(asm: impl AsRef<str>) -> Result<Box<[u8]>, AssembleError> {

} 
