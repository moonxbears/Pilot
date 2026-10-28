using System;
using System.Collections;
using System.Collections.Generic;
using System.Runtime.CompilerServices;
using Antlr4.Runtime;
using Pilot.Diagnostics;
using Pilot.Runtime.ControlFlow;
using Pilot.Runtime.Environment;
using Pilot.Runtime.Execution;
using Pilot.Runtime.Values;

namespace Pilot.Runtime.Bytecode;

/// <summary>
/// High-performance stack-based virtual machine execution engine (PilVM).
/// Implements direct array operand stack, slot-based local variable registers,
/// inline hardware arithmetic fast paths, O(1) ADT/integer jump tables,
/// and bidirectional interop with AST and native code.
/// </summary>
public sealed class PilVirtualMachine
{
    private const int DefaultStackCapacity = 512;
    private const int MaxCallDepth = 1000;

    /// <summary>
    /// Executes a bytecode chunk within the given execution context.
    /// </summary>
    public static PilotValue Execute(
        PilotBytecodeChunk chunk,
        PilotExecutionContext context,
        IReadOnlyList<PilotValue>? arguments = null,
        PilotFunction? calleeFunction = null)
    {
        if (chunk == null) throw new ArgumentNullException(nameof(chunk));
        if (context == null) throw new ArgumentNullException(nameof(context));

        if (context.CallDepth >= context.Options.MaxCallDepth)
        {
            throw new PilotRecursionException($"Stack overflow: exceeded maximum call depth ({context.Options.MaxCallDepth}).");
        }

        var stack = new PilotValue[DefaultStackCapacity];
        int sp = 0;

        int localCount = Math.Max(chunk.LocalCount, arguments?.Count ?? 0);
        var locals = new PilotValue[localCount];

        var instructions = chunk.Instructions;
        int instCount = instructions.Count;
        var constants = chunk.Constants;
        var identifiers = chunk.IdentifierPool;
        var scope = context.Environment.CurrentScope;

        if (arguments != null && arguments.Count > 0)
        {
            for (int i = 0; i < arguments.Count && i < locals.Length; i++)
            {
                locals[i] = arguments[i];
            }
        }

        if (chunk.LocalSlotMap.Count > 0)
        {
            int argCount = arguments?.Count ?? 0;
            foreach (var (locName, slot) in chunk.LocalSlotMap)
            {
                if (slot >= argCount && slot < locals.Length && scope.TryResolve(locName, out var scopeVal))
                {
                    locals[slot] = scopeVal;
                }
            }
        }

        int ip = 0;

        while (ip < instCount)
        {
            ref readonly var inst = ref System.Runtime.InteropServices.CollectionsMarshal.AsSpan(instructions)[ip++];

            switch (inst.Opcode)
            {
                case PilotOpcode.Nop:
                    break;

                case PilotOpcode.Pop:
                    if (sp > 0) sp--;
                    break;

                case PilotOpcode.Dup:
                    EnsureStackCapacity(ref stack, sp + 1);
                    stack[sp] = stack[sp - 1];
                    sp++;
                    break;

                case PilotOpcode.Swap:
                {
                    var tmp = stack[sp - 1];
                    stack[sp - 1] = stack[sp - 2];
                    stack[sp - 2] = tmp;
                    break;
                }

                // Constants
                case PilotOpcode.Const:
                    EnsureStackCapacity(ref stack, sp + 1);
                    stack[sp++] = constants[inst.Operand];
                    break;

                case PilotOpcode.ConstNull:
                    EnsureStackCapacity(ref stack, sp + 1);
                    stack[sp++] = PilotValue.Null;
                    break;

                case PilotOpcode.ConstUndefined:
                    EnsureStackCapacity(ref stack, sp + 1);
                    stack[sp++] = PilotValue.Undefined;
                    break;

                case PilotOpcode.ConstTrue:
                    EnsureStackCapacity(ref stack, sp + 1);
                    stack[sp++] = PilotValue.True;
                    break;

                case PilotOpcode.ConstFalse:
                    EnsureStackCapacity(ref stack, sp + 1);
                    stack[sp++] = PilotValue.False;
                    break;

                case PilotOpcode.ConstZero:
                    EnsureStackCapacity(ref stack, sp + 1);
                    stack[sp++] = PilotValue.FromInt(0);
                    break;

                case PilotOpcode.ConstOne:
                    EnsureStackCapacity(ref stack, sp + 1);
                    stack[sp++] = PilotValue.FromInt(1);
                    break;

                case PilotOpcode.ConstInt:
                    EnsureStackCapacity(ref stack, sp + 1);
                    stack[sp++] = PilotValue.FromInt(inst.Operand);
                    break;

                // Local slot registers
                case PilotOpcode.LoadLocal:
                    EnsureStackCapacity(ref stack, sp + 1);
                    stack[sp++] = locals[inst.Operand];
                    break;

                case PilotOpcode.StoreLocal:
                    locals[inst.Operand] = stack[--sp];
                    break;

                // Dynamic Named scope access
                case PilotOpcode.LoadNamed:
                {
                    string name = identifiers[inst.Operand];
                    EnsureStackCapacity(ref stack, sp + 1);
                    if (scope.TryResolve(name, out var val))
                    {
                        stack[sp++] = val;
                    }
                    else if (context.Environment.GlobalScope.TryResolve(name, out val))
                    {
                        stack[sp++] = val;
                    }
                    else if (context.Options.Builtins.TryGet(name, out var builtin))
                    {
                        stack[sp++] = PilotValue.FromFunction(builtin);
                    }
                    else
                    {
                        stack[sp++] = PilotValue.Undefined;
                    }
                    break;
                }

                case PilotOpcode.StoreNamed:
                {
                    string name = identifiers[inst.Operand];
                    var val = stack[--sp];
                    if (!scope.Assign(name, val))
                    {
                        if (!context.Environment.GlobalScope.Assign(name, val))
                        {
                            scope.Declare(name, val);
                        }
                    }
                    break;
                }

                case PilotOpcode.DeclareNamed:
                {
                    string name = identifiers[inst.Operand];
                    var val = stack[--sp];
                    if (scope.ContainsLocal(name))
                    {
                        var existing = scope.Resolve(name);
                        if (existing.Kind == PilotValueKind.Function && val.Kind == PilotValueKind.Function)
                        {
                            var oldFunc = existing.AsFunction();
                            var newFunc = val.AsFunction();
                            if (oldFunc.ClassDef == null && oldFunc.GetSignatureKey() == newFunc.GetSignatureKey())
                            {
                                oldFunc.ApplyPatchFrom(newFunc);
                                break;
                            }
                            var mg = new PilotMethodGroup(name, new[] { oldFunc, newFunc });
                            scope.ForceSet(name, PilotValue.FromMethodGroup(mg));
                            break;
                        }
                        else if (existing.Kind == PilotValueKind.MethodGroup && val.Kind == PilotValueKind.Function)
                        {
                            existing.AsMethodGroup().AddOverload(val.AsFunction());
                            break;
                        }
                        scope.ForceSet(name, val);
                    }
                    else
                    {
                        scope.Declare(name, val);
                    }
                    break;
                }

                // Arithmetic fast paths
                case PilotOpcode.Add:
                {
                    var b = stack[--sp];
                    var a = stack[--sp];
                    if (a.Kind == PilotValueKind.Int && b.Kind == PilotValueKind.Int)
                    {
                        stack[sp++] = PilotValue.FromInt(a.AsInt() + b.AsInt());
                    }
                    else if (a.Kind == PilotValueKind.Float && b.Kind == PilotValueKind.Float)
                    {
                        stack[sp++] = PilotValue.FromFloat(a.AsFloat() + b.AsFloat());
                    }
                    else if (a.Kind == PilotValueKind.String || b.Kind == PilotValueKind.String)
                    {
                        stack[sp++] = PilotValue.FromString(a.AsString() + b.AsString());
                    }
                    else
                    {
                        stack[sp++] = a + b;
                    }
                    break;
                }

                case PilotOpcode.Sub:
                {
                    var b = stack[--sp];
                    var a = stack[--sp];
                    if (a.Kind == PilotValueKind.Int && b.Kind == PilotValueKind.Int)
                    {
                        stack[sp++] = PilotValue.FromInt(a.AsInt() - b.AsInt());
                    }
                    else if (a.Kind == PilotValueKind.Float && b.Kind == PilotValueKind.Float)
                    {
                        stack[sp++] = PilotValue.FromFloat(a.AsFloat() - b.AsFloat());
                    }
                    else
                    {
                        stack[sp++] = a - b;
                    }
                    break;
                }

                case PilotOpcode.Mul:
                {
                    var b = stack[--sp];
                    var a = stack[--sp];
                    if (a.Kind == PilotValueKind.Int && b.Kind == PilotValueKind.Int)
                    {
                        stack[sp++] = PilotValue.FromInt(a.AsInt() * b.AsInt());
                    }
                    else if (a.Kind == PilotValueKind.Float && b.Kind == PilotValueKind.Float)
                    {
                        stack[sp++] = PilotValue.FromFloat(a.AsFloat() * b.AsFloat());
                    }
                    else
                    {
                        stack[sp++] = a * b;
                    }
                    break;
                }

                case PilotOpcode.Div:
                {
                    var b = stack[--sp];
                    var a = stack[--sp];
                    if (a.Kind == PilotValueKind.Float && b.Kind == PilotValueKind.Float)
                    {
                        stack[sp++] = PilotValue.FromFloat(a.AsFloat() / b.AsFloat());
                    }
                    else
                    {
                        stack[sp++] = a / b;
                    }
                    break;
                }

                case PilotOpcode.FloorDiv:
                {
                    var b = stack[--sp];
                    var a = stack[--sp];
                    long bInt = b.AsInt();
                    if (bInt == 0) throw new DivideByZeroException("Division by zero in integer floor division.");
                    stack[sp++] = PilotValue.FromInt(a.AsInt() / bInt);
                    break;
                }

                case PilotOpcode.Mod:
                {
                    var b = stack[--sp];
                    var a = stack[--sp];
                    stack[sp++] = a % b;
                    break;
                }

                case PilotOpcode.Pow:
                {
                    var b = stack[--sp];
                    var a = stack[--sp];
                    stack[sp++] = PilotValue.FromFloat(Math.Pow(a.AsFloat(), b.AsFloat()));
                    break;
                }

                case PilotOpcode.Neg:
                {
                    var a = stack[--sp];
                    if (a.Kind == PilotValueKind.Int) stack[sp++] = PilotValue.FromInt(-a.AsInt());
                    else if (a.Kind == PilotValueKind.Float) stack[sp++] = PilotValue.FromFloat(-a.AsFloat());
                    else stack[sp++] = -a;
                    break;
                }

                case PilotOpcode.Pos:
                    break;

                // Bitwise & Logic
                case PilotOpcode.BitAnd:
                {
                    var b = stack[--sp];
                    var a = stack[--sp];
                    stack[sp++] = a & b;
                    break;
                }

                case PilotOpcode.BitOr:
                {
                    var b = stack[--sp];
                    var a = stack[--sp];
                    stack[sp++] = a | b;
                    break;
                }

                case PilotOpcode.BitXor:
                {
                    var b = stack[--sp];
                    var a = stack[--sp];
                    stack[sp++] = a ^ b;
                    break;
                }

                case PilotOpcode.BitNot:
                {
                    var a = stack[--sp];
                    stack[sp++] = PilotValue.FromInt(~a.AsInt());
                    break;
                }

                case PilotOpcode.ShiftLeft:
                {
                    var b = stack[--sp];
                    var a = stack[--sp];
                    stack[sp++] = PilotValue.FromInt(a.AsInt() << (int)b.AsInt());
                    break;
                }

                case PilotOpcode.ShiftRight:
                {
                    var b = stack[--sp];
                    var a = stack[--sp];
                    stack[sp++] = PilotValue.FromInt(a.AsInt() >> (int)b.AsInt());
                    break;
                }

                case PilotOpcode.Not:
                {
                    var a = stack[--sp];
                    stack[sp++] = PilotValue.FromBool(!a.IsTrue);
                    break;
                }

                // Comparisons
                case PilotOpcode.Eq:
                {
                    var b = stack[--sp];
                    var a = stack[--sp];
                    stack[sp++] = PilotValue.FromBool(a == b);
                    break;
                }

                case PilotOpcode.Ne:
                {
                    var b = stack[--sp];
                    var a = stack[--sp];
                    stack[sp++] = PilotValue.FromBool(a != b);
                    break;
                }

                case PilotOpcode.Lt:
                {
                    var b = stack[--sp];
                    var a = stack[--sp];
                    if (a.Kind == PilotValueKind.Int && b.Kind == PilotValueKind.Int)
                    {
                        stack[sp++] = PilotValue.FromBool(a.AsInt() < b.AsInt());
                    }
                    else
                    {
                        stack[sp++] = PilotValue.FromBool(a < b);
                    }
                    break;
                }

                case PilotOpcode.Le:
                {
                    var b = stack[--sp];
                    var a = stack[--sp];
                    if (a.Kind == PilotValueKind.Int && b.Kind == PilotValueKind.Int)
                    {
                        stack[sp++] = PilotValue.FromBool(a.AsInt() <= b.AsInt());
                    }
                    else
                    {
                        stack[sp++] = PilotValue.FromBool(a <= b);
                    }
                    break;
                }

                case PilotOpcode.Gt:
                {
                    var b = stack[--sp];
                    var a = stack[--sp];
                    if (a.Kind == PilotValueKind.Int && b.Kind == PilotValueKind.Int)
                    {
                        stack[sp++] = PilotValue.FromBool(a.AsInt() > b.AsInt());
                    }
                    else
                    {
                        stack[sp++] = PilotValue.FromBool(a > b);
                    }
                    break;
                }

                case PilotOpcode.Ge:
                {
                    var b = stack[--sp];
                    var a = stack[--sp];
                    if (a.Kind == PilotValueKind.Int && b.Kind == PilotValueKind.Int)
                    {
                        stack[sp++] = PilotValue.FromBool(a.AsInt() >= b.AsInt());
                    }
                    else
                    {
                        stack[sp++] = PilotValue.FromBool(a >= b);
                    }
                    break;
                }

                case PilotOpcode.Identical:
                {
                    var b = stack[--sp];
                    var a = stack[--sp];
                    stack[sp++] = PilotValue.FromBool(a.Equals(b));
                    break;
                }

                case PilotOpcode.NotIdentical:
                {
                    var b = stack[--sp];
                    var a = stack[--sp];
                    stack[sp++] = PilotValue.FromBool(!a.Equals(b));
                    break;
                }

                // Control flow & Jumps
                case PilotOpcode.Jump:
                    ip = inst.Operand;
                    break;

                case PilotOpcode.JumpIfFalse:
                {
                    var cond = stack[--sp];
                    if (!cond.IsTrue) ip = inst.Operand;
                    break;
                }

                case PilotOpcode.JumpIfTrue:
                {
                    var cond = stack[--sp];
                    if (cond.IsTrue) ip = inst.Operand;
                    break;
                }

                case PilotOpcode.JumpIfFalseOrPop:
                {
                    var cond = stack[sp - 1];
                    if (!cond.IsTrue) ip = inst.Operand;
                    else sp--;
                    break;
                }

                case PilotOpcode.JumpIfTrueOrPop:
                {
                    var cond = stack[sp - 1];
                    if (cond.IsTrue) ip = inst.Operand;
                    else sp--;
                    break;
                }

                case PilotOpcode.JumpIfNull:
                {
                    var val = stack[--sp];
                    if (val.IsNull || val.IsUndefined) ip = inst.Operand;
                    break;
                }

                case PilotOpcode.JumpIfNotNull:
                {
                    var val = stack[--sp];
                    if (!val.IsNull && !val.IsUndefined) ip = inst.Operand;
                    break;
                }

                // Loops & Iterators
                case PilotOpcode.GetIter:
                {
                    var iterable = stack[--sp];
                    var iter = CreateIterator(iterable);
                    EnsureStackCapacity(ref stack, sp + 1);
                    stack[sp++] = PilotValue.FromForeign(iter);
                    break;
                }

                case PilotOpcode.ForIter:
                {
                    var iterObj = stack[--sp].AsForeign() as IEnumerator<PilotValue>;
                    if (iterObj != null && iterObj.MoveNext())
                    {
                        EnsureStackCapacity(ref stack, sp + 1);
                        stack[sp++] = iterObj.Current;
                    }
                    else
                    {
                        ip = inst.Operand; // Jump past loop
                    }
                    break;
                }

                // Function Calls & Closures
                case PilotOpcode.Call:
                {
                    int argc = inst.Operand;
                    var callArgs = new PilotValue[argc];
                    for (int a = argc - 1; a >= 0; a--)
                    {
                        callArgs[a] = stack[--sp];
                    }
                    var callee = stack[--sp];
                    var callResult = InvokeCallable(callee, callArgs, context);
                    EnsureStackCapacity(ref stack, sp + 1);
                    stack[sp++] = callResult;
                    break;
                }

                case PilotOpcode.CallMethod:
                {
                    string methodName = identifiers[inst.Operand];
                    int argc = inst.Extra;
                    var callArgs = new PilotValue[argc];
                    for (int a = argc - 1; a >= 0; a--)
                    {
                        callArgs[a] = stack[--sp];
                    }
                    var target = stack[--sp];
                    var callResult = InvokeMethod(target, methodName, callArgs, context);
                    EnsureStackCapacity(ref stack, sp + 1);
                    stack[sp++] = callResult;
                    break;
                }

                case PilotOpcode.Return:
                    return PilotValue.Undefined;

                case PilotOpcode.ReturnValue:
                    return sp > 0 ? stack[--sp] : PilotValue.Undefined;

                case PilotOpcode.MakeClosure:
                {
                    var nestedChunk = chunk.NestedChunks[inst.Operand];

                    // Sync current locals into scope so the closure can access captured outer variables
                    foreach (var (locName, slot) in chunk.LocalSlotMap)
                    {
                        if (!string.IsNullOrEmpty(locName) && !locName.StartsWith("<") && slot < locals.Length)
                        {
                            scope.Declare(locName, locals[slot]);
                        }
                    }

                    bool isGen = nestedChunk.FuncDefContext?.suite() != null &&
                                 PilotInterpreterVisitor.ContainsYield(nestedChunk.FuncDefContext.suite());
                    bool isAsync = nestedChunk.FuncDefContext?.ASYNC() != null;

                    var fn = new PilotFunction(
                        nestedChunk.Name,
                        CreateParameters(nestedChunk.ParameterNames, nestedChunk.FuncDefContext, context),
                        astBody: nestedChunk.FuncDefContext?.suite(),
                        closureScope: scope,
                        isAsync: isAsync,
                        isGenerator: isGen);
                    fn.BytecodeChunk = nestedChunk;
                    fn.SourceBodyText = nestedChunk.FuncDefContext?.suite()?.GetText();

                    if (nestedChunk.FuncDefContext != null)
                    {
                        if (nestedChunk.FuncDefContext.type_spec() != null)
                        {
                            fn.ReturnTypeSpec = nestedChunk.FuncDefContext.type_spec().GetText();
                            if (nestedChunk.FuncDefContext.type_spec().EXCLAMATION() != null)
                            {
                                fn.IsFallible = true;
                                fn.SuccessTypeSpec = nestedChunk.FuncDefContext.type_spec().union_type()?.GetText();
                                fn.ErrorTypeSpec = nestedChunk.FuncDefContext.type_spec().error_union_type()?.GetText();
                            }
                            else
                            {
                                fn.SuccessTypeSpec = fn.ReturnTypeSpec;
                            }
                        }
                        Pilot.Runtime.Burst.PilotBurstEngine.TryAttachBurstKernel(fn, nestedChunk.FuncDefContext);
                    }

                    // Automatically bind in scope if named (preserving MethodGroup overloads!)
                    if (!string.IsNullOrEmpty(nestedChunk.Name) && nestedChunk.Name != "<lambda>")
                    {
                        if (scope.ContainsLocal(nestedChunk.Name))
                        {
                            var existing = scope.Resolve(nestedChunk.Name);
                            if (existing.Kind == PilotValueKind.Function)
                            {
                                var oldFn = existing.AsFunction();
                                if (oldFn.ClassDef == null && oldFn.GetSignatureKey() != fn.GetSignatureKey())
                                {
                                    var mg = new PilotMethodGroup(nestedChunk.Name, new[] { oldFn, fn });
                                    scope.ForceSet(nestedChunk.Name, PilotValue.FromMethodGroup(mg));
                                }
                                else
                                {
                                    scope.Declare(nestedChunk.Name, PilotValue.FromFunction(fn));
                                }
                            }
                            else if (existing.Kind == PilotValueKind.MethodGroup)
                            {
                                existing.AsMethodGroup().AddOverload(fn);
                            }
                            else
                            {
                                scope.Declare(nestedChunk.Name, PilotValue.FromFunction(fn));
                            }
                        }
                        else
                        {
                            scope.Declare(nestedChunk.Name, PilotValue.FromFunction(fn));
                        }
                    }

                    EnsureStackCapacity(ref stack, sp + 1);
                    stack[sp++] = PilotValue.FromFunction(fn);
                    break;
                }

                // Collections
                case PilotOpcode.BuildList:
                {
                    int count = inst.Operand;
                    var list = new List<PilotValue>(count);
                    for (int i = 0; i < count; i++) list.Add(PilotValue.Null);
                    for (int i = count - 1; i >= 0; i--) list[i] = stack[--sp];
                    EnsureStackCapacity(ref stack, sp + 1);
                    stack[sp++] = PilotValue.FromList(list);
                    break;
                }

                case PilotOpcode.BuildTuple:
                {
                    int count = inst.Operand;
                    var arr = new PilotValue[count];
                    for (int i = count - 1; i >= 0; i--) arr[i] = stack[--sp];
                    EnsureStackCapacity(ref stack, sp + 1);
                    stack[sp++] = PilotValue.FromTuple(arr);
                    break;
                }

                case PilotOpcode.BuildMap:
                {
                    int pairCount = inst.Operand;
                    var map = new Dictionary<PilotValue, PilotValue>(pairCount);
                    var pairs = new (PilotValue Key, PilotValue Val)[pairCount];
                    for (int i = pairCount - 1; i >= 0; i--)
                    {
                        var val = stack[--sp];
                        var key = stack[--sp];
                        pairs[i] = (key, val);
                    }
                    for (int i = 0; i < pairCount; i++)
                    {
                        map[pairs[i].Key] = pairs[i].Val;
                    }
                    EnsureStackCapacity(ref stack, sp + 1);
                    stack[sp++] = PilotValue.FromMap(map);
                    break;
                }

                case PilotOpcode.BuildSet:
                {
                    int count = inst.Operand;
                    var set = new HashSet<PilotValue>(count);
                    for (int i = 0; i < count; i++)
                    {
                        set.Add(stack[--sp]);
                    }
                    EnsureStackCapacity(ref stack, sp + 1);
                    stack[sp++] = PilotValue.FromSet(set);
                    break;
                }

                case PilotOpcode.BuildRange:
                {
                    bool inclusive = inst.Operand == 0;
                    var end = stack[--sp];
                    var start = stack[--sp];
                    EnsureStackCapacity(ref stack, sp + 1);
                    stack[sp++] = PilotValue.FromRange(new PilotRange(start.AsInt(), end.AsInt(), 1, inclusive));
                    break;
                }

                // Member & Index Access
                case PilotOpcode.GetMember:
                {
                    string memberName = identifiers[inst.Operand];
                    var target = stack[--sp];
                    EnsureStackCapacity(ref stack, sp + 1);
                    stack[sp++] = target.GetMember(memberName, context);
                    break;
                }

                case PilotOpcode.SetMember:
                {
                    string memberName = identifiers[inst.Operand];
                    var val = stack[--sp];
                    var target = stack[--sp];
                    if (target.Kind is PilotValueKind.Record or PilotValueKind.Adt or PilotValueKind.KeywordVariant)
                    {
                        throw new PilotRuntimeException(
                            $"[PILOT101] Cannot mutate field '{memberName}' on immutable {target.Kind.ToString().ToLowerInvariant()} value.");
                    }
                    if (target.Kind == PilotValueKind.Object)
                    {
                        target.AsObject().SetMember(memberName, val);
                    }
                    break;
                }

                case PilotOpcode.GetIndex:
                {
                    var idx = stack[--sp];
                    var target = stack[--sp];
                    EnsureStackCapacity(ref stack, sp + 1);
                    stack[sp++] = target[idx];
                    break;
                }

                case PilotOpcode.SetIndex:
                {
                    var val = stack[--sp];
                    var idx = stack[--sp];
                    var target = stack[--sp];
                    target[idx] = val;
                    break;
                }

                case PilotOpcode.Slice:
                {
                    bool inclusive = inst.Operand == 0;
                    var end = stack[--sp];
                    var start = stack[--sp];
                    var target = stack[--sp];
                    var rangeVal = PilotValue.FromRange(new PilotRange(start.AsInt(), end.AsInt(), 1, inclusive));
                    EnsureStackCapacity(ref stack, sp + 1);
                    stack[sp++] = target[rangeVal];
                    break;
                }

                // AST Fallback (Guarantees 100% full language coverage for complex edge constructs)
                case PilotOpcode.EvalAst:
                {
                    if (chunk.LocalSlotMap.Count > 0)
                    {
                        foreach (var (locName, slot) in chunk.LocalSlotMap)
                        {
                            if (!string.IsNullOrEmpty(locName) && !locName.StartsWith("<") && slot < locals.Length)
                            {
                                if (!scope.Assign(locName, locals[slot]))
                                {
                                    scope.Declare(locName, locals[slot]);
                                }
                            }
                        }
                    }
                    var node = chunk.AstFallbackNodes[inst.Operand];
                    var visitor = context.GetOrCreateVisitor();
                    var astResult = visitor.Visit(node);

                    if (context.PendingFlow == PilotFlowSignalKind.Return)
                    {
                        var retVal = context.PendingReturnValue;
                        context.PendingFlow = PilotFlowSignalKind.None;
                        return retVal;
                    }

                    if (chunk.LocalSlotMap.Count > 0)
                    {
                        foreach (var (locName, slot) in chunk.LocalSlotMap)
                        {
                            if (!string.IsNullOrEmpty(locName) && !locName.StartsWith("<") && slot < locals.Length)
                            {
                                if (scope.TryResolve(locName, out var updatedVal))
                                {
                                    locals[slot] = updatedVal;
                                }
                            }
                        }
                    }

                    EnsureStackCapacity(ref stack, sp + 1);
                    stack[sp++] = astResult;
                    break;
                }

                case PilotOpcode.Halt:
                    return sp > 0 ? stack[--sp] : PilotValue.Undefined;

                // =====================================================================
                // Milestone 11: ADTs, Records & Decision-Tree / O(1) Jump Table Opcodes
                // =====================================================================

                case PilotOpcode.JumpTable:
                {
                    var discVal = stack[--sp];
                    var jt = chunk.JumpTables[inst.Operand];
                    long disc = 0;
                    bool valid = true;
                    switch (discVal.Kind)
                    {
                        case PilotValueKind.Int:
                        case PilotValueKind.UInt:
                            disc = discVal.AsInt();
                            break;
                        case PilotValueKind.Bool:
                            disc = discVal.IsTrue ? 1L : 0L;
                            break;
                        case PilotValueKind.Char:
                            disc = discVal.AsChar();
                            break;
                        case PilotValueKind.Adt:
                            disc = discVal.GetAdtTag();
                            break;
                        default:
                            valid = false;
                            break;
                    }

                    if (valid && disc >= jt.MinValue && disc < jt.MinValue + jt.Targets.Length)
                    {
                        ip = jt.Targets[(int)(disc - jt.MinValue)];
                    }
                    else
                    {
                        ip = jt.DefaultTarget;
                    }
                    break;
                }

                case PilotOpcode.GetAdtTag:
                {
                    var val = stack[--sp];
                    if (val.IsFailure) val = val.UnwrapFailure();
                    EnsureStackCapacity(ref stack, sp + 1);
                    if (val.Kind == PilotValueKind.Adt)
                    {
                        var vd = val.GetAdtVariantDefinition();
                        if (inst.Operand < 0 || string.Equals(vd.ParentAdt.Name, identifiers[inst.Operand], StringComparison.Ordinal))
                        {
                            stack[sp++] = PilotValue.FromInt(vd.Tag);
                            break;
                        }
                    }
                    stack[sp++] = PilotValue.FromInt(-1);
                    break;
                }

                case PilotOpcode.GetAdtField:
                {
                    var val = stack[--sp];
                    int idx = inst.Operand;
                    EnsureStackCapacity(ref stack, sp + 1);
                    if (inst.Extra == 2)
                    {
                        stack[sp++] = val.UnwrapOption();
                        break;
                    }
                    if (inst.Extra == 3)
                    {
                        stack[sp++] = val.UnwrapFailure();
                        break;
                    }
                    if (val.IsFailure) val = val.UnwrapFailure();
                    switch (val.Kind)
                    {
                        case PilotValueKind.Adt:
                            stack[sp++] = val.GetAdtField(idx);
                            break;
                        case PilotValueKind.Record:
                            stack[sp++] = val.GetRecordField(idx);
                            break;
                        case PilotValueKind.KeywordVariant:
                        {
                            var kv = val.AsKeywordVariant();
                            int realIdx = idx >= 0 ? idx : kv.Fields.Length + idx;
                            stack[sp++] = realIdx >= 0 && realIdx < kv.Fields.Length ? kv.Fields[realIdx] : PilotValue.Undefined;
                            break;
                        }
                        case PilotValueKind.Tuple:
                        {
                            var tup = val.AsTuple();
                            // inst.Extra == 1 indicates keyword-tagged tuple (:ok, payload) where element 0 is the keyword tag
                            int adjusted = inst.Extra == 1 ? idx + 1 : idx;
                            int realIdx = adjusted >= 0 ? adjusted : tup.Count + adjusted;
                            stack[sp++] = realIdx >= 0 && realIdx < tup.Count ? tup[realIdx] : PilotValue.Undefined;
                            break;
                        }
                        case PilotValueKind.List:
                        {
                            var list = val.AsList();
                            int realIdx = idx >= 0 ? idx : list.Count + idx;
                            stack[sp++] = realIdx >= 0 && realIdx < list.Count ? list[realIdx] : PilotValue.Undefined;
                            break;
                        }
                        default:
                            stack[sp++] = PilotValue.Undefined;
                            break;
                    }
                    break;
                }

                case PilotOpcode.DeclareTypeDescriptor:
                {
                    var td = chunk.TypeDescriptors[inst.Operand];
                    DeclareTypeDescriptorInContext(td, context, scope);
                    break;
                }

                case PilotOpcode.GetSequenceLen:
                {
                    var val = stack[--sp];
                    EnsureStackCapacity(ref stack, sp + 1);
                    int len = val.Kind == PilotValueKind.List ? val.AsList().Count
                            : val.Kind == PilotValueKind.Tuple ? val.AsTuple().Count
                            : val.Kind == PilotValueKind.SliceBuffer ? val.AsSliceBuffer().Length
                            : -1;
                    stack[sp++] = PilotValue.FromInt(len);
                    break;
                }

                case PilotOpcode.SliceSublist:
                {
                    var val = stack[--sp];
                    int start = inst.Operand;
                    int suffix = inst.Extra;
                    IReadOnlyList<PilotValue> items = val.Kind == PilotValueKind.List ? val.AsList()
                        : val.Kind == PilotValueKind.Tuple ? val.AsTuple()
                        : Array.Empty<PilotValue>();
                    int end = items.Count - suffix;
                    var sub = new List<PilotValue>(Math.Max(0, end - start));
                    for (int i = start; i < end; i++)
                    {
                        sub.Add(items[i]);
                    }
                    EnsureStackCapacity(ref stack, sp + 1);
                    stack[sp++] = PilotValue.FromList(sub);
                    break;
                }

                case PilotOpcode.CheckKeywordTag:
                {
                    var val = stack[--sp];
                    var targetKw = PilotKeyword.Intern(identifiers[inst.Operand]);
                    bool matched = false;
                    if (inst.Extra < 0)
                    {
                        if (val.Kind == PilotValueKind.Keyword)
                        {
                            matched = val.AsKeyword().Equals(targetKw);
                        }
                        else if (val.Kind == PilotValueKind.KeywordVariant)
                        {
                            var kv = val.AsKeywordVariant();
                            matched = kv.Fields.Length == 0 && kv.Keyword.Equals(targetKw);
                        }
                    }
                    else
                    {
                        if (val.Kind == PilotValueKind.KeywordVariant)
                        {
                            var kv = val.AsKeywordVariant();
                            matched = kv.Fields.Length == inst.Extra && kv.Keyword.Equals(targetKw);
                        }
                        else if (val.Kind == PilotValueKind.Tuple)
                        {
                            var tup = val.AsTuple();
                            matched = tup.Count == inst.Extra + 1 &&
                                      tup[0].Kind == PilotValueKind.Keyword &&
                                      tup[0].AsKeyword().Equals(targetKw);
                        }
                        else if (val.Kind == PilotValueKind.Adt)
                        {
                            var vd = val.GetAdtVariantDefinition();
                            matched = vd.Arity == inst.Extra &&
                                      string.Equals(vd.Name, targetKw.Name, StringComparison.OrdinalIgnoreCase);
                        }
                    }
                    EnsureStackCapacity(ref stack, sp + 1);
                    stack[sp++] = PilotValue.FromBool(matched);
                    break;
                }

                case PilotOpcode.CheckRecordType:
                {
                    var val = stack[--sp];
                    if (val.IsFailure) val = val.UnwrapFailure();
                    string expectedName = identifiers[inst.Operand];
                    bool matched = CheckRuntimeTypeMatch(val, expectedName, inst.Extra, context);
                    EnsureStackCapacity(ref stack, sp + 1);
                    stack[sp++] = PilotValue.FromBool(matched);
                    break;
                }

                // =====================================================================
                // Milestone 13: Values-First Error System (T ! E) & Postfix `?` Opcodes
                // =====================================================================

                case PilotOpcode.TryPropagate:
                {
                    var val = stack[--sp];
                    if (val.IsFailure)
                    {
                        string? targetErrType = context.CurrentFunction?.ErrorTypeSpec;
                        var adaptedErr = context.AdaptPropagatedError(val.UnwrapFailure(), targetErrType);
                        return PilotValue.FromFailure(adaptedErr);
                    }
                    if (val.IsNull || val.IsUndefined)
                    {
                        return PilotValue.Null;
                    }
                    EnsureStackCapacity(ref stack, sp + 1);
                    stack[sp++] = val.UnwrapOption();
                    break;
                }

                case PilotOpcode.JumpIfFailure:
                {
                    var val = stack[--sp];
                    if (val.IsFailure) ip = inst.Operand;
                    break;
                }

                case PilotOpcode.JumpIfNotFailure:
                {
                    var val = stack[--sp];
                    if (!val.IsFailure) ip = inst.Operand;
                    break;
                }

                // =====================================================================
                // Milestone 14: Unboxed Primitive Intrinsics & Deep Optional Chaining
                // =====================================================================

                case PilotOpcode.CallPrimIntrinsic:
                {
                    string methodName = identifiers[inst.Operand];
                    int argc = inst.Extra;
                    if (argc == 0)
                    {
                        var target = stack[--sp];
                        if (PilotValue.TryInvokePrimitiveIntrinsic(target, methodName, ReadOnlySpan<PilotValue>.Empty, context, out var primRes0))
                        {
                            EnsureStackCapacity(ref stack, sp + 1);
                            stack[sp++] = primRes0;
                            break;
                        }
                        var callRes0 = InvokeMethod(target, methodName, Array.Empty<PilotValue>(), context);
                        EnsureStackCapacity(ref stack, sp + 1);
                        stack[sp++] = callRes0;
                        break;
                    }
                    var callArgs = new PilotValue[argc];
                    for (int a = argc - 1; a >= 0; a--)
                    {
                        callArgs[a] = stack[--sp];
                    }
                    var recv = stack[--sp];
                    if (PilotValue.TryInvokePrimitiveIntrinsic(recv, methodName, callArgs.AsSpan(), context, out var primRes))
                    {
                        EnsureStackCapacity(ref stack, sp + 1);
                        stack[sp++] = primRes;
                        break;
                    }
                    var callResult = InvokeMethod(recv, methodName, callArgs, context);
                    EnsureStackCapacity(ref stack, sp + 1);
                    stack[sp++] = callResult;
                    break;
                }

                case PilotOpcode.JumpIfNullShortCircuit:
                {
                    var val = stack[sp - 1];
                    if (val.IsNull || val.IsUndefined)
                    {
                        stack[sp - 1] = PilotValue.Null;
                        ip = inst.Operand;
                    }
                    else if (val.IsNestedSome)
                    {
                        stack[sp - 1] = val.UnwrapOption();
                    }
                    break;
                }
            }
        }

        return sp > 0 ? stack[--sp] : PilotValue.Undefined;
    }

