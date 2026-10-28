using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using Pilot.Runtime.Environment;
using Pilot.Runtime.Values;

namespace Pilot.Runtime.Execution;

/// <summary>
/// Evaluates pattern matching rules against runtime <see cref="PilotValue"/> instances.
/// Supports discriminated sum-type ADTs, immutable records, keyword variants, `@` whole-value aliases,
/// inclusive/half-open range patterns, collection slice patterns, structural property patterns,
/// typestate patterns, and transactional variable binding (zero partial-binding leakage on failed arms).
/// </summary>
public static class PilotPatternMatcher
{
    private static readonly HashSet<string> KnownPrimitiveTypes = new(StringComparer.OrdinalIgnoreCase)
    {
        "int", "i8", "i16", "i32", "i64", "uint", "u8", "u16", "u32", "u64",
        "float", "f8", "f16", "f32", "f64", "double", "decimal",
        "str", "string", "bool", "char", "list", "tuple", "map", "set", "range", "matrix", "object", "keyword", "symbol"
    };

    public static bool Match(
        PilotParser.PatternContext context,
        PilotValue value,
        PilotScope scope,
        PilotInterpreterVisitor visitor)
    {
        if (context == null) return false;

        var bindings = new Dictionary<string, (PilotValue Value, bool IsConst)>(StringComparer.Ordinal);
        if (!MatchOrPattern(context.or_pattern(), value, scope, visitor, bindings))
        {
            return false;
        }

        foreach (var kvp in bindings)
        {
            scope.Declare(kvp.Key, kvp.Value.Value, isConst: kvp.Value.IsConst);
        }
        return true;
    }

    private static bool MatchPatternInternal(
        PilotParser.PatternContext? context,
        PilotValue value,
        PilotScope scope,
        PilotInterpreterVisitor visitor,
        Dictionary<string, (PilotValue Value, bool IsConst)> bindings)
    {
        if (context == null) return false;
        return MatchOrPattern(context.or_pattern(), value, scope, visitor, bindings);
    }

    private static bool MatchOrPattern(
        PilotParser.Or_patternContext context,
        PilotValue value,
        PilotScope scope,
        PilotInterpreterVisitor visitor,
        Dictionary<string, (PilotValue Value, bool IsConst)> bindings)
    {
        var branches = context.and_pattern();
        if (branches.Length == 1)
        {
            return MatchAndPattern(branches[0], value, scope, visitor, bindings);
        }

        foreach (var andCtx in branches)
        {
            var branchBindings = new Dictionary<string, (PilotValue Value, bool IsConst)>(bindings, StringComparer.Ordinal);
            if (MatchAndPattern(andCtx, value, scope, visitor, branchBindings))
            {
                bindings.Clear();
                foreach (var kvp in branchBindings)
                {
                    bindings[kvp.Key] = kvp.Value;
                }
                return true;
            }
        }
        return false;
    }

    private static bool MatchAndPattern(
        PilotParser.And_patternContext context,
        PilotValue value,
        PilotScope scope,
        PilotInterpreterVisitor visitor,
        Dictionary<string, (PilotValue Value, bool IsConst)> bindings)
    {
        foreach (var notCtx in context.not_pattern())
        {
            if (!MatchNotPattern(notCtx, value, scope, visitor, bindings))
            {
                return false;
            }
        }
        return true;
    }

    private static bool MatchNotPattern(
        PilotParser.Not_patternContext context,
        PilotValue value,
        PilotScope scope,
        PilotInterpreterVisitor visitor,
        Dictionary<string, (PilotValue Value, bool IsConst)> bindings)
    {
        if (context.NOT() != null)
        {
            var discarded = new Dictionary<string, (PilotValue Value, bool IsConst)>(StringComparer.Ordinal);
            return !MatchNotPattern(context.not_pattern(), value, scope, visitor, discarded);
        }

        // Whole-value '@' alias pattern: e.g. sq @ Rectangle(w, h)
        if (context.AT() != null && context.identifier() != null)
        {
            if (!MatchNotPattern(context.not_pattern(), value, scope, visitor, bindings))
            {
                return false;
            }
            string aliasName = context.identifier().GetText();
            if (aliasName != "_")
            {
                bindings[aliasName] = (value, context.VAR() == null);
            }
            return true;
        }

        return MatchPrimaryPattern(context.primary_pattern(), value, scope, visitor, bindings);
    }

