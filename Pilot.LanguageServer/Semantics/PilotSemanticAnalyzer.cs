using System;
using System.Collections.Generic;
using System.Linq;
using Antlr4.Runtime;
using Antlr4.Runtime.Tree;
using Pilot.Preprocessing;

namespace Pilot.LanguageServer.Semantics;

public sealed class PilotSemanticAnalyzer : PilotBaseVisitor<string?>
{
    private readonly PilotAnalysisResult _result;
    private PilotScope _currentScope;
    private PilotPreprocessResult? _preprocResult;
    private readonly HashSet<string> _activeSavepoints = new(StringComparer.Ordinal);
    private readonly Dictionary<string, string> _unitAliases = new(StringComparer.Ordinal);

    private static readonly HashSet<string> BuiltinTypes = new(StringComparer.OrdinalIgnoreCase)
    {
        // Characters & Strings
        "char", "str", "string",

        // Signed Integers
        "i8", "i16", "i32", "int", "i64",

        // Unsigned Integers
        "u8", "u16", "u32", "uint", "u64",

        // Floating Point & Numeric
        "f8", "f16", "f32", "float", "f64", "double", "decimal", "complex",

        // Collections & Sequences
        "tuple", "Tuple", "list", "List", "range", "map", "Map", "dict", "Dict", "set", "Set", "frozenset",

        // Binary & Memory Buffers
        "bool", "bytes", "bytearray", "memoryview",

        // Nullability & Undefined
        "null", "none", "None", "undefined", "void", "any", "object",

        // Domain & Concurrency primitives
        "symbol", "dynamic", "Table", "Dataset", "Relation", "Channel", "Chan", "Task", "Future",
        "Result", "Option", "Span", "Memory", "AstNode", "Exception", "ValueError"
    };

    private static readonly System.Text.RegularExpressions.Regex MatrixTypeRegex = new(@"^matrix\d+x\d+$", System.Text.RegularExpressions.RegexOptions.IgnoreCase | System.Text.RegularExpressions.RegexOptions.Compiled);
    private static bool IsBuiltinType(string typeName)
    {
        if (BuiltinTypes.Contains(typeName)) return true;
        // matrix#x# pattern, e.g. matrix3x3, matrix4x4, matrix2x4, etc.
        if (MatrixTypeRegex.IsMatch(typeName))
            return true;
        return false;
    }

    public static readonly Dictionary<string, (string ReturnType, string Signature, string Doc)> RootObjectMethods = new(StringComparer.Ordinal)
    {
        ["to_string"] = ("str", "pure def to_string() -> str", "Returns a string representing the current object instance."),
        ["ToString"] = ("str", "pure def ToString() -> str", "C#-compatible: returns a string representing the current object instance."),
        ["hash"] = ("int", "pure def hash() -> int", "Serves as the default hash function for hashing, hash tables, and set collections."),
        ["GetHashCode"] = ("int", "pure def GetHashCode() -> int", "C#-compatible: serves as the default hash function."),
        ["equals"] = ("bool", "pure def equals(other: any) -> bool", "Determines whether the specified object is structurally or referentially equal to the current instance."),
        ["Equals"] = ("bool", "pure def Equals(other: any) -> bool", "C#-compatible: determines whether the specified object is equal to the current instance."),
        ["clone"] = ("any", "def clone() -> any", "Creates and returns a copy of the current object instance."),
        ["Clone"] = ("any", "def Clone() -> any", "C#-compatible: creates and returns a copy of the current object instance."),
        ["type_of"] = ("str", "pure def type_of() -> str", "Gets the runtime type descriptor or metadata name of the current instance."),
        ["GetType"] = ("any", "pure def GetType() -> any", "C#-compatible: gets the runtime Type metadata of the current instance."),
        ["deconstruct"] = ("tuple", "pure def deconstruct() -> tuple", "Deconstructs the object instance into positional members for pattern matching and tuple assignment."),
        ["Deconstruct"] = ("tuple", "pure def Deconstruct() -> tuple", "C#-compatible: deconstructs the object instance into positional members."),
        ["__str__"] = ("str", "pure def __str__() -> str", "Python-compatible string representation."),
        ["__repr__"] = ("str", "pure def __repr__() -> str", "Python-compatible official representation."),
        ["__hash__"] = ("int", "pure def __hash__() -> int", "Python-compatible hash code."),
        ["__eq__"] = ("bool", "pure def __eq__(other: any) -> bool", "Python-compatible equality comparison."),
        ["__ne__"] = ("bool", "pure def __ne__(other: any) -> bool", "Python-compatible inequality comparison."),
        ["__init__"] = ("void", "def __init__(*args, **kwargs) -> void", "Python-compatible object initializer."),
        ["__clone__"] = ("any", "def __clone__() -> any", "Python-compatible object copy.")
    };

    public static readonly Dictionary<string, PilotSymbol> RootMethodSymbols = new(StringComparer.Ordinal);

    static PilotSemanticAnalyzer()
    {
        foreach (var kvp in RootObjectMethods)
        {
            RootMethodSymbols[kvp.Key] = new PilotSymbol
            {
                Name = kvp.Key,
                Kind = PilotSymbolKind.Method,
                TypeName = kvp.Value.ReturnType,
                Domain = "Core / Object",
                Signature = kvp.Value.Signature,
                Documentation = kvp.Value.Doc
            };
        }
    }

    private static readonly Dictionary<string, (string ReturnType, string Signature, string Doc)> BuiltinFunctions = new(StringComparer.Ordinal)
    {
        ["print"] = ("void", "def print(*values: any) -> void", "Writes values to the standard output stream."),
        ["println"] = ("void", "def println(*values: any) -> void", "Writes values to the standard output stream followed by a newline."),
        ["assert"] = ("bool", "def assert(condition: bool, message: str = \"\") -> bool", "Asserts that a condition is true; raises PilotRuntimeException if false."),
        ["type_of"] = ("str", "pure def type_of(value: any) -> str", "Returns the runtime type descriptor or name of the value."),
        ["clock"] = ("float", "pure def clock() -> float", "Returns the high-resolution elapsed time in milliseconds."),
        ["matrix"] = ("Matrix", "pure def matrix(rows: int = 1, cols: int = 1) -> Matrix", "Constructs a zero-initialized real-time matrix."),
        ["eye"] = ("Matrix", "pure def eye(size: int = 3) -> Matrix", "Constructs an n-by-n identity matrix."),
        ["zeros"] = ("Matrix", "pure def zeros(rows: int = 3, cols: int = 3) -> Matrix", "Constructs a rows-by-cols matrix populated with zeros."),
        ["ones"] = ("Matrix", "pure def ones(rows: int = 3, cols: int = 3) -> Matrix", "Constructs a rows-by-cols matrix populated with ones."),
        ["str"] = ("str", "pure def str(val: any = \"\") -> str", "Converts a value to its string representation."),
        ["int"] = ("int", "pure def int(val: any = 0) -> int", "Converts a value to an integer."),
        ["float"] = ("float", "pure def float(val: any = 0.0) -> float", "Converts a value to a 64-bit float."),
        ["bool"] = ("bool", "pure def bool(val: any = false) -> bool", "Converts a value to a boolean."),
        ["next"] = ("any", "def next(generator: any) -> any", "Retrieves the next yielded value from a generator."),
        ["spawn"] = ("Future", "def spawn(callable: any, *args: any) -> Future", "Spawns an asynchronous concurrent task."),
        ["parallel_map"] = ("List[any]", "def parallel_map(callable: any, list: List[any]) -> List[any]", "Applies a function over elements in parallel."),
        ["lazy"] = ("Lazy", "def lazy(callable: any) -> Lazy", "Constructs a lazily-evaluated deferred value."),
        ["json_dumps"] = ("str", "pure def json_dumps(value: any, formatted: bool = false) -> str", "Serializes a value or object graph to JSON string."),
        ["json_loads"] = ("any", "def json_loads(json_string: str, target_class: any = null) -> any", "Deserializes a JSON string into Pilot values or typed objects."),
        ["json"] = ("object", "object json", "Built-in JSON serialization module (`dumps`, `loads`)."),
        ["register_adapter"] = ("bool", "def register_adapter(from_type: str, to_type: str, converter: any) -> bool", "Registers a custom bidirectional type adapter."),
        ["TypeAdapter"] = ("object", "object TypeAdapter", "Built-in TypeAdapter registry object (`register`)."),
        ["vectorize"] = ("any", "def vectorize(target: any) -> any", "Vectorizes a scalar mathematical function over collections."),
        ["auto_repeat"] = ("str", "str auto_repeat", "Auto repeat execution policy."),
        ["repeat"] = ("str", "str repeat", "Repeat execution policy."),
        ["venv"] = ("object", "object venv", "Virtual environment manager module (`create`, `activate`, `deactivate`, `is_active`, `active_path`)."),
        ["virtual_environment"] = ("object", "object virtual_environment", "Alias for the `venv` virtual environment module."),
        ["transpile"] = ("str", "def transpile(pilot_code: str, target_lang: str) -> str", "Transpiles Pilot code to target language (Python, C#, Rust, C)."),
        ["transpile_file"] = ("str", "def transpile_file(input_path: str, target_lang: str, output_path: str = \"\") -> str", "Transpiles a Pilot source file to target language."),
        ["ingest"] = ("str", "def ingest(foreign_code: str, source_lang: str) -> str", "Ingests foreign code into equivalent idiomatic Pilot code."),
        ["ingest_file"] = ("str", "def ingest_file(input_path: str, source_lang: str = \"auto\", output_path: str = \"\") -> str", "Ingests a foreign source file into Pilot code."),
        ["transpile_library"] = ("List[str]", "def transpile_library(pilot_dir: str, target_lang: str, output_dir: str) -> List[str]", "Batch transpiles an entire Pilot package/library."),
        ["ingest_library"] = ("List[str]", "def ingest_library(source_dir: str, source_lang: str, output_dir: str) -> List[str]", "Batch ingests an entire foreign library into Pilot."),
        ["hot_reload"] = ("Map", "def hot_reload(code_or_path: str) -> Map", "Applies source patch or file updates to active runtime session."),
        ["apply_patch"] = ("Map", "def apply_patch(patch_code: str, label: str = \"\") -> Map", "Applies an inline code patch in real-time."),
        ["patch_file"] = ("Map", "def patch_file(file_path: str, only_if_changed: bool = false) -> Map", "Applies a patch file to running system."),
        ["rollback_patch"] = ("Map", "def rollback_patch(steps: int = 1) -> Map", "Rolls back applied hot-reload patches."),
        ["patch_binary"] = ("str", "def patch_binary(base_path: str, patch_path: str, output_path: str = \"\") -> str", "Directly patches a compiled Pilot binary program."),
        ["HotReload"] = ("object", "object HotReload", "Comprehensive live hot-reloading manager object."),
        ["cwd"] = ("str", "def cwd() -> str", "Returns the current working directory."),
        ["exit"] = ("void", "def exit(code: int = 0) -> void", "Terminates script execution with an exit code."),
        ["read_text"] = ("str", "def read_text(path: str) -> str", "Reads the entire contents of a file as a string."),
        ["write_text"] = ("bool", "def write_text(path: str, content: str) -> bool", "Writes text content to a file."),
        ["file_exists"] = ("bool", "def file_exists(path: str) -> bool", "Returns true if the specified file exists."),
        ["delete_file"] = ("bool", "def delete_file(path: str) -> bool", "Deletes the specified file."),
        ["args"] = ("List[str]", "List[str] args", "Command-line argument list passed to the script."),
        ["argv"] = ("List[str]", "List[str] argv", "Command-line argument list passed to the script."),
        ["env"] = ("Map[str, str]", "Map[str, str] env", "Dictionary of system environment variables."),
        ["script_path"] = ("str", "str script_path", "Full path of the currently executing script."),
        ["len"] = ("int", "pure def len(collection: any) -> int", "Returns the number of elements in a collection or string."),
        ["range"] = ("List[int]", "pure def range(start: int, end: int) -> List[int]", "Generates a sequence of integers."),
        ["connect"] = ("Database", "def connect(connection_string: str) -> Database", "Connects generically to a Postgres, SQLite, or in-memory database."),
        ["sum"] = ("float", "pure def sum(expr: any) -> float", "Aggregate function computing the sum of values."),
        ["avg"] = ("float", "pure def avg(expr: any) -> float", "Aggregate function computing the arithmetic mean."),
        ["min"] = ("any", "pure def min(expr: any) -> any", "Aggregate function returning the minimum value."),
        ["max"] = ("any", "pure def max(expr: any) -> any", "Aggregate function returning the maximum value."),
        ["count"] = ("int", "pure def count(expr: any = *) -> int", "Aggregate function returning the count of rows or items."),
        ["rank"] = ("int", "pure def rank() -> int", "Postgres window function computing rank within a partition."),
        ["row_number"] = ("int", "pure def row_number() -> int", "Postgres window function computing sequential row number."),
        ["dense_rank"] = ("int", "pure def dense_rank() -> int", "Postgres window function computing dense rank without gaps."),
        ["getattr"] = ("any", "def getattr(obj: any, name: str) -> any", "Retrieves an attribute value dynamically or in a macro."),

        // Lisp-style homoiconic metaprogramming & AST manipulation
        ["gensym"] = ("AstNode", "def gensym(prefix: str = \"g\") -> AstNode", "Generates a unique uninterned compile-time AST symbol for hygienic macros."),
        ["macroexpand"] = ("AstNode", "def macroexpand(ast: AstNode) -> AstNode", "Recursively expands all macro forms within the provided AST."),
        ["macroexpand_1"] = ("AstNode", "def macroexpand_1(ast: AstNode) -> AstNode", "Expands only the top-level macro form in the provided AST."),
        ["eval"] = ("any", "def eval(ast: any) -> any", "Evaluates an AST expression in the current environment."),
        ["cons"] = ("AstNode", "pure def cons(head: any, tail: any) -> AstNode", "Constructs a new homoiconic AST list pair from head and tail components."),
        ["car"] = ("any", "pure def car(list: any) -> any", "Returns the first element (head) of a homoiconic AST list."),
        ["cdr"] = ("any", "pure def cdr(list: any) -> any", "Returns the tail of a homoiconic AST list."),
        ["atom"] = ("bool", "pure def atom(node: any) -> bool", "Returns true if the node is an atomic AST element rather than a list."),
        ["list_ast"] = ("AstNode", "def list_ast(*elements: any) -> AstNode", "Constructs an AST list node containing the provided elements."),
        ["symbol"] = ("AstNode", "pure def symbol(name: str) -> AstNode", "Constructs a named AST identifier/symbol node.")
    };

