import * as vscode from 'vscode';
import * as cp from 'child_process';
import * as path from 'path';
import * as fs from 'fs';

const TOKEN_TYPES = [
    'namespace', 'type', 'class', 'enum', 'interface', 'struct',
    'typeParameter', 'parameter', 'variable', 'property', 'enumMember',
    'event', 'function', 'method', 'macro', 'keyword', 'modifier',
    'comment', 'string', 'number', 'regexp', 'operator', 'decorator'
];

const TOKEN_MODIFIERS = [
    'declaration', 'definition', 'readonly', 'static', 'deprecated',
    'abstract', 'async', 'modification', 'documentation', 'defaultLibrary'
];

function pickNewestExistingFile(candidates: string[]): string {
    let bestPath = '';
    let bestMtime = -1;
    for (const candidate of candidates) {
        if (!candidate) {
            continue;
        }
        try {
            if (fs.existsSync(candidate)) {
                const stat = fs.statSync(candidate);
                if (stat.isFile() && stat.mtimeMs > bestMtime) {
                    bestMtime = stat.mtimeMs;
                    bestPath = candidate;
                }
            }
        } catch {
            // Ignore inaccessible candidate paths
        }
    }
    return bestPath;
}

function resolvePilotExecutionPrefix(extensionPath: string, configuredPilotPath?: string): string {
    if (configuredPilotPath && configuredPilotPath !== 'pilot') {
        return `"${configuredPilotPath}"`;
    }

    const wsRoot = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath || '';
    const repoRoot = path.resolve(extensionPath, '..');

    const nativeCandidates: string[] = [
        path.join(extensionPath, 'server', 'pilot'),
        path.join(repoRoot, 'target', 'release', 'pilot'),
        path.join(repoRoot, 'target', 'debug', 'pilot'),
    ];
    if (wsRoot) {
        nativeCandidates.push(
            path.join(wsRoot, 'target', 'release', 'pilot'),
            path.join(wsRoot, 'target', 'debug', 'pilot')
        );
    }

    for (const bin of nativeCandidates) {
        if (fs.existsSync(bin)) {
            return `"${bin}"`;
        }
    }

    return 'pilot';
}

class PilotLspClient implements vscode.Disposable {
    private process: cp.ChildProcessWithoutNullStreams | null = null;
    private nextId = 1;
    private pending = new Map<number, { resolve: (val: any) => void; reject: (err: any) => void }>();
    private buffer: Buffer<any> = Buffer.alloc(0);
    private diagnostics: vscode.DiagnosticCollection;
    private statusBar: vscode.StatusBarItem;
    private outputChannel: vscode.OutputChannel;
    private extensionPath: string;
    private semanticTokensEmitter?: vscode.EventEmitter<void>;

    constructor(extensionPath: string, diagnostics: vscode.DiagnosticCollection, statusBar: vscode.StatusBarItem, outputChannel: vscode.OutputChannel) {
        this.extensionPath = extensionPath;
        this.diagnostics = diagnostics;
        this.statusBar = statusBar;
        this.outputChannel = outputChannel;
    }

    public setSemanticTokensEmitter(emitter: vscode.EventEmitter<void>): void {
        this.semanticTokensEmitter = emitter;
    }

    public async start(): Promise<void> {
        this.stop();

        const config = vscode.workspace.getConfiguration('pilot.languageServer');
        const customPath = config.get<string>('serverPath') || '';
        const wsRoot = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath || '';
        const repoRoot = path.resolve(this.extensionPath, '..');

        const nativeCandidates: string[] = [
            path.join(this.extensionPath, 'server', 'pilot-lsp'),
            path.join(repoRoot, 'target', 'release', 'pilot-lsp'),
            path.join(repoRoot, 'target', 'debug', 'pilot-lsp'),
        ];
        if (wsRoot) {
            nativeCandidates.push(
                path.join(wsRoot, 'target', 'release', 'pilot-lsp'),
                path.join(wsRoot, 'target', 'debug', 'pilot-lsp')
            );
        }

        let nativeLsp = '';
        let nativeLspArgs: string[] = [];
        if (customPath && fs.existsSync(customPath) && !customPath.endsWith('.dll') && !customPath.endsWith('.csproj')) {
            nativeLsp = customPath;
            if (path.basename(customPath) === 'pilot') {
                nativeLspArgs = ['lsp'];
            }
        } else {
            const newest = pickNewestExistingFile(nativeCandidates);
            if (newest) {
                nativeLsp = newest;
                if (path.basename(newest) === 'pilot') {
                    nativeLspArgs = ['lsp'];
                }
            }
        }

        // Fallback to checking `pilot lsp` if standalone `pilot-lsp` binary isn't found
        if (!nativeLsp) {
            const pilotBin = resolvePilotExecutionPrefix(this.extensionPath);
            const unquotedPilot = pilotBin.replace(/^"(.*)"$/, '$1');
            if (unquotedPilot !== 'pilot' && fs.existsSync(unquotedPilot)) {
                nativeLsp = unquotedPilot;
                nativeLspArgs = ['lsp'];
            }
        }

        if (nativeLsp) {
            this.outputChannel.appendLine(`[Pilot LSP] Starting server: ${nativeLsp} ${nativeLspArgs.join(' ')}`);
            const workingDir = wsRoot || path.dirname(nativeLsp);
            this.process = cp.spawn(nativeLsp, nativeLspArgs, {
                cwd: workingDir,
                stdio: ['pipe', 'pipe', 'pipe']
            });
        } else {
            this.statusBar.text = '$(error) Pilot LSP (Not Found)';
            vscode.window.showErrorMessage(`Pilot Language Server (pilot-lsp) not found.`);
            return;
        }

        const spawnedProcess = this.process;
        this.process.stdout.on('data', (chunk: Buffer) => this.onData(chunk));
        this.process.stderr.on('data', (chunk: Buffer) => {
            this.outputChannel.appendLine(`[Pilot LSP stderr] ${chunk.toString('utf8')}`);
        });
        this.process.on('exit', (code) => {
            this.outputChannel.appendLine(`[Pilot LSP] Process exited with code ${code}`);
            for (const [id, p] of this.pending) {
                p.resolve(null);
            }
            this.pending.clear();
            if (this.process === spawnedProcess) {
                this.process = null;
                this.statusBar.text = '$(sync~spin) Pilot LSP (Restarting...)';
                setTimeout(() => {
                    if (!this.process) {
                        this.start().catch(() => {});
                    }
                }, 800);
            } else {
                this.statusBar.text = '$(warning) Pilot LSP (Stopped)';
            }
        });

        const initResult = await this.sendRequest('initialize', {
            processId: process.pid,
            rootUri: vscode.workspace.workspaceFolders?.[0]?.uri.toString() ?? null,
            capabilities: {}
        });
        this.sendNotification('initialized', {});
        const srvVersion: string = initResult?.serverInfo?.version || '';
        const srvCommit: string = initResult?.serverInfo?.commitSha || '';
        if (srvVersion) {
            this.statusBar.text = `$(check) Pilot LSP v${srvVersion}`;
            this.statusBar.tooltip = `Pilot Language Server v${srvVersion}${srvCommit ? ` (commit ${srvCommit})` : ''} — Click to restart`;
            this.outputChannel.appendLine(`[Pilot LSP] Connected to Pilot Language Server v${srvVersion}`);
        } else {
            this.statusBar.text = '$(check) Pilot LSP';
        }

        for (const doc of vscode.workspace.textDocuments) {
            if (doc.languageId === 'pilot') {
                this.notifyDidOpen(doc);
            }
        }
        if (this.semanticTokensEmitter) {
            this.semanticTokensEmitter.fire();
        }
    }

