using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using Antlr4.Runtime.Tree;
using Pilot.Runtime.Bytecode;
using Pilot.Runtime.Values;

namespace Pilot.Compiler;

/// <summary>
/// Compiles Pilot <c>match</c> statements/expressions and conditional pattern bindings
/// (<c>if let</c>, <c>elif let</c>, <c>while let</c>, <c>guard let</c>) into PilVM
/// DAG decision trees and dense O(1) <see cref="PilotOpcode.JumpTable"/> dispatch blocks.
/// Shared outer variant/integer discriminants are tested at most once, and arm bodies are
/// emitted once as shared DAG join targets.
/// </summary>
internal sealed class PilotMatchBytecodeLowerer
{
    private readonly PilotBytecodeCompiler _compiler;
    private readonly PilotBytecodeChunk _chunk;

    public PilotMatchBytecodeLowerer(PilotBytecodeCompiler compiler)
    {
        _compiler = compiler;
        _chunk = compiler.Chunk;
    }

    private sealed class StagedBinding
    {
        public string Name { get; }
        public int StagedSlot { get; }
        public bool IsMutable { get; set; }

        public StagedBinding(string name, int stagedSlot, bool isMutable)
        {
            Name = name;
            StagedSlot = stagedSlot;
            IsMutable = isMutable;
        }
    }

    private enum ArmBranchKind
    {
        CatchAll,
        AdtVariant,
        BoolLiteral,
        IntValues,
        General
    }

    private sealed class AnalyzedBranch
    {
        public int ArmIndex { get; init; }
        public PilotParser.And_patternContext BranchAst { get; init; } = null!;
        public PilotParser.TestContext? GuardAst { get; init; }
        public ArmBranchKind Kind { get; init; }
        public PilotBytecodeTypeDescriptor? Adt { get; init; }
        public int VariantTag { get; init; } = -1;
        public PilotSerializedVariantDescriptor? Variant { get; init; }
        public bool BoolValue { get; init; }
        public long IntLo { get; init; }
        public long IntHi { get; init; }
        public bool IsIrrefutableGivenTag { get; init; }
    }

    // =========================================================================
    // 1. Match Statement & Match Expression Lowering (O(1) JumpTable + DAG)
    // =========================================================================

    public void LowerMatch(PilotParser.Match_stmtContext context, bool isExpression)
    {
        bool produceValue = isExpression || !_compiler.DiscardExpressionResult;
        var arms = context.match_suite()?.match_arm() ?? Array.Empty<PilotParser.Match_armContext>();

        // Evaluate scrutinee exactly once
        bool prevDiscard = _compiler.DiscardExpressionResult;
        _compiler.DiscardExpressionResult = false;
        _compiler.Visit(context.test());
        _compiler.DiscardExpressionResult = prevDiscard;

        int scrutSlot = _compiler.AllocateTempSlot("match_scrut");
        _chunk.Emit(PilotOpcode.StoreLocal, scrutSlot);

        int resultSlot = -1;
        if (produceValue)
        {
            resultSlot = _compiler.AllocateTempSlot("match_res");
            _chunk.Emit(PilotOpcode.ConstUndefined);
            _chunk.Emit(PilotOpcode.StoreLocal, resultSlot);
        }

        if (arms.Length == 0)
        {
            if (produceValue)
            {
                _chunk.Emit(PilotOpcode.LoadLocal, resultSlot);
            }
            return;
        }

        var branches = AnalyzeArmBranches(arms);
        var incomingBodyJumps = new List<int>[arms.Length];
        for (int i = 0; i < arms.Length; i++)
        {
            incomingBodyJumps[i] = new List<int>();
        }
        var unmatchedExitJumps = new List<int>();

        // Try O(1) ADT JumpTable lowering first, then O(1) Bool JumpTable, then O(1) Dense Int JumpTable,
        // otherwise lower through general DAG branch chain.
        if (TryEmitAdtJumpTable(scrutSlot, branches, incomingBodyJumps, unmatchedExitJumps))
        {
            // Dispatched via O(1) GetAdtTag + JumpTable
        }
        else if (TryEmitBoolJumpTable(scrutSlot, branches, incomingBodyJumps, unmatchedExitJumps))
        {
            // Dispatched via O(1) Bool JumpTable
        }
        else if (TryEmitDenseIntJumpTable(scrutSlot, branches, incomingBodyJumps, unmatchedExitJumps))
        {
            // Dispatched via O(1) Dense Integer JumpTable
        }
        else
        {
            EmitBranchChain(scrutSlot, branches, skipOuterCheck: false, incomingBodyJumps, unmatchedExitJumps);
        }

        // Emit each arm's suite body ONCE (DAG join targets)
        var armEndJumps = new List<int>(arms.Length);
        for (int i = 0; i < arms.Length; i++)
        {
            // Pre-declare pattern bindings in compiler metadata so body references resolve to local slots inside functions
            PredeclarePatternBindingsInCompiler(arms[i].pattern());

            int bodyStartIp = _chunk.InstructionCount;
            foreach (int jmp in incomingBodyJumps[i])
            {
                _chunk.PatchJump(jmp, bodyStartIp);
            }

            EmitArmBody(arms[i].suite(), produceValue, resultSlot, prevDiscard);
            armEndJumps.Add(_chunk.EmitJump(PilotOpcode.Jump));
        }

        int matchEndIp = _chunk.InstructionCount;
        foreach (int ej in unmatchedExitJumps)
        {
            _chunk.PatchJump(ej, matchEndIp);
        }
        foreach (int aj in armEndJumps)
        {
            _chunk.PatchJump(aj, matchEndIp);
        }

        if (produceValue)
        {
            _chunk.Emit(PilotOpcode.LoadLocal, resultSlot);
        }
    }

    private List<AnalyzedBranch> AnalyzeArmBranches(PilotParser.Match_armContext[] arms)
    {
        var list = new List<AnalyzedBranch>();
        for (int i = 0; i < arms.Length; i++)
        {
            var arm = arms[i];
            var guard = arm.test();
            var orParts = arm.pattern()?.or_pattern()?.and_pattern();
            if (orParts == null || orParts.Length == 0) continue;

            foreach (var branch in orParts)
            {
                list.Add(ClassifyBranch(i, branch, guard));
            }
        }
        return list;
    }

