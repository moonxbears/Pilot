using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text;
using System.Text.Json;
using System.Text.Json.Nodes;
using System.Threading;
using System.Threading.Tasks;
using Pilot.LanguageServer.Semantics;

namespace Pilot.LanguageServer.Lsp;

public sealed class PilotLspServer
{
    private readonly ConcurrentDictionary<string, string> _documents = new(StringComparer.OrdinalIgnoreCase);
    private readonly ConcurrentDictionary<string, PilotAnalysisResult> _analyses = new(StringComparer.OrdinalIgnoreCase);
    private readonly object _writeLock = new();
    private Stream? _outputStream;
    private bool _shutdownRequested;

    private static readonly Dictionary<string, (string Domain, string Summary)> KeywordDocs = new(StringComparer.Ordinal)
    {
        ["guard"] = ("Core / Preconditions", "`guard <cond> else { ... }` — Mandatory early-exit precondition. The `else` suite is statically required to exit the enclosing scope (`return`, `raise`, `break`, or `continue`). Supports `guard let x = opt else { ... }` for null-safe unwrapping."),
        ["defer"] = ("Core / Cleanup", "`defer <stmt | { ... }>` — Schedules deterministic cleanup execution when exiting the current function or scope."),
        ["let"] = ("Core / Type Safety", "`let <name>: <type> = <expr>` — Declares an immutable binding. Reassignment produces compile-time error `PILOT101`."),
        ["var"] = ("Core / Type Safety", "`var <name>: <type> = <expr>` — Declares a mutable variable or indexed in-memory `Table[T]`."),
        ["const"] = ("Core / Type Safety", "`const <name>: <type> = <expr>` — Declares a compile-time constant."),
        ["noalloc"] = ("Hard Real-Time", "`noalloc { ... }` — Enforces zero heap/GC allocations inside the block. All loops inside `noalloc` must specify a `bounded` iteration limit (`PILOT104`)."),
        ["stackalloc"] = ("Hard Real-Time", "`stackalloc <type>[<count>]` — Allocates a fixed-size buffer directly on the stack with zero GC pressure."),
        ["bounded"] = ("Hard Real-Time", "`for ... bounded <N>` / `while ... bounded <N>` — Establishes a static upper bound on loop iterations for worst-case execution time (WCET) verification."),
        ["deadline"] = ("Soft/Hard Real-Time", "`deadline <duration> [jitter <duration>] { ... } [on_missed { ... }]` — Executes a block under a strict temporal deadline with an optional `on_missed` handler."),
        ["within"] = ("Soft Real-Time", "`within <duration> { ... } fallback { ... }` — Time-budgeted execution block that switches to `fallback` if the time budget expires."),
        ["periodic"] = ("Real-Time Scheduling", "`periodic <duration> [priority <N>] [jitter <duration>] { ... }` — Schedules a deterministic periodic real-time task."),
        ["actor"] = ("Concurrency", "`actor <Name> { ... }` — Defines a message-driven concurrent actor with isolated state."),
        ["spawn"] = ("Concurrency", "`spawn { ... }` — Spawns an asynchronous task or lightweight thread."),
        ["parallel"] = ("Concurrency", "`parallel for <item> in <collection> { ... }` — Data-parallel multi-threaded loop."),
        ["atomic"] = ("Concurrency", "`atomic { ... }` — Executes state mutations atomically."),
        ["sync"] = ("Concurrency", "`sync(<lock>) { ... }` — Acquires a mutual-exclusion lock for the duration of the block."),
        ["isolate"] = ("Concurrency", "`isolate { ... }` — Runs a block in an isolated memory/execution zone."),
        ["select"] = ("Concurrency / Database", "In Concurrency: `select { case msg = <-ch { ... } timeout 50ms { ... } }` (CSP channel multiplexer).\nIn Database: `select <cols> from <table> where <pred>` (SQL/LINQ relational projection)."),
        ["database"] = ("Database Integration", "`database <name> = connect(<uri>)` — Declares a generic database connection (Postgres, SQLite, or in-memory)."),
        ["transaction"] = ("Database Integration", "`transaction(<db>) { ... }` — Executes relational mutations within an ACID transaction supporting `savepoint` and `rollback`."),
        ["savepoint"] = ("Database Integration", "`savepoint <name>` — Marks a named transaction savepoint that can be targeted by `rollback to savepoint <name>`."),
        ["lookup"] = ("Database Integration", "`lookup <table> with index(<col>) where <pred>` — Performs an $O(1)$ / $O(\\log N)$ direct indexed point lookup on an in-memory table."),
        ["unit"] = ("UDSA", "`unit <name> [= <unit_expr>]` — Declares a first-class dimensional unit of measure (e.g., `unit meter`, `unit speed = meter / second`)."),
        ["dsl"] = ("UDSA", "`dsl <name>(<params>) { ... }` — Defines an embedded domain-specific language abstraction block."),
        ["macro"] = ("UDSA Metaprogramming", "`macro <name>(<params>) { quote { ... } }` — Defines a compile-time hygienic AST macro."),
        ["quote"] = ("UDSA Metaprogramming", "`quote { ... }` — Constructs a compile-time AST fragment with `$splice` interpolation."),
        ["property"] = ("C# Robustness", "`property <name>: <type> { get { ... } set(v) { ... } }` — Declares a C#-style encapsulated property."),
        ["operator"] = ("C# Robustness", "`operator <op>(self, other: <T>) -> <R> { ... }` — Overloads an arithmetic, comparison, or index operator on a `class` or `struct`."),
        ["extension"] = ("C# Robustness", "`extension [<Name>] for <Type> [where ...] { ... }` — Adds C#-style extension methods to an existing type.")
    };

