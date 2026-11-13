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
//!     // `e.get_enumerator()`, then `move_next()` and `current` on the result;
//!     // a loop left early calls `close()` on an enumerator that has one.
//!     __engine("bind_enumerator", nameof(get_enumerator), nameof(move_next), nameof(current), nameof(close));
//!     // Calling a function whose body yields builds a `Generator` record
//!     // with `generator_of(frame)`, the engine frame in field `frame`;
//!     // `yield *xs` over a value that is not one walks `each_item(xs)`.
//!     __engine("bind_generator", nameof(Generator), nameof(frame), nameof(generator_of), nameof(each_item));
//!     // `x?` on a record unwraps it through its `op ?` when `x.is_success()`;
//!     // `catch e` binds a failed record's `failure()`.
//!     __engine("bind_try_protocol", nameof(is_success), "?", nameof(failure));
//!     // `f"{x:#_###}"` calls `format_by_pattern(x, "#_###")`.
//!     __engine("bind_format_spec", nameof(format_by_pattern));
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
/// The hook that names the generator type and the functions that build it.
pub const HOOK_GENERATOR: &str = "bind_generator";
/// The hook that names the success predicate of the `op ?` protocol.
pub const HOOK_TRY_PROTOCOL: &str = "bind_try_protocol";
/// The hook that names the function an f-string hole's `:pattern` calls:
/// `f"{x:#_###}"` is `f(x, "#_###")`.
pub const HOOK_FORMAT_SPEC: &str = "bind_format_spec";

/// The hook that names the type whose values are Unicode scalar values
/// shown as their character.
pub const HOOK_SCALAR_TEXT: &str = "bind_scalar_text";

/// The hook that names the function `x in xs` calls: `f(x, xs)`.
pub const HOOK_MEMBERSHIP: &str = "bind_membership";

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
    /// Called on the enumerator when the loop is left (by `break`, `return`
    /// or its end), when the enumerator's type declares it; empty when none
    /// is bound.
    pub close: String,
}

/// What calling a generator function builds, and how `yield *xs` reaches
/// the items of `xs`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratorProtocol {
    /// The struct a generator function's call returns.
    pub type_name: String,
    /// Its field holding the engine's frame handle.
    pub frame_field: String,
    /// `fn(frame) -> Generator`: builds the record around a new frame.
    pub construct: String,
    /// `fn(xs) -> Generator`: a generator yielding each item of `xs`.
    pub each_item: String,
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

    /// The generator type and its builders, if bound.
    pub fn generator(&self) -> Option<&GeneratorProtocol> {
        self.generator.as_ref()
    }

    /// The methods postfix `x?` calls on a record, if bound.
    pub fn try_protocol(&self) -> Option<&TryProtocol> {
        self.try_protocol.as_ref()
    }

    /// The function an f-string hole's format pattern calls, if one is bound.
    pub fn format_spec(&self) -> Option<&str> {
        self.format_spec.as_deref()
    }

    /// The type whose values are Unicode scalar values, if one is bound.
    pub fn scalar_text_type(&self) -> Option<&str> {
        self.scalar_text.as_deref()
    }

    pub(crate) fn bind_scalar_text(&mut self, type_name: String) {
        if !type_name.is_empty() {
            self.scalar_text = Some(type_name);
        }
    }

    /// The function `x in xs` calls, if one is bound.
    pub fn membership(&self) -> Option<&str> {
        self.membership.as_deref()
    }

    pub(crate) fn bind_membership(&mut self, function: String) {
        if !function.is_empty() {
            self.membership = Some(function);
        }
    }

    pub(crate) fn bind_format_spec(&mut self, function: String) {
        if !function.is_empty() {
            self.format_spec = Some(function);
        }
    }

    pub(crate) fn bind_enumerator(&mut self, get_enumerator: String, move_next: String, current: String, close: String) {
        if !get_enumerator.is_empty() && !move_next.is_empty() && !current.is_empty() {
            self.enumerator = Some(EnumeratorProtocol { get_enumerator, move_next, current, close });
        }
    }

    pub(crate) fn bind_generator(&mut self, type_name: String, frame_field: String, construct: String, each_item: String) {
        if [&type_name, &frame_field, &construct, &each_item].iter().all(|s| !s.is_empty()) {
            self.generator = Some(GeneratorProtocol { type_name, frame_field, construct, each_item });
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