    private AnalyzedBranch ClassifyBranch(
        int armIndex,
        PilotParser.And_patternContext branch,
        PilotParser.TestContext? guard)
    {
        var notPats = branch.not_pattern();
        if (notPats == null || notPats.Length != 1)
        {
            return new AnalyzedBranch { ArmIndex = armIndex, BranchAst = branch, GuardAst = guard, Kind = ArmBranchKind.General };
        }

        // Unwrap `@` bindings (`sq @ Inner`) to inspect structural discriminant
        var cur = notPats[0];
        while (cur.AT() != null && cur.not_pattern() != null)
        {
            cur = cur.not_pattern();
        }

        if (cur.NOT() != null || cur.primary_pattern() == null)
        {
            return new AnalyzedBranch { ArmIndex = armIndex, BranchAst = branch, GuardAst = guard, Kind = ArmBranchKind.General };
        }

        var prim = cur.primary_pattern();

        // 1. Wildcard `_`
        if (prim.UNDERSCORE() != null)
        {
            return new AnalyzedBranch
            {
                ArmIndex = armIndex,
                BranchAst = branch,
                GuardAst = guard,
                Kind = ArmBranchKind.CatchAll,
                IsIrrefutableGivenTag = guard == null
            };
        }

        // 2. Parenthesized single pattern `(inner)`
        if (prim.tuple_pattern_match() is { } tup &&
            tup.pattern()?.Length == 1 &&
            (tup.COMMA() == null || tup.COMMA().Length == 0))
        {
            var innerOr = tup.pattern(0).or_pattern()?.and_pattern();
            if (innerOr?.Length == 1)
            {
                var innerClassified = ClassifyBranch(armIndex, innerOr[0], guard);
                return new AnalyzedBranch
                {
                    ArmIndex = armIndex,
                    BranchAst = branch,
                    GuardAst = guard,
                    Kind = innerClassified.Kind,
                    Adt = innerClassified.Adt,
                    VariantTag = innerClassified.VariantTag,
                    Variant = innerClassified.Variant,
                    BoolValue = innerClassified.BoolValue,
                    IntLo = innerClassified.IntLo,
                    IntHi = innerClassified.IntHi,
                    IsIrrefutableGivenTag = innerClassified.IsIrrefutableGivenTag
                };
            }
        }

        // 3. Boolean / Integer Literals
        if (prim.literal() is { } lit)
        {
            if (lit.TRUE() != null || lit.FALSE() != null)
            {
                return new AnalyzedBranch
                {
                    ArmIndex = armIndex,
                    BranchAst = branch,
                    GuardAst = guard,
                    Kind = ArmBranchKind.BoolLiteral,
                    BoolValue = lit.TRUE() != null,
                    IsIrrefutableGivenTag = guard == null
                };
            }
            if (TryParseLiteralInt(lit, out long litInt))
            {
                return new AnalyzedBranch
                {
                    ArmIndex = armIndex,
                    BranchAst = branch,
                    GuardAst = guard,
                    Kind = ArmBranchKind.IntValues,
                    IntLo = litInt,
                    IntHi = litInt,
                    IsIrrefutableGivenTag = guard == null
                };
            }
        }

        if (prim.signed_number_pattern() is { } snp && snp.FLOAT_NUMBER() == null)
        {
            long negVal = -ParseUnsignedIntToken(snp.INTEGER()?.GetText(), snp.HEX_INTEGER()?.GetText(), snp.BIN_INTEGER()?.GetText(), snp.OCT_INTEGER()?.GetText());
            return new AnalyzedBranch
            {
                ArmIndex = armIndex,
                BranchAst = branch,
                GuardAst = guard,
                Kind = ArmBranchKind.IntValues,
                IntLo = negVal,
                IntHi = negVal,
                IsIrrefutableGivenTag = guard == null
            };
        }

        // 4. Integer Range `lo..hi` / `lo..<hi`
        if (prim.range_pattern() is { } rp && TryExtractIntRange(rp, out long rLo, out long rHi) && rLo <= rHi)
        {
            return new AnalyzedBranch
            {
                ArmIndex = armIndex,
                BranchAst = branch,
                GuardAst = guard,
                Kind = ArmBranchKind.IntValues,
                IntLo = rLo,
                IntHi = rHi,
                IsIrrefutableGivenTag = guard == null
            };
        }

        // 5. Type / Variant / Catch-All Identifier
        if (prim.type_or_var_pattern() is { } tv)
        {
            // `let x` or `var x`
            if ((tv.LET() != null || tv.VAR() != null) && tv.dotted_name() == null && tv.identifier() != null)
            {
                return new AnalyzedBranch
                {
                    ArmIndex = armIndex,
                    BranchAst = branch,
                    GuardAst = guard,
                    Kind = ArmBranchKind.CatchAll,
                    IsIrrefutableGivenTag = guard == null
                };
            }

            if (tv.dotted_name() != null)
            {
                var parts = tv.dotted_name().identifier();
                string fullDotted = tv.dotted_name().GetText();
                var genSuffix = tv.generic_pattern_suffix();
                bool hasParens = tv.LPAR() != null;
                var patArgs = tv.pattern_arg();
                var propPat = tv.property_pattern();
                var trailingId = tv.identifier();

                string? explicitParent = null;
                string targetName = fullDotted;
                if (genSuffix?.identifier() != null)
                {
                    explicitParent = fullDotted;
                    targetName = genSuffix.identifier().GetText();
                }
                else if (parts.Length == 2)
                {
                    explicitParent = parts[0].GetText();
                    targetName = parts[1].GetText();
                }

                // ADT Variant!
                if (propPat == null &&
                    _compiler.TryResolveAdtVariant(explicitParent, targetName, out var adtDesc, out int vTag, out var varDesc))
                {
                    int argCount = patArgs?.Length ?? 0;
                    bool arityMatches = (!hasParens && varDesc.Fields.Count == 0) ||
                                        (hasParens && argCount == varDesc.Fields.Count);
                    if (arityMatches)
                    {
                        bool fieldsIrrefutable = true;
                        if (hasParens && patArgs != null)
                        {
                            foreach (var pa in patArgs)
                            {
                                if (!IsIrrefutableSubpattern(pa.pattern()))
                                {
                                    fieldsIrrefutable = false;
                                    break;
                                }
                            }
                        }

                        return new AnalyzedBranch
                        {
                            ArmIndex = armIndex,
                            BranchAst = branch,
                            GuardAst = guard,
                            Kind = ArmBranchKind.AdtVariant,
                            Adt = adtDesc,
                            VariantTag = vTag,
                            Variant = varDesc,
                            IsIrrefutableGivenTag = guard == null && fieldsIrrefutable
                        };
                    }
                }

                // Bare catch-all variable (`default`, `other`, `x`) when not a known type/record/variant
                if (explicitParent == null && genSuffix == null && !hasParens && propPat == null && trailingId == null &&
                    !IsPrimitiveTypeName(targetName) &&
                    !_compiler.TryResolveRecordType(targetName, out _) &&
                    !_compiler.TryResolveAdtType(targetName, out _) &&
                    !_compiler.IsKnownClassOrStructName(targetName))
                {
                    return new AnalyzedBranch
                    {
                        ArmIndex = armIndex,
                        BranchAst = branch,
                        GuardAst = guard,
                        Kind = ArmBranchKind.CatchAll,
                        IsIrrefutableGivenTag = guard == null
                    };
                }
            }
        }

        return new AnalyzedBranch { ArmIndex = armIndex, BranchAst = branch, GuardAst = guard, Kind = ArmBranchKind.General };
    }

    private bool TryEmitAdtJumpTable(
        int scrutSlot,
        List<AnalyzedBranch> branches,
        List<int>[] incomingBodyJumps,
        List<int> unmatchedExitJumps)
    {
        PilotBytecodeTypeDescriptor? targetAdt = null;
        int adtBranchCount = 0;

        foreach (var b in branches)
        {
            if (b.Kind == ArmBranchKind.CatchAll) continue;
            if (b.Kind != ArmBranchKind.AdtVariant || b.Adt == null)
                return false;

            if (targetAdt == null)
            {
                targetAdt = b.Adt;
            }
            else if (!string.Equals(targetAdt.Name, b.Adt.Name, StringComparison.Ordinal))
            {
                return false;
            }
            adtBranchCount++;
        }

        if (targetAdt == null || adtBranchCount == 0 || targetAdt.Variants.Count == 0)
            return false;

        int variantCount = targetAdt.Variants.Count;
        var targets = new int[variantCount];
        var jumpTable = new PilotJumpTable(0, targets, 0);
        int jtIndex = _chunk.AddJumpTable(jumpTable);
        int adtId = _chunk.AddIdentifier(targetAdt.Name);

        // Emit O(1) tag extraction + jump table dispatch
        _chunk.Emit(PilotOpcode.LoadLocal, scrutSlot);
        _chunk.Emit(PilotOpcode.GetAdtTag, adtId);
        _chunk.Emit(PilotOpcode.JumpTable, jtIndex);

        var tagFallthroughToDefaultJumps = new List<int>();

        // Emit specialized decision chain per variant tag 0..K-1 (outer tag is ALREADY verified!)
        for (int tag = 0; tag < variantCount; tag++)
        {
            targets[tag] = _chunk.InstructionCount;

            var applicable = new List<AnalyzedBranch>();
            foreach (var b in branches)
            {
                if (b.Kind == ArmBranchKind.CatchAll || (b.Kind == ArmBranchKind.AdtVariant && b.VariantTag == tag))
                {
                    applicable.Add(b);
                    if (b.IsIrrefutableGivenTag)
                    {
                        break;
                    }
                }
            }

            if (applicable.Count == 0)
            {
                tagFallthroughToDefaultJumps.Add(_chunk.EmitJump(PilotOpcode.Jump));
            }
            else
            {
                EmitBranchChain(scrutSlot, applicable, skipOuterCheck: true, incomingBodyJumps, unmatchedExitJumps);
            }
        }

        // Default target for non-ADT dynamic values (runs CatchAll branches if any, else exits match)
        jumpTable.DefaultTarget = _chunk.InstructionCount;
        foreach (int jmp in tagFallthroughToDefaultJumps)
        {
            _chunk.PatchJump(jmp, jumpTable.DefaultTarget);
        }

        var catchAlls = branches.Where(b => b.Kind == ArmBranchKind.CatchAll).ToList();
        if (catchAlls.Count > 0)
        {
            EmitBranchChain(scrutSlot, catchAlls, skipOuterCheck: false, incomingBodyJumps, unmatchedExitJumps);
        }
        else
        {
            unmatchedExitJumps.Add(_chunk.EmitJump(PilotOpcode.Jump));
        }

        return true;
    }

    private bool TryEmitBoolJumpTable(
        int scrutSlot,
        List<AnalyzedBranch> branches,
        List<int>[] incomingBodyJumps,
        List<int> unmatchedExitJumps)
    {
        int boolCount = 0;
        foreach (var b in branches)
        {
            if (b.Kind == ArmBranchKind.CatchAll) continue;
            if (b.Kind != ArmBranchKind.BoolLiteral) return false;
            boolCount++;
        }
        if (boolCount == 0) return false;

        var targets = new int[2];
        var jt = new PilotJumpTable(0, targets, 0);
        int jtIndex = _chunk.AddJumpTable(jt);

        // Verify runtime value is bool before indexing 0..1 JumpTable so Int(0)/Int(1) don't collide dynamically
        int boolTypeId = _chunk.AddIdentifier("bool");
        _chunk.Emit(PilotOpcode.LoadLocal, scrutSlot);
        _chunk.Emit(PilotOpcode.CheckRecordType, boolTypeId, extra: -1);
        int nonBoolJump = _chunk.EmitJump(PilotOpcode.JumpIfFalse);

        _chunk.Emit(PilotOpcode.LoadLocal, scrutSlot);
        _chunk.Emit(PilotOpcode.JumpTable, jtIndex);

        for (int bit = 0; bit <= 1; bit++)
        {
            bool targetVal = bit == 1;
            targets[bit] = _chunk.InstructionCount;
            var applicable = new List<AnalyzedBranch>();
            foreach (var b in branches)
            {
                if (b.Kind == ArmBranchKind.CatchAll || (b.Kind == ArmBranchKind.BoolLiteral && b.BoolValue == targetVal))
                {
                    applicable.Add(b);
                    if (b.IsIrrefutableGivenTag) break;
                }
            }
            EmitBranchChain(scrutSlot, applicable, skipOuterCheck: true, incomingBodyJumps, unmatchedExitJumps);
        }

        jt.DefaultTarget = _chunk.InstructionCount;
        _chunk.PatchJump(nonBoolJump, jt.DefaultTarget);

        var catchAlls = branches.Where(b => b.Kind == ArmBranchKind.CatchAll).ToList();
        if (catchAlls.Count > 0)
        {
            EmitBranchChain(scrutSlot, catchAlls, skipOuterCheck: false, incomingBodyJumps, unmatchedExitJumps);
        }
        else
        {
            unmatchedExitJumps.Add(_chunk.EmitJump(PilotOpcode.Jump));
        }

        return true;
    }

