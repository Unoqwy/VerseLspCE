import * as path from "path";
import * as fs from "fs";
import * as os from "os";

import * as vscode from "vscode";
import * as tmp from "tmp";

import {
	LanguageClient,
    LanguageClientOptions,
	RevealOutputChannelOn,
    ServerOptions,
	TransportKind
} from 'vscode-languageclient/node';

let client: LanguageClient;
let tmpServerBinaryPath: string;

function getBinaryPlatform(): string {
  const arch = os.arch();
  switch (os.type()) {
    case 'Windows_NT':
      return arch === 'x64' ? 'Win64' : 'Win32';
    case 'Linux':
      return 'Linux';
    case 'Darwin':
      return 'Mac';
    default:
      return 'Unknown';
  }
}

export function activate(context: vscode.ExtensionContext) {
    const extensionConfig = vscode.workspace.getConfiguration("verseCE");

	let serverBinaryPath = process.env.VERSE_LSP_CE_BIN
        ?? extensionConfig.get("lspBinary")
		?? path.join(context.extensionPath, "dist", "bin", `VerseLspCE-${getBinaryPlatform()}-Shipping`);
	if (os.type() === "Windows_NT" && !serverBinaryPath.endsWith(".exe")) {
		serverBinaryPath += ".exe";
	}

    const outputChannel = vscode.window.createOutputChannel("VerseLspCE");
	if (process.env.VERSE_LSP_CE_DEV_SHOW_OUTPUT === "1") {
		outputChannel.show(true);
	}

	if (!fs.existsSync(serverBinaryPath)) {
		outputChannel.appendLine(`ERROR! Could not find LSP server binary ${serverBinaryPath}.`);
		return;
	}

	if (!tmpServerBinaryPath) {
		// use a temporary file for faster development iteration
		// otherwise the server cannot be compiled because the file is in use
		tmpServerBinaryPath = tmp.tmpNameSync({
			template: "versede_lsp-XXXXXX.exe"
		});
	}
	fs.copyFileSync(serverBinaryPath, tmpServerBinaryPath);

	outputChannel.appendLine(`Using LSP server binary ${serverBinaryPath} (copied to ${tmpServerBinaryPath}).`)
    const serverOptions: ServerOptions = {
        command: tmpServerBinaryPath,
        args: [],
        transport: TransportKind.stdio,
    };

    vscode.extensions.all

	const clientOptions: LanguageClientOptions = {
		documentSelector: [{ scheme: "file", language: "verse" }],
		synchronize: {
			fileEvents: vscode.workspace.createFileSystemWatcher("**/.verse"),
		},
		outputChannel,
		revealOutputChannelOn: RevealOutputChannelOn.Error,
	};

	client = new LanguageClient(
		"verse_ce",
        "VerseLspCE",
		serverOptions,
		clientOptions,
	);
	client.registerProposedFeatures();

	client.start();
}

export function deactivate(): Thenable<void> | undefined {
    const tmpBinaryPath = tmpServerBinaryPath;
    tmpServerBinaryPath = undefined;
	if (!client) {
		serverCleanup(tmpBinaryPath);
		return;
	}
	const activeClient = client;
	client = undefined;
	return activeClient.stop()
		.then(() => serverCleanup(tmpBinaryPath));
}

function serverCleanup(tmpBinaryPath: string | undefined) {
    if (tmpBinaryPath) {
        fs.unlinkSync(tmpServerBinaryPath);
    }
}