    public async Task RunAsync(Stream input, Stream output, CancellationToken cancellationToken = default)
    {
        _outputStream = output;

        while (!cancellationToken.IsCancellationRequested)
        {
            string? json = await ReadMessageAsync(input, cancellationToken);
            if (json == null)
                break;

            var responses = ProcessMessage(json);
            foreach (var resp in responses)
            {
                SendRawJson(resp);
            }

            if (_shutdownRequested && json.Contains("\"exit\"", StringComparison.Ordinal))
                break;
        }
    }

    public List<string> ProcessMessage(string json)
    {
        var outgoing = new List<string>();
        JsonNode? root;
        try
        {
            root = JsonNode.Parse(json);
        }
        catch
        {
            return outgoing;
        }

        if (root == null) return outgoing;

        string? method = root["method"]?.GetValue<string>();
        JsonNode? idNode = root["id"];
        JsonNode? @params = root["params"];

        if (string.IsNullOrEmpty(method))
            return outgoing;

        switch (method)
        {
            case "initialize":
            {
                var result = new JsonObject
                {
                    ["capabilities"] = new JsonObject
                    {
                        ["textDocumentSync"] = new JsonObject
                        {
                            ["openClose"] = true,
                            ["change"] = 1, // Full document sync
                            ["save"] = new JsonObject { ["includeText"] = true }
                        },
                        ["hoverProvider"] = true,
                        ["definitionProvider"] = true,
                        ["documentSymbolProvider"] = true,
                        ["completionProvider"] = new JsonObject
                        {
                            ["resolveProvider"] = false,
                            ["triggerCharacters"] = new JsonArray(".", ":", "@", "<")
                        },
                        ["semanticTokensProvider"] = new JsonObject
                        {
                            ["legend"] = new JsonObject
                            {
                                ["tokenTypes"] = new JsonArray(PilotSemanticTokenLegend.TokenTypes.Select(t => (JsonNode)JsonValue.Create(t)!).ToArray()),
                                ["tokenModifiers"] = new JsonArray(PilotSemanticTokenLegend.TokenModifiers.Select(m => (JsonNode)JsonValue.Create(m)!).ToArray())
                            },
                            ["full"] = true
                        }
                    },
                    ["serverInfo"] = new JsonObject
                    {
                        ["name"] = "Pilot Language Server",
                        ["version"] = "1.0.0"
                    }
                };
                outgoing.Add(BuildResponse(idNode, result));
                break;
            }

            case "initialized":
                break;

            case "shutdown":
                _shutdownRequested = true;
                outgoing.Add(BuildResponse(idNode, null));
                break;

            case "exit":
                _shutdownRequested = true;
                break;

            case "textDocument/didOpen":
            {
                var doc = @params?["textDocument"];
                string? uri = doc?["uri"]?.GetValue<string>();
                string? text = doc?["text"]?.GetValue<string>();
                if (uri != null && text != null)
                {
                    outgoing.Add(UpdateDocumentAndBuildDiagnostics(uri, text));
                }
                break;
            }

            case "textDocument/didChange":
            {
                string? uri = @params?["textDocument"]?["uri"]?.GetValue<string>();
                var changes = @params?["contentChanges"]?.AsArray();
                if (uri != null && changes != null && changes.Count > 0)
                {
                    string? text = changes[^1]?["text"]?.GetValue<string>();
                    if (text != null)
                    {
                        outgoing.Add(UpdateDocumentAndBuildDiagnostics(uri, text));
                    }
                }
                break;
            }

            case "textDocument/didSave":
            {
                string? uri = @params?["textDocument"]?["uri"]?.GetValue<string>();
                string? text = @params?["text"]?.GetValue<string>();
                if (uri != null)
                {
                    if (text == null && _documents.TryGetValue(uri, out var cached))
                        text = cached;
                    if (text != null)
                        outgoing.Add(UpdateDocumentAndBuildDiagnostics(uri, text));
                }
                break;
            }

            case "textDocument/didClose":
            {
                string? uri = @params?["textDocument"]?["uri"]?.GetValue<string>();
                if (uri != null)
                {
                    _documents.TryRemove(uri, out _);
                    _analyses.TryRemove(uri, out _);
                    var clearDiag = new JsonObject
                    {
                        ["jsonrpc"] = "2.0",
                        ["method"] = "textDocument/publishDiagnostics",
                        ["params"] = new JsonObject
                        {
                            ["uri"] = uri,
                            ["diagnostics"] = new JsonArray()
                        }
                    };
                    outgoing.Add(clearDiag.ToJsonString());
                }
                break;
            }

            case "textDocument/semanticTokens/full":
            {
                string? uri = @params?["textDocument"]?["uri"]?.GetValue<string>();
                int[] data = Array.Empty<int>();
                if (uri != null && TryGetAnalysis(uri, out var analysis))
                {
                    data = analysis.EncodeSemanticTokens();
                }

                var dataArray = new JsonArray(data.Select(n => (JsonNode)JsonValue.Create(n)).ToArray());
                outgoing.Add(BuildResponse(idNode, new JsonObject { ["data"] = dataArray }));
                break;
            }

            case "textDocument/hover":
            {
                string? uri = @params?["textDocument"]?["uri"]?.GetValue<string>();
                int line = @params?["position"]?["line"]?.GetValue<int>() ?? 0;
                int character = @params?["position"]?["character"]?.GetValue<int>() ?? 0;

                JsonNode? hoverResult = null;
                if (uri != null && TryGetAnalysis(uri, out var analysis))
                {
                    var sym = analysis.FindSymbolAtPosition(line, character);
                    if (sym != null)
                    {
                        hoverResult = new JsonObject
                        {
                            ["contents"] = new JsonObject
                            {
                                ["kind"] = "markdown",
                                ["value"] = sym.FormatHoverMarkdown()
                            },
                            ["range"] = CreateRange(sym.SelectionLine, sym.SelectionColumn, sym.SelectionLine, sym.SelectionEndColumn)
                        };
                    }
                    else if (_documents.TryGetValue(uri, out var source))
                    {
                        string? word = ExtractWordAt(source, line, character);
                        if (word != null && KeywordDocs.TryGetValue(word, out var kwDoc))
                        {
                            hoverResult = new JsonObject
                            {
                                ["contents"] = new JsonObject
                                {
                                    ["kind"] = "markdown",
                                    ["value"] = $"**Pilot Keyword** (`{kwDoc.Domain}`)\n\n{kwDoc.Summary}"
                                }
                            };
                        }
                    }
                }

                outgoing.Add(BuildResponse(idNode, hoverResult));
                break;
            }

            case "textDocument/definition":
            {
                string? uri = @params?["textDocument"]?["uri"]?.GetValue<string>();
                int line = @params?["position"]?["line"]?.GetValue<int>() ?? 0;
                int character = @params?["position"]?["character"]?.GetValue<int>() ?? 0;

                JsonNode? defResult = null;
                if (uri != null && TryGetAnalysis(uri, out var analysis))
                {
                    var sym = analysis.FindSymbolAtPosition(line, character);
                    if (sym != null && sym.SelectionEndColumn > sym.SelectionColumn)
                    {
                        defResult = new JsonObject
                        {
                            ["uri"] = uri,
                            ["range"] = CreateRange(sym.SelectionLine, sym.SelectionColumn, sym.SelectionLine, sym.SelectionEndColumn)
                        };
                    }
                }

                outgoing.Add(BuildResponse(idNode, defResult));
                break;
            }

            case "textDocument/documentSymbol":
            {
                string? uri = @params?["textDocument"]?["uri"]?.GetValue<string>();
                var symbolsArray = new JsonArray();
                if (uri != null && TryGetAnalysis(uri, out var analysis))
                {
                    foreach (var sym in analysis.TopLevelSymbols)
                    {
                        symbolsArray.Add(ConvertToDocumentSymbol(sym));
                    }
                }

                outgoing.Add(BuildResponse(idNode, symbolsArray));
                break;
            }

            case "textDocument/completion":
            {
                string? uri = @params?["textDocument"]?["uri"]?.GetValue<string>();
                int line = @params?["position"]?["line"]?.GetValue<int>() ?? 0;
                var items = new JsonArray();

                if (uri != null && TryGetAnalysis(uri, out var analysis))
                {
                    var scope = analysis.FindScopeAtLine(line);
                    foreach (var sym in scope.GetAllVisibleSymbols())
                    {
                        items.Add(new JsonObject
                        {
                            ["label"] = sym.Name,
                            ["kind"] = GetCompletionItemKind(sym.Kind),
                            ["detail"] = string.IsNullOrEmpty(sym.Signature) ? sym.TypeName : sym.Signature,
                            ["documentation"] = new JsonObject
                            {
                                ["kind"] = "markdown",
                                ["value"] = sym.FormatHoverMarkdown()
                            }
                        });
                    }
                }

                foreach (var kvp in KeywordDocs)
                {
                    items.Add(new JsonObject
                    {
                        ["label"] = kvp.Key,
                        ["kind"] = 14, // Keyword
                        ["detail"] = $"Pilot keyword ({kvp.Value.Domain})",
                        ["documentation"] = new JsonObject
                        {
                            ["kind"] = "markdown",
                            ["value"] = kvp.Value.Summary
                        }
                    });
                }

                outgoing.Add(BuildResponse(idNode, items));
                break;
            }
        }

        return outgoing;
    }