    private static void DeclareTypeDescriptorInContext(
        PilotBytecodeTypeDescriptor td,
        PilotExecutionContext context,
        PilotScope scope)
    {
        if (td.IsRecord)
        {
            if (!context.RecordDefinitions.TryGetValue(td.Name, out var recDef) ||
                recDef.Fields.Count != td.RecordFields.Count)
            {
                var fields = new List<PilotAdtFieldDefinition>(td.RecordFields.Count);
                for (int i = 0; i < td.RecordFields.Count; i++)
                {
                    fields.Add(new PilotAdtFieldDefinition(td.RecordFields[i].Name, td.RecordFields[i].TypeName, i));
                }
                recDef = new PilotRecordDefinition(td.Name, fields, td.GenericParameters);
                context.RecordDefinitions[td.Name] = recDef;
                RefreshLayouts(context);
            }

            var ctorVal = PilotValue.FromFunction(recDef.ConstructorFunction);
            scope.ForceSet(td.Name, ctorVal);
            return;
        }

        if (!context.AdtDefinitions.TryGetValue(td.Name, out var adtDef) ||
            adtDef.Variants.Count != td.Variants.Count)
        {
            adtDef = new PilotAdtDefinition(td.Name, td.GenericParameters);
            foreach (var vd in td.Variants)
            {
                var fields = new List<PilotAdtFieldDefinition>(vd.Fields.Count);
                for (int i = 0; i < vd.Fields.Count; i++)
                {
                    fields.Add(new PilotAdtFieldDefinition(vd.Fields[i].Name, vd.Fields[i].TypeName, i));
                }
                adtDef.AddVariant(vd.Name, fields);
            }
            context.AdtDefinitions[td.Name] = adtDef;
            RefreshLayouts(context);
        }

        scope.ForceSet(td.Name, PilotValue.FromAdtType(adtDef));
        foreach (var variant in adtDef.Variants)
        {
            context.VariantDefinitions[variant.Name] = variant;
            PilotValue varVal = variant.IsUnit
                ? variant.UnitSingleton
                : PilotValue.FromFunction(variant.ConstructorFunction!);
            scope.ForceSet(variant.Name, varVal);
        }
    }

