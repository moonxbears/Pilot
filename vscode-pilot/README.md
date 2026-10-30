# Pilot Language Support for Visual Studio Code

Official VS Code extension for the **Pilot** programming language.

## Features

- **Rich Syntax Highlighting**: TextMate grammar for all Pilot keywords, operators, strings, comments, types, and annotations.
- **Language Server Protocol (LSP)**:
  - Real-time syntax and semantic diagnostics with sub-millisecond latency.
  - Hover tooltips with full function signatures and documentation.
  - Auto-completions for built-in intrinsics, types, and keywords.
  - Inlay hints for variable types and parameter names.
  - Go to Definition (`F12`) and Document Symbols (`Ctrl+Shift+O`).
- **Interactive Visual Debugger (DAP)**:
  - Set breakpoints in the editor gutter or via `F9`.
  - Pause on entry or at breakpoints.
  - Inspect Call Stack frames with file path, line, and column coordinates.
  - Inspect Variables (Locals and Globals) with live runtime types and values.
  - Interactive Stepping: Continue (`F5`), Step Over (`F10`), Step Into (`F11`), Step Out (`Shift+F11`).
  - Hover over variables during debugging to inspect their live values.
  - Evaluate expressions dynamically in the Debug Console.
- **Built-in Bundled Toolchain**: Includes native `pilot`, `pilot-lsp`, and `pilot-dap` binaries so no external installation is required.

## Quick Start

1. Install the `.vsix` extension package into VS Code.
2. Open any `.pilot` file.
3. Press `F5` to debug, or click the **Run** / **Debug** icons in the top-right of the editor title bar.
