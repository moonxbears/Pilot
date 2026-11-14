//! Codes of the diagnostics the seed compiler reports.
//!
//! A diagnostic carries only its code and message. The title, explanation
//! and severity of each code live in the Pilot diagnostic catalog
//! (`selfhost/diagnostics/errors.pilot`), which renders them.

/// Malformed syntax that parsed but cannot mean anything.
pub const SYNTAX: &str = "SYN001";
/// A construct the seed cannot run yet (a seed limit, not a program error).
pub const SEED: &str = "SEED001";
/// A name that no declaration in scope provides.
pub const UNDECLARED: &str = "DECL001";
/// An assignment to a variable a lambda captured by value.
pub const CAPTURED_ASSIGN: &str = "DECL004";
/// An assignment to a property that has a getter and no setter.
pub const GET_ONLY_ASSIGN: &str = "DECL004";
/// `break` outside a loop or switch.
pub const BREAK_OUTSIDE_LOOP: &str = "FLOW003";
/// `continue` outside a loop.
pub const CONTINUE_OUTSIDE_LOOP: &str = "FLOW004";
/// A member (enum variant, field, method) the type does not have.
pub const NO_MEMBER: &str = "METH001";
/// A pattern that cannot match the value it is given.
pub const PATTERN: &str = "PAT001";
/// A struct construction that leaves a field without a value.
pub const MISSING_FIELD: &str = "INIT001";
/// A name hidden from importers by `without`.
pub const NOT_EXPORTED: &str = "VIS001";
/// A call with the wrong number of arguments.
pub const ARG_COUNT: &str = "CALL001";
/// Named arguments that do not fit the callee's parameters.
pub const NAMED_ARG: &str = "CALL003";
/// A dimensional literal whose unit no unit family declares.
pub const UNKNOWN_UNIT: &str = "DIM002";
/// A `yield` that belongs to no generator function and no collecting `for`.
pub const GEN_ORPHAN_YIELD: &str = "GEN001";
