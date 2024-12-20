import * as vscode from "vscode";
import * as cp from "child_process";
import { clearInterval } from "timers";

// Called when an activation event is triggered. Our activation event is the
// presence of "tests/suite/playground.typ".
export function activate(context: vscode.ExtensionContext) {
  new TestHelper(context);
}

export function deactivate() {}

class TestHelper {
  // The opened tab for a test or `undefined` if none. A non-empty "opened"
  // means there is a TestHelper editor tab present, but the content within
  // that tab (i.e. the WebView panel) might not be visible yet.
  opened?: {
    // The tests's name.
    name: string;
    // The WebView panel that displays the test images and output.
    panel: vscode.WebviewPanel;
  };

  // The current zoom scale.
  scale = 1.0;

  // The extension's status bar item.
  statusItem: vscode.StatusBarItem;

  // The active message of the status item.
  statusMessage?: string;

  // Whether the status item is currently in spinning state.
  statusSpinning = false;

  // Sets the extension up.
  constructor(private readonly context: vscode.ExtensionContext) {
    // Code lens that displays commands inline with the tests.
    this.context.subscriptions.push(
      vscode.languages.registerCodeLensProvider(
        { pattern: "**/*.typ" },
        { provideCodeLenses: (document) => this.lens(document) }
      )
    );

    // Triggered when clicking "View" in the lens.
    this.registerCommand("typst-test-helper.viewFromLens", (name) =>
      this.viewFromLens(name)
    );

    // Triggered when clicking "Run" in the lens.
    this.registerCommand("typst-test-helper.runFromLens", (name) =>
      this.runFromLens(name)
    );

    // Triggered when clicking "Save" in the lens.
    this.registerCommand("typst-test-helper.saveFromLens", (name) =>
      this.saveFromLens(name)
    );

    // Triggered when clicking "Terminal" in the lens.
    this.registerCommand("typst-test-helper.runInTerminal", (name) =>
      this.runInTerminal(name)
    );

    // Triggered when clicking the "Refresh" button in the WebView toolbar.
    this.registerCommand("typst-test-helper.refreshFromPreview", () =>
      this.refreshFromPreview()
    );

    // Triggered when clicking the "Run" button in the WebView toolbar.
    this.registerCommand("typst-test-helper.runFromPreview", () =>
      this.runFromPreview()
    );

    // Triggered when clicking the "Save" button in the WebView toolbar.
    this.registerCommand("typst-test-helper.saveFromPreview", () =>
      this.saveFromPreview()
    );

    // Triggered when clicking the "Increase Resolution" button in the WebView
    // toolbar.
    this.registerCommand("typst-test-helper.increaseResolution", () =>
      this.adjustResolution(2.0)
    );

    // Triggered when clicking the "Decrease Resolution" button in the WebView
    // toolbar.
    this.registerCommand("typst-test-helper.decreaseResolution", () =>
      this.adjustResolution(0.5)
    );

    // Triggered when performing a right-click on an image in the WebView.
    this.registerCommand(
      "typst-test-helper.copyImageFilePathFromPreviewContext",
      (e) => this.copyImageFilePathFromPreviewContext(e.webviewSection)
    );

    // Set's up the status bar item that shows a spinner while running a test.
    this.statusItem = this.createStatusItem();
    this.context.subscriptions.push(this.statusItem);

    // Triggered when clicking on the status item.
    this.registerCommand("typst-test-helper.showTestProgress", () =>
      this.showTestProgress()
    );

    this.setRunButtonEnabled(true);
  }

  // Register a command with VS Code.
  private registerCommand(id: string, callback: (...args: any[]) => any) {
    this.context.subscriptions.push(
      vscode.commands.registerCommand(id, callback)
    );
  }

  // The test lens that provides "View | Run | Save | Terminal" commands inline
  // with the test sources.
  private lens(document: vscode.TextDocument) {
    const lenses = [];
    for (let nr = 0; nr < document.lineCount; nr++) {
      const line = document.lineAt(nr);
      const re = /^--- ([\d\w-]+)( [\d\w-]+)* ---$/;
      const m = line.text.match(re);
      if (!m) {
        continue;
      }

      const name = m[1];
      lenses.push(
        new vscode.CodeLens(line.range, {
          title: "View",
          tooltip: "View the test output and reference in a new tab",
          command: "typst-test-helper.viewFromLens",
          arguments: [name],
        }),
        new vscode.CodeLens(line.range, {
          title: "Run",
          tooltip: "Run the test and view the results in a new tab",
          command: "typst-test-helper.runFromLens",
          arguments: [name],
        }),
        new vscode.CodeLens(line.range, {
          title: "Save",
          tooltip: "Run and view the test and save the reference output",
          command: "typst-test-helper.saveFromLens",
          arguments: [name],
        }),
        new vscode.CodeLens(line.range, {
          title: "Terminal",
          tooltip: "Run the test in the integrated terminal",
          command: "typst-test-helper.runInTerminal",
          arguments: [name],
        })
      );
    }
    return lenses;
  }

