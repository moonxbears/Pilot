using System;

namespace Pilot.Runtime.Bytecode;

/// <summary>
/// Virtual machine opcode instruction set for the PilVM bytecode engine.
/// Provides a high-performance stack/slot hybrid architecture with direct-indexed locals,
/// fast arithmetic dispatch, compact control-flow jumping, and full host/AST interoperability.
/// </summary>
public enum PilotOpcode : byte
{
    /// <summary>No operation.</summary>
    Nop = 0,

    /// <summary>Pops and discards the top value from the evaluation stack.</summary>
    Pop,

    /// <summary>Duplicates the top value on the evaluation stack.</summary>
    Dup,

    /// <summary>Swaps the top two values on the evaluation stack.</summary>
    Swap,

    // Constant loads
    /// <summary>Loads a constant value from the chunk's constant pool onto the stack. Operand: constant pool index.</summary>
    Const,
    /// <summary>Pushes PilotValue.Null onto the stack.</summary>
    ConstNull,
    /// <summary>Pushes PilotValue.Undefined onto the stack.</summary>
    ConstUndefined,
    /// <summary>Pushes PilotValue.True onto the stack.</summary>
    ConstTrue,
    /// <summary>Pushes PilotValue.False onto the stack.</summary>
    ConstFalse,
    /// <summary>Pushes integer 0 onto the stack.</summary>
    ConstZero,
    /// <summary>Pushes integer 1 onto the stack.</summary>
    ConstOne,
    /// <summary>Pushes an inline 32-bit integer onto the stack. Operand: 32-bit integer value.</summary>
    ConstInt,

    // Local variable slots (O(1) stack/slot indexing)
    /// <summary>Loads a local variable from the current frame's slot array onto the stack. Operand: slot index.</summary>
    LoadLocal,
    /// <summary>Stores the top value from the stack into the current frame's slot array. Operand: slot index.</summary>
    StoreLocal,

    // Scope & Named variables (globals, dynamic fallback, closures)
    /// <summary>Resolves a variable by name from the active scope hierarchy. Operand: identifier pool index.</summary>
    LoadNamed,
    /// <summary>Assigns the top value on the stack to an existing binding in the scope hierarchy. Operand: identifier pool index.</summary>
    StoreNamed,
    /// <summary>Declares a variable in the current scope. Operand: identifier pool index.</summary>
    DeclareNamed,

    // Fast-path Arithmetic
    /// <summary>Pops b, pops a, pushes a + b.</summary>
    Add,
    /// <summary>Pops b, pops a, pushes a - b.</summary>
    Sub,
    /// <summary>Pops b, pops a, pushes a * b.</summary>
    Mul,
    /// <summary>Pops b, pops a, pushes a / b.</summary>
    Div,
    /// <summary>Pops b, pops a, pushes a // b (floor division).</summary>
    FloorDiv,
    /// <summary>Pops b, pops a, pushes a % b.</summary>
    Mod,
    /// <summary>Pops b, pops a, pushes a ** b (power).</summary>
    Pow,
    /// <summary>Pops a, pushes -a.</summary>
    Neg,
    /// <summary>Pops a, pushes +a.</summary>
    Pos,

    // Bitwise & Logic
    /// <summary>Pops b, pops a, pushes a &amp; b.</summary>
    BitAnd,
    /// <summary>Pops b, pops a, pushes a | b.</summary>
    BitOr,
    /// <summary>Pops b, pops a, pushes a ^ b.</summary>
    BitXor,
    /// <summary>Pops a, pushes ~a.</summary>
    BitNot,
    /// <summary>Pops b, pops a, pushes a &lt;&lt; b.</summary>
    ShiftLeft,
    /// <summary>Pops b, pops a, pushes a &gt;&gt; b.</summary>
    ShiftRight,
    /// <summary>Pops a, pushes logical !a.</summary>
    Not,