    private PilotSemanticAnalyzer(string uri)
    {
        var rootScope = new PilotScope
        {
            Name = "global",
            StartLine = 0,
            EndLine = int.MaxValue
        };

        foreach (var kvp in BuiltinFunctions)
        {
            rootScope.Symbols[kvp.Key] = new PilotSymbol
            {
                Name = kvp.Key,
                Kind = kvp.Key is "json" or "TypeAdapter" or "venv" or "virtual_environment" or "HotReload" ? PilotSymbolKind.Class
                    : kvp.Key is "args" or "argv" or "env" or "script_path" or "auto_repeat" or "repeat" ? PilotSymbolKind.Variable
                    : PilotSymbolKind.Function,

                TypeName = kvp.Value.ReturnType,
                Signature = kvp.Value.Signature,
                Documentation = kvp.Value.Doc,
                Domain = kvp.Key is "connect" or "sum" or "avg" or "count" or "rank" or "row_number" or "dense_rank" ? "Database"
                    : kvp.Key is "gensym" or "macroexpand" or "macroexpand_1" or "eval" or "cons" or "car" or "cdr" or "atom" or "list_ast" or "symbol" ? "Metaprogramming / Lisp"
                    : kvp.Key is "spawn" or "parallel_map" ? "Concurrency"
                    : kvp.Key is "matrix" or "eye" or "zeros" or "ones" ? "Numerics / Matrix"
                    : kvp.Key is "hot_reload" or "apply_patch" or "patch_file" or "rollback_patch" or "patch_binary" or "HotReload" ? "Hot Reload"
                    : kvp.Key is "transpile" or "transpile_file" or "ingest" or "ingest_file" or "transpile_library" or "ingest_library" ? "Transpilation"
                    : kvp.Key is "read_text" or "write_text" or "file_exists" or "delete_file" or "cwd" or "exit" or "args" or "argv" or "env" or "script_path" ? "Scripting / IO"
                    : "Core"
            };
        }

        // Register root 'object' and 'Object' classes
        var objectSym = new PilotSymbol
        {
            Name = "object",
            Kind = PilotSymbolKind.Class,
            TypeName = "object",
            Domain = "Core / Object",
            Signature = "class object",
            Documentation = "The ultimate root base class of all Pilot and polyglot types."
        };
        foreach (var rm in RootMethodSymbols.Values)
        {
            objectSym.Members[rm.Name] = rm;
            objectSym.Children.Add(rm);
        }
        rootScope.Symbols["object"] = objectSym;
        rootScope.Symbols["Object"] = objectSym;

        // Register homoiconic 'AstNode' class
        var astSym = new PilotSymbol
        {
            Name = "AstNode",
            Kind = PilotSymbolKind.Class,
            TypeName = "AstNode",
            Domain = "Metaprogramming / Lisp",
            Signature = "class AstNode",
            Documentation = "First-class homoiconic AST node representing Pilot / Lisp code-as-data."
        };
        astSym.Members["car"] = new PilotSymbol { Name = "car", Kind = PilotSymbolKind.Method, TypeName = "AstNode", Domain = "Metaprogramming", Signature = "pure def car() -> AstNode", Documentation = "Lisp homoiconic: returns the head element of this AST list node." };
        astSym.Members["cdr"] = new PilotSymbol { Name = "cdr", Kind = PilotSymbolKind.Method, TypeName = "AstNode", Domain = "Metaprogramming", Signature = "pure def cdr() -> AstNode", Documentation = "Lisp homoiconic: returns the tail list of this AST list node." };
        astSym.Members["head"] = astSym.Members["car"];
        astSym.Members["tail"] = astSym.Members["cdr"];
        astSym.Members["cons"] = new PilotSymbol { Name = "cons", Kind = PilotSymbolKind.Method, TypeName = "AstNode", Domain = "Metaprogramming", Signature = "def cons(other: AstNode) -> AstNode", Documentation = "Constructs a new AST pair prepending this node to another." };
        astSym.Members["eval"] = new PilotSymbol { Name = "eval", Kind = PilotSymbolKind.Method, TypeName = "any", Domain = "Metaprogramming", Signature = "def eval() -> any", Documentation = "Evaluates this AST node in the current environment." };
        astSym.Members["to_source"] = new PilotSymbol { Name = "to_source", Kind = PilotSymbolKind.Method, TypeName = "str", Domain = "Metaprogramming", Signature = "pure def to_source() -> str", Documentation = "Converts this AST node back into formatted Pilot source code." };
        astSym.Members["is_atom"] = new PilotSymbol { Name = "is_atom", Kind = PilotSymbolKind.Property, TypeName = "bool", Domain = "Metaprogramming", Signature = "var is_atom: bool", Documentation = "True if this AST node is an atomic literal or symbol." };
        astSym.Members["is_list"] = new PilotSymbol { Name = "is_list", Kind = PilotSymbolKind.Property, TypeName = "bool", Domain = "Metaprogramming", Signature = "var is_list: bool", Documentation = "True if this AST node is a composite list or s-expression." };
        astSym.Members["is_symbol"] = new PilotSymbol { Name = "is_symbol", Kind = PilotSymbolKind.Property, TypeName = "bool", Domain = "Metaprogramming", Signature = "var is_symbol: bool", Documentation = "True if this AST node is an identifier or variable symbol." };
        astSym.Members["name"] = new PilotSymbol { Name = "name", Kind = PilotSymbolKind.Property, TypeName = "str", Domain = "Metaprogramming", Signature = "var name: str", Documentation = "Symbol name or node operator token." };
        astSym.Members["value"] = new PilotSymbol { Name = "value", Kind = PilotSymbolKind.Property, TypeName = "any", Domain = "Metaprogramming", Signature = "var value: any", Documentation = "Literal value or payload of the AST node." };
        astSym.Members["children"] = new PilotSymbol { Name = "children", Kind = PilotSymbolKind.Property, TypeName = "List[AstNode]", Domain = "Metaprogramming", Signature = "var children: List[AstNode]", Documentation = "Child AST subtrees." };

        foreach (var rm in RootMethodSymbols.Values)
        {
            if (!astSym.Members.ContainsKey(rm.Name))
                astSym.Members[rm.Name] = rm;
        }
        rootScope.Symbols["AstNode"] = astSym;

        _currentScope = rootScope;
        _result = new PilotAnalysisResult
        {
            Uri = uri,
            RootScope = rootScope
        };
    }

    public static PilotAnalysisResult Analyze(string code, string uri = "inmemory://test.pilot", PilotPreprocessorOptions? preprocessorOptions = null)
    {
        var preprocResult = PilotPreprocessor.Preprocess(code ?? string.Empty, uri, preprocessorOptions);

        var analyzer = new PilotSemanticAnalyzer(uri)
        {
            _preprocResult = preprocResult
        };

        // Transfer preprocessor diagnostics
        foreach (var pd in preprocResult.Diagnostics)
        {
            analyzer._result.Diagnostics.Add(new PilotDiagnostic
            {
                StartLine = Math.Max(0, pd.Line - 1),
                StartColumn = Math.Max(0, pd.Column),
                EndLine = Math.Max(0, pd.Line - 1),
                EndColumn = Math.Max(1, pd.Column + 1),
                Severity = pd.Severity == PreprocessorDiagnosticSeverity.Error ? PilotDiagnosticSeverity.Error : PilotDiagnosticSeverity.Warning,
                Code = pd.Code,
                Message = pd.Message
            });
        }

        var inputStream = new AntlrInputStream(preprocResult.PreprocessedSource);
        var lexer = new PilotLexer(inputStream);
        var lexerErrors = new LspErrorListener(analyzer._result.Diagnostics);
        lexer.RemoveErrorListeners();

        var tokenStream = new CommonTokenStream(lexer);
        var parser = new PilotParser(tokenStream);
        parser.RemoveErrorListeners();
        parser.AddErrorListener(lexerErrors);

        analyzer._result.Regions.AddRange(preprocResult.Regions);
        var tree = parser.ParseProgramFast();

        // Pass 1: Pre-register top-level type, model, actor, unit, and function symbols for forward references
        analyzer.PreRegisterTopLevelDeclarations(tree);

        // Pass 2: Full semantic walk
        analyzer.Visit(tree);

        // Pass 3: Milestone 3 Advanced Static Analysis (Type Inference, Borrow Checking, Dimensional Analysis, Contracts)
        var analysisRes = Pilot.Analysis.PilotAnalysisPipeline.Analyze(tree);
        foreach (var diag in analysisRes.Diagnostics)
        {
            if (diag.Code is "PILOT301" or "PILOT302" or "PILOT303" or "PILOT304" or "PILOT305" or "PILOT306")
            {
                analyzer._result.Diagnostics.Add(new PilotDiagnostic
                {
                    StartLine = Math.Max(0, diag.Line - 1),
                    StartColumn = Math.Max(0, diag.Column),
                    EndLine = Math.Max(0, diag.Line - 1),
                    EndColumn = Math.Max(1, diag.Column + 1),
                    Severity = diag.IsError ? PilotDiagnosticSeverity.Error : PilotDiagnosticSeverity.Warning,
                    Code = diag.Code,
                    Message = diag.Message
                });
            }
        }

        return analyzer._result;
    }

    private void PreRegisterTopLevelDeclarations(PilotParser.ProgramContext program)
    {
        foreach (var stmt in program.stmt())
        {
            var compound = stmt.compound_stmt();
            if (compound == null) continue;

            if (compound.class_def() is { } classDef && classDef.identifier() is { } classId)
            {
                RegisterTypeStub(classId, PilotSymbolKind.Class, "Core / C#");
            }
            else if (compound.struct_def() is { } structDef && structDef.identifier() is { } structId)
            {
                RegisterTypeStub(structId, PilotSymbolKind.Struct, "Core / Real-Time");
            }
            else if (compound.interface_def() is { } ifaceDef && ifaceDef.identifier() is { } ifaceId)
            {
                RegisterTypeStub(ifaceId, PilotSymbolKind.Interface, "Core / C#");
            }
            else if (compound.enum_def() is { } enumDef && enumDef.identifier() is { } enumId)
            {
                RegisterTypeStub(enumId, PilotSymbolKind.Enum, "Core");
            }
            else if (compound.actor_def() is { } actorDef && actorDef.identifier() is { } actorId)
            {
                RegisterTypeStub(actorId, PilotSymbolKind.Actor, "Concurrency");
            }
            else if (compound.model_def() is { } modelDef && modelDef.identifier() is { } modelId)
            {
                RegisterTypeStub(modelId, PilotSymbolKind.Model, "Database");
            }
            else if (compound.extern_def() is { } externDef)
            {
                string lang = externDef.STRING().Length > 0 ? externDef.STRING(0).GetText().Trim('"', '\'')
                    : externDef.identifier().Length > 0 ? externDef.identifier(0).GetText()
                    : "foreign";

                string domain = lang.ToLowerInvariant() switch
                {
                    "python" => "Python Interop",
                    "csharp" => "C# / .NET Interop",
                    "rust" => "Rust FFI",
                    "c" => "C FFI",
                    _ => $"{char.ToUpperInvariant(lang[0])}{lang[1..]} Interop"
                };

                foreach (var member in externDef.extern_member())
                {
                    if (member.identifier() is { } mId)
                    {
                        if (member.CLASS() != null)
                            RegisterTypeStub(mId, PilotSymbolKind.Class, domain);
                        else if (member.STRUCT() != null)
                            RegisterTypeStub(mId, PilotSymbolKind.Struct, domain);
                        else if (member.INTERFACE() != null)
                            RegisterTypeStub(mId, PilotSymbolKind.Interface, domain);
                        else if (member.DEF() != null || member.FN() != null)
                        {
                            string retType = member.type_spec()?.GetText() ?? "void";
                            RegisterFunctionStub(mId, retType, domain);
                        }
                    }
                }
            }
            else if (compound.macro_def() is { } macroDef && macroDef.identifier() is { } macroId)
            {
                RegisterFunctionStub(macroId, "AstNode", "Metaprogramming / Lisp");
            }
        }
    }

    private void RegisterFunctionStub(PilotParser.IdentifierContext idCtx, string returnType, string domain)
    {
        string name = idCtx.GetText();
        var (line, col, endCol) = GetTokenCoords(idCtx.Start);
        if (!_currentScope.Symbols.ContainsKey(name))
        {
            _currentScope.Symbols[name] = new PilotSymbol
            {
                Name = name,
                Kind = PilotSymbolKind.Function,

                TypeName = returnType,
                Domain = domain,
                SelectionLine = line,
                SelectionColumn = col,
                SelectionEndColumn = endCol,
                StartLine = line,
                StartColumn = col,
                EndLine = line,
                EndColumn = endCol,
                Signature = $"fn {name}(...) -> {returnType}"
            };
        }
    }

    private void RegisterTypeStub(PilotParser.IdentifierContext idCtx, PilotSymbolKind kind, string domain)
    {
        string name = idCtx.GetText();
        var (line, col, endCol) = GetTokenCoords(idCtx.Start);
        if (!_currentScope.Symbols.ContainsKey(name))
        {
            _currentScope.Symbols[name] = new PilotSymbol
            {
                Name = name,
                Kind = kind,
                TypeName = name,
                Domain = domain,
                SelectionLine = line,
                SelectionColumn = col,
                SelectionEndColumn = endCol,
                StartLine = line,
                StartColumn = col,
                EndLine = line,
                EndColumn = endCol,
                Signature = $"{kind.ToString().ToLowerInvariant()} {name}"
            };
        }
    }

    private void DeclareBindingPattern(
        PilotParser.Binding_patternContext patternCtx,
        ParserRuleContext stmtCtx,
        string finalType,
        bool isMutable,
        bool isConstant,
        string docPrefix,
        string domain = "Core")
    {
        if (patternCtx.identifier() is { } idCtx)
        {
            string name = idCtx.GetText();
            string sig = $"{(isConstant ? "const" : isMutable ? "var" : "let")} {name}: {finalType}";
            DeclareVariableSymbol(idCtx, stmtCtx, name, finalType, isMutable, isConstant, sig, $"{docPrefix} `{name}`.", domain);
            return;
        }

        if (patternCtx.splice_expr() is { } spliceCtx)
        {
            string name = spliceCtx.GetText();
            string sig = $"{(isConstant ? "const" : isMutable ? "var" : "let")} {name}: {finalType}";
            DeclareVariableSymbolFromToken(spliceCtx.Start, stmtCtx, name, finalType, isMutable, isConstant, sig, $"{docPrefix} `{name}`.", domain);
            return;
        }

        if (patternCtx.tuple_pattern() is { } tuplePat)
        {
            foreach (var item in tuplePat.binding_pattern_item())
            {
                string itemType = item.type_spec()?.GetText() ?? "any";
                DeclareBindingPattern(item.binding_pattern(), stmtCtx, itemType, isMutable, isConstant, docPrefix, domain);
            }
        }
        else if (patternCtx.list_pattern() is { } listPat)
        {
            foreach (var item in listPat.binding_pattern_item())
            {
                string itemType = item.type_spec()?.GetText() ?? "any";
                DeclareBindingPattern(item.binding_pattern(), stmtCtx, itemType, isMutable, isConstant, docPrefix, domain);
            }
        }
        else if (patternCtx.record_pattern() is { } recPat)
        {
            foreach (var field in recPat.record_pattern_field())
            {
                if (field.binding_pattern() is { } subPat)
                {
                    DeclareBindingPattern(subPat, stmtCtx, "any", isMutable, isConstant, docPrefix, domain);
                }
                else if (field.identifier() is { } fieldId)
                {
                    string name = fieldId.GetText();
                    string sig = $"{(isConstant ? "const" : isMutable ? "var" : "let")} {name}: any";
                    DeclareVariableSymbol(fieldId, stmtCtx, name, "any", isMutable, isConstant, sig, $"{docPrefix} `{name}`.", domain);
                }
            }
        }
    }

    public override string? VisitLet_stmt(PilotParser.Let_stmtContext context)
    {
        var pattern = context.binding_pattern();
        string name = pattern.GetText();
        string? explicitType = context.type_annotation()?.type_spec()?.GetText();
        string? inferredType = context.test() != null ? Visit(context.test()) : null;

        if (context.type_annotation()?.type_spec() != null)
            Visit(context.type_annotation().type_spec());

        string finalType = ValidateAndResolveType(name, explicitType, inferredType, context.test()?.Start ?? pattern.Start);
        DeclareBindingPattern(pattern, context, finalType, isMutable: false, isConstant: false, "Immutable variable binding (`let`). Cannot be reassigned.");
        return finalType;
    }

    public override string? VisitVar_stmt(PilotParser.Var_stmtContext context)
    {
        var pattern = context.binding_pattern();
        string name = pattern.GetText();
        string? explicitType = context.type_annotation()?.type_spec()?.GetText();
        string? inferredType = context.test() != null ? Visit(context.test()) : null;

        if (context.type_annotation()?.type_spec() != null)
            Visit(context.type_annotation().type_spec());

        string finalType = ValidateAndResolveType(name, explicitType, inferredType, context.test()?.Start ?? pattern.Start);
        bool isTable = finalType.StartsWith("Table[", StringComparison.OrdinalIgnoreCase) || context.index_clause() != null;

        DeclareBindingPattern(
            pattern,
            context,
            finalType,
            isMutable: true,
            isConstant: false,
            isTable ? "In-memory indexed relational table." : "Mutable variable binding (`var`).",
            isTable ? "Database" : "Core");

        if (context.index_clause() != null)
        {
            foreach (var idxField in context.index_clause().index_field())
            {
                string fieldName = idxField.identifier().GetText();
                var idxTypes = idxField.index_type().Select(t => t.GetText());
                AddSemanticToken(idxField.identifier().Start, PilotSemanticTokenLegend.TypeProperty, 0);
            }
        }

        return finalType;
    }

    public override string? VisitConst_stmt(PilotParser.Const_stmtContext context)
    {
        var pattern = context.binding_pattern();
        string name = pattern.GetText();
        string? explicitType = context.type_annotation()?.type_spec()?.GetText();
        string? inferredType = context.test() != null ? Visit(context.test()) : null;

        if (context.type_annotation()?.type_spec() != null)
            Visit(context.type_annotation().type_spec());

        string finalType = ValidateAndResolveType(name, explicitType, inferredType, context.test()?.Start ?? pattern.Start);
        DeclareBindingPattern(pattern, context, finalType, isMutable: false, isConstant: true, "Compile-time constant (`const`). Cannot be reassigned.");
        return finalType;
    }