    private static void RefreshLayouts(PilotExecutionContext context)
    {
        foreach (var adt in context.AdtDefinitions.Values)
        {
            adt.Layout = PilotAdtMemoryLayout.ComputeForAdt(adt, context.AdtDefinitions, context.RecordDefinitions);
        }
        foreach (var rec in context.RecordDefinitions.Values)
        {
            rec.Layout = PilotAdtMemoryLayout.ComputeForRecord(rec, context.AdtDefinitions, context.RecordDefinitions);
        }
    }

    private static bool CheckRuntimeTypeMatch(
        PilotValue val,
        string expectedName,
        int expectedArity,
        PilotExecutionContext context)
    {
        // Primitive type checks
        switch (expectedName)
        {
            case "int" or "i8" or "i16" or "i32" or "i64":
                return val.Kind == PilotValueKind.Int;
            case "uint" or "u8" or "u16" or "u32" or "u64":
                return val.Kind is PilotValueKind.Int or PilotValueKind.UInt;
            case "float" or "f16" or "f32" or "f64" or "double":
                return val.Kind is PilotValueKind.Float or PilotValueKind.Int;
            case "number":
                return val.IsNumber;
            case "decimal":
                return val.Kind == PilotValueKind.Decimal;
            case "str" or "string":
                return val.Kind == PilotValueKind.String;
            case "char_or_str":
                return val.Kind is PilotValueKind.String or PilotValueKind.Char;
            case "bool":
                return val.Kind == PilotValueKind.Bool;
            case "char":
                return val.Kind == PilotValueKind.Char;
            case "list" or "List" or "array" or "Array":
                return val.Kind == PilotValueKind.List;
            case "tuple" or "Tuple":
                return val.Kind == PilotValueKind.Tuple;
            case "map" or "Map" or "dict" or "Dict":
                return val.Kind == PilotValueKind.Map;
            case "set" or "Set":
                return val.Kind == PilotValueKind.Set;
            case "SliceBuffer" or "SharedBuffer" or "slice_buffer" or "shared_buffer":
                return val.Kind == PilotValueKind.SliceBuffer;
        }

        // Typestate pattern: `Conn[Open]` or generic `SliceBuffer[f64]`
        int lb = expectedName.IndexOf('[');
        if (lb > 0 && expectedName.EndsWith("]", StringComparison.Ordinal))
        {
            string baseCls = expectedName[..lb].Trim();
            string stateArg = expectedName[(lb + 1)..^1].Trim();
            if (baseCls is "SliceBuffer" or "SharedBuffer")
            {
                return val.Kind == PilotValueKind.SliceBuffer &&
                       val.AsSliceBuffer().ElementKind == Pilot.Runtime.NativeMemory.PilotSharedBuffer.ParseElementKind(stateArg);
            }
            if (val.Kind == PilotValueKind.Object)
            {
                var obj = val.AsObject();
                return MatchesClassHierarchy(obj.ClassDef, baseCls, context) &&
                       string.Equals(obj.CurrentState, stateArg, StringComparison.Ordinal);
            }
            return false;
        }

        // Qualified ADT variant: `Shape.Circle` or `Shape.Origin`
        string? parentAdt = null;
        string target = expectedName;
        int dot = expectedName.IndexOf('.');
        if (dot > 0)
        {
            parentAdt = expectedName[..dot];
            target = expectedName[(dot + 1)..];
        }

        if (val.Kind == PilotValueKind.Record)
        {
            var rd = val.GetRecordDefinition();
            if (parentAdt != null) return false;
            return string.Equals(rd.Name, target, StringComparison.Ordinal) &&
                   (expectedArity < 0 || rd.Fields.Count == expectedArity);
        }

        if (val.Kind == PilotValueKind.Adt)
        {
            var vd = val.GetAdtVariantDefinition();
            if (parentAdt != null && !string.Equals(vd.ParentAdt.Name, parentAdt, StringComparison.Ordinal))
            {
                return false;
            }
            if (string.Equals(vd.Name, target, StringComparison.Ordinal))
            {
                return expectedArity < 0 || vd.Arity == expectedArity;
            }
            if (parentAdt == null && expectedArity < 0 && string.Equals(vd.ParentAdt.Name, target, StringComparison.Ordinal))
            {
                return true;
            }
            return false;
        }

        if (val.Kind == PilotValueKind.Object)
        {
            return parentAdt == null && MatchesClassHierarchy(val.AsObject().ClassDef, target, context);
        }

        return false;
    }