  // Triggered when clicking "View" in the lens.
  private viewFromLens(name: string) {
    if (this.opened?.name == name) {
      this.opened.panel.reveal();
      return;
    }

    if (this.opened) {
      this.opened.name = name;
      this.opened.panel.title = name;
    } else {
      const panel = vscode.window.createWebviewPanel(
        "typst-test-helper.preview",
        name,
        vscode.ViewColumn.Beside,
        { enableFindWidget: true }
      );

      panel.onDidDispose(() => (this.opened = undefined));

      this.opened = { name, panel };
    }

    this.refreshWebView();
  }

  // Triggered when clicking "Run" in the lens.
  private runFromLens(name: string) {
    this.viewFromLens(name);
    this.runFromPreview();
  }

  // Triggered when clicking "Run" in the lens.
  private saveFromLens(name: string) {
    this.viewFromLens(name);
    this.saveFromPreview();
  }

  // Triggered when clicking "Terminal" in the lens.
  private runInTerminal(name: string) {
    const TESTIT = "cargo test --workspace --test tests --";

    if (vscode.window.terminals.length === 0) {
      vscode.window.createTerminal();
    }

    const terminal = vscode.window.terminals[0];
    terminal.show(true);
    terminal.sendText(`${TESTIT} --exact ${name}`, true);
  }

  // Triggered when clicking the "Refresh" button in the WebView toolbar.
  private refreshFromPreview() {
    if (this.opened) {
      this.opened.panel.reveal();
      this.refreshWebView();
    }
  }

  // Triggered when clicking the "Run" button in the WebView toolbar.
  private runFromPreview() {
    if (this.opened) {
      this.runCargoTest(this.opened.name);
    }
  }

  // Triggered when clicking the "Save" button in the WebView toolbar.
  private saveFromPreview() {
    if (this.opened) {
      this.runCargoTest(this.opened.name, true);
    }
  }

  // Triggered when clicking the one of the resolution buttons in the WebView
  // toolbar.
  private adjustResolution(factor: number) {
    this.scale = Math.round(Math.min(Math.max(this.scale * factor, 1.0), 8.0));
    this.runFromPreview();
  }

  // Runs a single test with cargo, optionally with `--update`.
  private runCargoTest(name: string, update?: boolean) {
    this.setRunButtonEnabled(false);
    this.statusItem.show();

    const dir = getWorkspaceRoot().path;
    const proc = cp.spawn("cargo", [
      "test",
      "--manifest-path",
      `${dir}/Cargo.toml`,
      "--workspace",
      "--test",
      "tests",
      "--",
      ...(this.scale != 1.0 ? ["--scale", `${this.scale}`] : []),
      "--exact",
      "--verbose",
      name,
      ...(update ? ["--update"] : []),
    ]);
    let outs = { stdout: "", stderr: "" };
    if (!proc.stdout || !proc.stderr) {
      throw new Error("child process was not spawned successfully.");
    }
    proc.stdout.setEncoding("utf8");
    proc.stdout.on("data", (data) => {
      outs.stdout += data.toString();
    });
    proc.stderr.setEncoding("utf8");
    proc.stderr.on("data", (data) => {
      let s = data.toString();
      outs.stderr += s;
      s = s.replace(/\(.+?\)/, "");
      this.statusMessage = s.length > 50 ? s.slice(0, 50) + "..." : s;
    });
    proc.on("close", (exitCode) => {
      this.setRunButtonEnabled(true);
      this.statusItem.hide();
      this.statusMessage = undefined;
      console.log(`Ran test ${name}, exit = ${exitCode}`);
      setTimeout(() => {
        this.refreshWebView(outs);
      }, 50);
    });
  }

  // Triggered when performing a right-click on an image in the WebView.
  private copyImageFilePathFromPreviewContext(webviewSection: string) {
    if (!this.opened) return;
    const { name } = this.opened;
    const { png, ref } = getImageUris(name);
    switch (webviewSection) {
      case "png":
        vscode.env.clipboard.writeText(png.fsPath);
        break;
      case "ref":
        vscode.env.clipboard.writeText(ref.fsPath);
        break;
      default:
        break;
    }
  }

