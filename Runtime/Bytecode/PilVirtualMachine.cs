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
/// inline hardware arithmetic fast paths, and bidirectional interop with AST and native code.
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

        if (arguments != null && arguments.Count > 0)
        {
            for (int i = 0; i < arguments.Count && i < locals.Length; i++)
            {
                locals[i] = arguments[i];
            }
        }

        var instructions = chunk.Instructions;
        int instCount = instructions.Count;
        var constants = chunk.Constants;
        var identifiers = chunk.IdentifierPool;
        var scope = context.Environment.CurrentScope;

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
                    stack[sp++] = PilotValue.FromInt(a.AsInt() ^ b.AsInt());
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
                        CreateParameters(nestedChunk.ParameterNames),
                        astBody: nestedChunk.FuncDefContext?.suite(),
                        closureScope: scope,
                        isAsync: isAsync,
                        isGenerator: isGen);
                    fn.BytecodeChunk = nestedChunk;

                    if (nestedChunk.FuncDefContext != null)
                    {
                        Pilot.Runtime.Burst.PilotBurstEngine.TryAttachBurstKernel(fn, nestedChunk.FuncDefContext);
                    }

                    // Automatically bind in scope if named
                    if (!string.IsNullOrEmpty(nestedChunk.Name) && nestedChunk.Name != "<lambda>")
                    {
                        scope.Declare(nestedChunk.Name, PilotValue.FromFunction(fn));
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
            }
        }

        return sp > 0 ? stack[--sp] : PilotValue.Undefined;
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
        // Direct fast-path method invocation
        if (target.Kind == PilotValueKind.List)
        {
            var list = target.AsList();
            if (methodName is "append" or "push" && args.Count > 0)
            {
                list.Add(args[0]);
                return target;
            }
            if (methodName == "pop")
            {
                if (list.Count == 0) return PilotValue.Undefined;
                var last = list[^1];
                list.RemoveAt(list.Count - 1);
                return last;
            }
            if (methodName == "clear")
            {
                list.Clear();
                return PilotValue.Null;
            }
        }

        // Standard member resolution & call
        var member = target.GetMember(methodName, context);
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

    private static IReadOnlyList<PilotParameter> CreateParameters(IReadOnlyList<string> paramNames)
    {
        if (paramNames.Count == 0) return Array.Empty<PilotParameter>();
        var parameters = new PilotParameter[paramNames.Count];
        for (int i = 0; i < paramNames.Count; i++)
        {
            parameters[i] = new PilotParameter(paramNames[i]);
        }
        return parameters;
    }
}