    public override string? VisitRef_stmt(PilotParser.Ref_stmtContext context)
    {
        var pattern = context.binding_pattern();
        string name = pattern.GetText();
        string? explicitType = context.type_annotation()?.type_spec()?.GetText();
        string? inferredType = context.test() != null ? Visit(context.test()) : null;

        if (context.type_annotation()?.type_spec() != null)
            Visit(context.type_annotation().type_spec());

        string finalType = ValidateAndResolveType(name, explicitType, inferredType, context.test()?.Start ?? pattern.Start);
        DeclareBindingPattern(pattern, context, finalType, isMutable: true, isConstant: false, "Reference binding (`ref`).");
        return finalType;
    }

    public override string? VisitDynamic_stmt(PilotParser.Dynamic_stmtContext context)
    {
        var pattern = context.binding_pattern();
        if (context.test() != null) Visit(context.test());
        DeclareBindingPattern(pattern, context, "dynamic", isMutable: true, isConstant: false, "Dynamic typing binding (`dynamic`).");
        return "dynamic";
    }

    public override string? VisitAnnassign_stmt(PilotParser.Annassign_stmtContext context)
    {
        var tests = context.test();
        string targetText = tests[0].GetText();
        string explicitType = context.type_spec().GetText();
        Visit(context.type_spec());

        string? inferredType = tests.Length > 1 ? Visit(tests[1]) : null;
        string finalType = ValidateAndResolveType(targetText, explicitType, inferredType, (tests.Length > 1 ? tests[1] : tests[0]).Start);

        var idCtx = TryExtractSimpleIdentifier(tests[0]);
        if (idCtx != null)
        {
            DeclareVariableSymbol(
                idCtx,
                context,
                idCtx.GetText(),
                finalType,
                isMutable: true,
                isConstant: false,
                $"{idCtx.GetText()}: {finalType}",
                "Type-annotated field or variable.");
        }
        else
        {
            Visit(tests[0]);
        }

        return finalType;
    }

    public override string? VisitType_alias_stmt(PilotParser.Type_alias_stmtContext context)
    {
        var idCtx = context.identifier();
        string name = idCtx.GetText();
        string targetType = context.type_spec().GetText();
        Visit(context.type_spec());

        var symbol = CreateSymbol(idCtx, context, name, PilotSymbolKind.TypeAlias, targetType, "Core",
            $"type {name} = {targetType}", "Type alias definition.");
        RegisterSymbol(symbol, PilotSemanticTokenLegend.TypeType, PilotSemanticTokenLegend.ModDeclaration | PilotSemanticTokenLegend.ModDefinition);
        return targetType;
    }

    public override string? VisitExpr_stmt(PilotParser.Expr_stmtContext context)
    {
        var testlist = context.testlist();
        bool isAssignment = context.augassign().Length > 0 || context.EQUAL().Length > 0 || context.WALRUS().Length > 0;

        if (!isAssignment)
        {
            return base.VisitExpr_stmt(context);
        }

        var lhsTest = testlist?.test().FirstOrDefault();
        var rhsTests = context.test();
        string? rhsType = null;
        foreach (var rhs in rhsTests)
        {
            rhsType = Visit(rhs);
        }

        if (lhsTest != null)
        {
            var idCtx = TryExtractSimpleIdentifier(lhsTest);
            if (idCtx != null)
            {
                string varName = idCtx.GetText();
                var existing = _currentScope.Resolve(varName);
                if (existing != null)
                {
                    AddSymbolReference(idCtx.Start, existing);
                    if (!existing.IsMutable && existing.Kind == PilotSymbolKind.Variable)
                    {
                        string bindingKind = existing.IsConstant ? "const" : "let";
                        AddDiagnostic(
                            idCtx.Start,
                            PilotDiagnosticSeverity.Error,
                            "PILOT101",
                            $"Cannot reassign immutable binding '{varName}' (declared with '{bindingKind}'). Use 'var' if mutation is intended.");
                    }
                    else
                    {
                        AddSemanticToken(idCtx.Start, PilotSemanticTokenLegend.TypeVariable, PilotSemanticTokenLegend.ModModification);
                        if (rhsType != null && !IsCompatibleType(existing.TypeName, rhsType))
                        {
                            AddDiagnostic(
                                rhsTests.FirstOrDefault()?.Start ?? idCtx.Start,
                                PilotDiagnosticSeverity.Error,
                                "PILOT102",
                                $"Type mismatch in assignment to '{varName}': cannot assign '{rhsType}' to '{existing.TypeName}'.");
                        }
                    }
                    return existing.TypeName;
                }
                else if (context.augassign().Length == 0)
                {
                    // Implicit Pythonic variable assignment in scope (e.g. conn = open_connection())
                    string inferred = rhsType ?? "any";
                    DeclareVariableSymbol(
                        idCtx,
                        context,
                        varName,
                        inferred,
                        isMutable: true,
                        isConstant: false,
                        $"{varName}: {inferred}",
                        "Inferred mutable variable binding.");
                    return inferred;
                }
            }

            Visit(lhsTest);
        }

        return rhsType;
    }

    public override string? VisitGuard_stmt(PilotParser.Guard_stmtContext context)
    {
        AddSemanticToken(context.GUARD().Symbol, PilotSemanticTokenLegend.TypeKeyword, 0);
        if (context.ELSE() != null)
            AddSemanticToken(context.ELSE().Symbol, PilotSemanticTokenLegend.TypeKeyword, 0);

        if (context.LET() != null && context.pattern() != null)
        {
            string? initType = context.test() != null ? Visit(context.test()) : "any";
            if (initType != null && initType.EndsWith("?", StringComparison.Ordinal))
                initType = initType[..^1]; // Guard unwrap narrows nullable T? -> T

            BindPatternIdentifiers(context.pattern(), initType ?? "any", isMutable: false, "Unwrapped non-null binding from `guard let` precondition.");
        }
        else if (context.test() != null)
        {
            Visit(context.test());
        }

        var suite = context.suite();
        if (suite != null)
        {
            Visit(suite);
            if (!SuiteGuaranteesExit(suite))
            {
                AddDiagnostic(
                    context.GUARD().Symbol,
                    PilotDiagnosticSeverity.Error,
                    "PILOT103",
                    "Mandatory precondition 'guard' else-block must exit the enclosing scope via 'return', 'raise', 'break', or 'continue'.");
            }
        }

        return "void";
    }

    public override string? VisitDefer_stmt(PilotParser.Defer_stmtContext context)
    {
        AddSemanticToken(context.DEFER().Symbol, PilotSemanticTokenLegend.TypeKeyword, 0);
        return base.VisitDefer_stmt(context);
    }

    public override string? VisitFunc_def(PilotParser.Func_defContext context)
    {
        var idCtx = context.identifier();
        string name = idCtx.GetText();
        bool isAsync = context.ASYNC() != null;
        bool isPure = context.PURE() != null;
        bool isStatic = context.declaration_modifier().Any(m => m.STATIC() != null);
        bool isHardRt = false;
        bool isSoftRt = false;
        string? wcetBudget = null;

        if (context.decorators() != null)
        {
            foreach (var dec in context.decorators().decorator())
            {
                AddSemanticToken(dec.Start, PilotSemanticTokenLegend.TypeDecorator, 0);
                string decName = dec.dotted_name().GetText();
                if (decName is "hard_realtime" or "hard_rt") isHardRt = true;
                else if (decName is "soft_realtime" or "soft_rt") isSoftRt = true;
                else if (decName == "wcet") wcetBudget = dec.arglist()?.GetText();
            }
        }

        string returnType = context.type_spec()?.GetText() ?? "void";
        if (context.type_spec() != null) Visit(context.type_spec());

        string domain = isHardRt || isSoftRt ? "Real-Time"
            : isAsync ? "Concurrency"
            : "Core";

        var kind = _currentScope.EnclosingType != null ? PilotSymbolKind.Method : PilotSymbolKind.Function;
        var funcSymbol = CreateSymbol(idCtx, context, name, kind, returnType, domain, "",
            isHardRt ? "Hard real-time deterministic function." : isAsync ? "Asynchronous function." : "Function definition.",
            isAsync: isAsync, isPure: isPure, isHardRt: isHardRt, isSoftRt: isSoftRt, wcet: wcetBudget, isStatic: isStatic);

        int mods = PilotSemanticTokenLegend.ModDeclaration | PilotSemanticTokenLegend.ModDefinition;
        if (isAsync) mods |= PilotSemanticTokenLegend.ModAsync;
        if (isStatic) mods |= PilotSemanticTokenLegend.ModStatic;

        RegisterSymbol(
            funcSymbol,
            kind == PilotSymbolKind.Method ? PilotSemanticTokenLegend.TypeMethod : PilotSemanticTokenLegend.TypeFunction,
            mods);

        if (_currentScope.EnclosingType != null)
        {
            _currentScope.EnclosingType.Members[name] = funcSymbol;
            _currentScope.EnclosingType.Children.Add(funcSymbol);
        }

        using (EnterScope($"def:{name}", context, inHardRt: isHardRt, inSoftRt: isSoftRt, inAsync: isAsync, returnType: returnType))
        {
            if (context.generic_params() != null)
            {
                foreach (var gp in context.generic_params().generic_param())
                {
                    string gpName = gp.identifier().GetText();
                    funcSymbol.GenericParameters.Add(gpName);
                    var gpSym = CreateSymbol(gp.identifier(), gp, gpName, PilotSymbolKind.TypeParameter, gp.type_spec()?.GetText() ?? "any", "Core / C#", gp.GetText(), "Generic type parameter.");
                    RegisterSymbol(gpSym, PilotSemanticTokenLegend.TypeTypeParameter, PilotSemanticTokenLegend.ModDeclaration);
                }
            }

            if (context.parameters()?.parameter() != null)
            {
                foreach (var param in context.parameters().parameter())
                {
                    var pId = param.identifier();
                    var pSplice = param.splice_expr();
                    string pName = pId?.GetText() ?? pSplice?.GetText() ?? "_";
                    var tokenForSym = pId?.Start ?? pSplice?.Start;

                    if (param.CONTEXT() != null)
                        AddSemanticToken(param.CONTEXT().Symbol, PilotSemanticTokenLegend.TypeKeyword, 0);
                    if (param.EACH() != null)
                        AddSemanticToken(param.EACH().Symbol, PilotSemanticTokenLegend.TypeKeyword, 0);
                    if (param.REPEAT() != null)
                        AddSemanticToken(param.REPEAT().Symbol, PilotSemanticTokenLegend.TypeKeyword, 0);

                    string pType = param.type_annotation()?.type_spec()?.GetText()
                        ?? (pName == "self" && _currentScope.EnclosingType != null ? _currentScope.EnclosingType.Name : "any");

                    if (param.type_annotation()?.type_spec() != null)
                        Visit(param.type_annotation().type_spec());

                    funcSymbol.Parameters.Add((pName, pType));
                    if (tokenForSym != null)
                    {
                        var pSym = CreateSymbolFromToken(tokenForSym, param, pName, PilotSymbolKind.Parameter, pType, domain, $"{pName}: {pType}", $"Parameter of `{name}`.", isMutable: false);
                        RegisterSymbol(pSym, PilotSemanticTokenLegend.TypeParameter, PilotSemanticTokenLegend.ModDeclaration);
                    }

                    if (param.test() != null) Visit(param.test());
                }
            }

            string paramListStr = string.Join(", ", funcSymbol.Parameters.Select(p => $"{p.Name}: {p.Type}"));
            string prefix = isPure ? "pure " : isAsync ? "async " : "";
            funcSymbol.Signature = $"{prefix}def {name}({paramListStr}) -> {returnType}";

            if (context.where_type_clause() != null) Visit(context.where_type_clause());
            if (context.contract_clause() != null)
            {
                foreach (var cc in context.contract_clause())
                {
                    Visit(cc);
                }
            }
            if (context.suite() != null) Visit(context.suite());
        }

        return returnType;
    }

    public override string? VisitContract_clause(PilotParser.Contract_clauseContext context)
    {
        if (context.REQUIRES() != null)
            AddSemanticToken(context.REQUIRES().Symbol, PilotSemanticTokenLegend.TypeKeyword, 0);
        else if (context.ENSURES() != null)
            AddSemanticToken(context.ENSURES().Symbol, PilotSemanticTokenLegend.TypeKeyword, 0);

        if (context.test() != null)
            Visit(context.test());

        return null;
    }

    public override string? VisitPattern_assign_stmt(PilotParser.Pattern_assign_stmtContext context)
    {
        if (context.tuple_pattern() != null) Visit(context.tuple_pattern());
        if (context.list_pattern() != null) Visit(context.list_pattern());
        if (context.record_pattern() != null) Visit(context.record_pattern());
        if (context.test() != null) Visit(context.test());
        return null;
    }

    public override string? VisitClass_def(PilotParser.Class_defContext context)
    {
        var idCtx = context.identifier();
        string name = idCtx.GetText();
        bool isStatic = context.declaration_modifier().Any(m => m.STATIC() != null);
        var symbol = CreateSymbol(idCtx, context, name, PilotSymbolKind.Class, name, "Core / C#", $"class {name}", "Reference-type class definition.", isStatic: isStatic);
        int mods = PilotSemanticTokenLegend.ModDeclaration | PilotSemanticTokenLegend.ModDefinition;
        if (isStatic) mods |= PilotSemanticTokenLegend.ModStatic;
        RegisterSymbol(symbol, PilotSemanticTokenLegend.TypeClass, mods);

        using (EnterScope($"class:{name}", context, enclosingType: symbol))
        {
            RegisterGenericParams(context.generic_params(), symbol);
            if (context.class_bases() != null) Visit(context.class_bases());
            if (context.where_type_clause() != null) Visit(context.where_type_clause());
            if (context.suite() != null) Visit(context.suite());
        }

        return name;
    }

    public override string? VisitStruct_def(PilotParser.Struct_defContext context)
    {
        var idCtx = context.identifier();
        string name = idCtx.GetText();
        var symbol = CreateSymbol(idCtx, context, name, PilotSymbolKind.Struct, name, "Core / Real-Time", $"struct {name}", "Unboxed value-type struct suitable for stack allocation and deterministic real-time execution.");
        RegisterSymbol(symbol, PilotSemanticTokenLegend.TypeStruct, PilotSemanticTokenLegend.ModDeclaration | PilotSemanticTokenLegend.ModDefinition);

        using (EnterScope($"struct:{name}", context, enclosingType: symbol))
        {
            RegisterGenericParams(context.generic_params(), symbol);
            foreach (var member in context.struct_member())
            {
                var mId = member.identifier();
                string mName = mId.GetText();
                string mType = member.type_annotation().type_spec().GetText();
                Visit(member.type_annotation().type_spec());
                bool isMut = member.VAR() != null;
                var mSym = CreateSymbol(mId, member, mName, PilotSymbolKind.Property, mType, "Core", $"{(isMut ? "var" : "let")} {mName}: {mType}", $"Field of struct `{name}`.", isMutable: isMut);
                RegisterSymbol(mSym, PilotSemanticTokenLegend.TypeProperty, PilotSemanticTokenLegend.ModDeclaration);
                symbol.Members[mName] = mSym;
                symbol.Children.Add(mSym);
                if (member.test() != null) Visit(member.test());
            }
            if (context.suite() != null) Visit(context.suite());
        }

        return name;
    }

    public override string? VisitInterface_def(PilotParser.Interface_defContext context)
    {
        var idCtx = context.identifier();
        string name = idCtx.GetText();
        var symbol = CreateSymbol(idCtx, context, name, PilotSymbolKind.Interface, name, "Core / C#", $"interface {name}", "Interface contract definition.");
        RegisterSymbol(symbol, PilotSemanticTokenLegend.TypeInterface, PilotSemanticTokenLegend.ModDeclaration | PilotSemanticTokenLegend.ModDefinition);

        using (EnterScope($"interface:{name}", context, enclosingType: symbol))
        {
            RegisterGenericParams(context.generic_params(), symbol);
            if (context.suite() != null) Visit(context.suite());
        }

        return name;
    }