    public notifyDidOpen(doc: vscode.TextDocument): void {
        this.sendNotification('textDocument/didOpen', {
            textDocument: {
                uri: doc.uri.toString(),
                languageId: doc.languageId,
                version: doc.version,
                text: doc.getText()
            }
        });
    }

    public notifyDidChange(doc: vscode.TextDocument): void {
        this.sendNotification('textDocument/didChange', {
            textDocument: {
                uri: doc.uri.toString(),
                version: doc.version
            },
            contentChanges: [{ text: doc.getText() }]
        });
    }

    public notifyDidClose(doc: vscode.TextDocument): void {
        this.sendNotification('textDocument/didClose', {
            textDocument: { uri: doc.uri.toString() }
        });
    }

    public sendRequest(method: string, params: any): Promise<any> {
        if (!this.process) {
            return Promise.resolve(null);
        }
        const id = this.nextId++;
        const payload = JSON.stringify({ jsonrpc: '2.0', id, method, params });
        const byteLen = Buffer.byteLength(payload, 'utf8');
        const message = `Content-Length: ${byteLen}\r\n\r\n${payload}`;

        return new Promise((resolve, reject) => {
            const timer = setTimeout(() => {
                if (this.pending.has(id)) {
                    this.pending.delete(id);
                    resolve(null);
                }
            }, 3000);
            this.pending.set(id, {
                resolve: (val) => {
                    clearTimeout(timer);
                    resolve(val);
                },
                reject: (err) => {
                    clearTimeout(timer);
                    reject(err);
                }
            });
            this.process!.stdin.write(message, 'utf8');
        });
    }

    public sendNotification(method: string, params: any): void {
        if (!this.process) {
            return;
        }
        const payload = JSON.stringify({ jsonrpc: '2.0', method, params });
        const byteLen = Buffer.byteLength(payload, 'utf8');
        this.process.stdin.write(`Content-Length: ${byteLen}\r\n\r\n${payload}`, 'utf8');
    }

    private onData(chunk: Buffer): void {
        this.buffer = (this.buffer.length === 0 ? chunk : Buffer.concat([this.buffer, chunk])) as Buffer;

        while (true) {
            const headerEnd = this.buffer.indexOf('\r\n\r\n');
            if (headerEnd === -1) {
                break;
            }

            const headerText = this.buffer.subarray(0, headerEnd).toString('ascii');
            const match = /Content-Length:\s*(\d+)/i.exec(headerText);
            if (!match) {
                this.buffer = this.buffer.subarray(headerEnd + 4);
                continue;
            }

            const contentLength = parseInt(match[1], 10);
            const totalLength = headerEnd + 4 + contentLength;
            if (this.buffer.length < totalLength) {
                break;
            }

            const jsonText = this.buffer.subarray(headerEnd + 4, totalLength).toString('utf8');
            this.buffer = this.buffer.subarray(totalLength);

            try {
                const msg = JSON.parse(jsonText);
                this.handleMessage(msg);
            } catch (err) {
                this.outputChannel.appendLine(`[Pilot LSP] JSON parse error: ${err}`);
            }
        }
    }