    private static bool MatchesClassHierarchy(PilotClassDefinition? cls, string targetName, PilotExecutionContext context)
    {
        while (cls != null)
        {
            if (string.Equals(cls.Name, targetName, StringComparison.Ordinal))
                return true;
            if (!string.IsNullOrEmpty(cls.BaseClassName) &&
                context.Environment.TryResolve(cls.BaseClassName, out var bVal) &&
                bVal.Kind == PilotValueKind.Function && bVal.AsFunction().ClassDef != null)
            {
                cls = bVal.AsFunction().ClassDef;
            }
            else
            {
                break;
            }
        }
        return false;
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    private static void EnsureStackCapacity(ref PilotValue[] stack, int required)
    {
        if (required >= stack.Length)
        {
            Array.Resize(ref stack, stack.Length * 2);
        }
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    private static PilotValue InvokeCallable(PilotValue callee, IReadOnlyList<PilotValue> args, PilotExecutionContext context)
    {
        if (callee.Kind == PilotValueKind.Function)
        {
            return callee.AsFunction().Invoke(context, args);
        }

        if (callee.Kind == PilotValueKind.MethodGroup)
        {
            return callee.AsMethodGroup().Invoke(context, args);
        }

        if (callee.Kind == PilotValueKind.Keyword)
        {
            if (args.Count > 0 && args[0].Kind is PilotValueKind.Map or PilotValueKind.Object or PilotValueKind.Null)
            {
                var defaultVal = args.Count > 1 ? args[1] : PilotValue.Undefined;
                return callee.AsKeyword().Invoke(args[0], defaultVal);
            }
            if (args.Count > 0)
            {
                var fieldArr = args is PilotValue[] arr ? arr : new List<PilotValue>(args).ToArray();
                return PilotValue.FromKeywordVariant(new PilotKeywordVariant(callee.AsKeyword(), fieldArr));
            }
            return PilotValue.Null;
        }

        if (callee.Kind == PilotValueKind.Object)
        {
            var callMember = callee.GetMember("__call__", context);
            if (callMember.Kind == PilotValueKind.Function)
            {
                return callMember.AsFunction().Invoke(context, args);
            }
        }

        throw new PilotRuntimeException($"Attempted to call non-callable value of type '{callee.Kind}'.");
    }

    private static PilotValue InvokeMethod(PilotValue target, string methodName, IReadOnlyList<PilotValue> args, PilotExecutionContext context)
    {
        var argSpan = args is PilotValue[] arr ? arr.AsSpan() : (args.Count == 0 ? ReadOnlySpan<PilotValue>.Empty : new List<PilotValue>(args).ToArray().AsSpan());
        if (PilotValue.TryInvokePrimitiveIntrinsic(target, methodName, argSpan, context, out var intrinsicRes))
        {
            return intrinsicRes;
        }

        // Standard member resolution & call
        var member = target.GetMember(methodName, context);
        if (args.Count == 0 && member.Kind != PilotValueKind.Function && member.Kind != PilotValueKind.MethodGroup && member.Kind != PilotValueKind.Keyword && member.Kind != PilotValueKind.Object)
        {
            return member;
        }
        return InvokeCallable(member, args, context);
    }

    private static IEnumerator<PilotValue> CreateIterator(PilotValue iterable)
    {
        switch (iterable.Kind)
        {
            case PilotValueKind.List:
                return iterable.AsList().GetEnumerator();
            case PilotValueKind.Tuple:
                return ((IEnumerable<PilotValue>)iterable.AsTuple()).GetEnumerator();
            case PilotValueKind.Range:
            {
                var r = iterable.AsRange();
                return EnumerateRange(r);
            }
            case PilotValueKind.Set:
                return iterable.AsSet().GetEnumerator();
            case PilotValueKind.Map:
                return iterable.AsMap().Keys.GetEnumerator();
            case PilotValueKind.String:
            {
                string s = iterable.AsString();
                return EnumerateString(s);
            }
            case PilotValueKind.Generator:
                return iterable.AsGenerator().GetEnumerator();
            case PilotValueKind.NativeArray:
                return iterable.AsNativeArray().Select(PilotValue.FromFloat).GetEnumerator();
            case PilotValueKind.SliceBuffer:
                return EnumerateSliceBuffer(iterable.AsSliceBuffer());
            case PilotValueKind.Foreign:
            {
                if (iterable.AsForeign() is IEnumerable<PilotValue> enumValues)
                    return enumValues.GetEnumerator();
                if (iterable.AsForeign() is IEnumerable enumerable)
                {
                    return EnumerateObjects(enumerable);
                }
                break;
            }
        }

        return EnumerateSingle(iterable);
    }

    private static IEnumerator<PilotValue> EnumerateSliceBuffer(Pilot.Runtime.NativeMemory.PilotSharedBuffer sb)
    {
        for (int i = 0; i < sb.Length; i++)
        {
            yield return sb.GetElement(i);
        }
    }

    private static IEnumerator<PilotValue> EnumerateRange(PilotRange range)
    {
        long step = range.Step == 0 ? 1 : range.Step;
        long cur = range.Start;
        long end = range.End;
        bool inc = range.Inclusive;

        while (step > 0 ? (inc ? cur <= end : cur < end) : (inc ? cur >= end : cur > end))
        {
            yield return PilotValue.FromInt(cur);
            cur += step;
        }
    }

    private static IEnumerator<PilotValue> EnumerateString(string s)
    {
        for (int i = 0; i < s.Length; i++)
        {
            yield return PilotValue.FromChar(s[i]);
        }
    }

    private static IEnumerator<PilotValue> EnumerateObjects(IEnumerable enumerable)
    {
        foreach (var obj in enumerable)
        {
            yield return obj is PilotValue pv ? pv : PilotValue.FromForeign(obj);
        }
    }

    private static IEnumerator<PilotValue> EnumerateSingle(PilotValue val)
    {
        yield return val;
    }

    private static IReadOnlyList<PilotParameter> CreateParameters(
        IReadOnlyList<string> paramNames,
        PilotParser.Func_defContext? funcDef = null,
        PilotExecutionContext? context = null)
    {
        if (funcDef?.parameters()?.parameter() is { Length: > 0 } astParams)
        {
            var astList = new PilotParameter[astParams.Length];
            var visitor = context?.GetOrCreateVisitor();
            for (int i = 0; i < astParams.Length; i++)
            {
                var p = astParams[i];
                string pName = p.identifier()?.GetText() ?? p.splice_expr()?.GetText() ?? (i < paramNames.Count ? paramNames[i] : "arg");
                string? pType = p.type_annotation()?.type_spec()?.GetText();
                PilotValue? defaultVal = (p.test() != null && visitor != null) ? visitor.Visit(p.test()) : default(PilotValue?);
                bool isParams = p.PARAMS() != null || p.STAR() != null;
                bool isKwargs = p.DOUBLESTAR() != null;
                bool isContextual = p.CONTEXT() != null;
                bool isEach = p.EACH() != null || p.REPEAT() != null;
                astList[i] = new PilotParameter(pName, pType, defaultVal, isParams, isKwargs, isContextual, isEach);
            }
            return astList;
        }

        if (paramNames.Count == 0) return Array.Empty<PilotParameter>();
        var parameters = new PilotParameter[paramNames.Count];
        for (int i = 0; i < paramNames.Count; i++)
        {
            parameters[i] = new PilotParameter(paramNames[i]);
        }
        return parameters;
    }
}