    public override string? VisitEnum_def(PilotParser.Enum_defContext context)
    {
        var idCtx = context.identifier();
        string name = idCtx.GetText();
        var symbol = CreateSymbol(idCtx, context, name, PilotSymbolKind.Enum, name, "Core", $"enum {name}", "Algebraic data type / enum definition.");
        RegisterSymbol(symbol, PilotSemanticTokenLegend.TypeEnum, PilotSemanticTokenLegend.ModDeclaration | PilotSemanticTokenLegend.ModDefinition);

        using (EnterScope($"enum:{name}", context, enclosingType: symbol))
        {
            RegisterGenericParams(context.generic_params(), symbol);
            if (context.enum_suite() != null)
            {
                foreach (var member in context.enum_suite().enum_member())
                {
                    var mId = member.identifier();
                    string mName = mId.GetText();
                    var mSym = CreateSymbol(mId, member, mName, PilotSymbolKind.EnumMember, name, "Core", $"{name}.{mName}", $"Variant of enum `{name}`.");
                    RegisterSymbol(mSym, PilotSemanticTokenLegend.TypeEnumMember, PilotSemanticTokenLegend.ModDeclaration | PilotSemanticTokenLegend.ModReadonly);
                    symbol.Members[mName] = mSym;
                    symbol.Children.Add(mSym);
                }
            }
        }

        return name;
    }

    public override string? VisitActor_def(PilotParser.Actor_defContext context)
    {
        var idCtx = context.identifier();
        string name = idCtx.GetText();
        var symbol = CreateSymbol(idCtx, context, name, PilotSymbolKind.Actor, name, "Concurrency", $"actor {name}", "Isolated concurrent actor with message-driven state.");
        RegisterSymbol(symbol, PilotSemanticTokenLegend.TypeClass, PilotSemanticTokenLegend.ModDeclaration | PilotSemanticTokenLegend.ModDefinition | PilotSemanticTokenLegend.ModAsync);

        using (EnterScope($"actor:{name}", context, inAsync: true, enclosingType: symbol))
        {
            RegisterGenericParams(context.generic_params(), symbol);
            if (context.suite() != null) Visit(context.suite());
        }

        return name;
    }

    public override string? VisitModel_def(PilotParser.Model_defContext context)
    {
        var idCtx = context.identifier();
        string name = idCtx.GetText();
        string kw = context.MODEL() != null ? "model" : context.TABLE() != null ? "table" : "schema";
        var symbol = CreateSymbol(idCtx, context, name, PilotSymbolKind.Model, name, "Database", $"{kw} {name}", "Relational database model / schema definition.");
        RegisterSymbol(symbol, PilotSemanticTokenLegend.TypeClass, PilotSemanticTokenLegend.ModDeclaration | PilotSemanticTokenLegend.ModDefinition);

        using (EnterScope($"model:{name}", context, enclosingType: symbol))
        {
            RegisterGenericParams(context.generic_params(), symbol);
            if (context.suite() != null) Visit(context.suite());
        }

        return name;
    }

    public override string? VisitProperty_def(PilotParser.Property_defContext context)
    {
        var idCtx = context.identifier();
        string name = idCtx.GetText();
        string propType = context.type_annotation()?.type_spec()?.GetText() ?? "any";
        if (context.type_annotation()?.type_spec() != null)
            Visit(context.type_annotation().type_spec());

        var symbol = CreateSymbol(idCtx, context, name, PilotSymbolKind.Property, propType, "C# Robustness", $"property {name}: {propType}", "C#-style property with get/set accessors.", isMutable: true);
        RegisterSymbol(symbol, PilotSemanticTokenLegend.TypeProperty, PilotSemanticTokenLegend.ModDeclaration | PilotSemanticTokenLegend.ModDefinition);

        if (_currentScope.EnclosingType != null)
        {
            _currentScope.EnclosingType.Members[name] = symbol;
            _currentScope.EnclosingType.Children.Add(symbol);
        }

        using (EnterScope($"property:{name}", context, returnType: propType))
        {
            if (_currentScope.EnclosingType != null)
            {
                _currentScope.Symbols["self"] = new PilotSymbol
                {
                    Name = "self",
                    Kind = PilotSymbolKind.Parameter,
                    TypeName = _currentScope.EnclosingType.Name,
                    Domain = "C# Robustness"
                };
            }

            if (context.property_body() != null)
            {
                foreach (var acc in context.property_body().property_accessor())
                {
                    if (acc.get_accessor()?.suite() != null)
                        Visit(acc.get_accessor().suite());

                    if (acc.set_accessor() != null)
                    {
                        using (EnterScope($"set:{name}", acc.set_accessor()))
                        {
                            var setParamId = acc.set_accessor().identifier();
                            string valParamName = setParamId?.GetText() ?? "value";
                            if (setParamId != null)
                            {
                                var valSym = CreateSymbol(setParamId, acc.set_accessor(), valParamName, PilotSymbolKind.Parameter, propType, "C# Robustness", $"{valParamName}: {propType}", "Property setter value parameter.");
                                RegisterSymbol(valSym, PilotSemanticTokenLegend.TypeParameter, PilotSemanticTokenLegend.ModDeclaration);
                            }
                            else
                            {
                                _currentScope.Symbols["value"] = new PilotSymbol
                                {
                                    Name = "value",
                                    Kind = PilotSymbolKind.Parameter,
                                    TypeName = propType,
                                    Domain = "C# Robustness"
                                };
                            }

                            if (acc.set_accessor().suite() != null)
                                Visit(acc.set_accessor().suite());
                        }
                    }
                }
            }
        }

        return propType;
    }

    public override string? VisitOperator_def(PilotParser.Operator_defContext context)
    {
        string opText = context.overloadable_op().GetText();
        string retType = context.type_spec()?.GetText() ?? "any";
        if (context.type_spec() != null) Visit(context.type_spec());

        var symbol = CreateSymbolFromToken(context.OPERATOR().Symbol, context, $"operator {opText}", PilotSymbolKind.Operator, retType, "C# Robustness",
            $"operator {opText}(...) -> {retType}", $"Custom overloaded operator `{opText}`.");
        if (_currentScope.EnclosingType != null)
            _currentScope.EnclosingType.Children.Add(symbol);

        using (EnterScope($"operator:{opText}", context, returnType: retType))
        {
            if (context.parameters()?.parameter() != null)
            {
                foreach (var param in context.parameters().parameter())
                {
                    var pId = param.identifier();
                    string pName = pId.GetText();
                    if (param.CONTEXT() != null)
                        AddSemanticToken(param.CONTEXT().Symbol, PilotSemanticTokenLegend.TypeKeyword, 0);
                    if (param.EACH() != null)
                        AddSemanticToken(param.EACH().Symbol, PilotSemanticTokenLegend.TypeKeyword, 0);
                    if (param.REPEAT() != null)
                        AddSemanticToken(param.REPEAT().Symbol, PilotSemanticTokenLegend.TypeKeyword, 0);

                    string pType = param.type_annotation()?.type_spec()?.GetText() ?? (_currentScope.EnclosingType?.Name ?? "any");
                    var pSym = CreateSymbol(pId, param, pName, PilotSymbolKind.Parameter, pType, "C# Robustness", $"{pName}: {pType}", "Operator parameter.");
                    RegisterSymbol(pSym, PilotSemanticTokenLegend.TypeParameter, PilotSemanticTokenLegend.ModDeclaration);
                }
            }
            if (context.suite() != null) Visit(context.suite());
        }

        return retType;
    }

    public override string? VisitExtension_def(PilotParser.Extension_defContext context)
    {
        string targetType = context.type_spec().GetText();
        string extName = context.identifier()?.GetText() ?? $"Extension<{targetType}>";
        var token = context.identifier()?.Start ?? context.EXTENSION().Symbol;

        var symbol = CreateSymbolFromToken(token, context, extName, PilotSymbolKind.Extension, targetType, "C# Robustness",
            $"extension {extName} for {targetType}", $"C#-style extension methods for `{targetType}`.");
        RegisterSymbol(symbol, PilotSemanticTokenLegend.TypeClass, PilotSemanticTokenLegend.ModDeclaration);

        using (EnterScope($"extension:{extName}", context, enclosingType: symbol))
        {
            if (context.where_type_clause() != null) Visit(context.where_type_clause());
            if (context.suite() != null) Visit(context.suite());
        }

        return targetType;
    }

    public override string? VisitAdapter_def(PilotParser.Adapter_defContext context)
    {
        string fromType = "any";
        string toType = "any";
        if (context.type_spec().Length >= 2)
        {
            fromType = context.type_spec(0).GetText();
            toType = context.type_spec(1).GetText();
        }
        else if (context.identifier() != null)
        {
            fromType = context.identifier().GetText();
        }

        string adapterName = $"adapter_{fromType}_to_{toType}";
        var token = context.identifier()?.Start ?? context.ADAPTER().Symbol;
        AddSemanticToken(context.ADAPTER().Symbol, PilotSemanticTokenLegend.TypeKeyword, 0);

        var symbol = CreateSymbolFromToken(token, context, adapterName, PilotSymbolKind.Function, toType, "Adapters",
            $"adapter {fromType} -> {toType}", $"Type adapter converting `{fromType}` to `{toType}`.");
        RegisterSymbol(symbol, PilotSemanticTokenLegend.TypeFunction, PilotSemanticTokenLegend.ModDeclaration);

        using (EnterScope($"adapter:{adapterName}", context, enclosingType: symbol))
        {
            if (context.suite() != null) Visit(context.suite());
        }

        return toType;
    }

    public override string? VisitUnit_stmt(PilotParser.Unit_stmtContext context)
    {
        var idCtx = context.identifier();
        string name = idCtx.GetText();
        string expr = context.unit_expr()?.GetText() ?? "base_unit";
        if (context.unit_expr() != null)
        {
            _unitAliases[name] = ExpandUnitExpression(expr.Replace(" ", ""));
        }
        var symbol = CreateSymbol(idCtx, context, name, PilotSymbolKind.Unit, $"unit<{name}>", "UDSA",
            context.unit_expr() != null ? $"unit {name} = {expr}" : $"unit {name}",
            "First-class dimensional unit of measure.");
        RegisterSymbol(symbol, PilotSemanticTokenLegend.TypeTypeParameter, PilotSemanticTokenLegend.ModDeclaration | PilotSemanticTokenLegend.ModReadonly);
        return symbol.TypeName;
    }

    public override string? VisitDsl_stmt(PilotParser.Dsl_stmtContext context)
    {
        var idCtx = context.identifier();
        string name = idCtx.GetText();
        var symbol = CreateSymbol(idCtx, context, name, PilotSymbolKind.Dsl, "DslBlock", "UDSA",
            $"dsl {name}", "Universal Domain-Specific Abstraction (UDSA) block.");
        RegisterSymbol(symbol, PilotSemanticTokenLegend.TypeNamespace, PilotSemanticTokenLegend.ModDeclaration);

        using (EnterScope($"dsl:{name}", context))
        {
            if (context.suite() != null) Visit(context.suite());
        }

        return "DslBlock";
    }

    public override string? VisitMacro_def(PilotParser.Macro_defContext context)
    {
        var kw = context.MACRO()?.Symbol ?? context.DEFMACRO()?.Symbol;
        if (kw != null)
            AddSemanticToken(kw, PilotSemanticTokenLegend.TypeKeyword, 0);

        var idCtx = context.identifier();
        string name = idCtx.GetText();
        string macroKind = context.DEFMACRO() != null ? "defmacro" : "macro";
        var symbol = CreateSymbol(idCtx, context, name, PilotSymbolKind.Macro, "AstNode", "Metaprogramming / Lisp",
            $"{macroKind} {name}{context.parameters()?.GetText() ?? "()"}", "Compile-time AST macro (Lisp homoiconicity).");
        RegisterSymbol(symbol, PilotSemanticTokenLegend.TypeMacro, PilotSemanticTokenLegend.ModDeclaration | PilotSemanticTokenLegend.ModDefinition);

        using (EnterScope($"macro:{name}", context))
        {
            if (context.parameters()?.parameter() != null)
            {
                foreach (var param in context.parameters().parameter())
                {
                    var pId = param.identifier();
                    if (pId == null) continue;
                    var pSym = CreateSymbol(pId, param, pId.GetText(), PilotSymbolKind.Parameter, "AstNode", "Metaprogramming", pId.GetText(), "Macro AST parameter.");
                    RegisterSymbol(pSym, PilotSemanticTokenLegend.TypeParameter, PilotSemanticTokenLegend.ModDeclaration);
                }
            }
            if (context.suite() != null) Visit(context.suite());
        }

        return "AstNode";
    }

    public override string? VisitQuote_stmt(PilotParser.Quote_stmtContext context)
    {
        var kw = context.QUOTE()?.Symbol ?? context.QUASIQUOTE()?.Symbol;
        if (kw != null)
            AddSemanticToken(kw, PilotSemanticTokenLegend.TypeKeyword, 0);
        if (context.suite() != null) Visit(context.suite());
        if (context.test() != null) Visit(context.test());
        return "AstNode";
    }

    public override string? VisitQuote_expr(PilotParser.Quote_exprContext context)
    {
        if (context.QUOTE() != null)
            AddSemanticToken(context.QUOTE().Symbol, PilotSemanticTokenLegend.TypeKeyword, 0);
        if (context.suite() != null) Visit(context.suite());
        if (context.test() != null) Visit(context.test());
        return "AstNode";
    }

    public override string? VisitQuasiquote_expr(PilotParser.Quasiquote_exprContext context)
    {
        var kw = context.QUASIQUOTE()?.Symbol ?? context.BACKQUOTE()?.Symbol;
        if (kw != null)
            AddSemanticToken(kw, PilotSemanticTokenLegend.TypeKeyword, 0);
        if (context.suite() != null) Visit(context.suite());
        if (context.test() != null) Visit(context.test());
        if (context.identifier() != null) Visit(context.identifier());
        if (context.literal() != null) Visit(context.literal());
        return "AstNode";
    }

    public override string? VisitSplice_expr(PilotParser.Splice_exprContext context)
    {
        var kw = context.DOLLAR()?.Symbol ?? context.SPLICE()?.Symbol ?? context.UNQUOTE()?.Symbol ?? context.UNQUOTE_SPLICING()?.Symbol;
        if (kw != null)
            AddSemanticToken(kw, PilotSemanticTokenLegend.TypeKeyword, 0);
        if (context.AT() != null)
            AddSemanticToken(context.AT().Symbol, PilotSemanticTokenLegend.TypeOperator, 0);
        if (context.identifier() != null) Visit(context.identifier());
        if (context.test() != null) Visit(context.test());
        return "AstNode";
    }

    public override string? VisitForeign_import_stmt(PilotParser.Foreign_import_stmtContext context)
    {
        if (context.IMPORT() != null)
            AddSemanticToken(context.IMPORT().Symbol, PilotSemanticTokenLegend.TypeKeyword, 0);
        if (context.FROM() != null)
            AddSemanticToken(context.FROM().Symbol, PilotSemanticTokenLegend.TypeKeyword, 0);
        if (context.AS() != null)
            AddSemanticToken(context.AS().Symbol, PilotSemanticTokenLegend.TypeKeyword, 0);

        var ft = context.foreign_target();
        string lang = (ft.foreign_language()?.GetText() ?? ft.identifier()?.GetText() ?? "foreign").ToLowerInvariant();

        var langTok = ft.foreign_language()?.Start ?? ft.identifier()?.Start;
        if (langTok != null)
            AddSemanticToken(langTok, PilotSemanticTokenLegend.TypeKeyword, 0);

        string targetModule = ft.dotted_name() != null
            ? ft.dotted_name().GetText()
            : (ft.STRING() != null ? ft.STRING().GetText().Trim('"', '\'') : string.Empty);

        string domain = lang switch
        {
            "python" => "Python Interop",
            "csharp" => "C# / .NET Interop",
            "rust" => "Rust FFI",
            "c" => "C FFI",
            _ => $"{char.ToUpperInvariant(lang[0])}{lang[1..]} Interop"
        };

        if (context.FROM() != null)
        {
            var names = context.import_as_names()?.import_as_name();
            if (names != null)
            {
                foreach (var nameCtx in names)
                {
                    var origId = nameCtx.identifier(0);
                    string origName = origId.GetText();
                    var asId = nameCtx.identifier().Length > 1 ? nameCtx.identifier(1) : null;
                    if (nameCtx.AS() != null)
                        AddSemanticToken(nameCtx.AS().Symbol, PilotSemanticTokenLegend.TypeKeyword, 0);

                    var bindId = asId ?? origId;
                    string bindName = bindId.GetText();

                    var kind = lang == "csharp"
                        ? (char.IsUpper(origName[0]) ? PilotSymbolKind.Class : PilotSymbolKind.Function)
                        : lang == "rust" || lang == "c"
                        ? (char.IsUpper(origName[0]) ? PilotSymbolKind.Struct : PilotSymbolKind.Function)
                        : (char.IsUpper(origName[0]) ? PilotSymbolKind.Class : PilotSymbolKind.Function);

                    string typeName = lang == "csharp" ? $"{targetModule}.{origName}" : "any";
                    string sig = $"from {lang}:{targetModule} import {origName}{(asId != null ? $" as {bindName}" : "")}";
                    string doc = $"Imported symbol `{origName}` from {domain} ({lang}:{targetModule}).";

                    var sym = CreateSymbol(bindId, context, bindName, kind, typeName, domain, sig, doc);
                    int tokenType = kind switch
                    {
                        PilotSymbolKind.Class => PilotSemanticTokenLegend.TypeClass,
                        PilotSymbolKind.Struct => PilotSemanticTokenLegend.TypeStruct,
                        _ => PilotSemanticTokenLegend.TypeFunction
                    };
                    RegisterSymbol(sym, tokenType, PilotSemanticTokenLegend.ModDeclaration | PilotSemanticTokenLegend.ModDefinition);
                }
            }
        }
        else
        {
            var asId = context.identifier();
            string bindName;
            PilotParser.IdentifierContext? bindId;

            if (asId != null)
            {
                bindName = asId.GetText();
                bindId = asId;
            }
            else
            {
                bindId = ft.dotted_name()?.identifier().Last();
                bindName = bindId?.GetText() ?? targetModule.Replace("lib", "").Replace(".dll", "").Replace(".so", "");
            }

            if (bindId != null)
            {
                string sig = $"import {lang}:{targetModule}{(asId != null ? $" as {bindName}" : "")}";
                string doc = $"Imported foreign module `{targetModule}` ({domain}).";
                var sym = CreateSymbol(bindId, context, bindName, PilotSymbolKind.Module, $"{lang}:{targetModule}", domain, sig, doc);
                RegisterSymbol(sym, PilotSemanticTokenLegend.TypeNamespace, PilotSemanticTokenLegend.ModDeclaration | PilotSemanticTokenLegend.ModDefinition);
            }
        }

        return null;
    }