    private handleMessage(msg: any): void {
        if (msg.id !== undefined && this.pending.has(msg.id)) {
            const handler = this.pending.get(msg.id)!;
            this.pending.delete(msg.id);
            handler.resolve(msg.result);
            return;
        }

        if (msg.method === 'textDocument/publishDiagnostics' && msg.params) {
            const uri = vscode.Uri.parse(msg.params.uri);
            const diags: vscode.Diagnostic[] = (msg.params.diagnostics || []).map((d: any) => {
                const range = new vscode.Range(
                    d.range.start.line,
                    d.range.start.character,
                    d.range.end.line,
                    d.range.end.character
                );
                const severity = d.severity === 1
                    ? vscode.DiagnosticSeverity.Error
                    : d.severity === 2
                    ? vscode.DiagnosticSeverity.Warning
                    : d.severity === 3
                    ? vscode.DiagnosticSeverity.Information
                    : vscode.DiagnosticSeverity.Hint;
                const diag = new vscode.Diagnostic(range, d.message, severity);
                diag.code = d.code;
                diag.source = d.source || 'pilot';
                return diag;
            });
            this.diagnostics.set(uri, diags);
        }
    }

    public stop(): void {
        if (this.process) {
            try {
                this.sendNotification('exit', {});
                this.process.kill();
            } catch {
                // ignore
            }
            this.process = null;
        }
        this.pending.clear();
    }

    public dispose(): void {
        this.stop();
    }
}