    private static bool MatchPrimaryPattern(
        PilotParser.Primary_patternContext context,
        PilotValue value,
        PilotScope scope,
        PilotInterpreterVisitor visitor,
        Dictionary<string, (PilotValue Value, bool IsConst)> bindings)
    {
        if (context == null) return false;

        // 1. Wildcard '_'
        if (context.UNDERSCORE() != null)
        {
            return true;
        }

        // 2. Range pattern: 200..299 (inclusive), 200..=299 (inclusive), 0..<10 (half-open), 'a'..'z'
        if (context.range_pattern() != null)
        {
            return MatchRangePattern(context.range_pattern(), value);
        }

        // 3. Signed negative numeric literal pattern: -1, -3.14
        if (context.signed_number_pattern() != null)
        {
            var negVal = ParseSignedNumber(context.signed_number_pattern());
            return value.Equals(negVal);
        }

        // 4. Literal pattern: 42, "hello", true, null, etc.
        if (context.literal() != null)
        {
            var litVal = visitor.VisitLiteral(context.literal());
            return value.Equals(litVal);
        }

        // 5. Keyword & keyword-variant pattern: :pending, :ok([head, ..tail])
        if (context.keyword_pattern() != null)
        {
            return MatchKeywordPattern(context.keyword_pattern(), value, scope, visitor, bindings);
        }

        // 6. Relational pattern: > 10, <= 20
        if (context.relational_pattern() != null)
        {
            var relCtx = context.relational_pattern();
            var rhsVal = visitor.Visit(relCtx.shift_expr());
            if (relCtx.EQEQUAL() != null) return value == rhsVal;
            if (relCtx.NOTEQUAL() != null) return value != rhsVal;
            if (!value.IsNumber && value.Kind != PilotValueKind.Char && value.Kind != PilotValueKind.String)
                return false;
            if (relCtx.GREATER() != null) return value > rhsVal;
            if (relCtx.GREATEREQUAL() != null) return value >= rhsVal;
            if (relCtx.LESS() != null) return value < rhsVal;
            if (relCtx.LESSEQUAL() != null) return value <= rhsVal;
            return false;
        }

        // 7. Variable, Type, ADT Variant, Record, or Typestate pattern
        if (context.type_or_var_pattern() != null)
        {
            return MatchTypeOrVarPattern(context.type_or_var_pattern(), value, scope, visitor, bindings);
        }

        // 8. Collection / Slice pattern: [head, ..mid, tail] or [1, 2, 3]
        if (context.collection_pattern() != null)
        {
            if (value.Kind != PilotValueKind.List && value.Kind != PilotValueKind.Tuple)
            {
                return false;
            }

            var elements = value.Kind == PilotValueKind.List
                ? (IReadOnlyList<PilotValue>)value.AsList()
                : value.AsTuple();

            var sliceElements = context.collection_pattern().slice_pattern_element();
            return MatchCollectionSlice(sliceElements, elements, scope, visitor, bindings);
        }

        // 9. Tuple / Parenthesized pattern: (a, b) or (p1 | p2)
        if (context.tuple_pattern_match() != null)
        {
            var patterns = context.tuple_pattern_match().pattern();
            bool hasTrailingComma = context.tuple_pattern_match().COMMA()?.Length > 0;

            // Single parenthesized pattern `(inner)` without trailing comma acts as grouping!
            if (patterns.Length == 1 && !hasTrailingComma)
            {
                return MatchPatternInternal(patterns[0], value, scope, visitor, bindings);
            }

            if (value.Kind != PilotValueKind.Tuple && value.Kind != PilotValueKind.List)
            {
                return false;
            }

            var elements = value.Kind == PilotValueKind.Tuple ? value.AsTuple() : value.AsList();
            if (elements.Count != patterns.Length) return false;

            for (int i = 0; i < patterns.Length; i++)
            {
                if (!MatchPatternInternal(patterns[i], elements[i], scope, visitor, bindings)) return false;
            }
            return true;
        }

        // 10. Structural property pattern: { x: 10, y: 20 }
        if (context.property_pattern() != null)
        {
            return MatchPropertyPattern(context.property_pattern(), value, scope, visitor, bindings);
        }

        return false;
    }

