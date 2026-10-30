import os
import re
import sys
import shutil
import tempfile
import time
import subprocess
from ipykernel.kernelbase import Kernel

PILOT_BUILTINS = [
    ("println", "println(values...) -> void", "Prints values separated by spaces with a newline to standard output."),
    ("print", "print(values...) -> void", "Prints values separated by spaces without a trailing newline."),
    ("len", "len(collection: any) -> int", "Returns the number of elements in a list, map, string, or collection."),
    ("range", "range(start: int, end: int, step: int = 1) -> List[int]", "Generates an iterable sequence of integers."),
    ("typeof", "typeof(val: any) -> str", "Returns the runtime type name of the given value."),
    ("type_of", "type_of(val: any) -> str", "Alias for typeof."),
    ("assert", "assert(condition: bool, msg: str = \"\") -> void", "Asserts that a condition is true, aborting if false."),
    ("sum", "sum(collection: any) -> int | float", "Calculates the sum of all elements in the collection."),
    ("min", "min(a: any, b: any) -> any", "Returns the lesser of two values."),
    ("max", "max(a: any, b: any) -> any", "Returns the greater of two values."),
    ("sqrt", "sqrt(x: float) -> float", "Returns the square root of a floating-point number."),
    ("sin", "sin(x: float) -> float", "Computes the sine of an angle in radians."),
    ("cos", "cos(x: float) -> float", "Computes the cosine of an angle in radians."),
    ("tan", "tan(x: float) -> float", "Computes the tangent of an angle in radians."),
    ("exp", "exp(x: float) -> float", "Computes e^x."),
    ("ln", "ln(x: float) -> float", "Computes natural logarithm."),
    ("floor", "floor(x: float) -> float", "Rounds down to nearest integer."),
    ("ceil", "ceil(x: float) -> float", "Rounds up to nearest integer."),
    ("pow", "pow(base: float, exponent: float) -> float", "Raises a number to power."),
    ("abs", "abs(x: int | float) -> int | float", "Returns absolute value."),
    ("is_some", "is_some(opt: Option[T]) -> bool", "Returns true if option contains a value."),
    ("is_none", "is_none(opt: Option[T]) -> bool", "Returns true if option is none."),
    ("unwrap", "unwrap(opt_or_res: any) -> any", "Extracts value or panics."),
    ("is_ok", "is_ok(res: Result[T, E]) -> bool", "Returns true if result is Ok."),
    ("is_err", "is_err(res: Result[T, E]) -> bool", "Returns true if result is Err."),
]

PILOT_KEYWORDS = [
    "fn", "let", "var", "const", "return", "if", "else", "match", "for", "in", "while",
    "loop", "struct", "class", "actor", "interface", "protocol", "extension", "type", "enum",
    "import", "from", "export", "pub", "private", "async", "await", "spawn", "yield",
    "break", "continue", "true", "false", "none", "self", "super", "try", "catch", "defer"
]

def find_pilot_executable():
    # 1. Environment variable
    if os.environ.get("PILOT_BIN") and os.path.isfile(os.environ["PILOT_BIN"]):
        return os.environ["PILOT_BIN"]

    # 2. Workspace root relative to this file
    this_dir = os.path.dirname(os.path.abspath(__file__))
    repo_root = os.path.abspath(os.path.join(this_dir, "..", ".."))

    candidates = [
        os.path.join(repo_root, "vscode-pilot", "server", "pilot"),
        os.path.join(repo_root, "target", "release", "pilot"),
        os.path.join(repo_root, "target", "debug", "pilot"),
    ]
    for c in candidates:
        if os.path.isfile(c) and os.access(c, os.X_OK):
            return c

    # 3. System PATH
    which_pilot = shutil.which("pilot")
    if which_pilot:
        return which_pilot

    return "pilot"


