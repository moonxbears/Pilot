using System;
using System.Collections.Generic;
using System.Linq;
using Pilot.Runtime.Environment;
using Pilot.Runtime.Values;

namespace Pilot.Runtime.Execution;

/// <summary>
/// Evaluates pattern matching rules against runtime <see cref="PilotValue"/> instances.
/// Implements collection slice patterns, structural patterns, relational checks, and variable binding.
/// </summary>
public static class PilotPatternMatcher
{
    public static bool Match(
        PilotParser.PatternContext context,
        PilotValue value,
        PilotScope scope,
        PilotInterpreterVisitor visitor)
    {
        if (context == null) return false;
        return MatchOrPattern(context.or_pattern(), value, scope, visitor);
    }

    private static bool MatchOrPattern(
        PilotParser.Or_patternContext context,
        PilotValue value,
        PilotScope scope,
        PilotInterpreterVisitor visitor)
    {
        foreach (var andCtx in context.and_pattern())
        {
            if (MatchAndPattern(andCtx, value, scope, visitor))
            {
                return true;
            }
        }
        return false;
    }

    private static bool MatchAndPattern(
        PilotParser.And_patternContext context,
        PilotValue value,
        PilotScope scope,
        PilotInterpreterVisitor visitor)
    {
        foreach (var notCtx in context.not_pattern())
        {
            if (!MatchNotPattern(notCtx, value, scope, visitor))
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
        PilotInterpreterVisitor visitor)
    {
        if (context.NOT() != null)
        {
            return !MatchNotPattern(context.not_pattern(), value, scope, visitor);
        }

        return MatchPrimaryPattern(context.primary_pattern(), value, scope, visitor);
    }

    private static bool MatchPrimaryPattern(
        PilotParser.Primary_patternContext context,
        PilotValue value,
        PilotScope scope,
        PilotInterpreterVisitor visitor)
    {
        if (context == null) return false;

        // Wildcard '_' matches everything
        if (context.UNDERSCORE() != null)
        {
            return true;
        }

        // Literal pattern: 42, "hello", true, etc.
        if (context.literal() != null)
        {
            var litVal = visitor.VisitLiteral(context.literal());
            return value.Equals(litVal);
        }

        // Relational pattern: > 10, <= 20
        if (context.relational_pattern() != null)
        {
            var relCtx = context.relational_pattern();
            var rhsVal = visitor.Visit(relCtx.shift_expr());
            if (relCtx.GREATER() != null) return value > rhsVal;
            if (relCtx.GREATEREQUAL() != null) return value >= rhsVal;
            if (relCtx.LESS() != null) return value < rhsVal;
            if (relCtx.LESSEQUAL() != null) return value <= rhsVal;
            if (relCtx.EQEQUAL() != null) return value == rhsVal;
            if (relCtx.NOTEQUAL() != null) return value != rhsVal;
            return false;
        }

        // Variable or Type pattern: let x, var x, int x, Vector2D(x, y)
        if (context.type_or_var_pattern() != null)
        {
            var tvCtx = context.type_or_var_pattern();
            if (tvCtx.identifier() != null)
            {
                string varName = tvCtx.identifier().GetText();
                scope.Declare(varName, value);
                return true;
            }

            if (tvCtx.dotted_name() != null)
            {
                string typeName = tvCtx.dotted_name().GetText();
                var lastId = tvCtx.identifier();

                // Known type names for type pattern: e.g. int x, str s
                var knownTypes = new HashSet<string>(StringComparer.OrdinalIgnoreCase)
                {
                    "int", "i8", "i16", "i32", "i64", "uint", "u8", "u16", "u32", "u64",
                    "float", "f8", "f16", "f32", "f64", "double", "decimal",
                    "str", "string", "bool", "char", "list", "tuple", "map", "set", "range", "matrix", "object"
                };

                if (knownTypes.Contains(typeName))
                {
                    if (typeName.Equals("int", StringComparison.OrdinalIgnoreCase) && value.Kind != PilotValueKind.Int) return false;
                    if (typeName.Equals("float", StringComparison.OrdinalIgnoreCase) && value.Kind != PilotValueKind.Float) return false;
                    if (typeName.Equals("str", StringComparison.OrdinalIgnoreCase) && value.Kind != PilotValueKind.String) return false;
                    if (typeName.Equals("bool", StringComparison.OrdinalIgnoreCase) && value.Kind != PilotValueKind.Bool) return false;

                    if (lastId != null)
                    {
                        scope.Declare(lastId.GetText(), value);
                    }
                    return true;
                }

                // Object type check or variable binding
                if (value.Kind == PilotValueKind.Object)
                {
                    if (!value.AsObject().TypeName.Equals(typeName, StringComparison.OrdinalIgnoreCase))
                        return false;

                    if (lastId != null)
                    {
                        scope.Declare(lastId.GetText(), value);
                    }
                    return true;
                }

                // If not a type test, treat identifier as variable capture: e.g. x, first, middle, last
                if (lastId == null && tvCtx.pattern()?.Length == 0 && tvCtx.property_pattern() == null)
                {
                    scope.Declare(typeName, value);
                    return true;
                }

                return false;
            }
        }

        // Collection pattern: [head, ..mid, tail] or [1, 2, 3]
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
            return MatchCollectionSlice(sliceElements, elements, scope, visitor);
        }

        // Tuple pattern: (a, b, c)
        if (context.tuple_pattern_match() != null)
        {
            if (value.Kind != PilotValueKind.Tuple && value.Kind != PilotValueKind.List)
            {
                return false;
            }

            var elements = value.Kind == PilotValueKind.Tuple ? value.AsTuple() : value.AsList();
            var patterns = context.tuple_pattern_match().pattern();
            if (elements.Count != patterns.Length) return false;

            for (int i = 0; i < patterns.Length; i++)
            {
                if (!Match(patterns[i], elements[i], scope, visitor)) return false;
            }
            return true;
        }

        // Property pattern: { x: 10, y: 20 }
        if (context.property_pattern() != null && value.Kind == PilotValueKind.Object)
        {
            var obj = value.AsObject();
            foreach (var mem in context.property_pattern().property_pattern_member())
            {
                string propName = mem.identifier().GetText();
                var propVal = obj.GetMember(propName);
                if (propVal.IsUndefined) return false;
                if (!Match(mem.pattern(), propVal, scope, visitor)) return false;
            }
            return true;
        }

        return false;
    }

