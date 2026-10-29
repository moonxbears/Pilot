//! Milestone 35.2: PilotTranspileIr Data Structures.

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TargetLanguage {
    C,
    Cpp,
    CSharp,
    Rust,
    Python,
    JavaScript,
    Lua,
}

impl TargetLanguage {
    pub fn parse(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "c" => Some(Self::C),
            "cpp" | "c++" => Some(Self::Cpp),
            "csharp" | "c#" | "cs" => Some(Self::CSharp),
            "rust" | "rs" => Some(Self::Rust),
            "python" | "py" => Some(Self::Python),
            "javascript" | "js" => Some(Self::JavaScript),
            "lua" => Some(Self::Lua),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub enum IrType {
    Void,
    Bool,
    Int,
    Float,
    String,
    List(Box<IrType>),
    Custom(String),
}

#[derive(Clone, Debug)]
pub enum IrExpr {
    LitInt(i64),
    LitFloat(f64),
    LitStr(String),
    LitBool(bool),
    Var(String),
    Binary {
        op: String,
        lhs: Box<IrExpr>,
        rhs: Box<IrExpr>,
    },
    Call {
        callee: String,
        args: Vec<IrExpr>,
    },
}

#[derive(Clone, Debug)]
pub enum IrStmt {
    Let {
        name: String,
        ty: IrType,
        init: IrExpr,
    },
    Assign {
        target: String,
        val: IrExpr,
    },
    Return(Option<IrExpr>),
    Expr(IrExpr),
}

#[derive(Clone, Debug)]
pub struct IrFunction {
    pub name: String,
    pub ret_type: IrType,
    pub params: Vec<(String, IrType)>,
    pub body: Vec<IrStmt>,
}

#[derive(Clone, Debug)]
pub struct IrModule {
    pub name: String,
    pub functions: Vec<IrFunction>,
}