    private static bool MatchRangePattern(PilotParser.Range_patternContext rangeCtx, PilotValue value)
    {
        var bounds = rangeCtx.range_bound();
        if (bounds == null || bounds.Length < 2) return false;

        bool inclusive = rangeCtx.DOTDOT_LESS() == null;
        var lo = EvaluateRangeBound(bounds[0]);
        var hi = EvaluateRangeBound(bounds[1]);

        // Character / single-char string comparison
        if (lo.Kind is PilotValueKind.String or PilotValueKind.Char &&
            hi.Kind is PilotValueKind.String or PilotValueKind.Char)
        {
            string? valStr = value.Kind == PilotValueKind.Char
                ? value.AsChar().ToString()
                : value.Kind == PilotValueKind.String ? value.ToString() : null;
            if (valStr == null) return false;

            string loStr = lo.ToString();
            string hiStr = hi.ToString();
            if (valStr.Length == 1 && loStr.Length == 1 && hiStr.Length == 1)
            {
                char vc = valStr[0], lc = loStr[0], hc = hiStr[0];
                return inclusive ? (vc >= lc && vc <= hc) : (vc >= lc && vc < hc);
            }
            int cmpLo = string.CompareOrdinal(valStr, loStr);
            int cmpHi = string.CompareOrdinal(valStr, hiStr);
            return inclusive ? (cmpLo >= 0 && cmpHi <= 0) : (cmpLo >= 0 && cmpHi < 0);
        }

        if (!value.IsNumber) return false;

        if (value.Kind == PilotValueKind.Int && lo.Kind == PilotValueKind.Int && hi.Kind == PilotValueKind.Int)
        {
            long v = value.AsInt();
            long l = lo.AsInt();
            long h = hi.AsInt();
            return inclusive ? (v >= l && v <= h) : (v >= l && v < h);
        }

        double dv = value.AsFloat();
        double dl = lo.AsFloat();
        double dh = hi.AsFloat();
        return inclusive ? (dv >= dl && dv <= dh) : (dv >= dl && dv < dh);
    }

    private static PilotValue EvaluateRangeBound(PilotParser.Range_boundContext bound)
    {
        if (bound.STRING() != null)
        {
            string raw = bound.STRING().GetText();
            string unquoted = raw.Length >= 2 ? raw[1..^1] : raw;
            return unquoted.Length == 1 ? PilotValue.FromChar(unquoted[0]) : PilotValue.FromString(unquoted);
        }

        bool neg = bound.MINUS() != null;
        if (bound.FLOAT_NUMBER() != null)
        {
            double d = double.Parse(bound.FLOAT_NUMBER().GetText().Replace("_", ""), CultureInfo.InvariantCulture);
            return PilotValue.FromFloat(neg ? -d : d);
        }

        long l = ParseUnsignedIntToken(bound.INTEGER()?.GetText(), bound.HEX_INTEGER()?.GetText(), bound.BIN_INTEGER()?.GetText(), bound.OCT_INTEGER()?.GetText());
        return PilotValue.FromInt(neg ? -l : l);
    }

    private static PilotValue ParseSignedNumber(PilotParser.Signed_number_patternContext ctx)
    {
        if (ctx.FLOAT_NUMBER() != null)
        {
            double d = double.Parse(ctx.FLOAT_NUMBER().GetText().Replace("_", ""), CultureInfo.InvariantCulture);
            return PilotValue.FromFloat(-d);
        }
        long l = ParseUnsignedIntToken(ctx.INTEGER()?.GetText(), ctx.HEX_INTEGER()?.GetText(), ctx.BIN_INTEGER()?.GetText(), ctx.OCT_INTEGER()?.GetText());
        return PilotValue.FromInt(-l);
    }

    private static long ParseUnsignedIntToken(string? dec, string? hex, string? bin, string? oct)
    {
        if (hex != null) return Convert.ToInt64(hex.Replace("_", "")[2..], 16);
        if (bin != null) return Convert.ToInt64(bin.Replace("_", "")[2..], 2);
        if (oct != null) return Convert.ToInt64(oct.Replace("_", "")[2..], 8);
        if (dec != null) return long.Parse(dec.Replace("_", ""), CultureInfo.InvariantCulture);
        return 0;
    }

