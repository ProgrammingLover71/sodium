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

    fn factor(&mut self, node_id: ast::NodeId) -> Result<u32, error::Error> {
        let node = self.program.get_node(node_id).ok_or(error::Error {
            pe_line: 0,
            pe_col: 0,
            pe_type: error::ErrorType::InvalidILError,
            pe_msg: String::new(),
        })?;

        match node.n_type {
            // We need to add fields for the function index and handle
            // the things for, like, the function and shit
            // (it's late okay!?)
            _ => todo!(),
        }
    }
}
