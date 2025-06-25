import * as vscode from "vscode";
import * as cp from "child_process";
import { clearInterval } from "timers";
const shiki = import("shiki"); // Normal import causes TypeScript problems.

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
    // The test's attributes.
    attrs: string[];
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
    this.registerCommand("typst-test-helper.viewFromLens", (name, attrs) =>
      this.viewFromLens(name, attrs)
    );

    // Triggered when clicking "Run" in the lens.
    this.registerCommand("typst-test-helper.runFromLens", (name, attrs) =>
      this.runFromLens(name, attrs)
    );

    // Triggered when clicking "Save" in the lens.
    this.registerCommand("typst-test-helper.saveFromLens", (name, attrs) =>
      this.saveFromLens(name, attrs)
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
      const re = /^--- ([\d\w-]+)(( [\d\w-]+)*) ---$/;
      const m = line.text.match(re);
      if (!m) {
        continue;
      }

      const name = m[1];
      const attrs = m[2].trim().split(" ");
      lenses.push(
        new vscode.CodeLens(line.range, {
          title: "View",
          tooltip: "View the test output and reference in a new tab",
          command: "typst-test-helper.viewFromLens",
          arguments: [name, attrs],
        }),
        new vscode.CodeLens(line.range, {
          title: "Run",
          tooltip: "Run the test and view the results in a new tab",
          command: "typst-test-helper.runFromLens",
          arguments: [name, attrs],
        }),
        new vscode.CodeLens(line.range, {
          title: "Save",
          tooltip: "Run and view the test and save the reference output",
          command: "typst-test-helper.saveFromLens",
          arguments: [name, attrs],
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
  private viewFromLens(name: string, attrs: string[]) {
    if (
      this.opened?.name == name &&
      this.opened.attrs.join(" ") == attrs.join(" ")
    ) {
      this.opened.panel.reveal();
      return;
    }

    if (this.opened) {
      this.opened.name = name;
      this.opened.attrs = attrs;
      this.opened.panel.title = name;
    } else {
      const panel = vscode.window.createWebviewPanel(
        "typst-test-helper.preview",
        name,
        vscode.ViewColumn.Beside,
        { enableFindWidget: true, enableScripts: true }
      );

      panel.onDidDispose(() => (this.opened = undefined));
      panel.webview.onDidReceiveMessage((message) => {
        if (message.command === "openFile") {
          vscode.env.openExternal(vscode.Uri.parse(message.uri));
        }
      });

      this.opened = { name, attrs, panel };
    }

    this.refreshWebView();
  }

  // Triggered when clicking "Run" in the lens.
  private runFromLens(name: string, attrs: string[]) {
    this.viewFromLens(name, attrs);
    this.runFromPreview();
  }

  // Triggered when clicking "Run" in the lens.
  private saveFromLens(name: string, attrs: string[]) {
    this.viewFromLens(name, attrs);
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
    const [bucket, format] = webviewSection.split("/");
    vscode.env.clipboard.writeText(
      getUri(name, bucket as Bucket, format as Format).fsPath
    );
  }

  // Reloads the web view.
  private refreshWebView(output?: { stdout: string; stderr: string }) {
    if (!this.opened) return;

    const { name, attrs, panel } = this.opened;

    if (panel) {
      console.log(
        `Refreshing WebView for ${name}` +
          (panel.visible ? " in background" : "")
      );

      panel.webview.html = "";

      // Make refresh notable.
      setTimeout(async () => {
        if (!panel) {
          throw new Error("panel to refresh is falsy after waiting");
        }
        panel.webview.html = await getWebviewContent(
          panel,
          name,
          attrs,
          output
        );
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

const EXTENSION = { html: "html", render: "png" };

type Bucket = "store" | "ref";
type Format = "html" | "render";

function getUri(name: string, bucket: Bucket, format: Format) {
  let path;
  if (bucket === "ref" && format === "render") {
    path = `tests/ref/${name}.png`;
  } else {
    path = `tests/${bucket}/${format}/${name}.${EXTENSION[format]}`;
  }
  return vscode.Uri.joinPath(getWorkspaceRoot(), path);
}

// Produces the content of the WebView.
async function getWebviewContent(
  panel: vscode.WebviewPanel,
  name: string,
  attrs: string[],
  output?: {
    stdout: string;
    stderr: string;
  }
): Promise<string> {
  const showHtml = attrs.includes("html");
  const showRender = !showHtml || attrs.includes("render");

  const stdout = output?.stdout
    ? `<h2>Standard output</h2><pre class="output">${escape(
        output.stdout
      )}</pre>`
    : "";
  const stderr = output?.stderr
    ? `<h2>Standard error</h2><pre class="output">${escape(
        output.stderr
      )}</pre>`
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
        h2 {
          margin-bottom: 12px;
        }
        h2 a {
          color: var(--vscode-editor-foreground);
          text-decoration: underline;
        }
        h2 a:hover {
          cursor: pointer;
        }
        .flex {
          display: flex;
          flex-wrap: wrap;
          justify-content: center;
        }
        .vertical {
          flex-direction: column;
        }
        .top-bottom {
          display: flex;
          flex-direction: column;
          padding-inline: 32px;
          width: calc(100vw - 64px);
        }
        pre {
          font-family: var(--vscode-editor-font-family);
          text-align: left;
          white-space: pre-wrap;
        }
        pre.output {
          display: inline-block;
          width: 80%;
          margin-block-start: 0;
        }
        pre.shiki {
          background-color: transparent !important;
          padding: 12px;
          margin-block-start: 0;
        }
        pre.shiki code {
          --vscode-textPreformat-background: transparent;
        }
        iframe, pre.shiki {
          border: 1px solid rgb(189, 191, 204);
          border-radius: 6px;
        }
      </style>
      <script>
        const api = acquireVsCodeApi()
        function openFile(uri) {
          api.postMessage({ command: 'openFile', uri });
        }
        function sizeIframe(obj){
          obj.style.height = 0;
          obj.style.height = obj.contentWindow.document.body.scrollHeight + 'px';
        }
      </script>
    </head>
    <body>
      ${showRender ? renderSection(panel, name) : ""}
      ${showHtml ? await htmlSection(name) : ""}
      ${stdout}
      ${stderr}
    </body>
  </html>`;
}

function renderSection(panel: vscode.WebviewPanel, name: string) {
  const outputUri = getUri(name, "store", "render");
  const refUri = getUri(name, "ref", "render");
  return `<div
    class="flex"
    data-vscode-context='{"preventDefaultContextMenuItems": true}'
  >
    <div>
      ${linkedTitle("Output", outputUri)}
      <img
        class="output"
        data-vscode-context='{"bucket":"store", format: "render"}'
        src="${panel.webview.asWebviewUri(outputUri)}"
        alt="Placeholder"
      >
    </div>

    <div>
      ${linkedTitle("Reference", refUri)}
      <img
        class="ref"
        data-vscode-context='{"bucket":"ref", format: "render"}'
        src="${panel.webview.asWebviewUri(refUri)}"
        alt="Placeholder"
      >
    </div>
  </div>`;
}

async function htmlSection(name: string) {
  const storeHtml = await htmlSnippet(
    "HTML Output",
    getUri(name, "store", "html")
  );
  const refHtml = await htmlSnippet(
    "HTML Reference",
    getUri(name, "ref", "html")
  );
  return `<div
    class="flex vertical"
    data-vscode-context='{"preventDefaultContextMenuItems": true}'
  >
    ${storeHtml}
    ${refHtml}
  </div>`;
}

async function htmlSnippet(title: string, uri: vscode.Uri): Promise<string> {
  try {
    const data = await vscode.workspace.fs.readFile(uri);
    const code = new TextDecoder("utf-8").decode(data);
    return `<div>
      ${linkedTitle(title, uri)}
      <div class="top-bottom">
        ${await highlight(code)}
        <iframe srcdoc="${escape(code)}"></iframe>
      </div>
    </div>`;
  } catch {
    return `<div><h2>${title}</h2>Not present</div>`;
  }
}

function linkedTitle(title: string, uri: vscode.Uri) {
  return `<h2><a onclick="openFile('${uri.toString()}')">${title}</a></h2>`;
}

async function highlight(code: string): Promise<string> {
  return (await shiki).codeToHtml(code, {
    lang: "html",
    theme: selectTheme(),
  });
}

function selectTheme() {
  switch (vscode.window.activeColorTheme.kind) {
    case vscode.ColorThemeKind.Light:
    case vscode.ColorThemeKind.HighContrastLight:
      return "github-light";
    case vscode.ColorThemeKind.Dark:
    case vscode.ColorThemeKind.HighContrast:
      return "github-dark";
  }
}

function escape(text: string) {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&apos;");
}
