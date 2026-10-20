use crate::parser::ast;
use crate::{error, sil};

pub struct SILGenerator<'a> {
    il_builder: sil::ILBuilder<'a>,
    program: &'a ast::Program<'a>,
}

impl<'a> SILGenerator<'a> {
    pub fn new(program: &'a ast::Program<'a>, allow_term_override: bool) -> Self {
        Self {
            il_builder: sil::ILBuilder::new(allow_term_override),
            program,
        }
    }

    pub fn compile(&'a mut self) -> Result<(&'a sil::ILBuilder<'a>, sil::OptId), error::Error> {
        // Create a `main` function for global-scoped code
        let f_main = self.il_builder.define_function(String::from("%main"), vec![], sil::Type::None);
        self.il_builder.set_current_function(f_main);

        let f_main_cb = self.il_builder.make_entry_block(f_main);

        // Traverse all the program roots
        for root in &self.program.roots {
            // Safe to .unwrap(), because `root` is guaranteed to point to an existent arena node
            let root_node = self.program.get_node(*root).unwrap();
            self.statement(root_node, f_main_cb)?;
        }
        
        Ok((&self.il_builder, f_main))
    }

    fn statement(&mut self, node: &ast::AstNode, current_block_id: sil::OptId) -> Result<(), error::Error> {
        match node.n_type {
            ast::AstNodeType::Return { value } => {
                let loc = self.expression(value, current_block_id)?;
                self.il_builder.ret(current_block_id, loc)?;
            }

            _ => {
                // Any other node type is invalid
                return Err(error::Error { 
                    pe_line: node.n_line, 
                    pe_col: node.n_col, 
                    pe_type: error::ErrorType::InvalidILError, 
                    pe_msg: format!("Expected statement node, got {:?}", node.n_type) 
                });
            }
        }
        Ok(())
    }

    fn expression(&mut self, node_id: ast::NodeId, current_block_id: sil::OptId) -> Result<u32, error::Error> {
        let node = self.program.get_node(node_id).ok_or(error::Error {
            pe_line: 0,
            pe_col: 0,
            pe_type: error::ErrorType::InvalidILError,
            pe_msg: format!("Node ID {} is invalid", node_id.0),
        })?;

        match node.n_type {
            ast::AstNodeType::BinaryOp { left, right, op } => {
                let loc_a = self.expression(left, current_block_id)?;
                let loc_b = self.expression(right, current_block_id)?;

                let loc_res: u32;

                match op {
                    ast::OpType::Add => {
                        loc_res = self.il_builder.add(current_block_id, loc_a, loc_b)?;
                        Ok(loc_res)
                    }

                    _ => {
                        // Other op types aren't supported yet
                        Err(error::Error { 
                            pe_line: node.n_line, 
                            pe_col: node.n_col, 
                            pe_type: error::ErrorType::InvalidILError, 
                            pe_msg: String::from("Only addition is supported for SIL generation")
                        })
                    }
                }
            }

            ast::AstNodeType::IntLiteral { value } => self.factor(node_id, current_block_id),

            _ => {
                Err(error::Error { pe_line: node.n_line, 
                    pe_col: node.n_col, 
                    pe_type: error::ErrorType::InvalidILError, 
                    pe_msg: format!("Expected expression node, got {}", node.n_type)
                })
            }
        }
    }

    fn factor(&mut self, node_id: ast::NodeId, current_block_id: sil::OptId) -> Result<u32, error::Error> {
        let node = self.program.get_node(node_id).ok_or(error::Error {
            pe_line: 0,
            pe_col: 0,
            pe_type: error::ErrorType::InvalidILError,
            pe_msg: format!("Node ID {} is invalid", node_id.0),
        })?;

        match node.n_type {
            ast::AstNodeType::IntLiteral { value } => {
                let loc = self.il_builder.const_i64(current_block_id, value)?;
                Ok(loc)
            }

            // Only int literals exist for now
            _ => {
                Err(error::Error { 
                    pe_line: node.n_line, 
                    pe_col: node.n_col, 
                    pe_type: error::ErrorType::InvalidILError, 
                    pe_msg: format!("Expected factor node, got {:?}", node.n_type)
                })
            }
        }
    }
}