    private static bool MatchKeywordPattern(
        PilotParser.Keyword_patternContext kwCtx,
        PilotValue value,
        PilotScope scope,
        PilotInterpreterVisitor visitor,
        Dictionary<string, (PilotValue Value, bool IsConst)> bindings)
    {
        var kwVal = visitor.VisitSymbol_literal(kwCtx.symbol_literal());
        var targetKw = kwVal.AsKeyword();

        if (kwCtx.LPAR() == null)
        {
            if (value.Kind == PilotValueKind.Keyword)
                return value.AsKeyword().Equals(targetKw);
            if (value.Kind == PilotValueKind.KeywordVariant)
                return value.AsKeywordVariant().Fields.Length == 0 && value.AsKeywordVariant().Keyword.Equals(targetKw);
            return false;
        }

        var args = kwCtx.pattern_arg();
        int argCount = args?.Length ?? 0;

        // 1. Match PilotKeywordVariant: :ok([1, 2, 3])
        if (value.Kind == PilotValueKind.KeywordVariant)
        {
            var kv = value.AsKeywordVariant();
            if (!kv.Keyword.Equals(targetKw) || kv.Fields.Length != argCount) return false;
            for (int i = 0; i < argCount; i++)
            {
                if (!MatchPatternInternal(args![i].pattern(), kv.Fields[i], scope, visitor, bindings))
                    return false;
            }
            return true;
        }

        // 2. Match Elixir-style tagged tuple: (:ok, [1, 2, 3])
        if (value.Kind == PilotValueKind.Tuple)
        {
            var tuple = value.AsTuple();
            if (tuple.Count != argCount + 1) return false;
            if (tuple[0].Kind != PilotValueKind.Keyword || !tuple[0].AsKeyword().Equals(targetKw)) return false;
            for (int i = 0; i < argCount; i++)
            {
                if (!MatchPatternInternal(args![i].pattern(), tuple[i + 1], scope, visitor, bindings))
                    return false;
            }
            return true;
        }

        // 3. Match ADT variant with matching name (case-insensitive convenience for :ok -> Ok)
        if (value.Kind == PilotValueKind.Adt)
        {
            var vd = value.GetAdtVariantDefinition();
            if (!string.Equals(vd.Name, targetKw.Name, StringComparison.OrdinalIgnoreCase) || vd.Arity != argCount)
                return false;
            return MatchAdtFields(value, vd, args, scope, visitor, bindings);
        }

        return false;
    }

