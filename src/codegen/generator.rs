use crate::sil;
use dynasmrt::{
    DynasmApi, DynasmLabelApi, dynasm,
    x64::{self, Assembler},
};

pub struct Codegen;

impl Codegen {
    pub fn new() -> Self {
        Self
    }

    pub fn compile_function(
        &self,
        func: &sil::Function,
    ) -> (dynasmrt::ExecutableBuffer, dynasmrt::AssemblyOffset) {
        let mut asm = Assembler::new().unwrap();

        // First block is always the first to be compiled, so
        // it's safe to get the offset herw without checking
        let start = asm.offset();

        for block in &func.blocks {
            self.compile_block(&mut asm, block);
        }

        (asm.finalize().unwrap(), start)
    }

    fn compile_block(&self, asm: &mut Assembler, block: &sil::Block) {
        for inst in &block.insts {
            self.compile_inst(&mut *asm, inst);
        }
        if let Some(term) = block.term {
            self.compile_term(&mut *asm, &term);
        }
    }

    fn compile_inst(&self, asm: &mut Assembler, inst: &sil::inst::Instruction) {
        match inst.i_typ {
            sil::inst::InstructionType::ConstI32 { dst, value } => {
                dynasm!(asm
                    ; mov Rd(self.get_reg(dst)), DWORD value
                )
            }

            sil::inst::InstructionType::Add { dst, lhs, rhs } => {
                dynasm!(asm
                    ; mov Rd(self.get_reg(dst)), Rd(self.get_reg(lhs))
                    ; add Rd(self.get_reg(dst)), Rd(self.get_reg(rhs))
                )
            }

            _ => unimplemented!(),
        }
    }

    fn compile_term(&self, asm: &mut Assembler, term: &sil::inst::Terminator) {
        match term {
            sil::inst::Terminator::Return { src } => {
                dynasm!(asm
                    ; mov eax, Rd(self.get_reg(*src))
                    ; ret
                )
            }
        }
    }

    // TODO: Fix this absolute MONSTROSITY of a register "allocator"
    //       I'll implement it in regalloc.rs
    fn get_reg(&self, idx: u32) -> u8 {
        match idx {
            0..=3 => idx as u8,
            4..=5 => 6,
            6..=7 => idx as u8,
            _ => panic!("Invalid register id: {}", idx)
        }
    }
}
