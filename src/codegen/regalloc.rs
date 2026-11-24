use std::collections::HashMap;

use crate::error;

pub trait Allocator {
    fn new() -> Self;
    fn alloc(&mut self, reg: u32) -> AllocSlot;
    fn free(&mut self, reg: u32) -> Result<(), error::Error>;
}

/// The physical location assigned to an SIL value.
///
/// `Stack` is a one-based 8-byte slot index, not a byte offset. Codegen turns
/// slot `n` into `[rbp - n * 8]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocSlot {
    Register(u8),
    Stack(u32),
}

const STACK_SLOT_COUNT: u32 = 256;

pub struct DefaultAllocator {
    free_regs: Vec<u8>,
    free_slots: Vec<u32>,
    alloc_map: HashMap<u32, AllocSlot>,
}

impl Allocator for DefaultAllocator {
    fn new() -> Self {
        Self {
            free_regs: vec![15, 14, 13, 12, 9, 8, 7, 6, 3, 2, 1, 0],
            alloc_map: HashMap::new(),
            free_slots: (1..=STACK_SLOT_COUNT).rev().collect(),
        }
    }

    fn alloc(&mut self, local: u32) -> AllocSlot {
        if let Some(&slot) = self.alloc_map.get(&local) {
            return slot;
        }

        let slot = if let Some(reg) = self.free_regs.pop() {
            AllocSlot::Register(reg)
        } else if let Some(offset) = self.free_slots.pop() {
            AllocSlot::Stack(offset)
        } else {
            panic!("Out of locations while allocating SIL value {local}");
        };

        self.alloc_map.insert(local, slot);
        slot
    }

    fn free(&mut self, local: u32) -> Result<(), error::Error> {
        let slot = match self.alloc_map.get(&local).copied() {
            Some(slot) => slot,
            None => {
                return Err(error::Error {
                    pe_line: 0,
                    pe_col: 0,
                    pe_type: error::ErrorType::CodegenError,
                    pe_msg: format!("Unallocated register/value {local}"),
                });
            }
        };

        match slot {
            AllocSlot::Register(reg) => {
                if reg >= 16 {
                    return Err(error::Error {
                        pe_line: 0,
                        pe_col: 0,
                        pe_type: error::ErrorType::CodegenError,
                        pe_msg: format!("Invalid register ID {reg}"),
                    });
                }
                if self.free_regs.contains(&reg) {
                    return Err(error::Error {
                        pe_line: 0,
                        pe_col: 0,
                        pe_type: error::ErrorType::CodegenError,
                        pe_msg: format!("Register {reg} was double-freed"),
                    });
                }
                self.free_regs.push(reg);
            }
            AllocSlot::Stack(offset) => {
                if !(1..=STACK_SLOT_COUNT).contains(&offset) {
                    return Err(error::Error {
                        pe_line: 0,
                        pe_col: 0,
                        pe_type: error::ErrorType::CodegenError,
                        pe_msg: format!("Invalid stack slot {offset}"),
                    });
                }
                if self.free_slots.contains(&offset) {
                    return Err(error::Error {
                        pe_line: 0,
                        pe_col: 0,
                        pe_type: error::ErrorType::CodegenError,
                        pe_msg: format!("Stack slot [rbp-{}] was double-freed", offset * 8),
                    });
                }
                self.free_slots.push(offset);
            }
        }

        // Remove the mapping too. Without this, allocating the same SIL value
        // later would return a location that has already been handed out again.
        self.alloc_map.remove(&local);
        Ok(())
    }
}
