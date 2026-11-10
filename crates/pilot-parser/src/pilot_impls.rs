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
//!     // `print(..)`, `xs.push(x)`, `s.len`, `ENV` run engine intrinsics.
//!     __engine("bind_native_fn", nameof(print), "rt_print");
//!     __engine("bind_native_method", nameof(push), "list_push");
//!     __engine("bind_native_property", nameof(len), "str_len");
//!     __engine("bind_native_value", nameof(ENV), "get_env");
//!     // `sizeof(T)` is answered from `T`'s register layout.
//!     __engine("bind_type_query", nameof(sizeof), "byte_size");
//!     // `for x in e` over a value that is not a list or range calls
//!     // `e.get_enumerator()`, then `move_next()` and `current` on the result.
//!     __engine("bind_enumerator", nameof(get_enumerator), nameof(move_next), nameof(current));
//!     // `x?` on a record unwraps it through its `op ?` when `x.is_success()`;
//!     // `catch e` binds a failed record's `failure()`.
//!     __engine("bind_try_protocol", nameof(is_success), "?", nameof(failure));
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

/// Hooks that bind a Pilot name to an engine intrinsic. The seed answers a
/// call, method, property or value with that name by running the intrinsic.
pub const HOOK_NATIVE_FN: &str = "bind_native_fn";
pub const HOOK_NATIVE_METHOD: &str = "bind_native_method";
pub const HOOK_NATIVE_PROPERTY: &str = "bind_native_property";
pub const HOOK_NATIVE_VALUE: &str = "bind_native_value";
/// Binds a function name to a compile-time question about its type
/// argument, such as `"byte_size"`.
pub const HOOK_TYPE_QUERY: &str = "bind_type_query";

/// The receptor kind and payload `expr_sugar.pilot` binds `nameof` to.
const RECEPTOR_SYMBOL_QUOTE: (u8, u8) = (33, 25);

/// The hook that names the enumerator protocol `for` loops use.
pub const HOOK_ENUMERATOR: &str = "bind_enumerator";
/// The hook that names the success predicate of the `op ?` protocol.
pub const HOOK_TRY_PROTOCOL: &str = "bind_try_protocol";

/// The methods postfix `x?` calls on a struct record `x`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TryProtocol {
    /// Whether `x` holds a success value; when it does not, `x` itself is
    /// the failure that leaves.
    pub is_success: String,
    /// The operator method that yields the success value.
    pub unwrap: String,
    /// The method that yields a failed record's error, which a `catch`
    /// clause binds; empty when none is bound, and the clause then binds the
    /// record itself.
    pub failure: String,
}

/// The methods a `for` loop calls on a value that is not a list or range.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnumeratorProtocol {
    /// Called once on the iterated value; returns the enumerator.
    pub get_enumerator: String,
    /// Called before each element; `false` ends the loop.
    pub move_next: String,
    /// Read after a successful `move_next`: the element.
    pub current: String,
}

/// Which table a native binding goes into.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NativeKind {
    /// `name(args)`.
    Function,
    /// `value.name(args)`, with the receiver first.
    Method,
    /// `value.name` without parentheses, when the value has no such field.
    Property,
    /// `name` on its own.
    Value,
    /// `name(Type)`, answered while compiling.
    TypeQuery,
}

/// The Pilot function an infix operator calls: `a op b` is `function(a, b)`.
/// The program links the function like any other: the package that binds it
/// is imported, by the prelude or by the program.
#[derive(Clone, Debug)]
pub struct OperatorFn {
    pub function: String,
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

    /// The engine intrinsic (or, for `TypeQuery`, the question) bound to
    /// `name` in table `kind`.
    pub fn native(&self, kind: NativeKind, name: &str) -> Option<&str> {
        self.natives.get(&(kind, name.to_string())).map(String::as_str)
    }

    /// Whether `name` is bound as the symbol-quoting receptor (`nameof`),
    /// whose argument is a name, not an expression.
    pub fn quotes_symbol(&self, name: &str) -> bool {
        self.receptors.get(name) == Some(&RECEPTOR_SYMBOL_QUOTE)
    }

    /// The enumerator protocol of `for` loops, if bound.
    pub fn enumerator(&self) -> Option<&EnumeratorProtocol> {
        self.enumerator.as_ref()
    }

    /// The methods postfix `x?` calls on a record, if bound.
    pub fn try_protocol(&self) -> Option<&TryProtocol> {
        self.try_protocol.as_ref()
    }

    pub(crate) fn bind_enumerator(&mut self, get_enumerator: String, move_next: String, current: String) {
        if !get_enumerator.is_empty() && !move_next.is_empty() && !current.is_empty() {
            self.enumerator = Some(EnumeratorProtocol { get_enumerator, move_next, current });
        }
    }

    pub(crate) fn bind_try_protocol(&mut self, is_success: String, unwrap: String, failure: String) {
        if !is_success.is_empty() && !unwrap.is_empty() {
            self.try_protocol = Some(TryProtocol { is_success, unwrap, failure });
        }
    }

    pub(crate) fn bind_native(&mut self, kind: NativeKind, name: String, target: String) {
        if !name.is_empty() && !target.is_empty() {
            self.natives.insert((kind, name), target);
        }
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
            self.operator_fns.insert(symbol, OperatorFn { function });
        }
    }

    /// Records `src` as the defining module of every hook bound while
    /// ingesting it.
    pub(crate) fn finish_ingest(&mut self, src: &str) {
        if let Some(h) = self.unit_literal.as_mut().filter(|h| h.module.is_empty()) {
            h.module = Arc::from(src);
        }
    }
}