    private bool TryEmitDenseIntJumpTable(
        int scrutSlot,
        List<AnalyzedBranch> branches,
        List<int>[] incomingBodyJumps,
        List<int> unmatchedExitJumps)
    {
        int intBranchCount = 0;
        long minVal = long.MaxValue;
        long maxVal = long.MinValue;

        foreach (var b in branches)
        {
            if (b.Kind == ArmBranchKind.CatchAll) continue;
            if (b.Kind != ArmBranchKind.IntValues) return false;
            if (b.IntLo < minVal) minVal = b.IntLo;
            if (b.IntHi > maxVal) maxVal = b.IntHi;
            intBranchCount++;
        }

        if (intBranchCount < 2 || maxVal < minVal) return false;
        // Emit dense O(1) table when span fits cleanly in <= 512 entries (covers u8 0..255, HTTP codes, opcodes, etc.)
        if ((ulong)(maxVal - minVal) > 512UL) return false;

        int span = (int)(maxVal - minVal) + 1;
        var targets = new int[span];
        var jt = new PilotJumpTable(minVal, targets, 0);
        int jtIndex = _chunk.AddJumpTable(jt);

        int intTypeId = _chunk.AddIdentifier("int");
        _chunk.Emit(PilotOpcode.LoadLocal, scrutSlot);
        _chunk.Emit(PilotOpcode.CheckRecordType, intTypeId, extra: -1);
        int nonIntJump = _chunk.EmitJump(PilotOpcode.JumpIfFalse);

        _chunk.Emit(PilotOpcode.LoadLocal, scrutSlot);
        _chunk.Emit(PilotOpcode.JumpTable, jtIndex);

        // Deduplicate identical per-value chains so a range like 200..299 emits ONE handler block shared by 100 table slots!
        var chainIpCache = new Dictionary<string, int>(StringComparer.Ordinal);

        for (int offset = 0; offset < span; offset++)
        {
            long val = minVal + offset;
            var applicable = new List<AnalyzedBranch>();
            foreach (var b in branches)
            {
                if (b.Kind == ArmBranchKind.CatchAll ||
                    (b.Kind == ArmBranchKind.IntValues && val >= b.IntLo && val <= b.IntHi))
                {
                    applicable.Add(b);
                    if (b.IsIrrefutableGivenTag) break;
                }
            }

            string chainKey = string.Join(",", applicable.Select(a => $"{a.ArmIndex}:{branches.IndexOf(a)}"));
            if (!chainIpCache.TryGetValue(chainKey, out int handlerIp))
            {
                handlerIp = _chunk.InstructionCount;
                chainIpCache[chainKey] = handlerIp;
                EmitBranchChain(scrutSlot, applicable, skipOuterCheck: true, incomingBodyJumps, unmatchedExitJumps);
            }
            targets[offset] = handlerIp;
        }

        jt.DefaultTarget = _chunk.InstructionCount;
        _chunk.PatchJump(nonIntJump, jt.DefaultTarget);

        var catchAlls = branches.Where(b => b.Kind == ArmBranchKind.CatchAll).ToList();
        if (catchAlls.Count > 0)
        {
            EmitBranchChain(scrutSlot, catchAlls, skipOuterCheck: false, incomingBodyJumps, unmatchedExitJumps);
        }
        else
        {
            unmatchedExitJumps.Add(_chunk.EmitJump(PilotOpcode.Jump));
        }

        return true;
    }

    private void EmitBranchChain(
        int scrutSlot,
        List<AnalyzedBranch> chain,
        bool skipOuterCheck,
        List<int>[] incomingBodyJumps,
        List<int> unmatchedExitJumps)
    {
        for (int i = 0; i < chain.Count; i++)
        {
            var b = chain[i];
            var failJumps = new List<int>();
            var staged = new Dictionary<string, StagedBinding>(StringComparer.Ordinal);

            bool omitOuter = skipOuterCheck &&
                             (b.Kind is ArmBranchKind.AdtVariant or ArmBranchKind.BoolLiteral or ArmBranchKind.IntValues);

            EmitAndPatternTest(b.BranchAst, scrutSlot, omitOuter, staged, failJumps);

            if (b.GuardAst != null)
            {
                // Save shadowed bindings if any so failed guards restore outer values cleanly
                var savedPrevSlots = new List<(string Name, int SavedSlot)>();
                foreach (var kvp in staged)
                {
                    if (_compiler.HasDeclaredBinding(kvp.Key))
                    {
                        int savedSlot = _compiler.AllocateTempSlot($"pat_prev_{kvp.Key}");
                        _compiler.EmitLoadBindingOrUndefined(kvp.Key);
                        _chunk.Emit(PilotOpcode.StoreLocal, savedSlot);
                        savedPrevSlots.Add((kvp.Key, savedSlot));
                    }
                }

                CommitStagedBindings(staged);

                bool prevDiscard = _compiler.DiscardExpressionResult;
                _compiler.DiscardExpressionResult = false;
                _compiler.Visit(b.GuardAst);
                _compiler.DiscardExpressionResult = prevDiscard;

                if (savedPrevSlots.Count == 0)
                {
                    failJumps.Add(_chunk.EmitJump(PilotOpcode.JumpIfFalse));
                }
                else
                {
                    int guardTrueJump = _chunk.EmitJump(PilotOpcode.JumpIfTrue);
                    foreach (var (name, savedSlot) in savedPrevSlots)
                    {
                        _chunk.Emit(PilotOpcode.LoadLocal, savedSlot);
                        _compiler.EmitRestoreBinding(name);
                    }
                    failJumps.Add(_chunk.EmitJump(PilotOpcode.Jump));
                    _chunk.PatchJump(guardTrueJump);
                }
            }
            else
            {
                CommitStagedBindings(staged);
            }

            incomingBodyJumps[b.ArmIndex].Add(_chunk.EmitJump(PilotOpcode.Jump));

            int nextBranchIp = _chunk.InstructionCount;
            foreach (int fj in failJumps)
            {
                _chunk.PatchJump(fj, nextBranchIp);
            }

            if (b.IsIrrefutableGivenTag && skipOuterCheck)
            {
                return;
            }
            if (b.Kind == ArmBranchKind.CatchAll && b.GuardAst == null)
            {
                return;
            }
        }

        unmatchedExitJumps.Add(_chunk.EmitJump(PilotOpcode.Jump));
    }

    private void EmitArmBody(
        PilotParser.SuiteContext suite,
        bool produceValue,
        int resultSlot,
        bool outerDiscard)
    {
        if (!produceValue)
        {
            bool prev = _compiler.DiscardExpressionResult;
            _compiler.DiscardExpressionResult = true;
            _compiler.VisitSuite(suite);
            _compiler.DiscardExpressionResult = prev;
            return;
        }

        if (SuiteLeavesExpressionOnStack(suite))
        {
            bool prev = _compiler.DiscardExpressionResult;
            _compiler.DiscardExpressionResult = false;
            _compiler.VisitSuite(suite);
            _compiler.DiscardExpressionResult = prev;
            _chunk.Emit(PilotOpcode.StoreLocal, resultSlot);
        }
        else
        {
            bool prev = _compiler.DiscardExpressionResult;
            _compiler.DiscardExpressionResult = true;
            _compiler.VisitSuite(suite);
            _compiler.DiscardExpressionResult = prev;
            _chunk.Emit(PilotOpcode.ConstUndefined);
            _chunk.Emit(PilotOpcode.StoreLocal, resultSlot);
        }
    }

    private static bool SuiteLeavesExpressionOnStack(PilotParser.SuiteContext suite)
    {
        if (suite.simple_stmt() != null)
        {
            return SimpleStmtLeavesExpressionOnStack(suite.simple_stmt());
        }
        var stmts = suite.stmt();
        if (stmts == null || stmts.Length == 0) return false;
        var last = stmts[^1];
        if (last.simple_stmt() != null)
        {
            return SimpleStmtLeavesExpressionOnStack(last.simple_stmt());
        }
        if (last.compound_stmt()?.match_stmt() != null)
        {
            return true;
        }
        return false;
    }

    private static bool SimpleStmtLeavesExpressionOnStack(PilotParser.Simple_stmtContext simple)
    {
        var smalls = simple.small_stmt();
        if (smalls == null || smalls.Length == 0) return false;
        var exprStmt = smalls[^1].expr_stmt();
        if (exprStmt == null) return false;
        if (exprStmt.EQUAL()?.Length > 0 || exprStmt.WALRUS()?.Length > 0 || exprStmt.augassign()?.Length > 0)
            return false;
        return exprStmt.testlist()?.test()?.Length > 0;
    }

    // =========================================================================
    // 2. Conditional Pattern Lowering: `if let`, `while let`, `guard let`
    // =========================================================================

    public void LowerIfStmt(PilotParser.If_stmtContext context)
    {
        var tests = context.test();
        var suites = context.suite();
        var patterns = context.pattern();
        var guards = context.pattern_guard();
        var endJumps = new List<int>();

        for (int i = 0; i < tests.Length; i++)
        {
            var prevSuite = i > 0 ? suites[i - 1] : null;
            var (branchPat, isVar) = FindBranchPatternAndMutability(context, patterns, tests[i], prevSuite);
            var branchGuard = FindBranchPatternGuard(guards, tests[i], suites[i]);

            if (branchPat != null)
            {
                bool prevDiscard = _compiler.DiscardExpressionResult;
                _compiler.DiscardExpressionResult = false;
                _compiler.Visit(tests[i]);
                _compiler.DiscardExpressionResult = prevDiscard;

                int scrutSlot = _compiler.AllocateTempSlot("iflet_scrut");
                _chunk.Emit(PilotOpcode.StoreLocal, scrutSlot);

                var failJumps = new List<int>();
                if (IsBareOptionalBindingPattern(branchPat))
                {
                    _chunk.Emit(PilotOpcode.LoadLocal, scrutSlot);
                    failJumps.Add(_chunk.EmitJump(PilotOpcode.JumpIfNull));
                }

                var staged = new Dictionary<string, StagedBinding>(StringComparer.Ordinal);
                EmitPatternTest(branchPat, scrutSlot, staged, failJumps, defaultMutable: isVar);
                EmitCommitWithOptionalGuard(staged, branchGuard?.test(), failJumps);

                _compiler.Visit(suites[i]);
                endJumps.Add(_chunk.EmitJump(PilotOpcode.Jump));

                int nextIp = _chunk.InstructionCount;
                foreach (int fj in failJumps) _chunk.PatchJump(fj, nextIp);
            }
            else
            {
                bool prevDiscard = _compiler.DiscardExpressionResult;
                _compiler.DiscardExpressionResult = false;
                _compiler.Visit(tests[i]);
                _compiler.DiscardExpressionResult = prevDiscard;

                int falseJump = _chunk.EmitJump(PilotOpcode.JumpIfFalse);
                _compiler.Visit(suites[i]);
                endJumps.Add(_chunk.EmitJump(PilotOpcode.Jump));
                _chunk.PatchJump(falseJump);
            }
        }

        if (suites.Length > tests.Length)
        {
            _compiler.Visit(suites[^1]);
        }

        foreach (int ej in endJumps)
        {
            _chunk.PatchJump(ej);
        }
    }