    // Comparisons
    /// <summary>Pops b, pops a, pushes a == b.</summary>
    Eq,
    /// <summary>Pops b, pops a, pushes a != b.</summary>
    Ne,
    /// <summary>Pops b, pops a, pushes a &lt; b.</summary>
    Lt,
    /// <summary>Pops b, pops a, pushes a &lt;= b.</summary>
    Le,
    /// <summary>Pops b, pops a, pushes a &gt; b.</summary>
    Gt,
    /// <summary>Pops b, pops a, pushes a &gt;= b.</summary>
    Ge,
    /// <summary>Pops b, pops a, pushes a is b (identity).</summary>
    Identical,
    /// <summary>Pops b, pops a, pushes a is not b.</summary>
    NotIdentical,

    // Control flow & Jumps
    /// <summary>Unconditionally jumps to target instruction. Operand: absolute instruction index.</summary>
    Jump,
    /// <summary>Pops condition; jumps if condition evaluates to false (falsy). Operand: absolute instruction index.</summary>
    JumpIfFalse,
    /// <summary>Pops condition; jumps if condition evaluates to true (truthy). Operand: absolute instruction index.</summary>
    JumpIfTrue,
    /// <summary>Peeks condition without popping; jumps if falsy. Operand: absolute instruction index.</summary>
    JumpIfFalseOrPop,
    /// <summary>Peeks condition without popping; jumps if truthy. Operand: absolute instruction index.</summary>
    JumpIfTrueOrPop,
    /// <summary>Pops value; jumps if null or undefined. Operand: absolute instruction index.</summary>
    JumpIfNull,
    /// <summary>Pops value; jumps if not null and not undefined. Operand: absolute instruction index.</summary>
    JumpIfNotNull,

    // Loops & Iteration
    /// <summary>Pops collection, pushes an iterator onto the stack.</summary>
    GetIter,
    /// <summary>Fetches next item from iterator on stack. If items remain, pushes item; if exhausted, jumps to target. Operand: exit instruction index.</summary>
    ForIter,

    // Function calls, returns, & closures
    /// <summary>Calls a callable object on stack with arguments. Operand: argument count.</summary>
    Call,
    /// <summary>Calls a named method directly on receiver. Operand: identifier pool index, Extra: argument count.</summary>
    CallMethod,
    /// <summary>Returns undefined from current function/frame.</summary>
    Return,
    /// <summary>Returns the top value on the stack from current function/frame.</summary>
    ReturnValue,
    /// <summary>Yields the top value on the stack from generator frame.</summary>
    Yield,
    /// <summary>Instantiates a closure function from a nested bytecode chunk. Operand: nested chunk index.</summary>
    MakeClosure,

    // Data structures & Collections
    /// <summary>Builds a list from stack values. Operand: element count.</summary>
    BuildList,
    /// <summary>Builds a tuple from stack values. Operand: element count.</summary>
    BuildTuple,
    /// <summary>Builds a map from key-value pairs on stack. Operand: pair count.</summary>
    BuildMap,
    /// <summary>Builds a set from stack values. Operand: element count.</summary>
    BuildSet,
    /// <summary>Builds a range from start and end on stack. Operand: 0 = inclusive, 1 = exclusive.</summary>
    BuildRange,

    // Member & Indexing
    /// <summary>Gets member of target object. Operand: identifier pool index.</summary>
    GetMember,
    /// <summary>Sets member of target object. Target, value on stack. Operand: identifier pool index.</summary>
    SetMember,
    /// <summary>Gets element at index. Target, index on stack.</summary>
    GetIndex,
    /// <summary>Sets element at index. Target, index, value on stack.</summary>
    SetIndex,
    /// <summary>Slices target. Target, start, end on stack. Operand: 0 = inclusive, 1 = exclusive.</summary>
    Slice,

    // Universal Fallback AST Evaluation (100% language coverage for complex dynamic macros/contracts)
    /// <summary>Evaluates an AST subtree through the interpreter visitor. Operand: fallback node index.</summary>
    EvalAst,

    /// <summary>Halts VM execution.</summary>
    Halt
}