    private string UpdateDocumentAndBuildDiagnostics(string uri, string text)
    {
        _documents[uri] = text;
        var analysis = PilotSemanticAnalyzer.Analyze(text, uri);
        _analyses[uri] = analysis;

        var diagArray = new JsonArray();
        foreach (var d in analysis.Diagnostics)
        {
            diagArray.Add(new JsonObject
            {
                ["range"] = CreateRange(d.StartLine, d.StartColumn, d.EndLine, d.EndColumn),
                ["severity"] = (int)d.Severity,
                ["code"] = d.Code,
                ["source"] = d.Source,
                ["message"] = d.Message
            });
        }

        var notification = new JsonObject
        {
            ["jsonrpc"] = "2.0",
            ["method"] = "textDocument/publishDiagnostics",
            ["params"] = new JsonObject
            {
                ["uri"] = uri,
                ["diagnostics"] = diagArray
            }
        };

        return notification.ToJsonString();
    }

    private bool TryGetAnalysis(string uri, out PilotAnalysisResult analysis)
    {
        if (_analyses.TryGetValue(uri, out var found))
        {
            analysis = found;
            return true;
        }
        if (_documents.TryGetValue(uri, out var text))
        {
            analysis = PilotSemanticAnalyzer.Analyze(text, uri);
            _analyses[uri] = analysis;
            return true;
        }
        analysis = null!;
        return false;
    }