    public void LowerWhileLetStmt(PilotParser.While_stmtContext context, int loopStart, List<int> exitJumps)
    {
        var pat = context.pattern()!;
        bool isMutable = context.VAR() != null;

        bool prevDiscard = _compiler.DiscardExpressionResult;
        _compiler.DiscardExpressionResult = false;
        _compiler.Visit(context.test(0));
        _compiler.DiscardExpressionResult = prevDiscard;

        int scrutSlot = _compiler.AllocateTempSlot("whilelet_scrut");
        _chunk.Emit(PilotOpcode.StoreLocal, scrutSlot);

        if (IsBareOptionalBindingPattern(pat))
        {
            _chunk.Emit(PilotOpcode.LoadLocal, scrutSlot);
            exitJumps.Add(_chunk.EmitJump(PilotOpcode.JumpIfNull));
        }

        var staged = new Dictionary<string, StagedBinding>(StringComparer.Ordinal);
        EmitPatternTest(pat, scrutSlot, staged, exitJumps, defaultMutable: isMutable);
        EmitCommitWithOptionalGuard(staged, context.pattern_guard()?.test(), exitJumps);
    }

    public void LowerGuardStmt(PilotParser.Guard_stmtContext context)
    {
        var pat = context.pattern();
        if (pat == null)
        {
            bool prevDiscard = _compiler.DiscardExpressionResult;
            _compiler.DiscardExpressionResult = false;
            _compiler.Visit(context.test());
            _compiler.DiscardExpressionResult = prevDiscard;

            int passJump = _chunk.EmitJump(PilotOpcode.JumpIfTrue);
            bool prevBodyDiscard = _compiler.DiscardExpressionResult;
            _compiler.DiscardExpressionResult = true;
            _compiler.Visit(context.suite());
            _compiler.DiscardExpressionResult = prevBodyDiscard;
            _chunk.PatchJump(passJump);
            return;
        }

        bool isMutable = context.VAR() != null;
        bool prev = _compiler.DiscardExpressionResult;
        _compiler.DiscardExpressionResult = false;
        _compiler.Visit(context.test());
        _compiler.DiscardExpressionResult = prev;

        int scrutSlot = _compiler.AllocateTempSlot("guardlet_scrut");
        _chunk.Emit(PilotOpcode.StoreLocal, scrutSlot);

        var failJumps = new List<int>();
        if (IsBareOptionalBindingPattern(pat))
        {
            _chunk.Emit(PilotOpcode.LoadLocal, scrutSlot);
            failJumps.Add(_chunk.EmitJump(PilotOpcode.JumpIfNull));
        }

        var staged = new Dictionary<string, StagedBinding>(StringComparer.Ordinal);
        EmitPatternTest(pat, scrutSlot, staged, failJumps, defaultMutable: isMutable);
        EmitCommitWithOptionalGuard(staged, context.pattern_guard()?.test(), failJumps);
        int okJump = _chunk.EmitJump(PilotOpcode.Jump);

        int elseIp = _chunk.InstructionCount;
        foreach (int fj in failJumps) _chunk.PatchJump(fj, elseIp);

        bool prevSuiteDiscard = _compiler.DiscardExpressionResult;
        _compiler.DiscardExpressionResult = true;
        _compiler.Visit(context.suite());
        _compiler.DiscardExpressionResult = prevSuiteDiscard;

        _chunk.PatchJump(okJump);
    }

    private void EmitCommitWithOptionalGuard(
        Dictionary<string, StagedBinding> staged,
        PilotParser.TestContext? guardExpr,
        List<int> failJumps)
    {
        if (guardExpr == null)
        {
            CommitStagedBindings(staged);
            return;
        }

        var savedPrevSlots = new List<(string Name, int SavedSlot)>();
        foreach (var kv in staged.Values)
        {
            if (_compiler.HasDeclaredBinding(kv.Name))
            {
                int prevSlot = _compiler.AllocateTempSlot($"prev_{kv.Name}");
                _compiler.EmitLoadBindingOrUndefined(kv.Name);
                _chunk.Emit(PilotOpcode.StoreLocal, prevSlot);
                savedPrevSlots.Add((kv.Name, prevSlot));
            }
        }

        CommitStagedBindings(staged);

        bool prevDiscard = _compiler.DiscardExpressionResult;
        _compiler.DiscardExpressionResult = false;
        _compiler.Visit(guardExpr);
        _compiler.DiscardExpressionResult = prevDiscard;

        if (savedPrevSlots.Count == 0)
        {
            failJumps.Add(_chunk.EmitJump(PilotOpcode.JumpIfFalse));
        }
        else
        {
            int guardTrueJump = _chunk.EmitJump(PilotOpcode.JumpIfTrue);
            foreach (var (name, savedSlot) in savedPrevSlots)
            {
                _chunk.Emit(PilotOpcode.LoadLocal, savedSlot);
                _compiler.EmitRestoreBinding(name);
            }
            failJumps.Add(_chunk.EmitJump(PilotOpcode.Jump));
            _chunk.PatchJump(guardTrueJump);
        }
    }

    private static PilotParser.Pattern_guardContext? FindBranchPatternGuard(
        PilotParser.Pattern_guardContext[]? guards,
        PilotParser.TestContext branchTest,
        PilotParser.SuiteContext branchSuite)
    {
        if (guards == null || guards.Length == 0) return null;
        int minTok = branchTest.Stop.TokenIndex;
        int maxTok = branchSuite.Start.TokenIndex;
        for (int i = 0; i < guards.Length; i++)
        {
            int gTok = guards[i].Start.TokenIndex;
            if (gTok > minTok && gTok < maxTok)
                return guards[i];
        }
        return null;
    }

    public void LowerIsPatternExpr(PilotParser.ComparisonContext context)
    {
        bool isNegated = context.NOT()?.Length > 0;
        bool prevDiscard = _compiler.DiscardExpressionResult;
        _compiler.DiscardExpressionResult = false;
        _compiler.Visit(context.range_expr(0));
        _compiler.DiscardExpressionResult = prevDiscard;

        int scrutSlot = _compiler.AllocateTempSlot("is_scrut");
        _chunk.Emit(PilotOpcode.StoreLocal, scrutSlot);

        var failJumps = new List<int>();
        var staged = new Dictionary<string, StagedBinding>(StringComparer.Ordinal);
        EmitPatternTest(context.pattern(0), scrutSlot, staged, failJumps, defaultMutable: false);

        if (!isNegated)
        {
            CommitStagedBindings(staged);
            _chunk.Emit(PilotOpcode.ConstTrue);
        }
        else
        {
            _chunk.Emit(PilotOpcode.ConstFalse);
        }

        int doneJump = _chunk.EmitJump(PilotOpcode.Jump);
        int failIp = _chunk.InstructionCount;
        foreach (int fj in failJumps) _chunk.PatchJump(fj, failIp);

        _chunk.Emit(isNegated ? PilotOpcode.ConstTrue : PilotOpcode.ConstFalse);
        _chunk.PatchJump(doneJump);
    }

    // =========================================================================
    // 3. Recursive Pattern Bytecode Compiler
    // =========================================================================

    private void EmitPatternTest(
        PilotParser.PatternContext pat,
        int valSlot,
        Dictionary<string, StagedBinding> staged,
        List<int> failJumps,
        bool defaultMutable = false)
    {
        var orBranches = pat.or_pattern()?.and_pattern();
        if (orBranches == null || orBranches.Length == 0) return;

        if (orBranches.Length == 1)
        {
            EmitAndPatternTest(orBranches[0], valSlot, skipOuterCheck: false, staged, failJumps, defaultMutable);
            return;
        }

        var successJumps = new List<int>();
        for (int i = 0; i < orBranches.Length; i++)
        {
            var branchFails = new List<int>();
            EmitAndPatternTest(orBranches[i], valSlot, skipOuterCheck: false, staged, branchFails, defaultMutable);
            if (i < orBranches.Length - 1)
            {
                successJumps.Add(_chunk.EmitJump(PilotOpcode.Jump));
                int nextTryIp = _chunk.InstructionCount;
                foreach (int bf in branchFails) _chunk.PatchJump(bf, nextTryIp);
            }
            else
            {
                failJumps.AddRange(branchFails);
            }
        }

        int matchedIp = _chunk.InstructionCount;
        foreach (int sj in successJumps) _chunk.PatchJump(sj, matchedIp);
    }

