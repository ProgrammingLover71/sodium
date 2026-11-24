use std::collections::HashMap;

use crate::codegen::regalloc;
use crate::error;
use crate::sil;
use dynasmrt::{dynasm, DynasmApi, x64::Assembler};

const SCRATCH1: u8 = 10; // r10
const SCRATCH2: u8 = 11; // r11
pub struct Codegen<'a, Alloc: regalloc::Allocator> {
    allocator: &'a mut Alloc,
    /// Number of remaining reads of each SIL value in this function.
    /// This lets us recycle a location only after its last use.
    uses_remaining: HashMap<u32, usize>,
}

impl<'a, Alloc: regalloc::Allocator> Codegen<'a, Alloc> {
    pub fn new(allocator: &'a mut Alloc) -> Self {
        Self {
            allocator,
            uses_remaining: HashMap::new(),
        }
    }

    pub fn compile_function(
        &mut self,
        func: &sil::Function,
    ) -> Result<(dynasmrt::ExecutableBuffer, dynasmrt::AssemblyOffset), error::Error> {
        let mut asm = Assembler::new().unwrap();
        self.uses_remaining = Self::count_uses(func);

        // This must point to the prologue: callers execute the function from
        // this offset, not from the first SIL instruction.
        let start = asm.offset();

        // Establish an RBP-based frame for stack slots. The allocator has 256
        // 8-byte slots, so reserve all 2048 bytes. Entry alignment is preserved:
        // push rbp aligns RSP, and subtracting 2048 keeps it aligned.
        dynasm!(asm
            ; push rbp
            ; mov rbp, rsp
            ; sub rsp, 2048
        );
        for block in &func.blocks {
            self.compile_block(&mut asm, block)?;
        }

        Ok((asm.finalize().unwrap(), start))
    }

    fn count_uses(func: &sil::Function) -> HashMap<u32, usize> {
        let mut uses = HashMap::new();
        let mut add_use = |value: u32| {
            *uses.entry(value).or_insert(0) += 1;
        };

        // This code generator currently emits ConstI32, Add and Return. Count
        // every input use so a source is not freed prematurely if used again.
        for block in &func.blocks {
            for inst in &block.insts {
                if let sil::inst::InstructionType::Add { lhs, rhs, .. } = inst.i_typ {
                    add_use(lhs);
                    add_use(rhs);
                }
            }
            if let Some(term) = block.term {
                match term {
                    sil::inst::Terminator::Return { src } => add_use(src),
                }
            }
        }

        uses
    }

    fn compile_block(
        &mut self,
        asm: &mut Assembler,
        block: &sil::Block,
    ) -> Result<(), error::Error> {
        for inst in &block.insts {
            self.compile_inst(asm, inst)?;
        }

        if let Some(term) = block.term {
            self.compile_term(asm, &term)?;
        }

        Ok(())
    }

    fn compile_inst(
        &mut self,
        asm: &mut Assembler,
        inst: &sil::inst::Instruction,
    ) -> Result<(), error::Error> {
        match inst.i_typ {
            sil::inst::InstructionType::ConstI32 { dst, value } => {
                // Do not load a destination before writing it: a stack slot may
                // be uninitialized. Materialize the constant, then store it if
                // its assigned location is memory.
                match self.alloc(dst) {
                    regalloc::AllocSlot::Register(dst_reg) => {
                        dynasm!(asm
                            ; mov Rq(dst_reg), QWORD value as i64
                        );
                    }
                    regalloc::AllocSlot::Stack(off) => {
                        dynasm!(asm
                            ; mov r10, QWORD value as i64
                            ; mov QWORD [rbp - (off as i32) * 8], r10
                        );
                    }
                }

                if self.remaining_uses(dst) == 0 {
                    self.free(dst)?;
                }
            }

            sil::inst::InstructionType::Add { dst, lhs, rhs } => {
                // Capture the destination location first. Inputs are already
                // assigned locations by their defining instructions.
                let dst_slot = self.alloc(dst);
                let lhs_reg = self.load(asm, lhs, SCRATCH1);
                let rhs_reg = self.load(asm, rhs, SCRATCH2);

                match dst_slot {
                    regalloc::AllocSlot::Register(dst_reg) => {
                        dynasm!(asm
                            ; mov Rq(dst_reg), Rq(lhs_reg)
                            ; add Rq(dst_reg), Rq(rhs_reg)
                        );
                    }
                    regalloc::AllocSlot::Stack(off) => {
                        // Both spilled inputs can coexist in r10/r11; calculate
                        // in r10 and write the result to the destination slot.
                        dynasm!(asm
                            ; mov r10, Rq(lhs_reg)
                            ; add r10, Rq(rhs_reg)
                            ; mov QWORD [rbp - (off as i32) * 8], r10
                        );
                    }
                }

                // A value may have more than one use. Recycle it only after the
                // final read, and don't release dst if it aliases an input ID.
                self.consume_use(lhs, Some(dst))?;
                self.consume_use(rhs, Some(dst))?;
                if self.remaining_uses(dst) == 0 {
                    self.free(dst)?;
                }
            }

            _ => {
                return Err(error::Error {
                    pe_line: inst.i_ln,
                    pe_col: inst.i_col,
                    pe_type: error::ErrorType::CodegenError,
                    pe_msg: format!("Invalid SIL instruction: `{}`", inst),
                });
            }
        }

        Ok(())
    }

    fn compile_term(
        &mut self,
        asm: &mut Assembler,
        term: &sil::inst::Terminator,
    ) -> Result<(), error::Error> {
        match term {
            sil::inst::Terminator::Return { src } => {
                let src_reg = self.load(asm, *src, SCRATCH1);

                // The epilogue restores the caller's RBP and stack pointer.
                dynasm!(asm
                    ; mov rax, Rq(src_reg)
                    ; mov rsp, rbp
                    ; pop rbp
                    ; ret
                );

                self.consume_use(*src, None)?;
            }
        }

        Ok(())
    }

    fn load(&mut self, asm: &mut Assembler, idx: u32, scratch: u8) -> u8 {
        match self.alloc(idx) {
            regalloc::AllocSlot::Register(reg) => reg,
            regalloc::AllocSlot::Stack(off) => {
                dynasm!(asm
                    ; mov Rq(scratch), QWORD [rbp - (off as i32) * 8]
                );
                scratch
            }
        }
    }

    fn alloc(&mut self, idx: u32) -> regalloc::AllocSlot {
        self.allocator.alloc(idx)
    }

    fn free(&mut self, idx: u32) -> Result<(), error::Error> {
        self.allocator.free(idx)
    }

    fn remaining_uses(&self, idx: u32) -> usize {
        self.uses_remaining.get(&idx).copied().unwrap_or(0)
    }

    fn consume_use(
        &mut self,
        idx: u32,
        keep_allocated: Option<u32>,
    ) -> Result<(), error::Error> {
        let is_last_use = {
            let remaining = self.uses_remaining.entry(idx).or_insert(0);
            if *remaining > 0 {
                *remaining -= 1;
            }
            *remaining == 0
        };

        // End the borrow of `uses_remaining` before borrowing `self` again
        // through `free()`.
        if is_last_use && keep_allocated != Some(idx) {
            self.free(idx)?;
        }
        Ok(())
    }
}