    private static JsonObject ConvertToDocumentSymbol(PilotSymbol sym)
    {
        var children = new JsonArray();
        foreach (var child in sym.Children)
        {
            children.Add(ConvertToDocumentSymbol(child));
        }

        return new JsonObject
        {
            ["name"] = sym.Name,
            ["detail"] = sym.TypeName,
            ["kind"] = sym.GetLspDocumentSymbolKind(),
            ["range"] = CreateRange(sym.StartLine, sym.StartColumn, sym.EndLine, Math.Max(sym.StartColumn + 1, sym.EndColumn)),
            ["selectionRange"] = CreateRange(sym.SelectionLine, sym.SelectionColumn, sym.SelectionLine, Math.Max(sym.SelectionColumn + 1, sym.SelectionEndColumn)),
            ["children"] = children
        };
    }

    private static int GetCompletionItemKind(PilotSymbolKind kind) => kind switch
    {
        PilotSymbolKind.Function => 3,       // Function
        PilotSymbolKind.Method => 2,         // Method
        PilotSymbolKind.Class or PilotSymbolKind.Actor or PilotSymbolKind.Model or PilotSymbolKind.Table => 7, // Class
        PilotSymbolKind.Struct => 22,        // Struct
        PilotSymbolKind.Interface => 8,      // Interface
        PilotSymbolKind.Enum => 13,          // Enum
        PilotSymbolKind.EnumMember => 20,    // EnumMember
        PilotSymbolKind.Property => 10,      // Property
        PilotSymbolKind.Unit => 11,          // Unit
        PilotSymbolKind.Variable or PilotSymbolKind.Parameter or PilotSymbolKind.QueryVariable => 6, // Variable
        _ => 6
    };