    private void EmitAndPatternTest(
        PilotParser.And_patternContext andPat,
        int valSlot,
        bool skipOuterCheck,
        Dictionary<string, StagedBinding> staged,
        List<int> failJumps,
        bool defaultMutable = false)
    {
        var notPats = andPat.not_pattern();
        if (notPats == null) return;
        for (int i = 0; i < notPats.Length; i++)
        {
            EmitNotPatternTest(notPats[i], valSlot, skipOuterCheck && notPats.Length == 1, staged, failJumps, defaultMutable);
        }
    }

    private void EmitNotPatternTest(
        PilotParser.Not_patternContext notPat,
        int valSlot,
        bool skipOuterCheck,
        Dictionary<string, StagedBinding> staged,
        List<int> failJumps,
        bool defaultMutable = false)
    {
        // `@` whole-value pattern binding: `sq @ Rectangle(w, h)` or `let sq @ ...`
        if (notPat.AT() != null && notPat.identifier() != null && notPat.not_pattern() != null)
        {
            string aliasName = notPat.identifier().GetText();
            bool isMut = notPat.VAR() != null || (notPat.LET() == null && defaultMutable);
            StageSlotValue(aliasName, valSlot, isMut, staged);
            EmitNotPatternTest(notPat.not_pattern(), valSlot, skipOuterCheck, staged, failJumps, defaultMutable);
            return;
        }

        if (notPat.NOT() != null && notPat.not_pattern() != null)
        {
            var innerFails = new List<int>();
            var discardedBindings = new Dictionary<string, StagedBinding>(StringComparer.Ordinal);
            EmitNotPatternTest(notPat.not_pattern(), valSlot, skipOuterCheck: false, discardedBindings, innerFails, defaultMutable);
            // If inner SUCCEEDED, `not inner` FAILS!
            failJumps.Add(_chunk.EmitJump(PilotOpcode.Jump));
            int passIp = _chunk.InstructionCount;
            foreach (int inf in innerFails) _chunk.PatchJump(inf, passIp);
            return;
        }

        if (notPat.primary_pattern() != null)
        {
            EmitPrimaryPatternTest(notPat.primary_pattern(), valSlot, skipOuterCheck, staged, failJumps, defaultMutable);
        }
    }

    private void EmitPrimaryPatternTest(
        PilotParser.Primary_patternContext prim,
        int valSlot,
        bool skipOuterCheck,
        Dictionary<string, StagedBinding> staged,
        List<int> failJumps,
        bool defaultMutable = false)
    {
        // 1. Wildcard `_`
        if (prim.UNDERSCORE() != null)
        {
            return;
        }

        // 2. Range pattern: `200..299`, `0..<10`, `'a'..'z'`
        if (prim.range_pattern() is { } rp)
        {
            if (skipOuterCheck) return;
            EmitRangePatternTest(rp, valSlot, failJumps);
            return;
        }

        // 3. Signed negative number pattern: `-1`, `-3.14`
        if (prim.signed_number_pattern() is { } snp)
        {
            if (skipOuterCheck) return;
            var negVal = ParseSignedNumberValue(snp);
            _chunk.Emit(PilotOpcode.LoadLocal, valSlot);
            _compiler.EmitLiteralConstant(negVal);
            _chunk.Emit(PilotOpcode.Identical);
            failJumps.Add(_chunk.EmitJump(PilotOpcode.JumpIfFalse));
            return;
        }

        // 4. Literal pattern
        if (prim.literal() is { } lit)
        {
            if (skipOuterCheck) return;
            _chunk.Emit(PilotOpcode.LoadLocal, valSlot);
            _compiler.VisitLiteral(lit);
            _chunk.Emit(PilotOpcode.Identical);
            failJumps.Add(_chunk.EmitJump(PilotOpcode.JumpIfFalse));
            return;
        }

        // 5. Keyword / KeywordVariant pattern: `:pending` or `:ok([head, ..tail])`
        if (prim.keyword_pattern() is { } kp)
        {
            EmitKeywordPatternTest(kp, valSlot, staged, failJumps, defaultMutable);
            return;
        }

        // 6. Relational pattern: `> 10`, `<= 100`
        if (prim.relational_pattern() is { } rel)
        {
            EmitRelationalPatternTest(rel, valSlot, failJumps);
            return;
        }

        // 7. Type / ADT Variant / Record / Variable pattern
        if (prim.type_or_var_pattern() is { } tv)
        {
            EmitTypeOrVarPatternTest(tv, valSlot, skipOuterCheck, staged, failJumps, defaultMutable);
            return;
        }

        // 8. Collection / Slice pattern: `[head, ..mid, tail]`
        if (prim.collection_pattern() is { } col)
        {
            EmitCollectionSlicePatternTest(col, valSlot, staged, failJumps, defaultMutable);
            return;
        }

        // 9. Tuple / Parenthesized pattern: `(a, b)` or `(p1 | p2)`
        if (prim.tuple_pattern_match() is { } tup)
        {
            var subPats = tup.pattern();
            bool hasComma = tup.COMMA()?.Length > 0;
            if (subPats.Length == 1 && !hasComma)
            {
                if (subPats[0].or_pattern()?.and_pattern()?.Length == 1)
                {
                    EmitAndPatternTest(subPats[0].or_pattern().and_pattern(0), valSlot, skipOuterCheck, staged, failJumps, defaultMutable);
                }
                else
                {
                    EmitPatternTest(subPats[0], valSlot, staged, failJumps, defaultMutable);
                }
                return;
            }

            _chunk.Emit(PilotOpcode.LoadLocal, valSlot);
            _chunk.Emit(PilotOpcode.GetSequenceLen, 0, extra: 1);
            _chunk.Emit(PilotOpcode.ConstInt, subPats.Length);
            _chunk.Emit(PilotOpcode.Eq);
            failJumps.Add(_chunk.EmitJump(PilotOpcode.JumpIfFalse));

            for (int i = 0; i < subPats.Length; i++)
            {
                int elemSlot = _compiler.AllocateTempSlot($"tup_el_{i}");
                _chunk.Emit(PilotOpcode.LoadLocal, valSlot);
                _chunk.Emit(PilotOpcode.GetAdtField, i, extra: 0);
                _chunk.Emit(PilotOpcode.StoreLocal, elemSlot);
                EmitPatternTest(subPats[i], elemSlot, staged, failJumps, defaultMutable);
            }
            return;
        }

        // 10. Structural property pattern: `{ x: 10, y: p }`
        if (prim.property_pattern() is { } propPat)
        {
            EmitPropertyPatternTest(propPat, valSlot, staged, failJumps, defaultMutable);
        }
    }

    private void EmitRangePatternTest(
        PilotParser.Range_patternContext rp,
        int valSlot,
        List<int> failJumps)
    {
        var bounds = rp.range_bound();
        bool inclusive = rp.DOTDOT_LESS() == null;
        var lo = EvaluateRangeBoundValue(bounds[0]);
        var hi = EvaluateRangeBoundValue(bounds[1]);

        if (lo.Kind is PilotValueKind.String or PilotValueKind.Char)
        {
            int typeId = _chunk.AddIdentifier("char_or_str");
            _chunk.Emit(PilotOpcode.LoadLocal, valSlot);
            _chunk.Emit(PilotOpcode.CheckRecordType, typeId, extra: -1);
            failJumps.Add(_chunk.EmitJump(PilotOpcode.JumpIfFalse));
        }
        else
        {
            int typeId = _chunk.AddIdentifier("number");
            _chunk.Emit(PilotOpcode.LoadLocal, valSlot);
            _chunk.Emit(PilotOpcode.CheckRecordType, typeId, extra: -1);
            failJumps.Add(_chunk.EmitJump(PilotOpcode.JumpIfFalse));
        }

        // val >= lo
        _chunk.Emit(PilotOpcode.LoadLocal, valSlot);
        _compiler.EmitLiteralConstant(lo);
        _chunk.Emit(PilotOpcode.Ge);
        failJumps.Add(_chunk.EmitJump(PilotOpcode.JumpIfFalse));

        // val <= hi (or val < hi for ..<)
        _chunk.Emit(PilotOpcode.LoadLocal, valSlot);
        _compiler.EmitLiteralConstant(hi);
        _chunk.Emit(inclusive ? PilotOpcode.Le : PilotOpcode.Lt);
        failJumps.Add(_chunk.EmitJump(PilotOpcode.JumpIfFalse));
    }

    private void EmitKeywordPatternTest(
        PilotParser.Keyword_patternContext kp,
        int valSlot,
        Dictionary<string, StagedBinding> staged,
        List<int> failJumps,
        bool defaultMutable)
    {
        string kwName = ExtractKeywordText(kp.symbol_literal());
        int kwId = _chunk.AddIdentifier(kwName);

        if (kp.LPAR() == null)
        {
            _chunk.Emit(PilotOpcode.LoadLocal, valSlot);
            _chunk.Emit(PilotOpcode.CheckKeywordTag, kwId, extra: -1);
            failJumps.Add(_chunk.EmitJump(PilotOpcode.JumpIfFalse));
            return;
        }

        var args = kp.pattern_arg();
        int argCount = args?.Length ?? 0;
        _chunk.Emit(PilotOpcode.LoadLocal, valSlot);
        _chunk.Emit(PilotOpcode.CheckKeywordTag, kwId, extra: argCount);
        failJumps.Add(_chunk.EmitJump(PilotOpcode.JumpIfFalse));

        for (int i = 0; i < argCount; i++)
        {
            int fieldSlot = _compiler.AllocateTempSlot($"kw_arg_{i}");
            _chunk.Emit(PilotOpcode.LoadLocal, valSlot);
            // extra: 1 handles both PilotKeywordVariant (:ok(x)) and Elixir-style tagged tuple (:ok, x)!
            _chunk.Emit(PilotOpcode.GetAdtField, i, extra: 1);
            _chunk.Emit(PilotOpcode.StoreLocal, fieldSlot);
            EmitPatternTest(args![i].pattern(), fieldSlot, staged, failJumps, defaultMutable);
        }
    }