    public override string? VisitExtern_def(PilotParser.Extern_defContext context)
    {
        AddSemanticToken(context.EXTERN().Symbol, PilotSemanticTokenLegend.TypeKeyword, 0);

        string lang = "foreign";
        if (context.STRING().Length > 0 && (context.identifier().Length == 0 || context.STRING(0).Symbol.TokenIndex < context.identifier(0).Start.TokenIndex))
        {
            lang = context.STRING(0).GetText().Trim('"', '\'');
        }
        else if (context.identifier().Length > 0)
        {
            lang = context.identifier(0).GetText();
            AddSemanticToken(context.identifier(0).Start, PilotSemanticTokenLegend.TypeKeyword, 0);
        }

        string domain = lang.ToLowerInvariant() switch
        {
            "python" => "Python Interop",
            "csharp" => "C# / .NET Interop",
            "rust" => "Rust FFI",
            "c" => "C FFI",
            _ => $"{char.ToUpperInvariant(lang[0])}{lang[1..]} Interop"
        };

        string? libKind = null;
        string? libName = null;
        IToken? specToken = null;

        if (context.ASSEMBLY() != null)
        {
            libKind = "assembly";
            specToken = context.ASSEMBLY().Symbol;
            AddSemanticToken(specToken, PilotSemanticTokenLegend.TypeKeyword, 0);
        }
        else if (context.MODULE() != null)
        {
            libKind = "module";
            specToken = context.MODULE().Symbol;
            AddSemanticToken(specToken, PilotSemanticTokenLegend.TypeKeyword, 0);
        }
        else if (context.LIB() != null)
        {
            libKind = "lib";
            specToken = context.LIB().Symbol;
            AddSemanticToken(specToken, PilotSemanticTokenLegend.TypeKeyword, 0);
        }

        if (specToken != null)
        {
            var libStr = context.STRING().FirstOrDefault(s => s.Symbol.TokenIndex > specToken.TokenIndex);
            var libId = context.identifier().FirstOrDefault(i => i.Start.TokenIndex > specToken.TokenIndex);
            if (libStr != null) libName = libStr.GetText().Trim('"', '\'');
            else if (libId != null) libName = libId.GetText();
        }

        foreach (var member in context.extern_member())
        {
            ProcessExternMember(member, lang, domain, libKind, libName);
        }

        return null;
    }

    private void ProcessExternMember(PilotParser.Extern_memberContext member, string lang, string domain, string? libKind, string? libName)
    {
        if (member.DEF() != null || member.FN() != null)
        {
            ProcessExternFunction(member, lang, domain, libKind, libName);
        }
        else if (member.CLASS() != null || member.STRUCT() != null || member.INTERFACE() != null)
        {
            ProcessExternType(member, lang, domain, libKind, libName);
        }
        else if (member.small_stmt() != null)
        {
            Visit(member.small_stmt());
        }
    }

    private void ProcessExternFunction(PilotParser.Extern_memberContext member, string lang, string domain, string? libKind, string? libName)
    {
        var idCtx = member.identifier();
        if (idCtx == null) return;
        string name = idCtx.GetText();

        var kw = member.DEF()?.Symbol ?? member.FN()?.Symbol;
        if (kw != null) AddSemanticToken(kw, PilotSemanticTokenLegend.TypeKeyword, 0);

        bool isStatic = member.declaration_modifier().Any(m => m.STATIC() != null);
        foreach (var mod in member.declaration_modifier())
        {
            AddSemanticToken(mod.Start, PilotSemanticTokenLegend.TypeModifier, 0);
        }

        string returnType = member.type_spec()?.GetText() ?? "void";
        if (member.type_spec() != null) Visit(member.type_spec());

        var kind = _currentScope.EnclosingType != null ? PilotSymbolKind.Method : PilotSymbolKind.Function;
        string doc = libName != null
            ? $"External {lang} function declared in {libKind} `{libName}`."
            : $"External {lang} function.";

        var funcSymbol = CreateSymbol(idCtx, member, name, kind, returnType, domain, "", doc, isStatic: isStatic);

        int mods = PilotSemanticTokenLegend.ModDeclaration | PilotSemanticTokenLegend.ModDefinition;
        if (isStatic) mods |= PilotSemanticTokenLegend.ModStatic;

        RegisterSymbol(
            funcSymbol,
            kind == PilotSymbolKind.Method ? PilotSemanticTokenLegend.TypeMethod : PilotSemanticTokenLegend.TypeFunction,
            mods);

        if (_currentScope.EnclosingType != null)
        {
            _currentScope.EnclosingType.Members[name] = funcSymbol;
            _currentScope.EnclosingType.Children.Add(funcSymbol);
        }

        using (EnterScope($"extern_fn:{name}", member, returnType: returnType))
        {
            if (member.parameters()?.parameter() != null)
            {
                foreach (var param in member.parameters().parameter())
                {
                    var pId = param.identifier();
                    if (pId == null) continue;
                    string pName = pId.GetText();
                    if (param.CONTEXT() != null)
                        AddSemanticToken(param.CONTEXT().Symbol, PilotSemanticTokenLegend.TypeKeyword, 0);
                    if (param.EACH() != null)
                        AddSemanticToken(param.EACH().Symbol, PilotSemanticTokenLegend.TypeKeyword, 0);
                    if (param.REPEAT() != null)
                        AddSemanticToken(param.REPEAT().Symbol, PilotSemanticTokenLegend.TypeKeyword, 0);

                    string pType = param.type_annotation()?.type_spec()?.GetText()
                        ?? (pName == "self" && _currentScope.EnclosingType != null ? _currentScope.EnclosingType.Name : "any");

                    if (param.type_annotation()?.type_spec() != null)
                        Visit(param.type_annotation().type_spec());

                    funcSymbol.Parameters.Add((pName, pType));
                    var pSym = CreateSymbol(pId, param, pName, PilotSymbolKind.Parameter, pType, domain, $"{pName}: {pType}", $"Parameter of `{name}`.", isMutable: false);
                    RegisterSymbol(pSym, PilotSemanticTokenLegend.TypeParameter, PilotSemanticTokenLegend.ModDeclaration);

                    if (param.test() != null) Visit(param.test());
                }
            }

            string paramListStr = string.Join(", ", funcSymbol.Parameters.Select(p => $"{p.Name}: {p.Type}"));
            funcSymbol.Signature = $"extern \"{lang}\" fn {name}({paramListStr}) -> {returnType}";

            if (member.suite() != null) Visit(member.suite());
        }
    }

    private void ProcessExternType(PilotParser.Extern_memberContext member, string lang, string domain, string? libKind, string? libName)
    {
        var idCtx = member.identifier();
        if (idCtx == null) return;
        string name = idCtx.GetText();

        var kw = member.CLASS()?.Symbol ?? member.STRUCT()?.Symbol ?? member.INTERFACE()?.Symbol;
        if (kw != null) AddSemanticToken(kw, PilotSemanticTokenLegend.TypeKeyword, 0);

        foreach (var mod in member.declaration_modifier())
        {
            AddSemanticToken(mod.Start, PilotSemanticTokenLegend.TypeModifier, 0);
        }

        var kind = member.CLASS() != null ? PilotSymbolKind.Class
            : member.STRUCT() != null ? PilotSymbolKind.Struct
            : PilotSymbolKind.Interface;

        int tokenType = kind switch
        {
            PilotSymbolKind.Class => PilotSemanticTokenLegend.TypeClass,
            PilotSymbolKind.Struct => PilotSemanticTokenLegend.TypeStruct,
            _ => PilotSemanticTokenLegend.TypeInterface
        };

        string typeKeyword = kind switch
        {
            PilotSymbolKind.Class => "class",
            PilotSymbolKind.Struct => "struct",
            _ => "interface"
        };

        string doc = libName != null
            ? $"External {lang} {typeKeyword} declared in {libKind} `{libName}`."
            : $"External {lang} {typeKeyword}.";

        string sig = $"extern \"{lang}\" {typeKeyword} {name}";

        var typeSymbol = CreateSymbol(idCtx, member, name, kind, name, domain, sig, doc);
        RegisterSymbol(typeSymbol, tokenType, PilotSemanticTokenLegend.ModDeclaration | PilotSemanticTokenLegend.ModDefinition);

        using (EnterScope($"extern_type:{name}", member, enclosingType: typeSymbol))
        {
            foreach (var nested in member.extern_member())
            {
                ProcessExternMember(nested, lang, domain, libKind, libName);
            }
            if (member.suite() != null)
            {
                Visit(member.suite());
            }
        }
    }

    public override string? VisitInline_foreign_block(PilotParser.Inline_foreign_blockContext context)
    {
        if (context.foreign_language() != null)
        {
            AddSemanticToken(context.foreign_language().Start, PilotSemanticTokenLegend.TypeKeyword, 0);
        }

        if (context.stmt() != null)
        {
            foreach (var stmt in context.stmt())
            {
                Visit(stmt);
            }
        }

        return "any";
    }

    public override string? VisitDatabase_stmt(PilotParser.Database_stmtContext context)
    {
        var idCtx = context.identifier();
        string name = idCtx.GetText();
        if (context.test() != null) Visit(context.test());
        var symbol = CreateSymbol(idCtx, context, name, PilotSymbolKind.Database, "Database", "Database",
            $"database {name} = {context.test()?.GetText()}", "Generic database connection handle (Postgres / SQLite / In-Memory).");
        RegisterSymbol(symbol, PilotSemanticTokenLegend.TypeNamespace, PilotSemanticTokenLegend.ModDeclaration);
        return "Database";
    }

    public override string? VisitTransaction_stmt(PilotParser.Transaction_stmtContext context)
    {
        AddSemanticToken(context.TRANSACTION().Symbol, PilotSemanticTokenLegend.TypeKeyword, 0);
        if (context.test() != null) Visit(context.test());

        using (EnterScope("transaction", context, inTransaction: true))
        {
            if (context.suite() != null) Visit(context.suite());
        }

        return "void";
    }

    public override string? VisitSavepoint_stmt(PilotParser.Savepoint_stmtContext context)
    {
        var idCtx = context.identifier();
        string name = idCtx.GetText();
        _activeSavepoints.Add(name);
        var symbol = CreateSymbol(idCtx, context, name, PilotSymbolKind.Savepoint, "Savepoint", "Database",
            $"savepoint {name}", "Database transaction savepoint.");
        RegisterSymbol(symbol, PilotSemanticTokenLegend.TypeEvent, PilotSemanticTokenLegend.ModDeclaration);
        return "Savepoint";
    }

    public override string? VisitSavepoint_block(PilotParser.Savepoint_blockContext context)
    {
        var idCtx = context.identifier();
        string name = idCtx.GetText();
        _activeSavepoints.Add(name);
        var symbol = CreateSymbol(idCtx, context, name, PilotSymbolKind.Savepoint, "Savepoint", "Database",
            $"savepoint {name}", "Database transaction savepoint.");
        RegisterSymbol(symbol, PilotSemanticTokenLegend.TypeEvent, PilotSemanticTokenLegend.ModDeclaration);
        if (context.suite() != null) Visit(context.suite());
        return "Savepoint";
    }

    public override string? VisitRollback_stmt(PilotParser.Rollback_stmtContext context)
    {
        if (context.identifier() != null)
        {
            string spName = context.identifier().GetText();
            var resolved = _currentScope.Resolve(spName);
            if (resolved != null)
            {
                AddSymbolReference(context.identifier().Start, resolved);
            }
            else if (!_activeSavepoints.Contains(spName))
            {
                AddDiagnostic(
                    context.identifier().Start,
                    PilotDiagnosticSeverity.Error,
                    "PILOT107",
                    $"Cannot rollback to undeclared savepoint '{spName}'. Declare 'savepoint {spName}' before rolling back to it.");
            }
        }
        return "void";
    }

    public override string? VisitNoalloc_stmt(PilotParser.Noalloc_stmtContext context)
    {
        AddSemanticToken(context.NOALLOC().Symbol, PilotSemanticTokenLegend.TypeKeyword, 0);
        using (EnterScope("noalloc", context, inNoAlloc: true))
        {
            if (context.suite() != null) Visit(context.suite());
        }
        return "void";
    }

    public override string? VisitWhile_stmt(PilotParser.While_stmtContext context)
    {
        if (context.WHILE() != null)
            AddSemanticToken(context.WHILE().Symbol, PilotSemanticTokenLegend.TypeKeyword, 0);
        if (context.BOUNDED() != null)
            AddSemanticToken(context.BOUNDED().Symbol, PilotSemanticTokenLegend.TypeKeyword, 0);
        if (context.ELSE() != null)
            AddSemanticToken(context.ELSE().Symbol, PilotSemanticTokenLegend.TypeKeyword, 0);

        Visit(context.test(0));
        bool isBounded = context.BOUNDED() != null;
        if (isBounded && context.test().Length > 1)
            Visit(context.test(1));

        if ((_currentScope.InHardRealtime || _currentScope.InNoAlloc) && !isBounded)
        {
            AddDiagnostic(
                context.WHILE().Symbol,
                PilotDiagnosticSeverity.Error,
                "PILOT104",
                "Unbounded 'while' loop is forbidden inside '@hard_realtime' or 'noalloc' contexts. Add 'bounded <limit>' to guarantee worst-case execution time (WCET).");
        }

        using (EnterScope("while", context, inLoop: true))
        {
            foreach (var s in context.suite()) Visit(s);
        }
        return "void";
    }

