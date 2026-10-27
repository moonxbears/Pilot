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

class PilotLspClient implements vscode.Disposable {
    private process: cp.ChildProcessWithoutNullStreams | null = null;
    private nextId = 1;
    private pending = new Map<number, { resolve: (val: any) => void; reject: (err: any) => void }>();
    private buffer = Buffer.alloc(0);
    private diagnostics: vscode.DiagnosticCollection;
    private statusBar: vscode.StatusBarItem;
    private outputChannel: vscode.OutputChannel;
    private extensionPath: string;

    constructor(extensionPath: string, diagnostics: vscode.DiagnosticCollection, statusBar: vscode.StatusBarItem, outputChannel: vscode.OutputChannel) {
        this.extensionPath = extensionPath;
        this.diagnostics = diagnostics;
        this.statusBar = statusBar;
        this.outputChannel = outputChannel;
    }

    public async start(): Promise<void> {
        this.stop();

        const config = vscode.workspace.getConfiguration('pilot.languageServer');
        const dotnetPath = config.get<string>('dotnetPath') || 'dotnet';
        const customPath = config.get<string>('serverPath') || '';
        const repoRoot = path.resolve(this.extensionPath, '..');

        const bundledDll = path.join(this.extensionPath, 'server', 'Pilot.LanguageServer.dll');
        const repoDebugDll = path.join(repoRoot, 'Pilot.LanguageServer', 'bin', 'Debug', 'net10.0', 'Pilot.LanguageServer.dll');
        const repoReleaseDll = path.join(repoRoot, 'Pilot.LanguageServer', 'bin', 'Release', 'net10.0', 'Pilot.LanguageServer.dll');
        const csprojCandidate = path.join(repoRoot, 'Pilot.LanguageServer', 'Pilot.LanguageServer.csproj');

        let serverDll = '';
        let serverCsproj = '';

        if (customPath) {
            if (customPath.endsWith('.dll') && fs.existsSync(customPath)) {
                serverDll = customPath;
            } else if (customPath.endsWith('.csproj') && fs.existsSync(customPath)) {
                serverCsproj = customPath;
            }
        }

        if (!serverDll && !serverCsproj) {
            if (fs.existsSync(bundledDll)) {
                serverDll = bundledDll;
            } else if (fs.existsSync(repoDebugDll)) {
                serverDll = repoDebugDll;
            } else if (fs.existsSync(repoReleaseDll)) {
                serverDll = repoReleaseDll;
            } else if (fs.existsSync(csprojCandidate)) {
                serverCsproj = csprojCandidate;
            }
        }

        let args: string[];
        if (serverDll) {
            args = [serverDll];
            this.outputChannel.appendLine(`[Pilot LSP] Starting server from DLL: ${serverDll}`);
        } else if (serverCsproj) {
            args = ['run', '--project', serverCsproj];
            this.outputChannel.appendLine(`[Pilot LSP] Starting server from project: ${serverCsproj}`);
        } else {
            this.statusBar.text = '$(error) Pilot LSP (Not Found)';
            vscode.window.showErrorMessage(`Pilot Language Server not found. Looked in bundled '${bundledDll}' and custom setting.`);
            return;
        }

        const workingDir = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath || path.dirname(serverDll || serverCsproj);
        this.process = cp.spawn(dotnetPath, args, {
            cwd: workingDir,
            stdio: ['pipe', 'pipe', 'pipe']
        });

        this.process.stdout.on('data', (chunk: Buffer) => this.onData(chunk));
        this.process.stderr.on('data', (chunk: Buffer) => {
            this.outputChannel.appendLine(`[Pilot LSP stderr] ${chunk.toString('utf8')}`);
        });
        this.process.on('exit', (code) => {
            this.outputChannel.appendLine(`[Pilot LSP] Process exited with code ${code}`);
            this.statusBar.text = '$(warning) Pilot LSP (Stopped)';
        });

        await this.sendRequest('initialize', {
            processId: process.pid,
            rootUri: vscode.workspace.workspaceFolders?.[0]?.uri.toString() ?? null,
            capabilities: {}
        });
        this.sendNotification('initialized', {});
        this.statusBar.text = '$(check) Pilot LSP';

        for (const doc of vscode.workspace.textDocuments) {
            if (doc.languageId === 'pilot') {
                this.notifyDidOpen(doc);
            }
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
            this.pending.set(id, { resolve, reject });
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
        this.buffer = this.buffer.length === 0 ? chunk : Buffer.concat([this.buffer, chunk]);

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
                    : vscode.DiagnosticSeverity.Information;
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

    await client.start();

    context.subscriptions.push(
        vscode.commands.registerCommand('pilot.restartLanguageServer', async () => {
            statusBar.text = '$(sync~spin) Pilot LSP';
            await client.start();
            vscode.window.showInformationMessage('Pilot Language Server restarted.');
        }),
        vscode.workspace.onDidOpenTextDocument((doc) => {
            if (doc.languageId === 'pilot') {
                client.notifyDidOpen(doc);
            }
        }),
        vscode.workspace.onDidChangeTextDocument((e) => {
            if (e.document.languageId === 'pilot') {
                client.notifyDidChange(e.document);
            }
        }),
        vscode.workspace.onDidCloseTextDocument((doc) => {
            if (doc.languageId === 'pilot') {
                client.notifyDidClose(doc);
            }
        })
    );

    const selector: vscode.DocumentSelector = { language: 'pilot' };
    const legend = new vscode.SemanticTokensLegend(TOKEN_TYPES, TOKEN_MODIFIERS);

    context.subscriptions.push(
        vscode.languages.registerDocumentSemanticTokensProvider(
            selector,
            {
                async provideDocumentSemanticTokens(document: vscode.TextDocument): Promise<vscode.SemanticTokens> {
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
                const res = await client.sendRequest('textDocument/hover', {
                    textDocument: { uri: document.uri.toString() },
                    position: { line: position.line, character: position.character }
                });
                if (!res || !res.contents) {
                    return null;
                }
                const md = new vscode.MarkdownString(res.contents.value);
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
                        if (item.documentation?.value) {
                            ci.documentation = new vscode.MarkdownString(item.documentation.value);
                        }
                        return ci;
                    });
                }
            },
            '.', ':', '@', '<'
        )
    );