    private static bool MatchTypeOrVarPattern(
        PilotParser.Type_or_var_patternContext tvCtx,
        PilotValue value,
        PilotScope scope,
        PilotInterpreterVisitor visitor,
        Dictionary<string, (PilotValue Value, bool IsConst)> bindings)
    {
        // Explicit `let x` or `var x` binding
        if ((tvCtx.LET() != null || tvCtx.VAR() != null) && tvCtx.dotted_name() == null && tvCtx.identifier() != null)
        {
            string varName = tvCtx.identifier().GetText();
            if (varName != "_")
            {
                bindings[varName] = (value, tvCtx.VAR() == null);
            }
            return true;
        }

        if (tvCtx.dotted_name() == null) return false;

        var nameParts = tvCtx.dotted_name().identifier();
        string fullDotted = tvCtx.dotted_name().GetText();
        var genericSuffix = tvCtx.generic_pattern_suffix();
        var trailingId = tvCtx.identifier();
        bool hasParens = tvCtx.LPAR() != null;
        var patArgs = tvCtx.pattern_arg();
        var propPat = tvCtx.property_pattern();

        // Check qualified generic variant like `Expr[int].Const(v)`
        string? explicitParentAdt = null;
        string targetName = fullDotted;
        if (genericSuffix?.identifier() != null)
        {
            explicitParentAdt = fullDotted;
            targetName = genericSuffix.identifier().GetText();
        }
        else if (nameParts.Length == 2)
        {
            explicitParentAdt = nameParts[0].GetText();
            targetName = nameParts[1].GetText();
        }

        // A0. Built-in Option `some(pat)` / `some x` pattern matching (Milestone 12)
        if ((explicitParentAdt == null || string.Equals(explicitParentAdt, "Option", StringComparison.Ordinal)) &&
            (string.Equals(targetName, "some", StringComparison.Ordinal) ||
             (string.Equals(targetName, "Some", StringComparison.Ordinal) && !visitor.Context.VariantDefinitions.ContainsKey("Some"))) &&
            (hasParens || trailingId != null))
        {
            // If `value` is a user-defined ADT whose variant is literally named `Some`/`some`, defer to ADT matching below
            if (value.Kind != PilotValueKind.Adt ||
                !string.Equals(value.GetAdtVariantDefinition().Name, targetName, StringComparison.Ordinal))
            {
                if (value.IsNull || value.IsUndefined) return false;
                var unwrapped = value.UnwrapOption();
                if (hasParens)
                {
                    if (patArgs != null && patArgs.Length == 1)
                    {
                        if (!MatchPatternInternal(patArgs[0].pattern(), unwrapped, scope, visitor, bindings))
                            return false;
                    }
                    else if (patArgs != null && patArgs.Length > 1)
                    {
                        return false;
                    }
                }
                if (trailingId != null && trailingId.GetText() != "_")
                {
                    bindings[trailingId.GetText()] = (unwrapped, true);
                }
                return true;
            }
        }

        // A0b. Built-in Result `ok(pat)` and `fail(pat)` / `err(pat)` pattern matching (Milestone 13)
        if ((explicitParentAdt == null || string.Equals(explicitParentAdt, "Result", StringComparison.Ordinal)) &&
            (hasParens || trailingId != null))
        {
            bool isOkCtor = string.Equals(targetName, "ok", StringComparison.Ordinal) ||
                            (string.Equals(targetName, "Ok", StringComparison.Ordinal) && !visitor.Context.VariantDefinitions.ContainsKey("Ok"));
            bool isFailCtor = string.Equals(targetName, "fail", StringComparison.Ordinal) ||
                              string.Equals(targetName, "err", StringComparison.Ordinal) ||
                              (string.Equals(targetName, "Fail", StringComparison.Ordinal) && !visitor.Context.VariantDefinitions.ContainsKey("Fail")) ||
                              (string.Equals(targetName, "Err", StringComparison.Ordinal) && !visitor.Context.VariantDefinitions.ContainsKey("Err"));

            if (isOkCtor)
            {
                if (value.IsFailure) return false;
                if (hasParens && patArgs != null && patArgs.Length == 1)
                {
                    if (!MatchPatternInternal(patArgs[0].pattern(), value, scope, visitor, bindings))
                        return false;
                }
                else if (hasParens && patArgs != null && patArgs.Length > 1)
                {
                    return false;
                }
                if (trailingId != null && trailingId.GetText() != "_")
                {
                    bindings[trailingId.GetText()] = (value, true);
                }
                return true;
            }
            if (isFailCtor)
            {
                if (!value.IsFailure) return false;
                var errPayload = value.UnwrapFailure();
                if (hasParens && patArgs != null && patArgs.Length == 1)
                {
                    if (!MatchPatternInternal(patArgs[0].pattern(), errPayload, scope, visitor, bindings))
                        return false;
                }
                else if (hasParens && patArgs != null && patArgs.Length > 1)
                {
                    return false;
                }
                if (trailingId != null && trailingId.GetText() != "_")
                {
                    bindings[trailingId.GetText()] = (errPayload, true);
                }
                return true;
            }
        }

        // Allow direct error variant matching when matching a `PilotValueKind.Failure` against an error ADT variant
        if (value.IsFailure)
        {
            value = value.UnwrapFailure();
        }

        // A. Built-in primitive type test: `int n`, `str s`, `bool b`
        if (explicitParentAdt == null && genericSuffix == null && !hasParens && propPat == null && KnownPrimitiveTypes.Contains(targetName))
        {
            if (!MatchesPrimitiveType(targetName, value)) return false;
            if (trailingId != null && trailingId.GetText() != "_")
            {
                bindings[trailingId.GetText()] = (value, true);
            }
            return true;
        }

        // B. Typestate pattern on class object: `Conn[Open]` or `Conn[Open] c`
        if (genericSuffix != null && genericSuffix.identifier() == null && !hasParens && value.Kind == PilotValueKind.Object)
        {
            var obj = value.AsObject();
            if (MatchesObjectType(obj, fullDotted, visitor.Context))
            {
                string stateArg = genericSuffix.type_arguments().GetText().Trim();
                if (!string.IsNullOrEmpty(obj.CurrentState))
                {
                    if (!string.Equals(obj.CurrentState, stateArg, StringComparison.Ordinal))
                        return false;
                    if (trailingId != null && trailingId.GetText() != "_")
                        bindings[trailingId.GetText()] = (value, true);
                    return true;
                }
            }
        }

        // C. ADT Variant matching (unit `Origin`, qualified `Shape.Origin`, positional `Circle(r)`, named `Rectangle(width: w, height: h)`)
        if (value.Kind == PilotValueKind.Adt)
        {
            var vd = value.GetAdtVariantDefinition();

            // Also allow matching whole ADT type name: `case Shape s =>`
            if (explicitParentAdt == null && !hasParens && propPat == null && trailingId != null &&
                string.Equals(vd.ParentAdt.Name, targetName, StringComparison.Ordinal))
            {
                if (trailingId.GetText() != "_")
                    bindings[trailingId.GetText()] = (value, true);
                return true;
            }

            bool isKnownVariantOrHasCtorSyntax =
                explicitParentAdt != null ||
                hasParens ||
                propPat != null ||
                visitor.Context.VariantDefinitions.ContainsKey(targetName) ||
                (scope.TryResolve(targetName, out var scopeSym) && scopeSym.Kind == PilotValueKind.Adt);

            if (isKnownVariantOrHasCtorSyntax)
            {
                if (explicitParentAdt != null && !string.Equals(vd.ParentAdt.Name, explicitParentAdt, StringComparison.Ordinal))
                    return false;
                if (!string.Equals(vd.Name, targetName, StringComparison.Ordinal))
                    return false;

                if (hasParens)
                {
                    if (!MatchAdtFields(value, vd, patArgs, scope, visitor, bindings))
                        return false;
                }
                else if (vd.Arity != 0 && propPat == null && trailingId == null)
                {
                    // Unit pattern written for a payload variant does not match unless destructured
                    return false;
                }

                if (propPat != null && !MatchPropertyPattern(propPat, value, scope, visitor, bindings))
                    return false;

                if (trailingId != null && trailingId.GetText() != "_")
                    bindings[trailingId.GetText()] = (value, true);

                return true;
            }
        }
        else
        {
            // Scrutinee is NOT an ADT, OR check if pattern was explicitly an ADT variant!
            if (explicitParentAdt != null && visitor.Context.AdtDefinitions.ContainsKey(explicitParentAdt))
                return false;
            if (!hasParens && propPat == null && trailingId == null && genericSuffix == null &&
                visitor.Context.VariantDefinitions.ContainsKey(targetName))
            {
                // Pattern is a registered unit ADT variant (e.g. `Origin` or `None`), so non-ADT scrutinee fails!
                return false;
            }
        }

        // D. Record destructuring: `Point(x, y)`, `Point(x: px, y: py)`, `Point { x: px, y: py }`, `Point p`
        if (value.Kind == PilotValueKind.Record)
        {
            var rd = value.GetRecordDefinition();
            bool isRecordTarget =
                hasParens ||
                propPat != null ||
                trailingId != null ||
                visitor.Context.RecordDefinitions.ContainsKey(targetName);

            if (isRecordTarget && string.Equals(rd.Name, targetName, StringComparison.Ordinal))
            {
                if (hasParens && !MatchRecordFields(value, rd, patArgs, scope, visitor, bindings))
                    return false;
                if (propPat != null && !MatchPropertyPattern(propPat, value, scope, visitor, bindings))
                    return false;
                if (trailingId != null && trailingId.GetText() != "_")
                    bindings[trailingId.GetText()] = (value, true);
                return true;
            }

            if (hasParens || propPat != null || visitor.Context.RecordDefinitions.ContainsKey(targetName))
                return false;
        }
        else if (visitor.Context.RecordDefinitions.ContainsKey(targetName) && (hasParens || propPat != null || trailingId != null))
        {
            return false;
        }

        // E. Class / Struct Object matching: `Circle c`, `Point { X: 1, Y: 2 }`, `Vector2D(x, y)`
        if (value.Kind == PilotValueKind.Object)
        {
            var obj = value.AsObject();
            bool isClassOrCtorPattern =
                hasParens ||
                propPat != null ||
                trailingId != null ||
                (scope.TryResolve(targetName, out var fnVal) && fnVal.Kind == PilotValueKind.Function && fnVal.AsFunction().ClassDef != null);

            if (isClassOrCtorPattern)
            {
                if (!MatchesObjectType(obj, targetName, visitor.Context))
                    return false;

                if (hasParens && patArgs != null && patArgs.Length > 0)
                {
                    var fieldKeys = obj.Fields.Keys.ToList();
                    for (int i = 0; i < patArgs.Length; i++)
                    {
                        var arg = patArgs[i];
                        PilotValue fVal;
                        if (arg.identifier() != null)
                        {
                            fVal = obj.GetMember(arg.identifier().GetText(), visitor.Context);
                        }
                        else if (i < fieldKeys.Count)
                        {
                            fVal = obj.Fields[fieldKeys[i]];
                        }
                        else
                        {
                            return false;
                        }
                        if (fVal.IsUndefined || !MatchPatternInternal(arg.pattern(), fVal, scope, visitor, bindings))
                            return false;
                    }
                }

                if (propPat != null && !MatchPropertyPattern(propPat, value, scope, visitor, bindings))
                    return false;

                if (trailingId != null && trailingId.GetText() != "_")
                    bindings[trailingId.GetText()] = (value, true);

                return true;
            }
        }

        // F. Unqualified bare identifier capture: `case x =>`, `case head =>`
        if (explicitParentAdt == null &&
            genericSuffix == null &&
            !hasParens &&
            propPat == null &&
            trailingId == null &&
            nameParts.Length == 1 &&
            !visitor.Context.VariantDefinitions.ContainsKey(targetName) &&
            !visitor.Context.AdtDefinitions.ContainsKey(targetName) &&
            !visitor.Context.RecordDefinitions.ContainsKey(targetName) &&
            !(scope.TryResolve(targetName, out var resolved) && resolved.Kind == PilotValueKind.Function && resolved.AsFunction().ClassDef != null))
        {
            if (targetName != "_")
            {
                bindings[targetName] = (value, true);
            }
            return true;
        }

        return false;
    }