    public override string? VisitFor_stmt(PilotParser.For_stmtContext context)
    {
        if (context.FOR() != null)
            AddSemanticToken(context.FOR().Symbol, PilotSemanticTokenLegend.TypeKeyword, 0);
        if (context.IN() != null)
            AddSemanticToken(context.IN().Symbol, PilotSemanticTokenLegend.TypeKeyword, 0);
        if (context.ASYNC() != null)
            AddSemanticToken(context.ASYNC().Symbol, PilotSemanticTokenLegend.TypeKeyword, PilotSemanticTokenLegend.ModAsync);
        if (context.PARALLEL() != null)
            AddSemanticToken(context.PARALLEL().Symbol, PilotSemanticTokenLegend.TypeKeyword, 0);
        if (context.BOUNDED() != null)
            AddSemanticToken(context.BOUNDED().Symbol, PilotSemanticTokenLegend.TypeKeyword, 0);
        if (context.ELSE() != null)
            AddSemanticToken(context.ELSE().Symbol, PilotSemanticTokenLegend.TypeKeyword, 0);

        bool isBounded = context.BOUNDED() != null;
        bool isParallel = context.PARALLEL() != null;
        bool isAsync = context.ASYNC() != null;

        if ((_currentScope.InHardRealtime || _currentScope.InNoAlloc) && !isBounded)
        {
            AddDiagnostic(
                context.FOR().Symbol,
                PilotDiagnosticSeverity.Error,
                "PILOT104",
                "Unbounded 'for' loop is forbidden inside '@hard_realtime' or 'noalloc' contexts. Add 'bounded <limit>' to guarantee worst-case execution time (WCET).");
        }

        string iterableType = context.test(0) != null ? Visit(context.test(0)) ?? "any" : "any";
        string elemType = ExtractElementType(iterableType);

        if (isBounded && context.test().Length > 1)
            Visit(context.test(1));

        using (EnterScope("for", context, inLoop: true, inAsync: isAsync || _currentScope.InAsync))
        {
            if (context.for_target() != null)
            {
                var bindingPatterns = context.for_target().binding_pattern();
                if (bindingPatterns != null && bindingPatterns.Length > 0)
                {
                    foreach (var bp in bindingPatterns)
                    {
                        DeclareBindingPattern(bp, context, elemType, isMutable: false, isConstant: false, "Loop iteration variable", isParallel ? "Concurrency" : "Core");
                    }
                }
                else if (context.for_target().exprlist()?.test() != null)
                {
                    foreach (var targetTest in context.for_target().exprlist().test())
                    {
                        var idCtx = TryExtractSimpleIdentifier(targetTest);
                        if (idCtx != null)
                        {
                            var loopVar = CreateSymbol(idCtx, targetTest, idCtx.GetText(), PilotSymbolKind.Variable, elemType,
                                isParallel ? "Concurrency" : "Core",
                                $"{idCtx.GetText()}: {elemType}", "Loop iteration variable.", isMutable: false);
                            RegisterSymbol(loopVar, PilotSemanticTokenLegend.TypeVariable, PilotSemanticTokenLegend.ModDeclaration);
                        }
                    }
                }
            }
            foreach (var s in context.suite()) Visit(s);
        }
        return "void";
    }

    public override string? VisitIf_stmt(PilotParser.If_stmtContext context)
    {
        if (context.IF() != null)
        {
            foreach (var ifTok in context.IF())
                AddSemanticToken(ifTok.Symbol, PilotSemanticTokenLegend.TypeKeyword, 0);
        }
        if (context.ELIF() != null)
        {
            foreach (var elifTok in context.ELIF())
                AddSemanticToken(elifTok.Symbol, PilotSemanticTokenLegend.TypeKeyword, 0);
        }
        if (context.ELSE() != null)
        {
            foreach (var elseTok in context.ELSE())
                AddSemanticToken(elseTok.Symbol, PilotSemanticTokenLegend.TypeKeyword, 0);
        }
        return base.VisitIf_stmt(context);
    }

    public override string? VisitFlow_stmt(PilotParser.Flow_stmtContext context)
    {
        if (context.BREAK() != null)
            AddSemanticToken(context.BREAK().Symbol, PilotSemanticTokenLegend.TypeKeyword, 0);
        if (context.CONTINUE() != null)
            AddSemanticToken(context.CONTINUE().Symbol, PilotSemanticTokenLegend.TypeKeyword, 0);
        if (context.RETURN() != null)
            AddSemanticToken(context.RETURN().Symbol, PilotSemanticTokenLegend.TypeKeyword, 0);
        if (context.YIELD() != null)
            AddSemanticToken(context.YIELD().Symbol, PilotSemanticTokenLegend.TypeKeyword, 0);
        if (context.RAISE() != null)
            AddSemanticToken(context.RAISE().Symbol, PilotSemanticTokenLegend.TypeKeyword, 0);
        if (context.FROM() != null)
            AddSemanticToken(context.FROM().Symbol, PilotSemanticTokenLegend.TypeKeyword, 0);

        return base.VisitFlow_stmt(context);
    }

    public override string? VisitPass_stmt(PilotParser.Pass_stmtContext context)
    {
        if (context.PASS() != null)
            AddSemanticToken(context.PASS().Symbol, PilotSemanticTokenLegend.TypeKeyword, 0);
        return base.VisitPass_stmt(context);
    }

    public override string? VisitAssert_stmt(PilotParser.Assert_stmtContext context)
    {
        if (context.ASSERT() != null)
            AddSemanticToken(context.ASSERT().Symbol, PilotSemanticTokenLegend.TypeKeyword, 0);
        return base.VisitAssert_stmt(context);
    }

    public override string? VisitComparison(PilotParser.ComparisonContext context)
    {
        if (context.IS() != null)
        {
            foreach (var isTok in context.IS())
                AddSemanticToken(isTok.Symbol, PilotSemanticTokenLegend.TypeKeyword, 0);
        }
        if (context.NOT() != null)
        {
            foreach (var notTok in context.NOT())
                AddSemanticToken(notTok.Symbol, PilotSemanticTokenLegend.TypeKeyword, 0);
        }
        return base.VisitComparison(context);
    }

    public override string? VisitComp_op(PilotParser.Comp_opContext context)
    {
        if (context.IN() != null)
            AddSemanticToken(context.IN().Symbol, PilotSemanticTokenLegend.TypeKeyword, 0);
        if (context.NOT() != null)
            AddSemanticToken(context.NOT().Symbol, PilotSemanticTokenLegend.TypeKeyword, 0);
        return base.VisitComp_op(context);
    }
    public override string? VisitSelect_arm(PilotParser.Select_armContext context)
    {
        using (EnterScope("select_arm", context))
        {
            var target = context.select_case_target();
            if (target?.identifier() != null)
            {
                string chType = target.test(0) != null ? Visit(target.test(0)) ?? "any" : "any";
                string msgType = ExtractElementType(chType);
                var msgSym = CreateSymbol(target.identifier(), target, target.identifier().GetText(), PilotSymbolKind.Variable, msgType, "Concurrency",
                    $"{target.identifier().GetText()}: {msgType}", "Channel message received in CSP `select` arm.");
                RegisterSymbol(msgSym, PilotSemanticTokenLegend.TypeVariable, PilotSemanticTokenLegend.ModDeclaration);
            }
            else if (target != null)
            {
                foreach (var t in target.test()) Visit(t);
            }

            if (context.duration_term() != null) Visit(context.duration_term());
            if (context.suite() != null) Visit(context.suite());
        }
        return "void";
    }

    public override string? VisitQuery_expr(PilotParser.Query_exprContext context)
    {
        using (EnterScope("linq_query", context))
        {
            var fromClause = context.from_clause();
            if (fromClause != null)
            {
                string srcType = Visit(fromClause.test()) ?? "Table[any]";
                string rowType = ExtractElementType(srcType);
                var rangeId = fromClause.identifier(0);
                var rangeSym = CreateSymbol(rangeId, fromClause, rangeId.GetText(), PilotSymbolKind.QueryVariable, rowType, "Database",
                    $"from {rangeId.GetText()}: {rowType}", $"LINQ query range variable over `{fromClause.test().GetText()}`.");
                RegisterSymbol(rangeSym, PilotSemanticTokenLegend.TypeVariable, PilotSemanticTokenLegend.ModDeclaration | PilotSemanticTokenLegend.ModReadonly);
            }

            foreach (var join in context.join_clause())
            {
                string joinSrcType = join.test().Length > 1 ? Visit(join.test(0)) ?? "Table[any]" : "any";
                string joinRowType = ExtractElementType(joinSrcType);
                var joinId = join.identifier(0);
                var joinSym = CreateSymbol(joinId, join, joinId.GetText(), PilotSymbolKind.QueryVariable, joinRowType, "Database",
                    $"join {joinId.GetText()}: {joinRowType}", "Relational JOIN range variable.");
                RegisterSymbol(joinSym, PilotSemanticTokenLegend.TypeVariable, PilotSemanticTokenLegend.ModDeclaration | PilotSemanticTokenLegend.ModReadonly);

                foreach (var t in join.test()) Visit(t);
            }

            if (context.where_clause() != null) Visit(context.where_clause());
            if (context.group_clause() != null)
            {
                Visit(context.group_clause().test(0));
                Visit(context.group_clause().test(1));
                if (context.group_clause().identifier() is { } intoId)
                {
                    var groupSym = CreateSymbol(intoId, context.group_clause(), intoId.GetText(), PilotSymbolKind.QueryVariable, "Grouping", "Database",
                        $"group into {intoId.GetText()}", "Grouped aggregation partition binding.");
                    RegisterSymbol(groupSym, PilotSemanticTokenLegend.TypeVariable, PilotSemanticTokenLegend.ModDeclaration | PilotSemanticTokenLegend.ModReadonly);
                }
                if (context.group_clause().test().Length > 2)
                    Visit(context.group_clause().test(2));
            }
            if (context.order_clause() != null) Visit(context.order_clause());
            if (context.select_clause() != null) Visit(context.select_clause());
            if (context.limit_clause() != null) Visit(context.limit_clause());
        }

        return "Dataset[any]";
    }

    public override string? VisitSql_query_expr(PilotParser.Sql_query_exprContext context)
    {
        using (EnterScope("sql_query", context))
        {
            var fromSrc = context.from_source();
            if (fromSrc != null)
            {
                string srcType = Visit(fromSrc.test()) ?? "Table[any]";
                string rowType = ExtractElementType(srcType);
                if (fromSrc.identifier() is { } aliasId)
                {
                    var aliasSym = CreateSymbol(aliasId, fromSrc, aliasId.GetText(), PilotSymbolKind.QueryVariable, rowType, "Database",
                        $"from {fromSrc.test().GetText()} as {aliasId.GetText()}: {rowType}", "SQL table alias binding.");
                    RegisterSymbol(aliasSym, PilotSemanticTokenLegend.TypeVariable, PilotSemanticTokenLegend.ModDeclaration | PilotSemanticTokenLegend.ModReadonly);
                }
            }

            foreach (var join in context.join_clause()) Visit(join);
            if (context.select_projection() != null) Visit(context.select_projection());
            if (context.where_clause() != null) Visit(context.where_clause());
            if (context.group_clause() != null) Visit(context.group_clause());
            if (context.order_clause() != null) Visit(context.order_clause());
            if (context.limit_clause() != null) Visit(context.limit_clause());
        }

        return "Dataset[any]";
    }

    public override string? VisitUnary_op(PilotParser.Unary_opContext context)
    {
        if (context.AWAIT() != null)
        {
            AddSemanticToken(context.AWAIT().Symbol, PilotSemanticTokenLegend.TypeKeyword, PilotSemanticTokenLegend.ModAsync);
            if (!_currentScope.InAsync)
            {
                AddDiagnostic(
                    context.AWAIT().Symbol,
                    PilotDiagnosticSeverity.Error,
                    "PILOT106",
                    "The 'await' operator can only be used inside an 'async def' function or 'actor' handler.");
            }
            string? inner = context.factor() != null ? Visit(context.factor()) : "any";
            return ExtractElementType(inner ?? "any");
        }

        if (context.quote_expr() is { } qe) return VisitQuote_expr(qe);
        if (context.quasiquote_expr() is { } qqe) return VisitQuasiquote_expr(qqe);
        if (context.splice_expr() is { } se) return VisitSplice_expr(se);

        return base.VisitUnary_op(context);
    }

    public override string? VisitPrimary_type(PilotParser.Primary_typeContext context)
    {
        if (context.base_type()?.dotted_name() is { } dn)
        {
            var firstId = dn.identifier(0);
            string typeName = firstId.GetText();
            var resolved = _currentScope.Resolve(typeName);
            if (resolved != null)
            {
                AddSymbolReference(firstId.Start, resolved);
                AddSemanticToken(firstId.Start, GetSemanticTokenTypeForSymbol(resolved), 0);
            }
            else if (IsBuiltinType(typeName))
            {
                AddSemanticToken(firstId.Start, PilotSemanticTokenLegend.TypeType, PilotSemanticTokenLegend.ModDefaultLibrary);
            }
        }

        if (context.unit_expr() != null)
        {
            foreach (var term in context.unit_expr().unit_term())
            {
                if (term.identifier() is { } uId)
                {
                    var uSym = _currentScope.Resolve(uId.GetText());
                    if (uSym != null)
                        AddSymbolReference(uId.Start, uSym);
                    AddSemanticToken(uId.Start, PilotSemanticTokenLegend.TypeTypeParameter, 0);
                }
            }
        }

        return base.VisitPrimary_type(context);
    }

    public override string? VisitAtom_expr(PilotParser.Atom_exprContext context)
    {
        var atom = context.atom();
        string? currentType = Visit(atom);

        foreach (var trailer in context.trailer())
        {
            if (trailer.cascade_trailer() is { } cascade)
            {
                VisitCascade_trailer(cascade, currentType ?? "any");
                continue;
            }
            else if (trailer.identifier() is { } memberId)
            {
                string memberName = memberId.GetText();
                // Resolve member on currentType if known
                if (currentType != null)
                {
                    string baseTypeName = currentType.TrimEnd('?');
                    var typeSym = _currentScope.Resolve(baseTypeName);
                    if (typeSym != null && typeSym.Members.TryGetValue(memberName, out var memberSym))
                    {
                        AddSymbolReference(memberId.Start, memberSym);
                        AddSemanticToken(memberId.Start, GetSemanticTokenTypeForSymbol(memberSym), 0);
                        currentType = memberSym.TypeName;
                        continue;
                    }
                }

                // Check Root Object Methods
                if (RootObjectMethods.TryGetValue(memberName, out var rootMethod))
                {
                    if (RootMethodSymbols.TryGetValue(memberName, out var rootMethodSym))
                    {
                        AddSymbolReference(memberId.Start, rootMethodSym);
                    }
                    AddSemanticToken(memberId.Start, PilotSemanticTokenLegend.TypeMethod, 0);
                    currentType = rootMethod.ReturnType;
                    continue;
                }

                AddSemanticToken(memberId.Start, trailer.LPAR() != null ? PilotSemanticTokenLegend.TypeMethod : PilotSemanticTokenLegend.TypeProperty, 0);
            }
            else if (trailer.unit_expr() != null)
            {
                foreach (var ut in trailer.unit_expr().unit_term())
                {
                    if (ut.identifier() is { } uId)
                    {
                        var uSym = _currentScope.Resolve(uId.GetText());
                        if (uSym != null) AddSymbolReference(uId.Start, uSym);
                        AddSemanticToken(uId.Start, PilotSemanticTokenLegend.TypeTypeParameter, 0);
                    }
                }
                currentType = $"{currentType ?? "float"}<{trailer.unit_expr().GetText()}>";
            }
            else
            {
                Visit(trailer);
            }
        }

        return currentType;
    }

    public override string? VisitTerm(PilotParser.TermContext context)
    {
        var factors = context.factor();
        if (factors.Length == 0) return null;
        string? currentType = Visit(factors[0]);
        if (factors.Length == 1) return currentType;

        for (int i = 1; i < factors.Length; i++)
        {
            string? rhsType = Visit(factors[i]);
            string op = context.GetChild(2 * i - 1).GetText();
            if (currentType != null && rhsType != null &&
                TryExtractUnit(currentType, out string? u1) &&
                TryExtractUnit(rhsType, out string? u2) &&
                (op == "*" || op == "/"))
            {
                currentType = $"float<{u1}{op}{u2}>";
            }
            else if (currentType == "float" || rhsType == "float")
            {
                currentType = "float";
            }
        }
        return currentType;
    }

    public override string? VisitLabeled_stmt(PilotParser.Labeled_stmtContext context)
    {
        string label = context.identifier().GetText();
        AddSemanticToken(context.identifier().Start, PilotSemanticTokenLegend.TypeVariable, 0);
        if (context.for_stmt() != null) return Visit(context.for_stmt());
        if (context.while_stmt() != null) return Visit(context.while_stmt());
        if (context.block() != null) return Visit(context.block());
        return null;
    }

    public override string? VisitBlock(PilotParser.BlockContext context)
    {
        foreach (var stmt in context.stmt())
        {
            Visit(stmt);
        }
        return null;
    }