    private void EmitRelationalPatternTest(
        PilotParser.Relational_patternContext rel,
        int valSlot,
        List<int> failJumps)
    {
        if (rel.EQEQUAL() == null && rel.NOTEQUAL() == null)
        {
            // Guard against non-numeric/non-string comparison coercion
            int numId = _chunk.AddIdentifier("number");
            int strId = _chunk.AddIdentifier("char_or_str");
            _chunk.Emit(PilotOpcode.LoadLocal, valSlot);
            _chunk.Emit(PilotOpcode.CheckRecordType, numId, extra: -1);
            int okJump = _chunk.EmitJump(PilotOpcode.JumpIfTrue);
            _chunk.Emit(PilotOpcode.LoadLocal, valSlot);
            _chunk.Emit(PilotOpcode.CheckRecordType, strId, extra: -1);
            failJumps.Add(_chunk.EmitJump(PilotOpcode.JumpIfFalse));
            _chunk.PatchJump(okJump);
        }

        _chunk.Emit(PilotOpcode.LoadLocal, valSlot);
        bool prevDiscard = _compiler.DiscardExpressionResult;
        _compiler.DiscardExpressionResult = false;
        _compiler.Visit(rel.shift_expr());
        _compiler.DiscardExpressionResult = prevDiscard;

        if (rel.EQEQUAL() != null) _chunk.Emit(PilotOpcode.Eq);
        else if (rel.NOTEQUAL() != null) _chunk.Emit(PilotOpcode.Ne);
        else if (rel.GREATER() != null) _chunk.Emit(PilotOpcode.Gt);
        else if (rel.GREATEREQUAL() != null) _chunk.Emit(PilotOpcode.Ge);
        else if (rel.LESS() != null) _chunk.Emit(PilotOpcode.Lt);
        else _chunk.Emit(PilotOpcode.Le);

        failJumps.Add(_chunk.EmitJump(PilotOpcode.JumpIfFalse));
    }

    private void EmitTypeOrVarPatternTest(
        PilotParser.Type_or_var_patternContext tv,
        int valSlot,
        bool skipOuterCheck,
        Dictionary<string, StagedBinding> staged,
        List<int> failJumps,
        bool defaultMutable)
    {
        if ((tv.LET() != null || tv.VAR() != null) && tv.dotted_name() == null && tv.identifier() != null)
        {
            string vName = tv.identifier().GetText();
            StageSlotValue(vName, valSlot, tv.VAR() != null, staged);
            return;
        }

        if (tv.dotted_name() == null) return;

        var parts = tv.dotted_name().identifier();
        string fullDotted = tv.dotted_name().GetText();
        var genSuffix = tv.generic_pattern_suffix();
        var trailingId = tv.identifier();
        bool hasParens = tv.LPAR() != null;
        var patArgs = tv.pattern_arg();
        var propPat = tv.property_pattern();

        string? explicitParent = null;
        string targetName = fullDotted;
        if (genSuffix?.identifier() != null)
        {
            explicitParent = fullDotted;
            targetName = genSuffix.identifier().GetText();
        }
        else if (parts.Length == 2)
        {
            explicitParent = parts[0].GetText();
            targetName = parts[1].GetText();
        }

        // A. Typestate pattern: `Conn[Open]` or `Conn[Open] c`
        if (genSuffix != null && genSuffix.identifier() == null && !hasParens)
        {
            string stateSpec = $"{fullDotted}[{genSuffix.type_arguments().GetText().Trim()}]";
            int tsId = _chunk.AddIdentifier(stateSpec);
            _chunk.Emit(PilotOpcode.LoadLocal, valSlot);
            _chunk.Emit(PilotOpcode.CheckRecordType, tsId, extra: -1);
            failJumps.Add(_chunk.EmitJump(PilotOpcode.JumpIfFalse));
            if (trailingId != null)
            {
                StageSlotValue(trailingId.GetText(), valSlot, defaultMutable, staged);
            }
            return;
        }

        // B. Known ADT Variant (`Origin`, `Shape.Origin`, `Circle(r)`, `Rectangle(width: w, height: h)`)
        if (_compiler.TryResolveAdtVariant(explicitParent, targetName, out var adtDesc, out int vTag, out var varDesc))
        {
            if (!skipOuterCheck)
            {
                int adtId = _chunk.AddIdentifier(adtDesc.Name);
                _chunk.Emit(PilotOpcode.LoadLocal, valSlot);
                _chunk.Emit(PilotOpcode.GetAdtTag, adtId);
                _chunk.Emit(PilotOpcode.ConstInt, vTag);
                _chunk.Emit(PilotOpcode.Eq);
                failJumps.Add(_chunk.EmitJump(PilotOpcode.JumpIfFalse));
            }

            if (hasParens && patArgs != null && patArgs.Length > 0)
            {
                EmitOrderedFieldSubpatterns(valSlot, varDesc.Fields, patArgs, staged, failJumps, defaultMutable);
            }

            if (propPat != null)
            {
                EmitPropertyPatternTest(propPat, valSlot, staged, failJumps, defaultMutable);
            }

            if (trailingId != null)
            {
                StageSlotValue(trailingId.GetText(), valSlot, defaultMutable, staged);
            }
            return;
        }

        // C. Known Record product type (`Point(x, y)` or `Point(y: py, x: px)`)
        if (explicitParent == null && _compiler.TryResolveRecordType(targetName, out var recDesc))
        {
            int recId = _chunk.AddIdentifier(recDesc.Name);
            _chunk.Emit(PilotOpcode.LoadLocal, valSlot);
            _chunk.Emit(PilotOpcode.CheckRecordType, recId, extra: hasParens ? (patArgs?.Length ?? 0) : -1);
            failJumps.Add(_chunk.EmitJump(PilotOpcode.JumpIfFalse));

            if (hasParens && patArgs != null && patArgs.Length > 0)
            {
                EmitOrderedFieldSubpatterns(valSlot, recDesc.RecordFields, patArgs, staged, failJumps, defaultMutable);
            }

            if (propPat != null)
            {
                EmitPropertyPatternTest(propPat, valSlot, staged, failJumps, defaultMutable);
            }

            if (trailingId != null)
            {
                StageSlotValue(trailingId.GetText(), valSlot, defaultMutable, staged);
            }
            return;
        }

        // D. Primitive type check (`int n`, `str s`, `bool b`) or whole ADT/Class check (`Shape s`, `Dog d`)
        if (IsPrimitiveTypeName(targetName) ||
            _compiler.TryResolveAdtType(targetName, out _) ||
            _compiler.IsKnownClassOrStructName(targetName) ||
            trailingId != null ||
            hasParens ||
            propPat != null)
        {
            // Bare variable check: if single identifier, no parens/props/trailingId, and NOT a primitive/ADT/record/class -> bind variable!
            if (explicitParent == null && !hasParens && propPat == null && trailingId == null &&
                !IsPrimitiveTypeName(targetName) &&
                !_compiler.TryResolveAdtType(targetName, out _) &&
                !_compiler.IsKnownClassOrStructName(targetName))
            {
                StageSlotValue(targetName, valSlot, defaultMutable, staged);
                return;
            }

            string checkKey = explicitParent != null ? $"{explicitParent}.{targetName}" : targetName;
            int typeId = _chunk.AddIdentifier(checkKey);
            _chunk.Emit(PilotOpcode.LoadLocal, valSlot);
            _chunk.Emit(PilotOpcode.CheckRecordType, typeId, extra: hasParens ? (patArgs?.Length ?? 0) : -1);
            failJumps.Add(_chunk.EmitJump(PilotOpcode.JumpIfFalse));

            if (hasParens && patArgs != null && patArgs.Length > 0)
            {
                for (int i = 0; i < patArgs.Length; i++)
                {
                    int fieldSlot = _compiler.AllocateTempSlot($"dyn_f_{i}");
                    _chunk.Emit(PilotOpcode.LoadLocal, valSlot);
                    if (patArgs[i].identifier() != null)
                    {
                        int mId = _chunk.AddIdentifier(patArgs[i].identifier().GetText());
                        _chunk.Emit(PilotOpcode.GetMember, mId);
                    }
                    else
                    {
                        _chunk.Emit(PilotOpcode.GetAdtField, i, extra: 0);
                    }
                    _chunk.Emit(PilotOpcode.StoreLocal, fieldSlot);
                    EmitPatternTest(patArgs[i].pattern(), fieldSlot, staged, failJumps, defaultMutable);
                }
            }

            if (propPat != null)
            {
                EmitPropertyPatternTest(propPat, valSlot, staged, failJumps, defaultMutable);
            }

            if (trailingId != null)
            {
                StageSlotValue(trailingId.GetText(), valSlot, defaultMutable, staged);
            }
            return;
        }

        // E. Unconditional variable binding (`x`, `other`, `default`)
        StageSlotValue(targetName, valSlot, defaultMutable, staged);
    }

    private void EmitOrderedFieldSubpatterns(
        int containerSlot,
        IReadOnlyList<PilotSerializedFieldDescriptor> fields,
        PilotParser.Pattern_argContext[] patArgs,
        Dictionary<string, StagedBinding> staged,
        List<int> failJumps,
        bool defaultMutable)
    {
        for (int i = 0; i < patArgs.Length; i++)
        {
            var pa = patArgs[i];
            int fieldIndex = i;
            if (pa.identifier() != null)
            {
                string fName = pa.identifier().GetText();
                fieldIndex = FindFieldIndex(fields, fName);
                if (fieldIndex < 0)
                {
                    // Guaranteed fail on nonexistent named field
                    failJumps.Add(_chunk.EmitJump(PilotOpcode.Jump));
                    return;
                }
            }

            var subPat = pa.pattern();
            if (TryGetSimpleWildcard(subPat))
            {
                continue;
            }

            int fieldSlot = _compiler.AllocateTempSlot($"adt_f_{fieldIndex}");
            _chunk.Emit(PilotOpcode.LoadLocal, containerSlot);
            _chunk.Emit(PilotOpcode.GetAdtField, fieldIndex, extra: 0);
            _chunk.Emit(PilotOpcode.StoreLocal, fieldSlot);
            EmitPatternTest(subPat, fieldSlot, staged, failJumps, defaultMutable);
        }
    }