    private static bool MatchAdtFields(
        PilotValue adtValue,
        PilotAdtVariantDefinition vd,
        PilotParser.Pattern_argContext[]? patArgs,
        PilotScope scope,
        PilotInterpreterVisitor visitor,
        Dictionary<string, (PilotValue Value, bool IsConst)> bindings)
    {
        int argCount = patArgs?.Length ?? 0;
        if (argCount == 0) return vd.Arity == 0;

        bool anyNamed = false;
        for (int i = 0; i < argCount; i++)
        {
            if (patArgs![i].identifier() != null)
            {
                anyNamed = true;
                break;
            }
        }

        if (!anyNamed && argCount != vd.Arity)
            return false;

        int posIdx = 0;
        for (int i = 0; i < argCount; i++)
        {
            var arg = patArgs![i];
            int fieldIndex;
            if (arg.identifier() != null)
            {
                fieldIndex = vd.GetFieldIndex(arg.identifier().GetText());
                if (fieldIndex < 0) return false;
            }
            else
            {
                fieldIndex = posIdx++;
                if (fieldIndex >= vd.Arity) return false;
            }

            var fieldVal = adtValue.GetAdtField(fieldIndex);
            if (!MatchPatternInternal(arg.pattern(), fieldVal, scope, visitor, bindings))
                return false;
        }

        return true;
    }

