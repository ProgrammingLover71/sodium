use std::collections::HashMap;

use crate::error::{self, Error};

pub trait Allocator {
    fn new() -> Self;
    fn alloc(&mut self, reg: u32) -> u8;
    fn free(&mut self, reg: u32) -> Result<(), error::Error>;
}


pub struct DefaultAllocator {
    free_regs: Vec<u8>,
    reg_map: HashMap<u32, u8>
}

impl Allocator for DefaultAllocator {
    fn new() -> Self {
        Self {
            free_regs: vec![15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 3, 2, 1, 0],
            reg_map: HashMap::new()
        }
    }

    fn alloc(&mut self, local: u32) -> u8 {
        if self.reg_map.contains_key(&local) {
            return *self.reg_map.get(&local).unwrap();
        }

        let reg_opt = self.free_regs.pop();
        if let Some(reg) = reg_opt {
            self.reg_map.insert(local, reg);
            reg
        } else {
            255u8   // 255 = stack spill
        }
    }

    fn free(&mut self, local: u32) -> Result<(), error::Error> {
        if !self.reg_map.contains_key(&local) {
            return Err(Error {
                pe_line: 0,
                pe_col: 0,
                pe_type: error::ErrorType::CodegenError,
                pe_msg: format!("Unallocated register {}", local)
            });
        }

        let reg = self.reg_map.get(&local).unwrap();

        if self.free_regs.contains(reg) {
            return Err(Error {
                pe_line: 0,
                pe_col: 0,
                pe_type: error::ErrorType::CodegenError,
                pe_msg: format!("Register {} was double-freed", reg)
            });
        }

        if *reg >= 16 {
            return Err(Error {
                pe_line: 0,
                pe_col: 0,
                pe_type: error::ErrorType::CodegenError,
                pe_msg: format!("Invalid register ID {}", reg)
            });
        }

        self.free_regs.push(*reg);
        Ok(())
    }
}