    private void EmitCollectionSlicePatternTest(
        PilotParser.Collection_patternContext col,
        int valSlot,
        Dictionary<string, StagedBinding> staged,
        List<int> failJumps,
        bool defaultMutable)
    {
        var elems = col.slice_pattern_element() ?? Array.Empty<PilotParser.Slice_pattern_elementContext>();
        int spreadIdx = -1;
        for (int i = 0; i < elems.Length; i++)
        {
            if (IsSpreadSliceElement(elems[i]))
            {
                spreadIdx = i;
                break;
            }
        }

        int lenSlot = _compiler.AllocateTempSlot("seq_len");
        _chunk.Emit(PilotOpcode.LoadLocal, valSlot);
        _chunk.Emit(PilotOpcode.GetSequenceLen, 0, extra: 0);
        _chunk.Emit(PilotOpcode.StoreLocal, lenSlot);

        if (spreadIdx < 0)
        {
            _chunk.Emit(PilotOpcode.LoadLocal, lenSlot);
            _chunk.Emit(PilotOpcode.ConstInt, elems.Length);
            _chunk.Emit(PilotOpcode.Eq);
            failJumps.Add(_chunk.EmitJump(PilotOpcode.JumpIfFalse));

            for (int i = 0; i < elems.Length; i++)
            {
                var subPat = elems[i].pattern();
                if (subPat == null || TryGetSimpleWildcard(subPat)) continue;
                int itemSlot = _compiler.AllocateTempSlot($"seq_el_{i}");
                _chunk.Emit(PilotOpcode.LoadLocal, valSlot);
                _chunk.Emit(PilotOpcode.GetAdtField, i, extra: 0);
                _chunk.Emit(PilotOpcode.StoreLocal, itemSlot);
                EmitPatternTest(subPat, itemSlot, staged, failJumps, defaultMutable);
            }
        }
        else
        {
            int prefixCount = spreadIdx;
            int suffixCount = elems.Length - spreadIdx - 1;
            int minLen = prefixCount + suffixCount;

            _chunk.Emit(PilotOpcode.LoadLocal, lenSlot);
            _chunk.Emit(PilotOpcode.ConstInt, minLen);
            _chunk.Emit(PilotOpcode.Ge);
            failJumps.Add(_chunk.EmitJump(PilotOpcode.JumpIfFalse));

            // Prefix elements: 0 .. prefixCount - 1
            for (int i = 0; i < prefixCount; i++)
            {
                var subPat = elems[i].pattern();
                if (subPat == null || TryGetSimpleWildcard(subPat)) continue;
                int itemSlot = _compiler.AllocateTempSlot($"seq_pre_{i}");
                _chunk.Emit(PilotOpcode.LoadLocal, valSlot);
                _chunk.Emit(PilotOpcode.GetAdtField, i, extra: 0);
                _chunk.Emit(PilotOpcode.StoreLocal, itemSlot);
                EmitPatternTest(subPat, itemSlot, staged, failJumps, defaultMutable);
            }

            // Middle rest slice: `..tail`
            var restPat = elems[spreadIdx].pattern();
            if (restPat != null && !TryGetSimpleWildcard(restPat))
            {
                int restSlot = _compiler.AllocateTempSlot("seq_rest");
                _chunk.Emit(PilotOpcode.LoadLocal, valSlot);
                _chunk.Emit(PilotOpcode.SliceSublist, prefixCount, extra: suffixCount);
                _chunk.Emit(PilotOpcode.StoreLocal, restSlot);
                EmitPatternTest(restPat, restSlot, staged, failJumps, defaultMutable);
            }

            // Suffix elements: negative index -(suffixCount - s) from end!
            for (int s = 0; s < suffixCount; s++)
            {
                var subPat = elems[spreadIdx + 1 + s].pattern();
                if (subPat == null || TryGetSimpleWildcard(subPat)) continue;
                int negIdx = -(suffixCount - s);
                int itemSlot = _compiler.AllocateTempSlot($"seq_suf_{s}");
                _chunk.Emit(PilotOpcode.LoadLocal, valSlot);
                _chunk.Emit(PilotOpcode.GetAdtField, negIdx, extra: 0);
                _chunk.Emit(PilotOpcode.StoreLocal, itemSlot);
                EmitPatternTest(subPat, itemSlot, staged, failJumps, defaultMutable);
            }
        }
    }

    private void EmitPropertyPatternTest(
        PilotParser.Property_patternContext propPat,
        int valSlot,
        Dictionary<string, StagedBinding> staged,
        List<int> failJumps,
        bool defaultMutable)
    {
        // Reject null/undefined targets
        _chunk.Emit(PilotOpcode.LoadLocal, valSlot);
        failJumps.Add(_chunk.EmitJump(PilotOpcode.JumpIfNull));

        var members = propPat.property_pattern_member();
        if (members == null) return;

        foreach (var member in members)
        {
            string propName = member.identifier().GetText();
            int propId = _chunk.AddIdentifier(propName);
            int propSlot = _compiler.AllocateTempSlot($"prop_{propName}");

            _chunk.Emit(PilotOpcode.LoadLocal, valSlot);
            _chunk.Emit(PilotOpcode.GetMember, propId);
            _chunk.Emit(PilotOpcode.Dup);
            // If member does not exist (Undefined), fail match
            int definedJump = _chunk.EmitJump(PilotOpcode.JumpIfNotNull);
            // Wait: Null IS a valid member value (`{ x: null }`), only Undefined means missing property!
            // Check Identical with ConstUndefined:
            _chunk.PatchJump(definedJump, _chunk.InstructionCount);
            _chunk.Emit(PilotOpcode.StoreLocal, propSlot);

            _chunk.Emit(PilotOpcode.LoadLocal, propSlot);
            _chunk.Emit(PilotOpcode.ConstUndefined);
            _chunk.Emit(PilotOpcode.NotIdentical);
            failJumps.Add(_chunk.EmitJump(PilotOpcode.JumpIfFalse));

            EmitPatternTest(member.pattern(), propSlot, staged, failJumps, defaultMutable);
        }
    }

    // =========================================================================
    // 4. Staged Binding Management & Static Helpers
    // =========================================================================

    private void StageSlotValue(
        string name,
        int sourceSlot,
        bool isMutable,
        Dictionary<string, StagedBinding> staged)
    {
        if (string.IsNullOrEmpty(name) || name == "_" || name == "default")
            return;

        if (!staged.TryGetValue(name, out var sb))
        {
            int stageSlot = _compiler.AllocateTempSlot($"stage_{name}");
            sb = new StagedBinding(name, stageSlot, isMutable);
            staged[name] = sb;
        }
        else if (isMutable)
        {
            sb.IsMutable = true;
        }

        _chunk.Emit(PilotOpcode.LoadLocal, sourceSlot);
        _chunk.Emit(PilotOpcode.StoreLocal, sb.StagedSlot);
    }

    private void CommitStagedBindings(Dictionary<string, StagedBinding> staged)
    {
        foreach (var sb in staged.Values)
        {
            _chunk.Emit(PilotOpcode.LoadLocal, sb.StagedSlot);
            _compiler.EmitStoreBinding(sb.Name, sb.IsMutable);
        }
    }

    private void PredeclarePatternBindingsInCompiler(PilotParser.PatternContext? pat)
    {
        if (pat == null) return;
        foreach (var (name, isMut) in CollectBoundVariables(pat))
        {
            _compiler.RegisterBindingMetadata(name, isMut);
        }
    }

    private IEnumerable<(string Name, bool IsMutable)> CollectBoundVariables(PilotParser.PatternContext pat)
    {
        var list = new List<(string, bool)>();
        CollectFromPattern(pat, list);
        return list;
    }

    private void CollectFromPattern(PilotParser.PatternContext? pat, List<(string, bool)> result)
    {
        var orBranches = pat?.or_pattern()?.and_pattern();
        if (orBranches == null) return;
        foreach (var ab in orBranches)
        {
            foreach (var np in ab.not_pattern())
            {
                CollectFromNotPattern(np, result);
            }
        }
    }

    private void CollectFromNotPattern(PilotParser.Not_patternContext np, List<(string, bool)> result)
    {
        if (np.AT() != null && np.identifier() != null && np.not_pattern() != null)
        {
            string alias = np.identifier().GetText();
            if (alias != "_") result.Add((alias, np.VAR() != null));
            CollectFromNotPattern(np.not_pattern(), result);
            return;
        }
        if (np.NOT() != null) return;

        var prim = np.primary_pattern();
        if (prim == null) return;

        if (prim.keyword_pattern()?.pattern_arg() is { } kwArgs)
        {
            foreach (var a in kwArgs) CollectFromPattern(a.pattern(), result);
            return;
        }

        if (prim.collection_pattern()?.slice_pattern_element() is { } sliceEls)
        {
            foreach (var el in sliceEls)
            {
                if (el.pattern() != null) CollectFromPattern(el.pattern(), result);
            }
            return;
        }

        if (prim.tuple_pattern_match()?.pattern() is { } tupPats)
        {
            foreach (var tp in tupPats) CollectFromPattern(tp, result);
            return;
        }

        if (prim.property_pattern()?.property_pattern_member() is { } members)
        {
            foreach (var m in members) CollectFromPattern(m.pattern(), result);
            return;
        }

        if (prim.type_or_var_pattern() is { } tv)
        {
            if ((tv.LET() != null || tv.VAR() != null) && tv.dotted_name() == null && tv.identifier() != null)
            {
                string id = tv.identifier().GetText();
                if (id != "_") result.Add((id, tv.VAR() != null));
                return;
            }

            if (tv.pattern_arg() != null)
            {
                foreach (var pa in tv.pattern_arg()) CollectFromPattern(pa.pattern(), result);
            }
            if (tv.property_pattern()?.property_pattern_member() is { } pm)
            {
                foreach (var m in pm) CollectFromPattern(m.pattern(), result);
            }
            if (tv.identifier() != null)
            {
                string tid = tv.identifier().GetText();
                if (tid != "_") result.Add((tid, false));
                return;
            }

            if (tv.dotted_name() != null &&
                tv.dotted_name().identifier().Length == 1 &&
                tv.generic_pattern_suffix() == null &&
                tv.LPAR() == null &&
                tv.property_pattern() == null)
            {
                string name = tv.dotted_name().GetText();
                if (name != "_" && name != "default" &&
                    !IsPrimitiveTypeName(name) &&
                    !_compiler.TryResolveAdtVariant(null, name, out _, out _, out _) &&
                    !_compiler.TryResolveAdtType(name, out _) &&
                    !_compiler.TryResolveRecordType(name, out _) &&
                    !_compiler.IsKnownClassOrStructName(name))
                {
                    result.Add((name, false));
                }
            }
        }
    }