    public override string? VisitLambda_expr(PilotParser.Lambda_exprContext context)
    {
        var enclosing = _currentScope;
        var (startLine, startCol, _) = GetTokenCoords(context.Start);
        var (endLine, _, endCol) = GetTokenCoords(context.Stop);

        var lambdaScope = new PilotScope
        {
            Name = "lambda",
            Parent = enclosing,
            StartLine = startLine,
            EndLine = endLine
        };
        enclosing.Children.Add(lambdaScope);
        _currentScope = lambdaScope;

        try
        {
            if (context.capture_list() is { } capList)
            {
                foreach (var item in capList.capture_item())
                {
                    if (item.identifier() is { } capId)
                    {
                        string capName = capId.GetText();
                        var capSym = enclosing.Resolve(capName);
                        string capType = capSym?.TypeName ?? "any";
                        DeclareVariableSymbol(capId, item, capName, capType, isMutable: false, isConstant: false, $"captured {capName}: {capType}", "Captured in closure.");
                    }
                }
            }

            if (context.parameters() is { } paramsCtx)
            {
                foreach (var p in paramsCtx.parameter())
                {
                    string pName = p.identifier().GetText();
                    string pType = p.type_annotation()?.type_spec()?.GetText() ?? "any";
                    if (p.type_annotation()?.type_spec() != null)
                        Visit(p.type_annotation().type_spec());
                    DeclareVariableSymbol(p.identifier(), p, pName, pType, isMutable: false, isConstant: false, $"{pName}: {pType}", "Closure parameter.");
                }
            }

            string returnType = context.type_spec()?.GetText() ?? "any";
            if (context.lambda_body() is { } body)
            {
                if (body.test() is { } bodyTest)
                {
                    string? inferredReturn = Visit(bodyTest);
                    if (returnType == "any" && inferredReturn != null)
                        returnType = inferredReturn;
                }
                else if (body.stmt() is { } stmts)
                {
                    foreach (var s in stmts)
                        Visit(s);
                }
            }
            else if (context.test() is { } testExpr)
            {
                string? inferredReturn = Visit(testExpr);
                if (returnType == "any" && inferredReturn != null)
                    returnType = inferredReturn;
            }
            else if (context.stmt() is { } directStmts)
            {
                foreach (var s in directStmts)
                    Visit(s);
            }

            return $"Func[{returnType}]";
        }
        finally
        {
            _currentScope = enclosing;
        }
    }

    public override string? VisitMacro_call(PilotParser.Macro_callContext context)
    {
        string macroName = context.identifier().GetText();
        AddSemanticToken(context.identifier().Start, PilotSemanticTokenLegend.TypeMacro, 0);
        if (context.arglist() != null) Visit(context.arglist());
        if (context.stmt() != null)
        {
            foreach (var s in context.stmt()) Visit(s);
        }
        return "any";
    }

    public override string? VisitSymbol_literal(PilotParser.Symbol_literalContext context)
    {
        if (context.identifier() != null)
        {
            AddSemanticToken(context.identifier().Start, PilotSemanticTokenLegend.TypeString, 0);
        }
        if (context.test() != null)
        {
            Visit(context.test());
        }
        return "symbol";
    }

    private void VisitCascade_trailer(PilotParser.Cascade_trailerContext cascade, string receiverType)
    {
        var target = cascade.cascade_target();
        if (target == null) return;

        string memberType = "any";
        if (target.dotted_name() is { } dotted)
        {
            string methodName = dotted.GetText();
            AddSemanticToken(dotted.Start, PilotSemanticTokenLegend.TypeMethod, 0);

            string baseTypeName = receiverType.TrimEnd('?');
            var typeSym = _currentScope.Resolve(baseTypeName);
            if (typeSym != null && typeSym.Members.TryGetValue(methodName, out var memberSym))
            {
                AddSymbolReference(dotted.Start, memberSym);
                memberType = memberSym.TypeName;
            }
        }

        if (target.arglist() != null)
        {
            Visit(target.arglist());
        }

        if (target.test() != null)
        {
            Visit(target.test());
        }

        var patterns = target.binding_pattern();
        if (patterns.Length > 0)
        {
            bool isMut = target.VAR() != null;
            var assignPattern = patterns[0];
            DeclareBindingPattern(assignPattern, target, memberType, isMut, isConstant: false, "Cascaded method result assigned to");
        }
    }

    public override string? VisitNameof_expr(PilotParser.Nameof_exprContext context)
    {
        AddSemanticToken(context.NAMEOF().Symbol, PilotSemanticTokenLegend.TypeKeyword, 0);
        if (context.dotted_name() is { } dotted)
        {
            var symbol = _currentScope.Resolve(dotted.GetText());
            if (symbol != null)
            {
                AddSymbolReference(dotted.Start, symbol);
            }
            AddSemanticToken(dotted.Start, PilotSemanticTokenLegend.TypeVariable, 0);
        }
        else if (context.test() != null)
        {
            Visit(context.test());
        }
        return "str";
    }

    public override string? VisitAtom(PilotParser.AtomContext context)
    {
        if (context.inline_foreign_block() is { } ifb)
        {
            return VisitInline_foreign_block(ifb);
        }

        if (context.quote_expr() is { } qe)
        {
            return VisitQuote_expr(qe);
        }

        if (context.quasiquote_expr() is { } qqe)
        {
            return VisitQuasiquote_expr(qqe);
        }

        if (context.splice_expr() is { } se)
        {
            return VisitSplice_expr(se);
        }

        if (context.literal() is { } lit)
        {
            if (lit.FSTRING() is { Length: > 0 } fstrings)
            {
                foreach (var fstrNode in fstrings)
                {
                    ProcessFStringNode(fstrNode);
                }
                return "str";
            }
            return InferLiteralType(lit);
        }

        if (context.macro_call() is { } mc)
        {
            return VisitMacro_call(mc);
        }

        if (context.symbol_literal() is { } symLit)
        {
            return VisitSymbol_literal(symLit);
        }

        if (context.nameof_expr() is { } nameofExpr)
        {
            return VisitNameof_expr(nameofExpr);
        }

        if (context.identifier() is { } idCtx)
        {
            string name = idCtx.GetText();
            var symbol = _currentScope.Resolve(name);
            if (symbol != null)
            {
                AddSymbolReference(idCtx.Start, symbol);
                int mod = (!symbol.IsMutable && symbol.Kind == PilotSymbolKind.Variable) ? PilotSemanticTokenLegend.ModReadonly : 0;
                AddSemanticToken(idCtx.Start, GetSemanticTokenTypeForSymbol(symbol), mod);
                return symbol.TypeName;
            }
            return "any";
        }

        if (context.collection_expr() != null)
        {
            base.VisitAtom(context);
            return "List[any]";
        }

        if (context.LBRACE() != null)
        {
            base.VisitAtom(context);
            if (context.dictorsetmaker() != null && context.dictorsetmaker().dict_entry().Length == 0)
                return "Set[any]";
            return "Dict[any, any]";
        }

        if (context.LPAR() != null && context.testlist_comp()?.test().Length > 1)
        {
            var elemTypes = context.testlist_comp().test().Select(t => Visit(t) ?? "any");
            return $"({string.Join(", ", elemTypes)})";
        }

        return base.VisitAtom(context);
    }

    private static string InferLiteralType(PilotParser.LiteralContext lit)
    {
        if (lit.INTEGER() != null || lit.HEX_INTEGER() != null || lit.BIN_INTEGER() != null || lit.OCT_INTEGER() != null)
            return "int";
        if (lit.FLOAT_NUMBER() != null)
            return "float";
        if (lit.STRING().Length > 0 || lit.TRIPLE_STRING().Length > 0 || lit.FSTRING().Length > 0)
            return "str";
        if (lit.TRUE() != null || lit.FALSE() != null)
            return "bool";
        if (lit.NONE() != null || lit.NULL() != null)
            return "null";
        if (lit.UNDEFINED() != null)
            return "undefined";
        if (lit.DURATION_LITERAL() != null)
            return "Duration";
        if (lit.FREQUENCY_LITERAL() != null)
            return "Frequency";
        if (lit.MEMORY_LITERAL() != null)
            return "MemorySize";
        return "any";
    }

    private string ValidateAndResolveType(string varName, string? explicitType, string? inferredType, IToken errorToken)
    {
        if (string.IsNullOrWhiteSpace(explicitType))
            return inferredType ?? "any";

        if (!string.IsNullOrWhiteSpace(inferredType) && inferredType != "any" && !IsCompatibleType(explicitType, inferredType))
        {
            if (inferredType == "null" && !explicitType.EndsWith("?", StringComparison.Ordinal) && !explicitType.Contains("None") && !explicitType.Contains("null"))
            {
                AddDiagnostic(
                    errorToken,
                    PilotDiagnosticSeverity.Error,
                    "PILOT102",
                    $"Null-safety violation on '{varName}': cannot assign 'null' to non-nullable type '{explicitType}'. Use '{explicitType}?' for optional values.");
            }
            else
            {
                AddDiagnostic(
                    errorToken,
                    PilotDiagnosticSeverity.Error,
                    "PILOT102",
                    $"Type mismatch on '{varName}': cannot assign value of type '{inferredType}' to declared type '{explicitType}'.");
            }
        }

        return explicitType;
    }

    private bool IsCompatibleType(string targetType, string valueType)
    {
        if (targetType == "any" || valueType == "any" || targetType == valueType)
            return true;

        // Union types: int | str
        if (targetType.Contains('|'))
        {
            return targetType.Split('|').Select(s => s.Trim()).Any(branch => IsCompatibleType(branch, valueType));
        }

        // Nullable type: T? accepts T or null
        if (targetType.EndsWith("?", StringComparison.Ordinal))
        {
            if (valueType == "null") return true;
            return IsCompatibleType(targetType[..^1], valueType);
        }

        if (valueType == "null")
            return false;

        // Built-in integral conversions: i8, i16, i32, i64, u8, u16, u32, u64, uint, int
        if (targetType is "int" or "i8" or "i16" or "i32" or "i64" or "u8" or "u16" or "u32" or "u64" or "uint" && valueType == "int")
            return true;

        // Built-in numeric widening int -> float / double / decimal / f8 / f16 / f32 / f64 / complex
        if (targetType is "float" or "f8" or "f16" or "f32" or "f64" or "double" or "decimal" or "complex" && valueType is "int" or "float")
            return true;

        // String / char compatibility
        if (targetType is "str" or "string" or "char" && valueType == "str")
            return true;

        // Numeric widening int -> float
        if (targetType is "float" or "f32" or "f64" && valueType == "int")
            return true;

        // Dimensional units: float<meter> vs float<second> (with unit alias expansion)
        if (TryExtractUnit(targetType, out string? targetUnit) && TryExtractUnit(valueType, out string? valueUnit))
        {
            string expandedTarget = ExpandUnitExpression(targetUnit!);
            string expandedValue = ExpandUnitExpression(valueUnit!);
            return string.Equals(expandedTarget, expandedValue, StringComparison.Ordinal);
        }
        if (targetType.Contains('<') && !valueType.Contains('<') && (valueType == "float" || valueType == "int"))
        {
            return true;
        }

        // Generic containers: list[int] vs List[any]
        int targetBracket = targetType.IndexOf('[');
        int valueBracket = valueType.IndexOf('[');
        if (targetBracket > 0 && valueBracket > 0)
        {
            string tBase = targetType[..targetBracket];
            string vBase = valueType[..valueBracket];
            return string.Equals(tBase, vBase, StringComparison.OrdinalIgnoreCase);
        }

        // Strict primitive type checking
        if (IsStrictPrimitive(targetType) && IsStrictPrimitive(valueType))
        {
            return string.Equals(NormalizePrimitive(targetType), NormalizePrimitive(valueType), StringComparison.OrdinalIgnoreCase);
        }

        return true;
    }

    private static bool IsStrictPrimitive(string t) =>
        t is "int" or "i8" or "i16" or "i32" or "i64" or "u8" or "u16" or "u32" or "u64"
            or "float" or "f32" or "f64" or "decimal"
            or "str" or "string" or "bytes"
            or "bool" or "Duration" or "Frequency" or "MemorySize";

    private static string NormalizePrimitive(string t) => t switch
    {
        "i8" or "i16" or "i32" or "i64" or "u8" or "u16" or "u32" or "u64" => "int",
        "f32" or "f64" or "decimal" => "float",
        "string" => "str",
        _ => t
    };

    private static bool TryExtractUnit(string typeName, out string? unitExpr)
    {
        int start = typeName.IndexOf('<');
        int end = typeName.LastIndexOf('>');
        if (start > 0 && end > start + 1)
        {
            unitExpr = typeName[(start + 1)..end].Replace(" ", "");
            return true;
        }
        unitExpr = null;
        return false;
    }

    private string ExpandUnitExpression(string unitExpr)
    {
        string result = unitExpr.Replace(" ", "");
        foreach (var kvp in _unitAliases)
        {
            if (result == kvp.Key)
                return kvp.Value;
        }
        return result;
    }

    private static bool SuiteGuaranteesExit(PilotParser.SuiteContext suite)
    {
        if (suite.simple_stmt() != null)
            return SimpleStmtExits(suite.simple_stmt());

        foreach (var stmt in suite.stmt())
        {
            if (stmt.simple_stmt() != null && SimpleStmtExits(stmt.simple_stmt()))
                return true;
        }
        return false;
    }

    private static bool SimpleStmtExits(PilotParser.Simple_stmtContext simple)
    {
        return simple.small_stmt().Any(s => s.flow_stmt() != null);
    }

    public override string? VisitMatch_stmt(PilotParser.Match_stmtContext context)
    {
        if (context.MATCH() != null)
            AddSemanticToken(context.MATCH().Symbol, PilotSemanticTokenLegend.TypeKeyword, 0);

        if (context.test() != null)
            Visit(context.test());

        if (context.match_suite() != null)
            Visit(context.match_suite());

        return null;
    }

    public override string? VisitMatch_arm(PilotParser.Match_armContext context)
    {
        using (EnterScope($"match_arm_{context.Start.Line}_{context.Start.Column}", context))
        {
            if (context.CASE() != null)
                AddSemanticToken(context.CASE().Symbol, PilotSemanticTokenLegend.TypeKeyword, 0);
            if (context.WHEN() != null)
                AddSemanticToken(context.WHEN().Symbol, PilotSemanticTokenLegend.TypeKeyword, 0);
            if (context.IF() != null)
                AddSemanticToken(context.IF().Symbol, PilotSemanticTokenLegend.TypeKeyword, 0);

            if (context.pattern() != null)
            {
                BindPatternIdentifiers(context.pattern(), "any", isMutable: false, "Pattern variable bound in match arm.");
                Visit(context.pattern());
            }

            if (context.test() != null)
            {
                Visit(context.test());
            }

            if (context.suite() != null)
            {
                Visit(context.suite());
            }
        }
        return null;
    }

    private void BindPatternIdentifiers(PilotParser.PatternContext? pattern, string typeName, bool isMutable, string doc)
    {
        if (pattern == null) return;
        BindPatternNode(pattern, typeName, isMutable, doc);
    }

    private void BindPatternNode(IParseTree node, string typeName, bool isMutable, string doc)
    {
        if (node is PilotParser.Type_or_var_patternContext typeVar)
        {
            if (typeVar.identifier() != null)
            {
                var idCtx = typeVar.identifier();
                var resolvedType = typeVar.dotted_name() != null ? typeVar.dotted_name().GetText() : typeName;
                var sym = CreateSymbol(idCtx, typeVar, idCtx.GetText(), PilotSymbolKind.Variable, resolvedType, "Core",
                    $"let {idCtx.GetText()}: {resolvedType}", doc, isMutable: isMutable);
                RegisterSymbol(sym, PilotSemanticTokenLegend.TypeVariable, PilotSemanticTokenLegend.ModDeclaration | PilotSemanticTokenLegend.ModReadonly);
            }
            else if (typeVar.dotted_name() != null && typeVar.LPAR() == null && typeVar.property_pattern() == null && typeVar.dotted_name().identifier().Length == 1)
            {
                var idCtx = typeVar.dotted_name().identifier(0);
                var sym = CreateSymbol(idCtx, typeVar, idCtx.GetText(), PilotSymbolKind.Variable, typeName, "Core",
                    $"let {idCtx.GetText()}: {typeName}", doc, isMutable: isMutable);
                RegisterSymbol(sym, PilotSemanticTokenLegend.TypeVariable, PilotSemanticTokenLegend.ModDeclaration | PilotSemanticTokenLegend.ModReadonly);
            }
        }
        else if (node is PilotParser.Slice_pattern_elementContext sliceElem)
        {
            if (sliceElem.pattern() != null)
            {
                BindPatternNode(sliceElem.pattern(), typeName, isMutable, doc);
            }
            return;
        }

        for (int i = 0; i < node.ChildCount; i++)
        {
            BindPatternNode(node.GetChild(i), typeName, isMutable, doc);
        }
    }

    private void RegisterGenericParams(PilotParser.Generic_paramsContext? gpCtx, PilotSymbol owner)
    {
        if (gpCtx == null) return;
        foreach (var gp in gpCtx.generic_param())
        {
            var id = gp.identifier();
            owner.GenericParameters.Add(id.GetText());
            var sym = CreateSymbol(id, gp, id.GetText(), PilotSymbolKind.TypeParameter, gp.type_spec()?.GetText() ?? "any", "Core / C#", gp.GetText(), "Generic type parameter.");
            RegisterSymbol(sym, PilotSemanticTokenLegend.TypeTypeParameter, PilotSemanticTokenLegend.ModDeclaration);
        }
    }