  // Reloads the web view.
  private refreshWebView(output?: { stdout: string; stderr: string }) {
    if (!this.opened) return;

    const { name, panel } = this.opened;
    const { png, ref } = getImageUris(name);

    if (panel) {
      console.log(
        `Refreshing WebView for ${name}` + (panel.visible ? " in background" : ""));
      const webViewSrcs = {
        png: panel.webview.asWebviewUri(png),
        ref: panel.webview.asWebviewUri(ref),
      };
      panel.webview.html = "";

      // Make refresh notable.
      setTimeout(() => {
        if (!panel) {
          throw new Error("panel to refresh is falsy after waiting");
        }
        panel.webview.html = getWebviewContent(webViewSrcs, output);
      }, 50);
    }
  }

  // Creates an item for the bottom status bar.
  private createStatusItem(): vscode.StatusBarItem {
    const item = vscode.window.createStatusBarItem(
      vscode.StatusBarAlignment.Right
    );
    item.text = "$(loading~spin) Running";
    item.backgroundColor = new vscode.ThemeColor(
      "statusBarItem.warningBackground"
    );
    item.tooltip =
      "test-helper rebuilds crates if necessary, so it may take some time.";
    item.command = "typst-test-helper.showTestProgress";
    return item;
  }

  // Triggered when clicking on the status bar item.
  private showTestProgress() {
    if (this.statusMessage === undefined || this.statusSpinning) {
      return;
    }
    this.statusSpinning = true;
    vscode.window.withProgress(
      {
        location: vscode.ProgressLocation.Notification,
        title: "test-helper",
      },
      (progress) =>
        new Promise<void>((resolve) => {
          // This progress bar intends to relieve the developer's doubt during
          // the possibly long waiting because of the rebuilding phase before
          // actually running the test. Therefore, a naive polling (updates
          // every few millisec) should be sufficient.
          const timerId = setInterval(() => {
            if (this.statusMessage === undefined) {
              this.statusSpinning = false;
              clearInterval(timerId);
              resolve();
            }
            progress.report({ message: this.statusMessage });
          }, 100);
        })
    );
  }

  // Configures whether the run and save buttons are enabled.
  private setRunButtonEnabled(enabled: boolean) {
    vscode.commands.executeCommand(
      "setContext",
      "typst-test-helper.runButtonEnabled",
      enabled
    );
  }
}

// Returns the root folder of the workspace.
function getWorkspaceRoot() {
  return vscode.workspace.workspaceFolders![0].uri;
}

// Returns the URIs for a test's images.
function getImageUris(name: string) {
  const root = getWorkspaceRoot();
  const png = vscode.Uri.joinPath(root, `tests/store/render/${name}.png`);
  const ref = vscode.Uri.joinPath(root, `tests/ref/${name}.png`);
  return { png, ref };
}

// Produces the content of the WebView.
function getWebviewContent(
  webViewSrcs: { png: vscode.Uri; ref: vscode.Uri },
  output?: {
    stdout: string;
    stderr: string;
  }
): string {
  const escape = (text: string) =>
    text.replace(/</g, "&lt;").replace(/>/g, "&gt;");

  const stdoutHtml = output?.stdout
    ? `<h1>Standard output</h1><pre>${escape(output.stdout)}</pre>`
    : "";
  const stderrHtml = output?.stderr
    ? `<h1>Standard error</h1><pre>${escape(output.stderr)}</pre>`
    : "";

  return `
  <!DOCTYPE html>
  <html lang="en">
    <head>
      <meta charset="UTF-8" />
      <meta name="viewport" content="width=device-width, initial-scale=1.0" />
      <title>Test output</title>
      <style>
        body,
        html {
          width: 100%;
          margin: 0;
          padding: 0;
          text-align: center;
        }
        img {
          position: relative;
          object-fit: contain;
          border: 1px solid black;
          max-width: 80%;
          image-rendering: pixelated;
          zoom: 2;
        }
        img[alt]:after {
          display: block;
          position: absolute;
          top: 0;
          left: 0;
          width: 100%;
          height: 100%;
          background-color: black;
          font-size: 10px;
          line-height: 1.6;
          text-align: center;
          color: #bebebe;
          content: "Not present";
        }
        pre {
          display: inline-block;
          font-family: var(--vscode-editor-font-family);
          text-align: left;
          width: 80%;
        }
        .flex {
          display: flex;
          flex-wrap: wrap;
          justify-content: center;
        }
      </style>
    </head>
    <body>
      <div
        class="flex"
        data-vscode-context='{"preventDefaultContextMenuItems": true}'
      >
        <div>
          <h1>Output</h1>
          <img
            class="output"
            data-vscode-context='{"webviewSection":"png"}'
            src="${webViewSrcs.png}"
            alt="Placeholder"
          />
        </div>

        <div>
          <h1>Reference</h1>
          <img
            class="ref"
            data-vscode-context='{"webviewSection":"ref"}'
            src="${webViewSrcs.ref}"
            alt="Placeholder"
          />
        </div>
      </div>
      ${stdoutHtml}
      ${stderrHtml}
    </body>
  </html>`;
}