    private bool IsIrrefutableSubpattern(PilotParser.PatternContext pat)
    {
        var orBranches = pat.or_pattern()?.and_pattern();
        if (orBranches == null || orBranches.Length != 1) return false;
        var notPats = orBranches[0].not_pattern();
        if (notPats == null || notPats.Length != 1) return false;

        var np = notPats[0];
        while (np.AT() != null && np.not_pattern() != null)
        {
            np = np.not_pattern();
        }
        if (np.NOT() != null || np.primary_pattern() == null) return false;

        var prim = np.primary_pattern();
        if (prim.UNDERSCORE() != null) return true;
        if (prim.type_or_var_pattern() is { } tv)
        {
            if ((tv.LET() != null || tv.VAR() != null) && tv.dotted_name() == null && tv.identifier() != null)
                return true;
            if (tv.dotted_name() != null &&
                tv.dotted_name().identifier().Length == 1 &&
                tv.generic_pattern_suffix() == null &&
                tv.LPAR() == null &&
                tv.property_pattern() == null &&
                tv.identifier() == null)
            {
                string name = tv.dotted_name().GetText();
                return !IsPrimitiveTypeName(name) &&
                       !_compiler.TryResolveAdtVariant(null, name, out _, out _, out _) &&
                       !_compiler.TryResolveAdtType(name, out _) &&
                       !_compiler.TryResolveRecordType(name, out _) &&
                       !_compiler.IsKnownClassOrStructName(name);
            }
        }
        return false;
    }

    private bool IsBareOptionalBindingPattern(PilotParser.PatternContext pat) => IsIrrefutableSubpattern(pat) &&
        pat.or_pattern()?.and_pattern(0)?.not_pattern(0)?.primary_pattern()?.UNDERSCORE() == null;

    private static bool TryGetSimpleWildcard(PilotParser.PatternContext pat)
    {
        var ab = pat.or_pattern()?.and_pattern();
        if (ab == null || ab.Length != 1) return false;
        var np = ab[0].not_pattern();
        return np?.Length == 1 && np[0].AT() == null && np[0].NOT() == null && np[0].primary_pattern()?.UNDERSCORE() != null;
    }

    private static bool IsSpreadSliceElement(PilotParser.Slice_pattern_elementContext el) =>
        el.DOTDOT() != null || el.ELLIPSIS() != null || el.STAR() != null;

    private static int FindFieldIndex(IReadOnlyList<PilotSerializedFieldDescriptor> fields, string name)
    {
        for (int i = 0; i < fields.Count; i++)
        {
            if (string.Equals(fields[i].Name, name, StringComparison.Ordinal) ||
                string.Equals($"_{i}", name, StringComparison.Ordinal) ||
                string.Equals($"item{i}", name, StringComparison.Ordinal))
            {
                return i;
            }
        }
        if (fields.Count == 1 && name == "value") return 0;
        return -1;
    }

    private static (PilotParser.PatternContext? Pattern, bool IsVar) FindBranchPatternAndMutability(
        PilotParser.If_stmtContext ifCtx,
        PilotParser.PatternContext[]? patterns,
        PilotParser.TestContext branchTest,
        PilotParser.SuiteContext? prevSuite)
    {
        if (patterns == null || patterns.Length == 0) return (null, false);
        int minToken = prevSuite != null ? prevSuite.Stop.TokenIndex : -1;
        int maxToken = branchTest.Start.TokenIndex;
        for (int i = 0; i < patterns.Length; i++)
        {
            int patStart = patterns[i].Start.TokenIndex;
            if (patStart > minToken && patStart < maxToken)
            {
                bool isVar = false;
                for (int c = 0; c < ifCtx.ChildCount; c++)
                {
                    if (ifCtx.GetChild(c) is ITerminalNode term &&
                        term.Symbol.TokenIndex > minToken &&
                        term.Symbol.TokenIndex < patStart &&
                        term.Symbol.Type == PilotParser.VAR)
                    {
                        isVar = true;
                        break;
                    }
                }
                return (patterns[i], isVar);
            }
        }
        return (null, false);
    }

    private static string ExtractKeywordText(PilotParser.Symbol_literalContext sym)
    {
        if (sym.identifier() != null)
            return sym.identifier().GetText();
        string raw = sym.SYMBOL()?.GetText() ?? ":empty";
        return raw.TrimStart(':');
    }

    private static bool IsPrimitiveTypeName(string name) => name is
        "int" or "i8" or "i16" or "i32" or "i64" or
        "uint" or "u8" or "u16" or "u32" or "u64" or
        "float" or "f8" or "f16" or "f32" or "f64" or "double" or
        "decimal" or "str" or "string" or "bool" or "char" or "bytes" or
        "list" or "List" or "array" or "Array" or
        "tuple" or "Tuple" or "map" or "Map" or "dict" or "Dict" or "set" or "Set";

    private static bool TryParseLiteralInt(PilotParser.LiteralContext lit, out long val)
    {
        val = 0;
        if (lit.INTEGER() == null && lit.HEX_INTEGER() == null && lit.BIN_INTEGER() == null && lit.OCT_INTEGER() == null)
            return false;
        val = ParseUnsignedIntToken(lit.INTEGER()?.GetText(), lit.HEX_INTEGER()?.GetText(), lit.BIN_INTEGER()?.GetText(), lit.OCT_INTEGER()?.GetText());
        return true;
    }

    private static bool TryExtractIntRange(PilotParser.Range_patternContext rp, out long lo, out long hi)
    {
        lo = 0;
        hi = -1;
        var bounds = rp.range_bound();
        if (bounds.Length != 2) return false;
        if (bounds[0].STRING() != null || bounds[0].FLOAT_NUMBER() != null ||
            bounds[1].STRING() != null || bounds[1].FLOAT_NUMBER() != null)
        {
            return false;
        }

        long rawLo = ParseUnsignedIntToken(bounds[0].INTEGER()?.GetText(), bounds[0].HEX_INTEGER()?.GetText(), bounds[0].BIN_INTEGER()?.GetText(), bounds[0].OCT_INTEGER()?.GetText());
        long rawHi = ParseUnsignedIntToken(bounds[1].INTEGER()?.GetText(), bounds[1].HEX_INTEGER()?.GetText(), bounds[1].BIN_INTEGER()?.GetText(), bounds[1].OCT_INTEGER()?.GetText());
        lo = bounds[0].MINUS() != null ? -rawLo : rawLo;
        long endBound = bounds[1].MINUS() != null ? -rawHi : rawHi;
        hi = rp.DOTDOT_LESS() != null ? endBound - 1 : endBound;
        return true;
    }

    private static PilotValue EvaluateRangeBoundValue(PilotParser.Range_boundContext bound)
    {
        if (bound.STRING() != null)
        {
            string raw = bound.STRING().GetText();
            string unquoted = raw.Length >= 2 ? raw[1..^1] : raw;
            return PilotValue.FromString(unquoted);
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

    private static PilotValue ParseSignedNumberValue(PilotParser.Signed_number_patternContext snp)
    {
        if (snp.FLOAT_NUMBER() != null)
        {
            double d = double.Parse(snp.FLOAT_NUMBER().GetText().Replace("_", ""), CultureInfo.InvariantCulture);
            return PilotValue.FromFloat(-d);
        }
        long l = ParseUnsignedIntToken(snp.INTEGER()?.GetText(), snp.HEX_INTEGER()?.GetText(), snp.BIN_INTEGER()?.GetText(), snp.OCT_INTEGER()?.GetText());
        return PilotValue.FromInt(-l);
    }

    private static long ParseUnsignedIntToken(string? dec, string? hex, string? bin, string? oct)
    {
        if (hex != null) return Convert.ToInt64(hex.Replace("_", "")[2..], 16);
        if (bin != null) return Convert.ToInt64(bin.Replace("_", "")[2..], 2);
        if (oct != null) return Convert.ToInt64(oct.Replace("_", "")[2..], 8);
        if (dec != null)
        {
            string clean = dec.Replace("_", "");
            if (clean.StartsWith("0x", StringComparison.OrdinalIgnoreCase)) return Convert.ToInt64(clean[2..], 16);
            if (clean.StartsWith("0b", StringComparison.OrdinalIgnoreCase)) return Convert.ToInt64(clean[2..], 2);
            if (clean.StartsWith("0o", StringComparison.OrdinalIgnoreCase)) return Convert.ToInt64(clean[2..], 8);
            return long.Parse(clean, CultureInfo.InvariantCulture);
        }
        return 0;
    }
}