    private PilotSymbol DeclareVariableSymbol(
        PilotParser.IdentifierContext idCtx,
        ParserRuleContext fullCtx,
        string name,
        string typeName,
        bool isMutable,
        bool isConstant,
        string signature,
        string doc,
        string domain = "Core",
        PilotSymbolKind kind = PilotSymbolKind.Variable)
    {
        return DeclareVariableSymbolFromToken(idCtx.Start, fullCtx, name, typeName, isMutable, isConstant, signature, doc, domain, kind);
    }

    private PilotSymbol DeclareVariableSymbolFromToken(
        IToken idToken,
        ParserRuleContext fullCtx,
        string name,
        string typeName,
        bool isMutable,
        bool isConstant,
        string signature,
        string doc,
        string domain = "Core",
        PilotSymbolKind kind = PilotSymbolKind.Variable)
    {
        if (_currentScope.Symbols.TryGetValue(name, out var prev) && prev.SelectionLine != idToken.Line - 1)
        {
            AddDiagnostic(
                idToken,
                PilotDiagnosticSeverity.Warning,
                "PILOT105",
                $"Symbol '{name}' is already declared in this scope (line {prev.SelectionLine + 1}).");
        }

        var symbol = CreateSymbolFromToken(idToken, fullCtx, name, kind, typeName, domain, signature, doc, isMutable: isMutable, isConstant: isConstant);
        int mods = PilotSemanticTokenLegend.ModDeclaration | PilotSemanticTokenLegend.ModDefinition;
        if (!isMutable || isConstant) mods |= PilotSemanticTokenLegend.ModReadonly;

        RegisterSymbol(symbol, PilotSemanticTokenLegend.TypeVariable, mods);
        if (_currentScope.EnclosingType != null && _currentScope.Parent == _result.RootScope)
        {
            _currentScope.EnclosingType.Members[name] = symbol;
            _currentScope.EnclosingType.Children.Add(symbol);
        }
        return symbol;
    }

    private PilotSymbol CreateSymbol(
        PilotParser.IdentifierContext idCtx,
        ParserRuleContext fullCtx,
        string name,
        PilotSymbolKind kind,
        string typeName,
        string domain,
        string signature,
        string doc,
        bool isMutable = false,
        bool isConstant = false,
        bool isAsync = false,
        bool isPure = false,
        bool isHardRt = false,
        bool isSoftRt = false,
        string? wcet = null,
        bool isStatic = false,
        bool isParams = false)
    {
        return CreateSymbolFromToken(idCtx.Start, fullCtx, name, kind, typeName, domain, signature, doc, isMutable, isConstant, isAsync, isPure, isHardRt, isSoftRt, wcet, isStatic, isParams);
    }

    private static PilotSymbol CreateSymbolFromToken(
        IToken idToken,
        ParserRuleContext fullCtx,
        string name,
        PilotSymbolKind kind,
        string typeName,
        string domain,
        string signature,
        string doc,
        bool isMutable = false,
        bool isConstant = false,
        bool isAsync = false,
        bool isPure = false,
        bool isHardRt = false,
        bool isSoftRt = false,
        string? wcet = null,
        bool isStatic = false,
        bool isParams = false)
    {
        var (selLine, selCol, selEndCol) = GetTokenCoords(idToken);
        var (startLine, startCol, _) = GetTokenCoords(fullCtx.Start);
        var (endLine, _, endCol) = GetTokenCoords(fullCtx.Stop ?? fullCtx.Start);

        return new PilotSymbol
        {
            Name = name,
            Kind = kind,
            TypeName = typeName,
            Domain = domain,
            IsMutable = isMutable,
            IsConstant = isConstant,
            IsAsync = isAsync,
            IsPure = isPure,
            IsStatic = isStatic,
            IsParams = isParams,
            IsHardRealtime = isHardRt,
            IsSoftRealtime = isSoftRt,
            WcetBudget = wcet,
            SelectionLine = selLine,
            SelectionColumn = selCol,
            SelectionEndColumn = selEndCol,
            StartLine = startLine,
            StartColumn = startCol,
            EndLine = Math.Max(startLine, endLine),
            EndColumn = endCol,
            Signature = signature,
            Documentation = doc
        };
    }

    private void RegisterSymbol(PilotSymbol symbol, int tokenType, int tokenModifiers)
    {
        _currentScope.Symbols[symbol.Name] = symbol;
        if (_currentScope == _result.RootScope)
        {
            _result.TopLevelSymbols.RemoveAll(s => s.Name == symbol.Name);
            _result.TopLevelSymbols.Add(symbol);
        }

        _result.References.Add(new PilotSymbolReference
        {
            Line = symbol.SelectionLine,
            StartColumn = symbol.SelectionColumn,
            EndColumn = symbol.SelectionEndColumn,
            Symbol = symbol
        });

        _result.SemanticTokens.Add(new PilotSemanticToken
        {
            Line = symbol.SelectionLine,
            StartColumn = symbol.SelectionColumn,
            Length = Math.Max(1, symbol.SelectionEndColumn - symbol.SelectionColumn),
            TokenType = tokenType,
            TokenModifiers = tokenModifiers
        });
    }

    private void AddSymbolReferenceDirect(int line, int col, int endCol, PilotSymbol symbol)
    {
        _result.References.Add(new PilotSymbolReference
        {
            Line = line,
            StartColumn = col,
            EndColumn = endCol,
            Symbol = symbol
        });
    }

    private void AddSemanticTokenDirect(int line, int col, int length, int tokenType, int tokenModifiers)
    {
        _result.SemanticTokens.Add(new PilotSemanticToken
        {
            Line = line,
            StartColumn = col,
            Length = Math.Max(1, length),
            TokenType = tokenType,
            TokenModifiers = tokenModifiers
        });
    }

    private static readonly System.Text.RegularExpressions.Regex FStringIdentifierRegex =
        new(@"\b[a-zA-Z_][a-zA-Z0-9_]*\b", System.Text.RegularExpressions.RegexOptions.Compiled);

    private static readonly HashSet<string> FStringKeywords = new(StringComparer.Ordinal)
    {
        "if", "elif", "else", "and", "or", "not", "in", "is", "for", "while",
        "true", "false", "True", "False", "null", "none", "None", "undefined"
    };

    private void ProcessFStringNode(ITerminalNode fstrNode)
    {
        string rawText = fstrNode.GetText();
        if (string.IsNullOrEmpty(rawText)) return;

        var (startLine, startCol, _) = GetTokenCoords(fstrNode.Symbol);

        int currentLine = startLine;
        int currentCol = startCol;

        int i = 0;
        while (i < rawText.Length)
        {
            char c = rawText[i];
            if (c == '\r')
            {
                i++;
                continue;
            }
            if (c == '\n')
            {
                currentLine++;
                currentCol = 0;
                i++;
                continue;
            }

            if (c == '{')
            {
                if (i + 1 < rawText.Length && rawText[i + 1] == '{')
                {
                    i += 2;
                    currentCol += 2;
                    continue;
                }

                int holeStartCol = currentCol + 1;
                int holeStartLine = currentLine;

                i++;
                currentCol++;

                int depth = 1;
                int exprStartIndex = i;
                int exprEndIndex = i;

                while (i < rawText.Length && depth > 0)
                {
                    char hc = rawText[i];
                    if (hc == '\r')
                    {
                        i++;
                        continue;
                    }
                    if (hc == '\n')
                    {
                        currentLine++;
                        currentCol = 0;
                        i++;
                        continue;
                    }

                    if (hc == '{')
                    {
                        depth++;
                    }
                    else if (hc == '}')
                    {
                        depth--;
                        if (depth == 0)
                        {
                            exprEndIndex = i;
                            break;
                        }
                    }

                    i++;
                    currentCol++;
                }

                if (exprEndIndex > exprStartIndex)
                {
                    string holeExpr = rawText.Substring(exprStartIndex, exprEndIndex - exprStartIndex);
                    foreach (System.Text.RegularExpressions.Match match in FStringIdentifierRegex.Matches(holeExpr))
                    {
                        int idRelOffset = match.Index;
                        string idName = match.Value;

                        int idLine = holeStartLine;
                        int idCol = holeStartCol;

                        for (int k = 0; k < idRelOffset; k++)
                        {
                            if (holeExpr[k] == '\n')
                            {
                                idLine++;
                                idCol = 0;
                            }
                            else if (holeExpr[k] != '\r')
                            {
                                idCol++;
                            }
                        }

                        if (FStringKeywords.Contains(idName))
                        {
                            AddSemanticTokenDirect(idLine, idCol, match.Length, PilotSemanticTokenLegend.TypeKeyword, 0);
                        }
                        else
                        {
                            var resolved = _currentScope.Resolve(idName);
                            if (resolved != null)
                            {
                                AddSymbolReferenceDirect(idLine, idCol, idCol + match.Length, resolved);
                                int mod = (!resolved.IsMutable && resolved.Kind == PilotSymbolKind.Variable) ? PilotSemanticTokenLegend.ModReadonly : 0;
                                AddSemanticTokenDirect(idLine, idCol, match.Length, GetSemanticTokenTypeForSymbol(resolved), mod);
                            }
                            else
                            {
                                AddSemanticTokenDirect(idLine, idCol, match.Length, PilotSemanticTokenLegend.TypeVariable, 0);
                            }
                        }
                    }
                }

                if (i < rawText.Length && rawText[i] == '}')
                {
                    i++;
                    currentCol++;
                }
                continue;
            }

            i++;
            currentCol++;
        }
    }

    private void AddSymbolReference(IToken token, PilotSymbol symbol)
    {
        var (line, col, endCol) = GetTokenCoords(token);
        _result.References.Add(new PilotSymbolReference
        {
            Line = line,
            StartColumn = col,
            EndColumn = endCol,
            Symbol = symbol
        });
    }

    private void AddSemanticToken(IToken token, int tokenType, int tokenModifiers)
    {
        var (line, col, endCol) = GetTokenCoords(token);
        _result.SemanticTokens.Add(new PilotSemanticToken
        {
            Line = line,
            StartColumn = col,
            Length = Math.Max(1, endCol - col),
            TokenType = tokenType,
            TokenModifiers = tokenModifiers
        });
    }

    private void AddDiagnostic(IToken token, PilotDiagnosticSeverity severity, string code, string message)
    {
        if (_preprocResult != null && _preprocResult.IsWarningSuppressed(code, token.Line))
            return;

        var (line, col, endCol) = GetTokenCoords(token);
        _result.Diagnostics.Add(new PilotDiagnostic
        {
            StartLine = line,
            StartColumn = col,
            EndLine = line,
            EndColumn = Math.Max(col + 1, endCol),
            Severity = severity,
            Code = code,
            Message = message
        });
    }

    private static int GetSemanticTokenTypeForSymbol(PilotSymbol symbol) => symbol.Kind switch
    {
        PilotSymbolKind.Class or PilotSymbolKind.Actor or PilotSymbolKind.Model or PilotSymbolKind.Table => PilotSemanticTokenLegend.TypeClass,
        PilotSymbolKind.Struct => PilotSemanticTokenLegend.TypeStruct,
        PilotSymbolKind.Interface => PilotSemanticTokenLegend.TypeInterface,
        PilotSymbolKind.Enum => PilotSemanticTokenLegend.TypeEnum,
        PilotSymbolKind.EnumMember => PilotSemanticTokenLegend.TypeEnumMember,
        PilotSymbolKind.Function => PilotSemanticTokenLegend.TypeFunction,
        PilotSymbolKind.Method => PilotSemanticTokenLegend.TypeMethod,
        PilotSymbolKind.Property => PilotSemanticTokenLegend.TypeProperty,
        PilotSymbolKind.Parameter => PilotSemanticTokenLegend.TypeParameter,
        PilotSymbolKind.Macro => PilotSemanticTokenLegend.TypeMacro,
        PilotSymbolKind.Unit or PilotSymbolKind.TypeParameter or PilotSymbolKind.TypeAlias => PilotSemanticTokenLegend.TypeTypeParameter,
        PilotSymbolKind.Database or PilotSymbolKind.Dsl or PilotSymbolKind.Module => PilotSemanticTokenLegend.TypeNamespace,
        _ => PilotSemanticTokenLegend.TypeVariable
    };

    private static (int Line, int Col, int EndCol) GetTokenCoords(IToken? token)
    {
        if (token == null) return (0, 0, 1);
        int line = Math.Max(0, token.Line - 1);
        int col = Math.Max(0, token.Column);
        int len = string.IsNullOrEmpty(token.Text) ? 1 : token.Text.Length;
        return (line, col, col + len);
    }

    private static string ExtractElementType(string collectionType)
    {
        int start = collectionType.IndexOf('[');
        int end = collectionType.LastIndexOf(']');
        if (start > 0 && end > start + 1)
            return collectionType[(start + 1)..end].Trim();
        return "any";
    }

    private static PilotParser.IdentifierContext? TryExtractSimpleIdentifier(PilotParser.TestContext test)
    {
        var atoms = test.GetRuleContexts<PilotParser.Pipe_exprContext>();
        if (atoms.Length != 1) return null;
        string text = test.GetText();
        if (text.Contains('.') || text.Contains('[') || text.Contains('(')) return null;
        return FindFirstIdentifier(test);
    }

    private static PilotParser.IdentifierContext? FindFirstIdentifier(IParseTree tree)
    {
        if (tree is PilotParser.IdentifierContext id) return id;
        for (int i = 0; i < tree.ChildCount; i++)
        {
            var found = FindFirstIdentifier(tree.GetChild(i));
            if (found != null) return found;
        }
        return null;
    }

    private static string Truncate(string value, int maxLen) =>
        value.Length <= maxLen ? value : value[..maxLen] + "...";

    private IDisposable EnterScope(
        string name,
        ParserRuleContext ctx,
        bool? inHardRt = null,
        bool? inSoftRt = null,
        bool? inNoAlloc = null,
        bool? inAsync = null,
        bool? inLoop = null,
        bool? inTransaction = null,
        string? returnType = null,
        PilotSymbol? enclosingType = null)
    {
        var parent = _currentScope;
        var child = new PilotScope
        {
            Name = name,
            Parent = parent,
            StartLine = Math.Max(0, (ctx.Start?.Line ?? 1) - 1),
            EndLine = Math.Max(0, (ctx.Stop?.Line ?? ctx.Start?.Line ?? 1) - 1),
            InHardRealtime = inHardRt ?? parent.InHardRealtime,
            InSoftRealtime = inSoftRt ?? parent.InSoftRealtime,
            InNoAlloc = inNoAlloc ?? parent.InNoAlloc,
            InAsync = inAsync ?? parent.InAsync,
            InLoop = inLoop ?? parent.InLoop,
            InTransaction = inTransaction ?? parent.InTransaction,
            EnclosingReturnType = returnType ?? parent.EnclosingReturnType,
            EnclosingType = enclosingType ?? parent.EnclosingType
        };
        parent.Children.Add(child);
        _currentScope = child;
        return new ScopeRestorer(this, parent);
    }

    private sealed class ScopeRestorer : IDisposable
    {
        private readonly PilotSemanticAnalyzer _analyzer;
        private readonly PilotScope _previous;
        public ScopeRestorer(PilotSemanticAnalyzer analyzer, PilotScope previous)
        {
            _analyzer = analyzer;
            _previous = previous;
        }
        public void Dispose() => _analyzer._currentScope = _previous;
    }

    private sealed class LspErrorListener : BaseErrorListener
    {
        private readonly List<PilotDiagnostic> _diagnostics;
        public LspErrorListener(List<PilotDiagnostic> diagnostics) => _diagnostics = diagnostics;

        public override void SyntaxError(
            IRecognizer recognizer,
            IToken offendingSymbol,
            int line,
            int charPositionInLine,
            string msg,
            RecognitionException e)
        {
            int zeroLine = Math.Max(0, line - 1);
            int col = Math.Max(0, charPositionInLine);
            int len = Math.Max(1, offendingSymbol?.Text?.Length ?? 1);
            _diagnostics.Add(new PilotDiagnostic
            {
                StartLine = zeroLine,
                StartColumn = col,
                EndLine = zeroLine,
                EndColumn = col + len,
                Severity = PilotDiagnosticSeverity.Error,
                Code = "PILOT001",
                Message = $"Syntax error: {msg}"
            });
        }
    }
}