    private static bool MatchCollectionSlice(
        PilotParser.Slice_pattern_elementContext[] patternElements,
        IReadOnlyList<PilotValue> collection,
        PilotScope scope,
        PilotInterpreterVisitor visitor)
    {
        int sliceIdx = -1;
        for (int i = 0; i < patternElements.Length; i++)
        {
            if (patternElements[i].DOTDOT() != null || patternElements[i].STAR() != null)
            {
                sliceIdx = i;
                break;
            }
        }

        // No slice (..) - exact count match required
        if (sliceIdx == -1)
        {
            if (collection.Count != patternElements.Length) return false;
            for (int i = 0; i < patternElements.Length; i++)
            {
                var pat = patternElements[i].pattern();
                if (pat != null && !Match(pat, collection[i], scope, visitor))
                {
                    return false;
                }
            }
            return true;
        }

        // With slice (e.g. [first, ..rest] or [first, ..mid, last])
        int prefixCount = sliceIdx;
        int suffixCount = patternElements.Length - 1 - sliceIdx;

        if (collection.Count < prefixCount + suffixCount)
        {
            return false;
        }

        // Match prefix
        for (int i = 0; i < prefixCount; i++)
        {
            var pat = patternElements[i].pattern();
            if (pat != null && !Match(pat, collection[i], scope, visitor))
            {
                return false;
            }
        }

        // Match suffix
        for (int i = 0; i < suffixCount; i++)
        {
            int colIdx = collection.Count - suffixCount + i;
            var pat = patternElements[sliceIdx + 1 + i].pattern();
            if (pat != null && !Match(pat, collection[colIdx], scope, visitor))
            {
                return false;
            }
        }

        // Bind middle slice if specified
        var slicePatCtx = patternElements[sliceIdx].pattern();
        if (slicePatCtx != null)
        {
            int sliceLen = collection.Count - prefixCount - suffixCount;
            var midItems = new List<PilotValue>(sliceLen);
            for (int i = 0; i < sliceLen; i++)
            {
                midItems.Add(collection[prefixCount + i]);
            }
            return Match(slicePatCtx, PilotValue.FromList(midItems), scope, visitor);
        }

        return true;
    }
}