export async function activate(context: vscode.ExtensionContext): Promise<void> {
    const diagnostics = vscode.languages.createDiagnosticCollection('pilot');
    const outputChannel = vscode.window.createOutputChannel('Pilot Language Server');
    const statusBar = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right, 100);
    statusBar.text = '$(sync~spin) Pilot LSP';
    statusBar.tooltip = 'Pilot Language Server (Syntax & Semantic Analyzer)';
    statusBar.command = 'pilot.restartLanguageServer';
    statusBar.show();

    const client = new PilotLspClient(context.extensionPath, diagnostics, statusBar, outputChannel);
    context.subscriptions.push(diagnostics, outputChannel, statusBar, client);

    const semanticTokensEmitter = new vscode.EventEmitter<void>();
    client.setSemanticTokensEmitter(semanticTokensEmitter);

    await client.start();

    context.subscriptions.push(
        semanticTokensEmitter,
        vscode.commands.registerCommand('pilot.restartLanguageServer', async () => {
            statusBar.text = '$(sync~spin) Pilot LSP';
            await client.start();
            semanticTokensEmitter.fire();
            vscode.window.showInformationMessage('Pilot Language Server restarted.');
        }),
        vscode.workspace.onDidOpenTextDocument((doc) => {
            if (doc.languageId === 'pilot') {
                client.notifyDidOpen(doc);
                semanticTokensEmitter.fire();
            }
        }),
        vscode.workspace.onDidChangeTextDocument((e) => {
            if (e.document.languageId === 'pilot') {
                client.notifyDidChange(e.document);
                semanticTokensEmitter.fire();
            }
        }),
        vscode.workspace.onDidCloseTextDocument((doc) => {
            if (doc.languageId === 'pilot') {
                client.notifyDidClose(doc);
            }
        }),
        vscode.window.onDidChangeActiveTextEditor((editor) => {
            if (editor && editor.document.languageId === 'pilot') {
                client.notifyDidOpen(editor.document);
                semanticTokensEmitter.fire();
            }
        })
    );

    const selector: vscode.DocumentSelector = { language: 'pilot' };
    const legend = new vscode.SemanticTokensLegend(TOKEN_TYPES, TOKEN_MODIFIERS);

    context.subscriptions.push(
        vscode.languages.registerDocumentSemanticTokensProvider(
            selector,
            {
                onDidChangeSemanticTokens: semanticTokensEmitter.event,
                async provideDocumentSemanticTokens(document: vscode.TextDocument): Promise<vscode.SemanticTokens> {
                    client.notifyDidOpen(document);
                    const res = await client.sendRequest('textDocument/semanticTokens/full', {
                        textDocument: { uri: document.uri.toString() }
                    });
                    const data = res?.data ?? [];
                    return new vscode.SemanticTokens(new Uint32Array(data));
                }
            },
            legend
        ),
        vscode.languages.registerHoverProvider(selector, {
            async provideHover(document: vscode.TextDocument, position: vscode.Position): Promise<vscode.Hover | null> {
                client.notifyDidOpen(document);
                const res = await client.sendRequest('textDocument/hover', {
                    textDocument: { uri: document.uri.toString() },
                    position: { line: position.line, character: position.character }
                });
                if (!res || !res.contents) {
                    return null;
                }
                let hoverStr = '';
                if (typeof res.contents === 'string') {
                    hoverStr = res.contents;
                } else if (res.contents && typeof res.contents.value === 'string') {
                    hoverStr = res.contents.value;
                } else if (Array.isArray(res.contents)) {
                    hoverStr = res.contents
                        .map((c: any) => (typeof c === 'string' ? c : (c?.value ?? '')))
                        .filter(Boolean)
                        .join('\n\n');
                }
                if (!hoverStr.trim()) {
                    return null;
                }
                const md = new vscode.MarkdownString(hoverStr);
                md.isTrusted = true;
                md.supportHtml = true;
                if (res.range) {
                    const range = new vscode.Range(
                        res.range.start.line,
                        res.range.start.character,
                        res.range.end.line,
                        res.range.end.character
                    );
                    return new vscode.Hover(md, range);
                }
                return new vscode.Hover(md);
            }
        }),
        vscode.languages.registerDefinitionProvider(selector, {
            async provideDefinition(document: vscode.TextDocument, position: vscode.Position): Promise<vscode.Location | null> {
                client.notifyDidOpen(document);
                const res = await client.sendRequest('textDocument/definition', {
                    textDocument: { uri: document.uri.toString() },
                    position: { line: position.line, character: position.character }
                });
                if (!res || !res.uri || !res.range) {
                    return null;
                }
                const range = new vscode.Range(
                    res.range.start.line,
                    res.range.start.character,
                    res.range.end.line,
                    res.range.end.character
                );
                return new vscode.Location(vscode.Uri.parse(res.uri), range);
            }
        }),
        vscode.languages.registerDocumentSymbolProvider(selector, {
            async provideDocumentSymbols(document: vscode.TextDocument): Promise<vscode.DocumentSymbol[]> {
                client.notifyDidOpen(document);
                const res = await client.sendRequest('textDocument/documentSymbol', {
                    textDocument: { uri: document.uri.toString() }
                });
                if (!Array.isArray(res)) {
                    return [];
                }
                const convert = (item: any): vscode.DocumentSymbol => {
                    const range = new vscode.Range(
                        item.range.start.line,
                        item.range.start.character,
                        item.range.end.line,
                        item.range.end.character
                    );
                    const selRange = new vscode.Range(
                        item.selectionRange.start.line,
                        item.selectionRange.start.character,
                        item.selectionRange.end.line,
                        item.selectionRange.end.character
                    );
                    const sym = new vscode.DocumentSymbol(
                        item.name,
                        item.detail || '',
                        (item.kind || 13) - 1,
                        range,
                        selRange
                    );
                    if (Array.isArray(item.children)) {
                        sym.children = item.children.map(convert);
                    }
                    return sym;
                };
                return res.map(convert);
            }
        }),
        vscode.languages.registerCompletionItemProvider(
            selector,
            {
                async provideCompletionItems(document: vscode.TextDocument, position: vscode.Position): Promise<vscode.CompletionItem[]> {
                    client.notifyDidOpen(document);
                    const res = await client.sendRequest('textDocument/completion', {
                        textDocument: { uri: document.uri.toString() },
                        position: { line: position.line, character: position.character }
                    });
                    if (!Array.isArray(res)) {
                        return [];
                    }
                    return res.map((item: any) => {
                        const ci = new vscode.CompletionItem(item.label, (item.kind || 6) - 1);
                        ci.detail = item.detail;
                        if (typeof item.documentation === 'string') {
                            ci.documentation = new vscode.MarkdownString(item.documentation);
                        } else if (item.documentation?.value) {
                            ci.documentation = new vscode.MarkdownString(item.documentation.value);
                        }
                        return ci;
                    });
                }
            },
            '.', ':', '@', '<'
        ),
        vscode.languages.registerSignatureHelpProvider(
            selector,
            {
                async provideSignatureHelp(document: vscode.TextDocument, position: vscode.Position): Promise<vscode.SignatureHelp | null> {
                    client.notifyDidOpen(document);
                    const res = await client.sendRequest('textDocument/signatureHelp', {
                        textDocument: { uri: document.uri.toString() },
                        position: { line: position.line, character: position.character }
                    });
                    if (!res || !Array.isArray(res.signatures) || res.signatures.length === 0) {
                        return null;
                    }
                    const sigHelp = new vscode.SignatureHelp();
                    sigHelp.activeSignature = res.activeSignature ?? 0;
                    sigHelp.activeParameter = res.activeParameter ?? 0;
                    sigHelp.signatures = res.signatures.map((s: any) => {
                        const si = new vscode.SignatureInformation(s.label, s.documentation ? new vscode.MarkdownString(s.documentation) : undefined);
                        if (Array.isArray(s.parameters)) {
                            si.parameters = s.parameters.map((p: any) => new vscode.ParameterInformation(p.label));
                        }
                        return si;
                    });
                    return sigHelp;
                }
            },
            '(', ','
        ),
        vscode.languages.registerInlayHintsProvider(
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
    );

    // Register Debug Provider & Adapter Factory
    const debugProvider = new PilotDebugConfigurationProvider();
    const debugFactory = new PilotDebugAdapterDescriptorFactory(context.extensionPath);
    context.subscriptions.push(
        vscode.debug.registerDebugConfigurationProvider('pilot', debugProvider),
        vscode.debug.registerDebugAdapterDescriptorFactory('pilot', debugFactory)
    );

    // Register Execution Commands
    context.subscriptions.push(
        vscode.commands.registerCommand('pilot.runCurrentFile', async (uri?: vscode.Uri) => {
            let filePath = uri?.fsPath;
            if (!filePath && vscode.window.activeTextEditor) {
                filePath = vscode.window.activeTextEditor.document.fileName;
            }
            if (!filePath) {
                vscode.window.showErrorMessage('No active Pilot file to run.');
                return;
            }
            const pilotCmd = resolvePilotExecutionPrefix(context.extensionPath);
            let terminal = vscode.window.terminals.find(t => t.name === 'Pilot Runner') || vscode.window.createTerminal('Pilot Runner');
            terminal.show();
            terminal.sendText(`${pilotCmd} run "${filePath}"`);
        }),
        vscode.commands.registerCommand('pilot.debugCurrentFile', async (uri?: vscode.Uri) => {
            let filePath = uri?.fsPath;
            if (!filePath && vscode.window.activeTextEditor) {
                filePath = vscode.window.activeTextEditor.document.fileName;
            }
            if (!filePath) {
                vscode.window.showErrorMessage('No active Pilot file to debug.');
                return;
            }
            if (vscode.window.activeTextEditor && vscode.window.activeTextEditor.document.isDirty) {
                await vscode.window.activeTextEditor.document.save();
            }
            await vscode.debug.startDebugging(undefined, {
                type: 'pilot',
                name: 'Pilot: Debug Current File',
                request: 'launch',
                program: filePath,
                stopOnEntry: false
            });
        }),
        vscode.commands.registerCommand('pilot.createLaunchConfig', async () => {
            await addOrUpdateLaunchJson(true);
        })
    );

    // Auto-create launch.json if workspace has no launch.json yet
    try {
        const folders = vscode.workspace.workspaceFolders;
        if (folders && folders.length > 0) {
            const launchPath = path.join(folders[0].uri.fsPath, '.vscode', 'launch.json');
            if (!fs.existsSync(launchPath)) {
                await addOrUpdateLaunchJson(false);
            }
        }
    } catch {}

    // Setup Pilot GUI Test Explorer & Test Suite integration
    setupPilotTesting(context);
}

