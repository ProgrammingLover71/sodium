use std::collections::HashMap;

use crate::error;
use crate::sil::block;
use crate::sil::func;
use crate::sil::inst;
use crate::sil::types;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OptId {
    None,
    Id(usize),
}

impl OptId {
    pub fn unwrap(&self) -> usize {
        match self {
            Self::None => panic!("Cannot unwrap OptId::None"),
            Self::Id(i) => *i,
        }
    }

    pub fn to_result(&self) -> Result<usize, error::Error> {
        match self {
            Self::None => Err(error::Error {
                pe_line: 0,
                pe_col: 0,
                pe_type: error::ErrorType::InvalidILError,
                pe_msg: String::from("Invalid SIL ID"),
            }),

            Self::Id(id) => Ok(*id),
        }
    }
}

pub struct ILBuilder<'a> {
    functions: Vec<func::Function<'a>>,
    current_function_idx: OptId,
    current_var_id: u32,

    allow_term_override: bool,
}

impl<'a> ILBuilder<'a> {
    pub fn new(allow_term_override: bool) -> Self {
        Self {
            functions: Vec::new(),
            current_function_idx: OptId::None,
            current_var_id: 0,
            allow_term_override,
        }
    }

    pub fn define_function(
        &mut self,
        name: String,
        params: Vec<types::Type<'a>>,
        ret: types::Type<'a>,
    ) -> OptId {
        let func = func::Function {
            name,
            params,
            returns: ret,
            blocks: Vec::new(),
            type_map: HashMap::new(),
        };
        self.functions.push(func);
        OptId::Id(self.functions.len() - 1)
    }

    pub fn set_current_function(&mut self, function_id: OptId) {
        self.current_function_idx = function_id;
    }

    pub fn get_function(&self, function_id: OptId) -> Result<&func::Function<'a>, error::Error> {
        let id = function_id.to_result()?;