    // Register Debug Provider & Adapter Factory
    const debugProvider = new PilotDebugConfigurationProvider();
    const debugFactory = new PilotDebugAdapterDescriptorFactory();
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
            let terminal = vscode.window.terminals.find(t => t.name === 'Pilot Runner') || vscode.window.createTerminal('Pilot Runner');
            terminal.show();
            terminal.sendText(`pilot run "${filePath}"`);
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
            let terminal = vscode.window.terminals.find(t => t.name === 'Pilot Debugger') || vscode.window.createTerminal('Pilot Debugger');
            terminal.show();
            terminal.sendText(`pilot debug "${filePath}"`);
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
                config.name = 'Pilot: Run Current File';
                config.request = 'launch';
                config.mode = 'run';
                config.program = '${file}';
                config.args = [];
            } else {
                return null;
            }
        }

        if (!config.program) {
            config.program = '${file}';
        }
        if (!config.mode) {
            config.mode = 'run';
        }

        return config;
    }
}

class PilotDebugAdapterDescriptorFactory implements vscode.DebugAdapterDescriptorFactory {
    createDebugAdapterDescriptor(session: vscode.DebugSession): vscode.ProviderResult<vscode.DebugAdapterDescriptor> {
        const config = session.configuration;
        let program = config.program || '${file}';
        if (program === '${file}' && vscode.window.activeTextEditor) {
            program = vscode.window.activeTextEditor.document.fileName;
        }

        const mode = config.mode || 'run';
        const args = Array.isArray(config.args) ? config.args.join(' ') : (config.args || '');
        const pilotExe = config.pilotPath || 'pilot';

        const terminalName = mode === 'debug' ? 'Pilot Debugger' : 'Pilot Runner';
        let terminal = vscode.window.terminals.find(t => t.name === terminalName);
        if (!terminal) {
            terminal = vscode.window.createTerminal(terminalName);
        }
        terminal.show();

        if (mode === 'debug') {
            terminal.sendText(`${pilotExe} debug "${program}" ${args}`);
        } else {
            terminal.sendText(`${pilotExe} run "${program}" ${args}`);
        }

        return new vscode.DebugAdapterInlineImplementation({
            onDidSendMessage: new vscode.EventEmitter<any>().event,
            handleMessage: () => {},
            dispose: () => {}
        });
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