function setupPilotTesting(context: vscode.ExtensionContext): void {
    const testController = vscode.tests.createTestController('pilotTestController', 'Pilot Tests');
    context.subscriptions.push(testController);

    function isTestFile(uri: vscode.Uri, content?: string): boolean {
        const p = uri.fsPath.replace(/\\/g, '/');
        if (!p.endsWith('.pilot')) {
            return false;
        }
        if (p.includes('/tests/') || p.includes('/test/') || /\btest_[\w-]+\.pilot$/.test(p) || /_test\.pilot$/.test(p)) {
            return true;
        }
        if (content) {
            if (/^\s*(?:(?:pure|async|inline)\s+)?(?:def|fn)\s+test_\w+\s*\(/m.test(content) || /@test\b/.test(content)) {
                return true;
            }
        }
        return false;
    }

    function updateTestItemForUri(uri: vscode.Uri) {
        if (!uri.fsPath.endsWith('.pilot')) return;
        let content: string;
        try {
            content = fs.readFileSync(uri.fsPath, 'utf8');
        } catch {
            return;
        }

        if (!isTestFile(uri, content)) {
            testController.items.delete(uri.toString());
            return;
        }

        const fileId = uri.toString();
        const baseName = path.basename(uri.fsPath);
        let fileItem = testController.items.get(fileId);
        if (!fileItem) {
            fileItem = testController.createTestItem(fileId, baseName, uri);
            testController.items.add(fileItem);
        }

        // Set range for file item at line 0
        fileItem.range = new vscode.Range(new vscode.Position(0, 0), new vscode.Position(0, 0));

        // Extract suite description if present
        const suiteMatch = content.match(/^(?:#|\/\/)\s*(?:Test\s+)?(Suite\s+\d+:[^\r\n]+)/m);
        if (suiteMatch) {
            fileItem.description = suiteMatch[1].trim();
        } else {
            const headingMatch = content.match(/^(?:#|\/\/)\s*={3,}\s*\r?\n(?:#|\/\/)\s*([^\r\n=]+)/m);
            if (headingMatch) {
                fileItem.description = headingMatch[1].trim();
            } else {
                fileItem.description = undefined;
            }
        }

        // Parse child tests (test functions or test sections)
        const lines = content.split(/\r?\n/);
        const fnRegex = /^\s*(?:(?:pure|async|inline)\s+)?(?:def|fn)\s+(test_\w+)\s*\(/;
        const childItems: vscode.TestItem[] = [];

        for (let i = 0; i < lines.length; i++) {
            const line = lines[i];
            const m = line.match(fnRegex);
            if (m) {
                const fnName = m[1];
                const col = line.indexOf(fnName);
                const childId = `${fileId}::${fnName}`;
                const childItem = testController.createTestItem(childId, fnName, uri);
                childItem.range = new vscode.Range(new vscode.Position(i, col), new vscode.Position(i, col + fnName.length));
                childItems.push(childItem);
            }
        }

        if (childItems.length === 0) {
            // Also recognize section headers like "# 1. Primitive & Compound Types" as nested runnable checkpoints
            const sectionRegex = /^(?:#|\/\/)\s*(?:---|===)?\s*(\d+\.[\w\s&,\(\)\-]+?)(?:\s*(?:---|===)|$)/;
            for (let i = 0; i < lines.length; i++) {
                const line = lines[i];
                const m = line.match(sectionRegex);
                if (m) {
                    const secName = m[1].trim();
                    const childId = `${fileId}::sec_${i}`;
                    const childItem = testController.createTestItem(childId, secName, uri);
                    childItem.range = new vscode.Range(new vscode.Position(i, 0), new vscode.Position(i, line.length));
                    childItems.push(childItem);
                }
            }
        }

        fileItem.children.replace(childItems);
        fileItem.canResolveChildren = false;
    }

    async function discoverAllTests() {
        const uris = await vscode.workspace.findFiles('**/*.pilot', '**/target/**');
        for (const uri of uris) {
            updateTestItemForUri(uri);
        }
    }

    testController.resolveHandler = async (item?: vscode.TestItem) => {
        if (!item) {
            await discoverAllTests();
        } else if (item.uri) {
            updateTestItemForUri(item.uri);
        }
    };

    testController.refreshHandler = async () => {
        await discoverAllTests();
    };

    // Watch workspace for .pilot files
    const watcher = vscode.workspace.createFileSystemWatcher('**/*.pilot');
    watcher.onDidCreate(uri => updateTestItemForUri(uri));
    watcher.onDidChange(uri => updateTestItemForUri(uri));
    watcher.onDidDelete(uri => testController.items.delete(uri.toString()));
    context.subscriptions.push(watcher);

    vscode.workspace.onDidChangeTextDocument(e => {
        if (e.document.languageId === 'pilot') {
            updateTestItemForUri(e.document.uri);
        }
    });

    async function runTestExecution(request: vscode.TestRunRequest, token: vscode.CancellationToken): Promise<void> {
        const run = testController.createTestRun(request);
        const pilotBin = resolvePilotExecutionPrefix(context.extensionPath).replace(/^"|"$/g, '');

        const filesToRun = new Map<string, { fileItem: vscode.TestItem; childItems: vscode.TestItem[] }>();

        function collectItem(item: vscode.TestItem) {
            if (request.exclude && request.exclude.includes(item)) return;
            const uri = item.uri;
            if (!uri) return;

            const fileId = item.id.includes('::') ? item.id.split('::')[0] : item.id;
            const fileItem = testController.items.get(fileId) || (item.uri.toString() === item.id ? item : undefined);
            if (!fileItem) return;

            if (!filesToRun.has(fileId)) {
                filesToRun.set(fileId, { fileItem, childItems: [] });
            }
            if (item.id.includes('::')) {
                filesToRun.get(fileId)!.childItems.push(item);
            }
        }

        if (request.include && request.include.length > 0) {
            for (const item of request.include) {
                collectItem(item);
            }
        } else {
            testController.items.forEach(item => collectItem(item));
        }

        for (const [fileId, { fileItem, childItems }] of filesToRun) {
            if (token.isCancellationRequested) break;

            run.started(fileItem);
            if (childItems.length > 0) {
                childItems.forEach(c => run.started(c));
            } else {
                fileItem.children.forEach(c => run.started(c));
            }

            const filePath = fileItem.uri!.fsPath;
            const cwd = vscode.workspace.getWorkspaceFolder(fileItem.uri!)?.uri.fsPath || path.dirname(filePath);

            await new Promise<void>((resolve) => {
                const proc = cp.spawn(pilotBin, ['test', '--json', filePath], {
                    cwd,
                    env: { ...process.env, RUST_BACKTRACE: '1' }
                });

                let stdoutBuf = '';
                let stderrBuf = '';
                let testPassed = false;
                let testFailed = false;
                let durationMs = 0;
                let failMsg = '';

                proc.stdout.on('data', data => {
                    const text = data.toString();
                    stdoutBuf += text;
                    const lines = text.split(/\r?\n/);
                    for (const line of lines) {
                        if (!line.trim().startsWith('{')) continue;
                        try {
                            const ev = JSON.parse(line.trim());
                            if (ev.event === 'pass') {
                                testPassed = true;
                                durationMs = ev.duration_ms || 0;
                                run.appendOutput(`\r\n\x1b[32m✔ PASS\x1b[0m: ${fileItem.label} (${durationMs.toFixed(2)}ms, ${ev.nodes || 0} nodes, ${ev.instructions || 0} instrs)\r\n`);
                            } else if (ev.event === 'fail') {
                                testFailed = true;
                                durationMs = ev.duration_ms || 0;
                                failMsg = ev.error || 'Test assertion failure';
                                run.appendOutput(`\r\n\x1b[31m✖ FAIL\x1b[0m: ${fileItem.label} (${durationMs.toFixed(2)}ms): ${failMsg}\r\n`);
                            }
                        } catch {
                            // Non-json stdout line
                        }
                    }
                });

                proc.stderr.on('data', data => {
                    stderrBuf += data.toString();
                });

                proc.on('close', code => {
                    if (testFailed || code !== 0) {
                        const message = new vscode.TestMessage(failMsg || stderrBuf.trim() || `Test failed with exit code ${code}`);
                        run.failed(fileItem, message, durationMs);
                        if (childItems.length > 0) {
                            childItems.forEach(c => run.failed(c, message, durationMs));
                        } else {
                            fileItem.children.forEach(c => run.failed(c, message, durationMs));
                        }
                    } else {
                        run.passed(fileItem, durationMs);
                        if (childItems.length > 0) {
                            childItems.forEach(c => run.passed(c, durationMs));
                        } else {
                            fileItem.children.forEach(c => run.passed(c, durationMs));
                        }
                    }
                    resolve();
                });

                proc.on('error', err => {
                    const message = new vscode.TestMessage(`Failed to execute pilot test: ${err.message}`);
                    run.failed(fileItem, message, 0);
                    resolve();
                });
            });
        }

        run.end();
    }

    testController.createRunProfile('Run Tests', vscode.TestRunProfileKind.Run, (request, token) => {
        return runTestExecution(request, token);
    }, true);

    testController.createRunProfile('Debug Tests', vscode.TestRunProfileKind.Debug, async (request, token) => {
        let targetItem = request.include?.[0];
        if (!targetItem) {
            for (const [, item] of testController.items) {
                targetItem = item;
                break;
            }
        }
        if (!targetItem || !targetItem.uri) {
            vscode.window.showErrorMessage('No test selected to debug.');
            return;
        }

        const workspaceFolder = vscode.workspace.getWorkspaceFolder(targetItem.uri);
        await vscode.debug.startDebugging(workspaceFolder, {
            type: 'pilot',
            name: `Debug Test: ${targetItem.label}`,
            request: 'launch',
            program: targetItem.uri.fsPath,
            stopOnEntry: false
        });
    }, false);

    // Initial discovery
    discoverAllTests();

    // Register test commands
    context.subscriptions.push(
        vscode.commands.registerCommand('pilot.runAllTests', async () => {
            const request = new vscode.TestRunRequest();
            await runTestExecution(request, new vscode.CancellationTokenSource().token);
        }),
        vscode.commands.registerCommand('pilot.runCurrentTest', async (uri?: vscode.Uri) => {
            let targetUri = uri || vscode.window.activeTextEditor?.document.uri;
            if (!targetUri) {
                vscode.window.showErrorMessage('No active Pilot test file to run.');
                return;
            }
            let item = testController.items.get(targetUri.toString());
            if (!item) {
                updateTestItemForUri(targetUri);
                item = testController.items.get(targetUri.toString());
            }
            if (item) {
                const request = new vscode.TestRunRequest([item]);
                await runTestExecution(request, new vscode.CancellationTokenSource().token);
            } else {
                vscode.window.showErrorMessage('The current file is not recognized as a Pilot test file.');
            }
        }),
        vscode.commands.registerCommand('pilot.refreshTests', async () => {
            await discoverAllTests();
            vscode.window.showInformationMessage('Pilot tests refreshed in Test Explorer.');
        })
    );
}

class PilotDebugConfigurationProvider implements vscode.DebugConfigurationProvider {
    provideDebugConfigurations(folder: vscode.WorkspaceFolder | undefined): vscode.ProviderResult<vscode.DebugConfiguration[]> {
        return [
            {
                type: 'pilot',
                request: 'launch',
                name: 'Pilot: Run Current File',
                mode: 'run',
                program: '${file}',
                args: []
            },
            {
                type: 'pilot',
                request: 'launch',
                name: 'Pilot: Debug Current File',
                mode: 'debug',
                program: '${file}',
                args: []
            }
        ];
    }

    resolveDebugConfiguration(
        folder: vscode.WorkspaceFolder | undefined,
        config: vscode.DebugConfiguration,
        token?: vscode.CancellationToken
    ): vscode.ProviderResult<vscode.DebugConfiguration> {
        if (!config.type && !config.request && !config.name) {
            const editor = vscode.window.activeTextEditor;
            if (editor && editor.document.languageId === 'pilot') {
                config.type = 'pilot';
                config.name = 'Pilot: Debug Current File';
                config.request = 'launch';
                config.program = editor.document.fileName;
                config.stopOnEntry = false;
            } else {
                return null;
            }
        }

        if (!config.program || config.program === '${file}') {
            if (vscode.window.activeTextEditor) {
                config.program = vscode.window.activeTextEditor.document.fileName;
            } else {
                config.program = '${file}';
            }
        }
        if (config.stopOnEntry === undefined) {
            config.stopOnEntry = false;
        }

        return config;
    }
}

class PilotDebugAdapter implements vscode.DebugAdapter {
    private sendMessageEmitter = new vscode.EventEmitter<vscode.DebugProtocolMessage>();
    readonly onDidSendMessage: vscode.Event<vscode.DebugProtocolMessage> = this.sendMessageEmitter.event;
    private childProcess: cp.ChildProcess | null = null;
    private seq = 1;

    constructor(
        private extensionPath: string,
        private session: vscode.DebugSession
    ) {}

    handleMessage(message: any): void {
        if (message.type === 'request') {
            this.handleRequest(message);
        }
    }

    private sendResponse(request: any, body?: any): void {
        this.sendMessageEmitter.fire({
            seq: this.seq++,
            type: 'response',
            request_seq: request.seq,
            command: request.command,
            success: true,
            body
        });
    }

    private sendEvent(event: string, body?: any): void {
        this.sendMessageEmitter.fire({
            seq: this.seq++,
            type: 'event',
            event,
            body
        });
    }

    private handleRequest(request: any): void {
        switch (request.command) {
            case 'initialize': {
                this.sendResponse(request, {
                    supportsConfigurationDoneRequest: true,
                    supportsTerminateRequest: true
                });
                this.sendEvent('initialized');
                break;
            }
            case 'launch': {
                this.sendResponse(request);
                this.startProcess();
                break;
            }
            case 'configurationDone': {
                this.sendResponse(request);
                break;
            }
            case 'threads': {
                this.sendResponse(request, {
                    threads: [{ id: 1, name: 'Main Thread' }]
                });
                break;
            }
            case 'disconnect':
            case 'terminate': {
                if (this.childProcess) {
                    try {
                        this.childProcess.kill();
                    } catch {}
                    this.childProcess = null;
                }
                this.sendResponse(request);
                this.sendEvent('terminated');
                break;
            }
            default: {
                this.sendResponse(request);
                break;
            }
        }
    }

    private startProcess(): void {
        const config = this.session.configuration;
        let program = config.program || '${file}';
        if (program === '${file}' && vscode.window.activeTextEditor) {
            program = vscode.window.activeTextEditor.document.fileName;
        }

        const mode = config.mode || 'run';
        const rawArgs = Array.isArray(config.args) ? config.args : (config.args ? [config.args] : []);
        let pilotExe = resolvePilotExecutionPrefix(this.extensionPath, config.pilotPath);
        if (pilotExe.startsWith('"') && pilotExe.endsWith('"')) {
            pilotExe = pilotExe.slice(1, -1);
        }

        const cmdArgs = [mode, program, ...rawArgs];
        const cwd = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath || path.dirname(program);

        this.sendEvent('output', {
            category: 'console',
            output: `[Pilot] Running: ${pilotExe} ${cmdArgs.join(' ')}\n`
        });

        try {
            const home = process.env.HOME || '';
            const cargoPath = home ? `${home}/.cargo/bin:` : '';
            this.childProcess = cp.spawn(pilotExe, cmdArgs, {
                cwd,
                env: { ...process.env, PATH: `${cargoPath}${process.env.PATH}` }
            });

            this.childProcess.stdout?.on('data', (data) => {
                this.sendEvent('output', {
                    category: 'stdout',
                    output: data.toString()
                });
            });

            this.childProcess.stderr?.on('data', (data) => {
                this.sendEvent('output', {
                    category: 'stderr',
                    output: data.toString()
                });
            });

            this.childProcess.on('error', (err) => {
                this.sendEvent('output', {
                    category: 'stderr',
                    output: `Error launching Pilot process: ${err.message}\n`
                });
                this.sendEvent('terminated');
            });

            this.childProcess.on('close', (code) => {
                this.sendEvent('output', {
                    category: 'console',
                    output: `\n[Pilot process completed with exit code ${code ?? 0}]\n`
                });
                this.sendEvent('exited', { exitCode: code ?? 0 });
                this.sendEvent('terminated');
                this.childProcess = null;
            });
        } catch (err: any) {
            this.sendEvent('output', {
                category: 'stderr',
                output: `Failed to spawn process: ${err?.message || err}\n`
            });
            this.sendEvent('terminated');
        }
    }

    dispose(): void {
        if (this.childProcess) {
            try {
                this.childProcess.kill();
            } catch {}
            this.childProcess = null;
        }
        this.sendMessageEmitter.dispose();
    }
}

class PilotDebugAdapterDescriptorFactory implements vscode.DebugAdapterDescriptorFactory {
    private extensionPath: string;

    constructor(extensionPath: string) {
        this.extensionPath = extensionPath;
    }

    createDebugAdapterDescriptor(session: vscode.DebugSession): vscode.ProviderResult<vscode.DebugAdapterDescriptor> {
        const config = session.configuration;
        const wsRoot = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath || '';
        const repoRoot = path.resolve(this.extensionPath, '..');

        // Check if custom DAP or Pilot path is configured
        const userDap = vscode.workspace.getConfiguration('pilot').get<string>('dap.serverPath');
        if (userDap && fs.existsSync(userDap)) {
            return new vscode.DebugAdapterExecutable(userDap, []);
        }

        // 1. Look for standalone pilot-dap binary
        const dapCandidates: string[] = [
            path.join(this.extensionPath, 'server', 'pilot-dap'),
            path.join(repoRoot, 'target', 'release', 'pilot-dap'),
            path.join(repoRoot, 'target', 'debug', 'pilot-dap'),
        ];
        if (wsRoot) {
            dapCandidates.push(
                path.join(wsRoot, 'target', 'release', 'pilot-dap'),
                path.join(wsRoot, 'target', 'debug', 'pilot-dap'),
            );
        }

        const newestDap = pickNewestExistingFile(dapCandidates);
        if (newestDap) {
            return new vscode.DebugAdapterExecutable(newestDap, []);
        }

        // 2. Look for pilot binary with 'debug --dap'
        let pilotExe = resolvePilotExecutionPrefix(this.extensionPath, config.pilotPath);
        if (pilotExe.startsWith('"') && pilotExe.endsWith('"')) {
            pilotExe = pilotExe.slice(1, -1);
        }
        if (pilotExe && (pilotExe === 'pilot' || fs.existsSync(pilotExe))) {
            return new vscode.DebugAdapterExecutable(pilotExe, ['debug', '--dap']);
        }

        // Fallback to inline adapter if no native binary found
        return new vscode.DebugAdapterInlineImplementation(new PilotDebugAdapter(this.extensionPath, session));
    }
}

async function addOrUpdateLaunchJson(showNotice = true): Promise<void> {
    const folders = vscode.workspace.workspaceFolders;
    if (!folders || folders.length === 0) {
        if (showNotice) {
            vscode.window.showWarningMessage('Open a workspace folder to configure launch.json.');
        }
        return;
    }

    const vscodeDir = path.join(folders[0].uri.fsPath, '.vscode');
    const launchJsonPath = path.join(vscodeDir, 'launch.json');

    const defaultPilotConfigs = [
        {
            type: 'pilot',
            request: 'launch',
            name: 'Pilot: Run Current File',
            mode: 'run',
            program: '${file}',
            args: []
        },
        {
            type: 'pilot',
            request: 'launch',
            name: 'Pilot: Run Current File (VM)',
            mode: 'run',
            program: '${file}',
            args: ['--vm']
        },
        {
            type: 'pilot',
            request: 'launch',
            name: 'Pilot: Debug Current File',
            mode: 'debug',
            program: '${file}',
            args: []
        }
    ];

    if (!fs.existsSync(vscodeDir)) {
        fs.mkdirSync(vscodeDir, { recursive: true });
    }

    if (!fs.existsSync(launchJsonPath)) {
        const content = {
            version: '0.2.0',
            configurations: defaultPilotConfigs
        };
        fs.writeFileSync(launchJsonPath, JSON.stringify(content, null, 4), 'utf8');
        if (showNotice) {
            vscode.window.showInformationMessage('Created .vscode/launch.json with Pilot Run & Debug configurations!');
            const doc = await vscode.workspace.openTextDocument(launchJsonPath);
            await vscode.window.showTextDocument(doc);
        }
    } else {
        try {
            const raw = fs.readFileSync(launchJsonPath, 'utf8');
            const data = JSON.parse(raw);
            data.configurations = data.configurations || [];
            let addedCount = 0;
            for (const cfg of defaultPilotConfigs) {
                if (!data.configurations.some((c: any) => c.name === cfg.name || (c.type === 'pilot' && c.mode === cfg.mode))) {
                    data.configurations.push(cfg);
                    addedCount++;
                }
            }
            if (addedCount > 0) {
                fs.writeFileSync(launchJsonPath, JSON.stringify(data, null, 4), 'utf8');
                if (showNotice) {
                    vscode.window.showInformationMessage(`Added Pilot configuration(s) to .vscode/launch.json!`);
                    const doc = await vscode.workspace.openTextDocument(launchJsonPath);
                    await vscode.window.showTextDocument(doc);
                }
            } else if (showNotice) {
                vscode.window.showInformationMessage('Pilot configurations are already present in .vscode/launch.json.');
                const doc = await vscode.workspace.openTextDocument(launchJsonPath);
                await vscode.window.showTextDocument(doc);
            }
        } catch (err: any) {
            if (showNotice) {
                vscode.window.showErrorMessage(`Failed to update launch.json: ${err.message}`);
            }
        }
    }
}

export function deactivate(): void {}
