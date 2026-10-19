use std::fmt;


#[derive(Debug)]
pub enum InstructionType {
    ConstI64 {
        dst: u32,
        value: i64
    },

    ConstF64 {
        dst: u32,
        value: f64
    },

    Assign {
        dst: u32,
        src: u32
    },

    Add {
        dst: u32,
        lhs: u32,
        rhs: u32
    },

    Call {
        dst: u32,
        callee: u32,
        args: Vec<u32>
    },
}

#[derive(Debug)]
pub struct Instruction {
    pub i_typ: InstructionType,
    pub i_ln: u32,
    pub i_col: u32
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Terminator {
    Return {
        src: u32
    }
}


impl fmt::Display for InstructionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConstI64 { dst, value } => write!(f, "%{} = const.i64 {}", dst, value)?,
            Self::ConstF64 { dst, value } => write!(f, "%{} = const.f64 {}", dst, value)?,
            Self::Add { dst, lhs, rhs } => write!(f, "%{} = add %{}, %{}", dst, lhs, rhs)?,
            Self::Assign { dst, src } => write!(f, "%{} = %{}", dst, src)?,
            Self::Call { dst, callee, args } => write!(f, "%{} = call %{} {:?}", dst, callee, args)?
        }

        Ok(())
    }
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.i_typ)?;
        Ok(())
    }
}


impl fmt::Display for Terminator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Return { src } => write!(f, "ret %{}", src)?,
        }

        Ok(())
    }
}
