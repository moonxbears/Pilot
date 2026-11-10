//! Seed constructs whose meaning a Pilot module supplies.
//!
//! A module names the Pilot functions that implement a construct through an
//! engine hook in a `comptime` block. The tables keep the names and that
//! module's source, so a compiler can run the functions (at compile time) or
//! compile them into the program that uses the construct. No construct's
//! meaning, and no unit or operator name, is written in Rust.
//!
//! ```text
//! comptime {
//!     // `250ms`: `convert(raw)` is the value in the base unit, `classify(raw)`
//!     // the unit family (`0` for an unknown unit).
//!     __engine("bind_unit_literal", nameof(convert), nameof(classify));
//!     // `a union b` calls `seq_union(a, b)`.
//!     __engine("bind_operator_fn", nameof(union), nameof(seq_union));
//! }
//! ```

use crate::ParserTables;
use std::sync::Arc;

/// The hook that `bind_unit_literal` registers.
pub const HOOK_UNIT_LITERAL: &str = "bind_unit_literal";
/// The hook that `bind_operator_fn` registers.
pub const HOOK_OPERATOR_FN: &str = "bind_operator_fn";

/// The Pilot functions that give a dimensional literal its value.
#[derive(Clone, Debug, Default)]
pub struct UnitLiteralHook {
    /// `fn(raw: str) -> i64`: the value in the base unit.
    pub convert: String,
    /// `fn(raw: str) -> i64`: the unit family, `0` when the unit is unknown.
    pub classify: String,
    /// Source of the module that defines both functions.
    pub module: Arc<str>,
}

/// The Pilot function an infix operator calls: `a op b` is `function(a, b)`.
#[derive(Clone, Debug)]
pub struct OperatorFn {
    pub function: String,
    /// Source of the module that defines it.
    pub module: Arc<str>,
}

impl ParserTables {
    /// The registered converter for dimensional literals, if any.
    pub fn unit_literal(&self) -> Option<&UnitLiteralHook> {
        self.unit_literal.as_ref()
    }

    /// The Pilot function that implements infix operator `symbol`, if any.
    pub fn operator_fn(&self, symbol: &str) -> Option<&OperatorFn> {
        self.operator_fns.get(symbol)
    }

    pub(crate) fn bind_unit_literal(&mut self, convert: String, classify: String) {
        if !convert.is_empty() && !classify.is_empty() {
            self.unit_literal = Some(UnitLiteralHook {
                convert,
                classify,
                module: Arc::from(""),
            });
        }
    }

    pub(crate) fn bind_operator_fn(&mut self, symbol: String, function: String) {
        if !symbol.is_empty() && !function.is_empty() {
            let module = Arc::from("");
            self.operator_fns.insert(symbol, OperatorFn { function, module });
        }
    }

    /// Records `src` as the defining module of every hook bound while
    /// ingesting it.
    pub(crate) fn finish_ingest(&mut self, src: &str) {
        let module: Arc<str> = Arc::from(src);
        if let Some(h) = self.unit_literal.as_mut().filter(|h| h.module.is_empty()) {
            h.module = module.clone();
        }
        for f in self.operator_fns.values_mut().filter(|f| f.module.is_empty()) {
            f.module = module.clone();
        }
    }
}
