using System;

namespace Pilot.Scripting;

/// <summary>
/// Thrown when the script requests program termination via exit(code).
/// </summary>
public sealed class PilotScriptExitException : Exception
{
    /// <summary>
    /// The numeric exit code specified by the script.
    /// </summary>
    public int ExitCode { get; }

    public PilotScriptExitException(int exitCode = 0)
        : base($"Script requested termination with exit code {exitCode}")
    {
        ExitCode = exitCode;
    }
}