class PilotKernel(Kernel):
    implementation = "Pilot"
    implementation_version = "1.1.51"
    language = "pilot"
    language_version = "1.1.51"
    language_info = {
        "name": "pilot",
        "mimetype": "text/x-pilot",
        "file_extension": ".pilot",
        "pygments_lexer": "python",
        "codemirror_mode": "python",
    }
    banner = "Pilot Language Interactive Jupyter Kernel v1.1.51"

    def __init__(self, **kwargs):
        super().__init__(**kwargs)
        self.pilot_bin = find_pilot_executable()
        # Active declarations: dict of name -> code string (e.g. function defs, imports, variable defs)
        self.declarations = {}
        # Raw imports list
        self.imports = []
        self.user_symbols = set()

    def do_execute(self, code, silent, store_history=True, user_expressions=None, allow_stdin=False):
        trimmed = code.strip()
        if not trimmed:
            return {"status": "ok", "execution_count": self.execution_count, "payload": [], "user_expressions": {}}

        # Handle magic commands
        if trimmed == "%reset" or trimmed == ":reset":
            self.declarations.clear()
            self.imports.clear()
            self.user_symbols.clear()
            if not silent:
                self.send_response(self.iopub_socket, "stream", {"name": "stdout", "text": "Session reset. All declarations cleared.\n"})
            return {"status": "ok", "execution_count": self.execution_count, "payload": [], "user_expressions": {}}

        if trimmed == "%info" or trimmed == ":info":
            info_text = f"Pilot Toolchain: {self.pilot_bin}\nKernel Version: {self.implementation_version}\nActive Declarations: {len(self.declarations)}\nActive Imports: {len(self.imports)}\n"
            if not silent:
                self.send_response(self.iopub_socket, "stream", {"name": "stdout", "text": info_text})
            return {"status": "ok", "execution_count": self.execution_count, "payload": [], "user_expressions": {}}

        measure_time = False
        if trimmed.startswith("%time"):
            measure_time = True
            trimmed = trimmed[5:].strip()

        # Prepare transformed cell code with auto-println for trailing expression
        transformed_code, is_expr = self._transform_code(trimmed)

        # Assemble full execution code including previous declarations
        full_code = self._assemble_code(transformed_code)

        start_time = time.perf_counter()

        with tempfile.NamedTemporaryFile("w", suffix=".pilot", delete=False) as f:
            f.write(full_code)
            tmp_path = f.name

        try:
            proc = subprocess.run(
                [self.pilot_bin, "run", tmp_path],
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                timeout=30,
            )
            elapsed_ms = (time.perf_counter() - start_time) * 1000

            if proc.stdout and not silent:
                self.send_response(self.iopub_socket, "stream", {"name": "stdout", "text": proc.stdout})
            if proc.stderr and not silent:
                self.send_response(self.iopub_socket, "stream", {"name": "stderr", "text": proc.stderr})

            if measure_time and not silent:
                self.send_response(self.iopub_socket, "stream", {"name": "stdout", "text": f"\n[Execution time: {elapsed_ms:.2f} ms]\n"})

            if proc.returncode == 0:
                # Code ran successfully, extract and preserve new declarations
                self._extract_declarations(trimmed)
                return {
                    "status": "ok",
                    "execution_count": self.execution_count,
                    "payload": [],
                    "user_expressions": {},
                }
            else:
                return {
                    "status": "error",
                    "execution_count": self.execution_count,
                    "ename": "PilotExecutionError",
                    "evalue": f"Exit code {proc.returncode}",
                    "traceback": proc.stderr.splitlines() if proc.stderr else ["Execution failed"],
                }
        except subprocess.TimeoutExpired:
            self.send_response(self.iopub_socket, "stream", {"name": "stderr", "text": "Execution timed out (30s limit)\n"})
            return {
                "status": "error",
                "execution_count": self.execution_count,
                "ename": "TimeoutError",
                "evalue": "Cell execution timed out",
                "traceback": [],
            }
        except Exception as e:
            self.send_response(self.iopub_socket, "stream", {"name": "stderr", "text": f"Error: {e}\n"})
            return {
                "status": "error",
                "execution_count": self.execution_count,
                "ename": type(e).__name__,
                "evalue": str(e),
                "traceback": [str(e)],
            }
        finally:
            try:
                os.remove(tmp_path)
            except OSError:
                pass

    def _transform_code(self, code: str):
        lines = code.splitlines()
        if not lines:
            return code, False

        # Find last non-empty, non-comment line
        last_idx = len(lines) - 1
        while last_idx >= 0 and (not lines[last_idx].strip() or lines[last_idx].strip().startswith("#")):
            last_idx -= 1

        if last_idx < 0:
            return code, False

        last_line = lines[last_idx].strip()
        no_wrap_prefixes = (
            "let ", "var ", "const ", "fn ", "struct ", "class ", "actor ", "interface ",
            "protocol ", "extension ", "type ", "enum ", "import ", "from ", "for ", "while ",
            "if ", "else", "match ", "return ", "println(", "print(", "assert(", "try ", "catch "
        )

        if not any(last_line.startswith(p) for p in no_wrap_prefixes) and not last_line.endswith("{") and not last_line.endswith(":"):
            # Trailing expression: wrap in println
            lines[last_idx] = f"println({last_line})"
            return "\n".join(lines), True

        return code, False

    def _assemble_code(self, cell_code: str) -> str:
        parts = []
        # 1. Imports
        if self.imports:
            parts.extend(self.imports)
        # 2. Cumulative declarations
        for decl in self.declarations.values():
            parts.append(decl)
        # 3. Current cell code
        parts.append(cell_code)
        return "\n\n".join(parts) + "\n"

    def _extract_declarations(self, code: str):
        lines = code.splitlines()
        i = 0
        while i < len(lines):
            line = lines[i]
            s = line.strip()

            if s.startswith("import ") or s.startswith("from "):
                if s not in self.imports:
                    self.imports.append(s)
                i += 1
                continue

            # Function, Struct, Class, Interface declarations
            m_fn = re.match(r"^(pub\s+)?fn\s+([A-Za-z0-9_]+)", s)
            m_decl = re.match(r"^(pub\s+)?(struct|class|actor|interface|protocol|extension|type|enum)\s+([A-Za-z0-9_]+)", s)

            if m_fn or m_decl:
                name = m_fn.group(2) if m_fn else m_decl.group(3)
                # Capture whole block
                block_lines = [line]
                open_braces = line.count("{") - line.count("}")
                i += 1
                while i < len(lines):
                    b_line = lines[i]
                    block_lines.append(b_line)
                    open_braces += b_line.count("{") - b_line.count("}")
                    i += 1
                    if open_braces <= 0 and ("{" in line or b_line.strip().endswith("}")):
                        break
                self.declarations[name] = "\n".join(block_lines)
                self.user_symbols.add(name)
                continue

            # Let / Var / Const at top-level
            m_var = re.match(r"^(pub\s+)?(let|var|const)\s+([A-Za-z0-9_]+)", s)
            if m_var:
                var_name = m_var.group(3)
                self.declarations[var_name] = line
                self.user_symbols.add(var_name)
                i += 1
                continue

            i += 1

    def do_complete(self, code, cursor_pos):
        # Extract word preceding cursor
        line_start = code.rfind("\n", 0, cursor_pos) + 1
        current_line = code[line_start:cursor_pos]
        m = re.search(r"([A-Za-z0-9_]+)$", current_line)
        if not m:
            return {"matches": [], "cursor_start": cursor_pos, "cursor_end": cursor_pos, "metadata": {}, "status": "ok"}

        prefix = m.group(1)
        start_pos = cursor_pos - len(prefix)

        matches = []
        for name, _, _ in PILOT_BUILTINS:
            if name.startswith(prefix):
                matches.append(name)
        for kw in PILOT_KEYWORDS:
            if kw.startswith(prefix) and kw not in matches:
                matches.append(kw)
        for sym in self.user_symbols:
            if sym.startswith(prefix) and sym not in matches:
                matches.append(sym)

        matches.sort()
        return {
            "matches": matches,
            "cursor_start": start_pos,
            "cursor_end": cursor_pos,
            "metadata": {},
            "status": "ok",
        }

    def do_inspect(self, code, cursor_pos, detail_level=0):
        # Find token under cursor
        m_word = None
        for m in re.finditer(r"[A-Za-z0-9_]+", code):
            if m.start() <= cursor_pos <= m.end():
                m_word = m.group(0)
                break

        if not m_word:
            return {"status": "ok", "data": {}, "found": False}

        # Check builtins
        for name, sig, summary in PILOT_BUILTINS:
            if name == m_word:
                text = f"**{name}**\n\n```pilot\n{sig}\n```\n\n{summary}"
                return {
                    "status": "ok",
                    "found": True,
                    "data": {
                        "text/plain": f"{sig}\n\n{summary}",
                        "text/markdown": text,
                    },
                }

        # Check user declarations
        if m_word in self.declarations:
            decl = self.declarations[m_word]
            return {
                "status": "ok",
                "found": True,
                "data": {
                    "text/plain": decl,
                    "text/markdown": f"```pilot\n{decl}\n```",
                },
            }

        return {"status": "ok", "data": {}, "found": False}
