use crate::sil;
use crate::error;
use dynasmrt::{
    DynasmApi, DynasmLabelApi, dynasm,
    x64::{self, Assembler},
};
use crate::codegen::regalloc;

pub struct Codegen<'a, Alloc: regalloc::Allocator> {
    allocator: &'a mut Alloc
}

impl<'a, Alloc: regalloc::Allocator> Codegen<'a, Alloc> {
    pub fn new(allocator: &'a mut Alloc) -> Self {
        Self {
            allocator,
        }
    }

    pub fn compile_function(
        &mut self,
        func: &sil::Function,
    ) -> Result<(dynasmrt::ExecutableBuffer, dynasmrt::AssemblyOffset), error::Error> {
        let mut asm = Assembler::new().unwrap();

        // First block is always the first to be compiled, so
        // it's safe to get the offset herw without checking
        let start = asm.offset();

        for block in &func.blocks {
            self.compile_block(&mut asm, block)?;
        }

        Ok((asm.finalize().unwrap(), start))
    }

    fn compile_block(&mut self, asm: &mut Assembler, block: &sil::Block) -> Result<(), error::Error> {
        for inst in &block.insts {
            self.compile_inst(&mut *asm, inst)?;
        }

        if let Some(term) = block.term {
            self.compile_term(&mut *asm, &term)?;
        }

        Ok(())
    }

    fn compile_inst(&mut self, asm: &mut Assembler, inst: &sil::inst::Instruction) -> Result<(), error::Error> {
        match inst.i_typ {
            sil::inst::InstructionType::ConstI32 { dst, value } => {
                dynasm!(asm
                    ; mov Rq(self.get_reg(dst)), QWORD value as i64
                );
            }

            sil::inst::InstructionType::Add { dst, lhs, rhs } => {
                dynasm!(asm
                    ; mov Rq(self.get_reg(dst)), Rq(self.get_reg(lhs))
                    ; add Rq(self.get_reg(dst)), Rq(self.get_reg(rhs))
                );
                self.free_reg(lhs)?;
                self.free_reg(rhs)?;
            }

            _ => {
                return Err(error::Error { 
                    pe_line: inst.i_ln, 
                    pe_col: inst.i_col, 
                    pe_type: error::ErrorType::CodegenError, 
                    pe_msg: format!("Invalid SIL instruction: `{}`", inst) 
                });
            }
        }

        Ok(())
    }

    fn compile_term(&mut self, asm: &mut Assembler, term: &sil::inst::Terminator) -> Result<(), error::Error> {
        match term {
            sil::inst::Terminator::Return { src } => {
                dynasm!(asm
                    ; mov rax, Rq(self.get_reg(*src))
                    ; ret
                );
                self.free_reg(*src)?;
            }
        }

        Ok(())
    }

    fn get_reg(&mut self, idx: u32) -> u8 {
        self.allocator.alloc(idx)
    }

    fn free_reg(&mut self, idx: u32) -> Result<(), error::Error> {
        self.allocator.free(idx)
    }
}