    private static bool MatchRecordFields(
        PilotValue recValue,
        PilotRecordDefinition rd,
        PilotParser.Pattern_argContext[]? patArgs,
        PilotScope scope,
        PilotInterpreterVisitor visitor,
        Dictionary<string, (PilotValue Value, bool IsConst)> bindings)
    {
        int argCount = patArgs?.Length ?? 0;
        if (argCount == 0) return rd.Fields.Count == 0;

        bool anyNamed = patArgs!.Any(a => a.identifier() != null);
        if (!anyNamed && argCount != rd.Fields.Count)
            return false;

        int posIdx = 0;
        for (int i = 0; i < argCount; i++)
        {
            var arg = patArgs![i];
            int fieldIndex;
            if (arg.identifier() != null)
            {
                fieldIndex = rd.GetFieldIndex(arg.identifier().GetText());
                if (fieldIndex < 0) return false;
            }
            else
            {
                fieldIndex = posIdx++;
                if (fieldIndex >= rd.Fields.Count) return false;
            }

            var fieldVal = recValue.GetRecordField(fieldIndex);
            if (!MatchPatternInternal(arg.pattern(), fieldVal, scope, visitor, bindings))
                return false;
        }

        return true;
    }

    private static bool MatchPropertyPattern(
        PilotParser.Property_patternContext propCtx,
        PilotValue value,
        PilotScope scope,
        PilotInterpreterVisitor visitor,
        Dictionary<string, (PilotValue Value, bool IsConst)> bindings)
    {
        if (value.Kind == PilotValueKind.Null || value.Kind == PilotValueKind.Undefined)
            return false;

        foreach (var mem in propCtx.property_pattern_member())
        {
            string propName = mem.identifier().GetText();
            PilotValue propVal = PilotValue.Undefined;

            if (value.Kind == PilotValueKind.Object)
            {
                propVal = value.AsObject().GetMember(propName, visitor.Context);
            }
            else if (value.Kind == PilotValueKind.Record || value.Kind == PilotValueKind.Adt)
            {
                propVal = value.GetMember(propName, visitor.Context);
            }
            else if (value.Kind == PilotValueKind.Map)
            {
                var map = value.AsMap();
                if (!map.TryGetValue(PilotValue.FromString(propName), out propVal) &&
                    !map.TryGetValue(PilotValue.FromKeyword(propName), out propVal))
                {
                    return false;
                }
            }
            else
            {
                return false;
            }

            if (propVal.IsUndefined) return false;
            if (!MatchPatternInternal(mem.pattern(), propVal, scope, visitor, bindings))
                return false;
        }
        return true;
    }

