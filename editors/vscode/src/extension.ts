import * as path from "path";
import { ExtensionContext, workspace, window } from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
} from "vscode-languageclient/node";

let client: LanguageClient | undefined;

export function activate(context: ExtensionContext) {
  const config = workspace.getConfiguration("intentlang");
  const enabled = config.get<boolean>("server.enabled", true);
  if (!enabled) {
    return;
  }

  const serverPath = resolveServerPath(config.get<string>("server.path", ""));

  const serverOptions: ServerOptions = {
    command: serverPath,
    args: [],
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: "file", language: "intent" }],
    synchronize: {
      fileEvents: workspace.createFileSystemWatcher("**/*.intent"),
    },
  };

  client = new LanguageClient(
    "intentlang",
    "IntentLang Language Server",
    serverOptions,
    clientOptions
  );

  client.start().catch((err) => {
    const msg = err instanceof Error ? err.message : String(err);
    if (msg.includes("ENOENT") || msg.includes("not found")) {
      window.showWarningMessage(
        `IntentLang LSP server not found at '${serverPath}'. ` +
          "Install it with: cargo install intent-lsp"
      );
    } else {
      window.showErrorMessage(`IntentLang LSP failed to start: ${msg}`);
    }
  });

  context.subscriptions.push({
    dispose: () => {
      if (client) {
        client.stop();
      }
    },
  });
}

export function deactivate(): Thenable<void> | undefined {
  return client?.stop();
}

function resolveServerPath(configured: string): string {
  if (configured && configured.length > 0) {
    return configured;
  }
  return "intent-lsp";
}