    private static JsonObject CreateRange(int startLine, int startCol, int endLine, int endCol) => new()
    {
        ["start"] = new JsonObject { ["line"] = Math.Max(0, startLine), ["character"] = Math.Max(0, startCol) },
        ["end"] = new JsonObject { ["line"] = Math.Max(0, endLine), ["character"] = Math.Max(0, endCol) }
    };

    private static string BuildResponse(JsonNode? idNode, JsonNode? result)
    {
        var resp = new JsonObject
        {
            ["jsonrpc"] = "2.0",
            ["id"] = idNode?.DeepClone(),
            ["result"] = result
        };
        return resp.ToJsonString();
    }

    private static string? ExtractWordAt(string source, int line, int character)
    {
        var lines = source.Replace("\r\n", "\n").Split('\n');
        if (line < 0 || line >= lines.Length) return null;
        string textLine = lines[line];
        if (character < 0 || character > textLine.Length) return null;

        int start = character;
        while (start > 0 && (char.IsLetterOrDigit(textLine[start - 1]) || textLine[start - 1] == '_'))
            start--;

        int end = character;
        while (end < textLine.Length && (char.IsLetterOrDigit(textLine[end]) || textLine[end] == '_'))
            end++;

        return end > start ? textLine[start..end] : null;
    }

    private void SendRawJson(string json)
    {
        if (_outputStream == null) return;
        byte[] payload = Encoding.UTF8.GetBytes(json);
        byte[] header = Encoding.ASCII.GetBytes($"Content-Length: {payload.Length}\r\n\r\n");

        lock (_writeLock)
        {
            _outputStream.Write(header, 0, header.Length);
            _outputStream.Write(payload, 0, payload.Length);
            _outputStream.Flush();
        }
    }

    private static async Task<string?> ReadMessageAsync(Stream input, CancellationToken ct)
    {
        int contentLength = -1;
        var headerBuilder = new StringBuilder();
        byte[] oneByte = new byte[1];

        while (!ct.IsCancellationRequested)
        {
            int read = await input.ReadAsync(oneByte.AsMemory(0, 1), ct);
            if (read == 0) return null;

            headerBuilder.Append((char)oneByte[0]);
            int len = headerBuilder.Length;
            if (len >= 4 &&
                headerBuilder[len - 4] == '\r' &&
                headerBuilder[len - 3] == '\n' &&
                headerBuilder[len - 2] == '\r' &&
                headerBuilder[len - 1] == '\n')
            {
                string headers = headerBuilder.ToString();
                foreach (var line in headers.Split("\r\n", StringSplitOptions.RemoveEmptyEntries))
                {
                    if (line.StartsWith("Content-Length:", StringComparison.OrdinalIgnoreCase) &&
                        int.TryParse(line["Content-Length:".Length..].Trim(), out int parsed))
                    {
                        contentLength = parsed;
                    }
                }
                break;
            }
        }

        if (contentLength <= 0) return null;

        byte[] body = new byte[contentLength];
        int offset = 0;
        while (offset < contentLength)
        {
            int read = await input.ReadAsync(body.AsMemory(offset, contentLength - offset), ct);
            if (read == 0) return null;
            offset += read;
        }

        return Encoding.UTF8.GetString(body);
    }
}