    private static bool MatchesObjectType(PilotObject obj, string targetTypeName, PilotExecutionContext context)
    {
        if (obj.TypeName.Equals(targetTypeName, StringComparison.OrdinalIgnoreCase))
            return true;

        var cls = obj.ClassDef;
        int guard = 0;
        while (cls != null && guard++ < 32)
        {
            if (cls.Name.Equals(targetTypeName, StringComparison.OrdinalIgnoreCase))
                return true;
            if (!string.IsNullOrEmpty(cls.BaseClassName))
            {
                if (cls.BaseClassName.Equals(targetTypeName, StringComparison.OrdinalIgnoreCase))
                    return true;
                if (context.Environment.TryResolve(cls.BaseClassName, out var baseVal) &&
                    baseVal.Kind == PilotValueKind.Function &&
                    baseVal.AsFunction().ClassDef != null)
                {
                    cls = baseVal.AsFunction().ClassDef;
                    continue;
                }
            }
            break;
        }
        return false;
    }

    private static bool MatchesPrimitiveType(string typeName, PilotValue value)
    {
        return typeName.ToLowerInvariant() switch
        {
            "int" or "i8" or "i16" or "i32" or "i64" => value.Kind == PilotValueKind.Int,
            "uint" or "u8" or "u16" or "u32" or "u64" => value.Kind is PilotValueKind.UInt or PilotValueKind.Int && value.AsInt() >= 0,
            "float" or "f8" or "f16" or "f32" or "f64" or "double" => value.Kind == PilotValueKind.Float,
            "decimal" => value.Kind == PilotValueKind.Decimal,
            "str" or "string" => value.Kind == PilotValueKind.String,
            "bool" => value.Kind == PilotValueKind.Bool,
            "char" => value.Kind == PilotValueKind.Char || (value.Kind == PilotValueKind.String && value.ToString().Length == 1),
            "keyword" or "symbol" => value.Kind == PilotValueKind.Keyword,
            "list" => value.Kind == PilotValueKind.List,
            "tuple" => value.Kind == PilotValueKind.Tuple,
            "map" => value.Kind == PilotValueKind.Map,
            "set" => value.Kind == PilotValueKind.Set,
            "range" => value.Kind == PilotValueKind.Range,
            "matrix" => value.Kind == PilotValueKind.Matrix,
            "object" => value.Kind is PilotValueKind.Object or PilotValueKind.Record or PilotValueKind.Adt,
            _ => false
        };
    }

    private static bool MatchCollectionSlice(
        PilotParser.Slice_pattern_elementContext[] patternElements,
        IReadOnlyList<PilotValue> collection,
        PilotScope scope,
        PilotInterpreterVisitor visitor,
        Dictionary<string, (PilotValue Value, bool IsConst)> bindings)
    {
        int sliceIdx = -1;
        for (int i = 0; i < patternElements.Length; i++)
        {
            if (patternElements[i].DOTDOT() != null || patternElements[i].ELLIPSIS() != null || patternElements[i].STAR() != null)
            {
                sliceIdx = i;
                break;
            }
        }

        if (sliceIdx == -1)
        {
            if (collection.Count != patternElements.Length) return false;
            for (int i = 0; i < patternElements.Length; i++)
            {
                var pat = patternElements[i].pattern();
                if (pat != null && !MatchPatternInternal(pat, collection[i], scope, visitor, bindings))
                {
                    return false;
                }
            }
            return true;
        }

        int prefixCount = sliceIdx;
        int suffixCount = patternElements.Length - 1 - sliceIdx;

        if (collection.Count < prefixCount + suffixCount)
        {
            return false;
        }

        for (int i = 0; i < prefixCount; i++)
        {
            var pat = patternElements[i].pattern();
            if (pat != null && !MatchPatternInternal(pat, collection[i], scope, visitor, bindings))
            {
                return false;
            }
        }

        for (int i = 0; i < suffixCount; i++)
        {
            int colIdx = collection.Count - suffixCount + i;
            var pat = patternElements[sliceIdx + 1 + i].pattern();
            if (pat != null && !MatchPatternInternal(pat, collection[colIdx], scope, visitor, bindings))
            {
                return false;
            }
        }

        var slicePatCtx = patternElements[sliceIdx].pattern();
        if (slicePatCtx != null)
        {
            int sliceLen = collection.Count - prefixCount - suffixCount;
            var midItems = new List<PilotValue>(sliceLen);
            for (int i = 0; i < sliceLen; i++)
            {
                midItems.Add(collection[prefixCount + i]);
            }
            return MatchPatternInternal(slicePatCtx, PilotValue.FromList(midItems), scope, visitor, bindings);
        }

        return true;
    }
}