        self.functions.get(id).ok_or(error::Error {
            pe_line: 0,
            pe_col: 0,
            pe_type: error::ErrorType::InvalidILError,
            pe_msg: format!("Undefined function with ID {}", id),
        })
    }

    pub fn current_function(&self) -> Result<&func::Function<'a>, error::Error> {
        let id = self.current_function_idx.to_result()?;

        self.functions.get(id).ok_or(error::Error {
            pe_line: 0,
            pe_col: 0,
            pe_type: error::ErrorType::InvalidILError,
            pe_msg: format!("Undefined function with ID {}", id),
        })
    }

    pub fn current_function_mut(&mut self) -> Result<&mut func::Function<'a>, error::Error> {
        let id = self.current_function_idx.to_result()?;

        self.functions.get_mut(id).ok_or(error::Error {
            pe_line: 0,
            pe_col: 0,
            pe_type: error::ErrorType::InvalidILError,
            pe_msg: format!("Undefined function with ID {}", id),
        })
    }

    pub fn make_entry_block(&mut self, function_id: OptId) -> OptId {
        let raw_params = self
            .functions
            .get(function_id.unwrap())
            .unwrap()
            .params
            .clone();
        let params = self.assign_params_to_locations(&raw_params);
        let func = self.functions.get_mut(function_id.unwrap()).unwrap();

        let block_id = func.blocks.len();

        func.blocks.push(block::Block {
            insts: Vec::new(),
            params,
            term: None,
        });

        OptId::Id(block_id)
    }

    fn assign_params_to_locations(
        &mut self,
        params: &Vec<types::Type<'a>>,
    ) -> Vec<(u32, types::Type<'a>)> {
        let mut located_params = Vec::new();

        for type_param in params {
            located_params.push((self.current_var_id, *type_param));
            self.current_var_id += 1;
        }

        located_params
    }

    fn push_inst(
        &mut self,
        block_id: OptId,
        inst: inst::Instruction,
        type_opt: Option<types::TypeInfo<'a>>,
    ) -> Result<(), error::Error> {
        let cur_fn = self.current_function_mut()?;
        let blk_opt = cur_fn.blocks.get_mut(block_id.unwrap());

        if let Some(blk) = blk_opt {
            blk.insts.push(inst);
        } else {
            return Err(error::Error {
                pe_line: 0,
                pe_col: 0,
                pe_type: error::ErrorType::InvalidILError,
                pe_msg: String::from("Cannot push instruction to non-existent block"),
            });
        }

        Ok(())
    }

    fn _are_terminators_equal(
        t1_opt: Option<inst::Terminator>,
        t2_opt: Option<inst::Terminator>,
    ) -> bool {
        if let Some(t1) = t1_opt {
            if let Some(t2) = t2_opt {
                return t1 == t2;
            }

            return false;
        }

        return false;
    }

    fn set_term(&mut self, block_id: OptId, term: inst::Terminator) -> Result<(), error::Error> {
        let ato = self.allow_term_override;

        let cur_fn = self.current_function_mut()?;
        let blk_opt = cur_fn.blocks.get_mut(block_id.to_result()?);

        if let Some(blk) = blk_opt {
            if (!ato) && Self::_are_terminators_equal(blk.term, Some(term)) {
                return Err(error::Error {
                    pe_line: 0,
                    pe_col: 0,
                    pe_type: error::ErrorType::InvalidILError,
                    pe_msg: String::from(
                        "Cannot override terminators in blocks after they have been set",
                    ),
                });
            }
            blk.term = Some(term);
            Ok(())
        } else {
            Err(error::Error {
                pe_line: 0,
                pe_col: 0,
                pe_type: error::ErrorType::InvalidILError,
                pe_msg: String::from("Cannot set terminator on non-existent block"),
            })
        }
    }

    fn get_new_var_id(&mut self) -> u32 {
        self.current_var_id += 1;
        self.current_var_id - 1
    }

    pub fn const_i64(&mut self, block_id: OptId, value: i64) -> Result<u32, error::Error> {
        let dst = self.get_new_var_id();

        let inst = inst::Instruction {
            i_typ: inst::InstructionType::ConstI64 { dst, value },
            i_ln: 0,
            i_col: 0,
        };

        self.push_inst(
            block_id,
            inst,
            Some(types::TypeInfo {
                var: dst,
                typ: types::Type::Int64,
            }),
        )?;

        Ok(dst)
    }

    pub fn const_f64(&mut self, block_id: OptId, value: f64) -> Result<u32, error::Error> {
        let dst = self.get_new_var_id();

        let inst = inst::Instruction {
            i_typ: inst::InstructionType::ConstF64 { dst, value },
            i_ln: 0,
            i_col: 0,
        };

        self.push_inst(
            block_id,
            inst,
            Some(types::TypeInfo {
                var: dst,
                typ: types::Type::Float64,
            }),
        )?;
        Ok(dst)
    }

    pub fn copy(&mut self, block_id: OptId, src: u32) -> Result<u32, error::Error> {
        let dst = self.get_new_var_id();

        let inst = inst::Instruction {
            i_typ: inst::InstructionType::Assign { dst, src },
            i_ln: 0,
            i_col: 0,
        };
        self.push_inst(
            block_id,
            inst,
            Some(types::TypeInfo {
                var: dst,
                typ: types::Type::None,
            }),
        )?;
        Ok(dst)
    }

    pub fn add(&mut self, block_id: OptId, lhs: u32, rhs: u32) -> Result<u32, error::Error> {
        let dst = self.get_new_var_id();

        let inst = inst::Instruction {
            i_typ: inst::InstructionType::Add { dst, lhs, rhs },
            i_ln: 0,
            i_col: 0,
        };

        self.push_inst(
            block_id,
            inst,
            Some(types::TypeInfo {
                var: dst,
                typ: types::Type::None,
            }),
        )?;
        Ok(dst)
    }

    pub fn ret(&mut self, block_id: OptId, src: u32) -> Result<(), error::Error> {
        let term = inst::Terminator::Return { src };
        self.set_term(block_id, term)?;
        Ok(())
    }

    pub fn call(
        &mut self,
        block_id: OptId,
        callee: u32,
        args: Vec<u32>,
    ) -> Result<u32, error::Error> {
        let dst = self.get_new_var_id();

        let inst = inst::Instruction {
            i_typ: inst::InstructionType::Call { dst, callee, args },
            i_ln: 0,
            i_col: 0,
        };

        self.push_inst(
            block_id,
            inst,
            Some(types::TypeInfo {
                var: dst,
                typ: types::Type::None,
            }),
        )?;
        Ok(dst)
    }
}
