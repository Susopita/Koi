use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IRProgram {
    #[serde(rename = "irType")]
    pub ir_type: String,
    pub functions: Vec<IRFunction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IRFunction {
    pub name: String,
    #[serde(rename = "returnType")]
    pub return_type: String,
    pub parameters: Vec<(String, String)>,
    pub blocks: Vec<BasicBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicBlock {
    pub label: String,
    pub instructions: Vec<Instruction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op")]
pub enum Instruction {
    #[serde(rename = "const")]
    Const {
        result: String,
        value: Value,
        #[serde(rename = "type")]
        ty: String,
    },
    #[serde(rename = "binop")]
    BinOp {
        result: String,
        lhs: String,
        rhs: String,
        #[serde(rename = "op_type")]
        op_type: String,
        #[serde(rename = "type")]
        ty: String,
    },
    #[serde(rename = "call")]
    Call {
        result: Option<String>,
        function: String,
        arguments: Vec<String>,
        #[serde(rename = "type")]
        ty: Option<String>,
    },
    #[serde(rename = "return")]
    Return { value: Option<String> },
    #[serde(rename = "jump")]
    Jump { label: String },
    #[serde(rename = "branch")]
    Branch {
        cond: String,
        true_label: String,
        false_label: String,
    },
    /// Merges values coming from different predecessor blocks into one SSA
    /// value -- needed for `if` used as an expression and for loop-carried
    /// variables, since neither can be expressed with Const/BinOp/Call alone.
    #[serde(rename = "phi")]
    Phi {
        result: String,
        /// (predecessor block label, value) pairs.
        incoming: Vec<(String, String)>,
        #[serde(rename = "type")]
        ty: String,
    },
    /// Calls a function *value* held in a register (e.g. a parameter that
    /// was passed a closure), as opposed to `call`'s statically-known
    /// function name.
    #[serde(rename = "call_indirect")]
    CallIndirect {
        result: Option<String>,
        function_value: String,
        arguments: Vec<String>,
        #[serde(rename = "type")]
        ty: Option<String>,
    },
    #[serde(rename = "alloc")]
    Alloc {
        result: String,
        #[serde(rename = "type")]
        ty: String,
        size: Option<String>,
    },
    #[serde(rename = "get_field")]
    GetField {
        result: String,
        object: String,
        field: String,
        #[serde(rename = "type")]
        ty: String,
    },
    #[serde(rename = "get_index")]
    GetIndex {
        result: String,
        array: String,
        index: String,
        #[serde(rename = "type")]
        ty: String,
    },
    /// Write counterpart to `get_index` (backs `aset!`) -- a store has no
    /// result value, so this drops `result` and adds `value` instead.
    #[serde(rename = "set_index")]
    SetIndex {
        array: String,
        index: String,
        value: String,
        #[serde(rename = "type")]
        ty: String,
    },
    /// Write counterpart to `get_field` -- a store has no result value, so
    /// this drops `result` and adds `value` instead (mirrors `set_index`
    /// relative to `get_index`). Used to populate a closure's env struct
    /// (one per captured variable) and the shared `Closure` wrapper's
    /// `fn_ptr`/`env_ptr` fields when generating a `MakeClosure` node.
    #[serde(rename = "set_field")]
    SetField {
        object: String,
        field: String,
        value: String,
        #[serde(rename = "type")]
        ty: String,
    },
    #[serde(rename = "addr_of")]
    AddrOf {
        result: String,
        operand: String,
        #[serde(rename = "type")]
        ty: String,
    },
    #[serde(rename = "deref")]
    Deref {
        result: String,
        operand: String,
        #[serde(rename = "type")]
        ty: String,
    },
}
