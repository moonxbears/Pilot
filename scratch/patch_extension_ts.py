with open('vscode-pilot/src/extension.ts', 'r', encoding='utf-8') as f:
    content = f.read()

target = """        vscode.languages.registerInlayHintsProvider(
            selector,
            {
                async provideInlayHints(document: vscode.TextDocument, range: vscode.Range): Promise<vscode.InlayHint[]> {
                    client.notifyDidOpen(document);
                    const res = await client.sendRequest('textDocument/inlayHint', {
                        textDocument: { uri: document.uri.toString() },
                        range: {
                            start: { line: range.start.line, character: range.start.character },
                            end: { line: range.end.line, character: range.end.character }
                        }
                    });
                    if (!Array.isArray(res)) {
                        return [];
                    }
                    return res.map((item: any) => {
                        const pos = new vscode.Position(item.position.line, item.position.character);
                        const hint = new vscode.InlayHint(
                            pos,
                            item.label,
                            item.kind === 2 ? vscode.InlayHintKind.Parameter : vscode.InlayHintKind.Type
                        );
                        hint.paddingLeft = !!item.paddingLeft;
                        hint.paddingRight = !!item.paddingRight;
                        return hint;
                    });
                }
            }
        )
    );"""

repl = """        vscode.languages.registerInlayHintsProvider(
            selector,
            {
                async provideInlayHints(document: vscode.TextDocument, range: vscode.Range): Promise<vscode.InlayHint[]> {
                    client.notifyDidOpen(document);
                    const res = await client.sendRequest('textDocument/inlayHint', {
                        textDocument: { uri: document.uri.toString() },
                        range: {
                            start: { line: range.start.line, character: range.start.character },
                            end: { line: range.end.line, character: range.end.character }
                        }
                    });
                    if (!Array.isArray(res)) {
                        return [];
                    }
                    return res.map((item: any) => {
                        const pos = new vscode.Position(item.position.line, item.position.character);
                        const hint = new vscode.InlayHint(
                            pos,
                            item.label,
                            item.kind === 2 ? vscode.InlayHintKind.Parameter : vscode.InlayHintKind.Type
                        );
                        hint.paddingLeft = !!item.paddingLeft;
                        hint.paddingRight = !!item.paddingRight;
                        return hint;
                    });
                }
            }
        ),
        vscode.languages.registerCodeActionsProvider(
            selector,
            {
                async provideCodeActions(
                    document: vscode.TextDocument,
                    range: vscode.Range | vscode.Selection,
                    context: vscode.CodeActionContext
                ): Promise<(vscode.Command | vscode.CodeAction)[]> {
                    client.notifyDidOpen(document);
                    const res = await client.sendRequest('textDocument/codeAction', {
                        textDocument: { uri: document.uri.toString() },
                        range: {
                            start: { line: range.start.line, character: range.start.character },
                            end: { line: range.end.line, character: range.end.character }
                        },
                        context: {
                            diagnostics: context.diagnostics.map(d => ({
                                range: {
                                    start: { line: d.range.start.line, character: d.range.start.character },
                                    end: { line: d.range.end.line, character: d.range.end.character }
                                },
                                message: d.message,
                                severity: d.severity === vscode.DiagnosticSeverity.Error ? 1 : 2,
                                code: d.code
                            }))
                        }
                    });
                    if (!Array.isArray(res)) {
                        return [];
                    }
                    return res.map((action: any) => {
                        const ca = new vscode.CodeAction(
                            action.title,
                            action.kind ? new vscode.CodeActionKind(action.kind) : vscode.CodeActionKind.QuickFix
                        );
                        ca.isPreferred = !!action.isPreferred;
                        if (action.command) {
                            ca.command = {
                                title: action.command.title || action.title,
                                command: action.command.command,
                                arguments: action.command.arguments
                            };
                        }
                        if (action.edit && action.edit.changes) {
                            const we = new vscode.WorkspaceEdit();
                            for (const [uriStr, textEdits] of Object.entries<any>(action.edit.changes)) {
                                const targetUri = vscode.Uri.parse(uriStr);
                                const edits = (textEdits || []).map((te: any) => new vscode.TextEdit(
                                    new vscode.Range(
                                        te.range.start.line,
                                        te.range.start.character,
                                        te.range.end.line,
                                        te.range.end.character
                                    ),
                                    te.newText
                                ));
                                we.set(targetUri, edits);
                            }
                            ca.edit = we;
                        }
                        return ca;
                    });
                }
            },
            {
                providedCodeActionKinds: [
                    vscode.CodeActionKind.QuickFix,
                    vscode.CodeActionKind.Refactor,
                    vscode.CodeActionKind.RefactorExtract,
                    vscode.CodeActionKind.SourceFixAll
                ]
            }
        )
    );"""

c_nl = '\r\n' if '\r\n' in content else '\n'
def norm(s):
    return s.replace('\r\n', '\n').replace('\n', c_nl)

content = content.replace(norm(target), norm(repl), 1)

with open('vscode-pilot/src/extension.ts', 'w', encoding='utf-8', newline='') as f:
    f.write(content)
print("Updated extension.ts with registerCodeActionsProvider")
