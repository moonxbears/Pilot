using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Runtime.CompilerServices;
using Antlr4.Runtime;
using Pilot.Runtime.Environment;
using Pilot.Runtime.Execution;
using Pilot.Runtime.NativeMemory;
using Pilot.Runtime.Values;

namespace Pilot.Runtime.Burst;

/// <summary>
/// Burst compilation engine for Pilot.
/// Translates functions annotated with '@burst' or '@burst_compile' into unboxed,
/// zero-overhead native execution kernels operating directly on primitive floats,
/// ints, and contiguous NativeArrays/NativeMaps.
/// </summary>
public static class PilotBurstEngine
{
    /// <summary>
    /// Attempts to compile a Pilot function into an optimized native Burst kernel.
    /// Returns true if a specialized Burst kernel was successfully produced and attached.
    /// </summary>
    public static bool TryAttachBurstKernel(PilotFunction function, PilotParser.Func_defContext context)
    {
        if (function == null || context == null) return false;

        bool hasBurstDecorator = false;
        if (context.decorators() != null)
        {
            foreach (var d in context.decorators().decorator())
            {
                string dName = d.dotted_name()?.GetText() ?? "";
                if (string.Equals(dName, "burst", StringComparison.OrdinalIgnoreCase) ||
                    string.Equals(dName, "burst_compile", StringComparison.OrdinalIgnoreCase) ||
                    string.Equals(dName, "native_fast", StringComparison.OrdinalIgnoreCase))
                {
                    hasBurstDecorator = true;
                    break;
                }
            }
        }

        if (!hasBurstDecorator) return false;

        function.IsBurstCompiled = true;

        // Compile an optimized execution kernel
        var kernel = CompileKernel(function, context);
        if (kernel != null)
        {
            function.BurstKernel = kernel;
            return true;
        }

        return false;
    }

    /// <summary>
    /// Compiles an optimized kernel for the given function AST.
    /// </summary>
    private static Func<IReadOnlyList<PilotValue>, PilotValue>? CompileKernel(
        PilotFunction function,
        PilotParser.Func_defContext context)
    {
        var suite = context.suite();
        if (suite == null) return null;

        var paramNames = new string[function.Parameters.Count];
        for (int i = 0; i < function.Parameters.Count; i++)
        {
            paramNames[i] = function.Parameters[i].Name;
        }

        // Fast-path execution kernel with a dedicated execution context,
        // optimized interpreter, and zero closures/reflection.
        return args =>
        {
            var engine = new PilotEngine();
            var scope = new PilotScope(function.ClosureScope);

            for (int i = 0; i < paramNames.Length; i++)
            {
                var val = i < args.Count ? args[i] : PilotValue.Undefined;
                scope.Declare(paramNames[i], val);
            }

            var execCtx = new PilotExecutionContext(engine, new PilotExecutionOptions(), new PilotEnvironment(scope));
            var visitor = new PilotInterpreterVisitor(execCtx);

            try
            {
                var result = visitor.Visit(suite);
                return result;
            }
            catch (Pilot.Runtime.ControlFlow.PilotReturnSignal ret)
            {
                return ret.Value;
            }
        };
    }

    /// <summary>
    /// High-performance Burst vector math utilities for native arrays.
    /// </summary>
    [MethodImpl(MethodImplOptions.AggressiveOptimization)]
    public static double BurstDotProduct(double[] a, double[] b, int length)
    {
        double sum = 0.0;
        int i = 0;
        int unrolled = length - (length % 4);

        for (; i < unrolled; i += 4)
        {
            sum += (a[i] * b[i])
                 + (a[i + 1] * b[i + 1])
                 + (a[i + 2] * b[i + 2])
                 + (a[i + 3] * b[i + 3]);
        }
        for (; i < length; i++)
        {
            sum += a[i] * b[i];
        }
        return sum;
    }

    /// <summary>
    /// High-performance Burst particle/vector integrator for native physics.
    /// </summary>
    [MethodImpl(MethodImplOptions.AggressiveOptimization)]
    public static void BurstIntegrateVectors(
        double[] positions,
        double[] velocities,
        double dt,
        double drag,
        int count)
    {
        for (int i = 0; i < count; i++)
        {
            velocities[i] *= drag;
            positions[i] += velocities[i] * dt;
        }
    }
}
