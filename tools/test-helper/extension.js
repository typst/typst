const vscode = require('vscode')
const cp = require('child_process')

/**
 * @param {vscode.ExtensionContext} context
 */
function activate(context) {
    let /** @type {vscode.WebviewPanel?} */ panel = null

    const cmdStatusBar = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right)
    cmdStatusBar.tooltip = "Typst test-helper"
    cmdStatusBar.backgroundColor = new vscode.ThemeColor('statusBarItem.warningBackground')

    /**
     * @param {vscode.Uri} uri
     * @param {string} stdout
     * @param {string} stderr
     */
    function refreshPanel(uri, stdout, stderr) {
        const { pngPath, refPath, rerunSvgPath } = getPaths(uri)

        if (panel && panel.visible) {
            console.log('Refreshing WebView')
            const webViewSrcs = {
                png: panel.webview.asWebviewUri(pngPath),
                ref: panel.webview.asWebviewUri(refPath),
                rerunSvg: panel.webview.asWebviewUri(rerunSvgPath),
            }
            panel.webview.html = ''

            // Make refresh notable.
            setTimeout(() => {
                if (!panel) {
                    throw new Error('state.panel is falsy')
                }
                panel.title = getTestOutputTabTitle(uri)
                panel.webview.html = getWebviewContent(
                    uri, webViewSrcs, stdout, stderr)
            }, 50)
        }
    }

    /** @param {vscode.Uri} uri */
    function rerunCmdImpl(uri) {
        const components = uri.fsPath.split(/tests[\/\\]/)
        const dir = components[0]
        const subPath = components[1]

        cmdStatusBar.text = "$(loading~spin) Running"
        cmdStatusBar.show();
        cp.exec(
            `cargo test --manifest-path ${dir}/Cargo.toml --all --test tests -- ${subPath}`,
            (err, stdout, stderr) => {
                cmdStatusBar.hide()
                console.log('Ran tests')
                refreshPanel(uri, stdout, stderr)
            }
        )
    }

    const openCmd = vscode.commands.registerCommand("ShortcutMenuBar.testOpen", () => {
        const uri = getActiveDocumentUri()
        panel = vscode.window.createWebviewPanel(
            'testOutput',
            getTestOutputTabTitle(getActiveDocumentUri()),
            vscode.ViewColumn.Beside,
            { enableScripts: true },
        )
        panel.webview.onDidReceiveMessage(
            /** @param {{command: string, testUriString: string}} message */
            message => {
                console.log(JSON.stringify(message))
                switch (message.command) {
                    case 'rerunCmd':
                        rerunCmdImpl(vscode.Uri.parse(message.testUriString));
                        return;
                }
            });
        panel.onDidDispose(() => {
            console.log('Webview panel closed.')
            panel = null
        })

        refreshPanel(uri, "", "")
    })

    const refreshCmd = vscode.commands.registerCommand("ShortcutMenuBar.testRefresh", () => {
        if (panel) {
            if (!panel.visible) {
                panel.reveal()
            }
            refreshPanel(getActiveDocumentUri(), "", "")
        }
    })

    const rerunCmd = vscode.commands.registerCommand("ShortcutMenuBar.testRerun", () => {
        rerunCmdImpl(getActiveDocumentUri())
    })

    const updateCmd = vscode.commands.registerCommand("ShortcutMenuBar.testUpdate", () => {
        const uri = getActiveDocumentUri()
        const { pngPath, refPath } = getPaths(uri)

        vscode.workspace.fs.copy(pngPath, refPath, { overwrite: true })
            .then(() => {
                cp.exec(`oxipng -o max -a ${refPath.fsPath}`, (err, stdout, stderr) => {
                    console.log('Copied to reference file')
                    refreshPanel(uri, stdout, stderr)
                })
            })
    })

    context.subscriptions.push(openCmd)
    context.subscriptions.push(refreshCmd)
    context.subscriptions.push(rerunCmd)
    context.subscriptions.push(updateCmd)

    context.subscriptions.push(cmdStatusBar)
}

function getActiveDocumentUri() {
    const editor = vscode.window.activeTextEditor
    if (!editor) {
        throw new Error('vscode.window.activeTextEditor is undefined.')
    }
    return editor.document.uri
}

/** @param {vscode.Uri} uri */
function getTestOutputTabTitle(uri) {
    return uri.path.split('/').pop()?.replace('.typ', '.png') ?? 'Test output'
}

function getPaths(uri) {
    const pngPath = vscode.Uri.file(uri.path
        .replace("tests/typ", "tests/png")
        .replace(".typ", ".png"))

    const refPath = vscode.Uri.file(uri.path
        .replace("tests/typ", "tests/ref")
        .replace(".typ", ".png"))

    const rerunSvgPath = vscode.Uri.file(
        uri.path.replace(/tests\/typ.+/, "tools/test-helper/images/rerun-light.svg"))

    return { pngPath, refPath, rerunSvgPath }
}

function getWebviewContent(testUri, webViewSrcs, stdout, stderr) {
    return `
    <!DOCTYPE html>
    <html lang="en">
    <head>
        <meta charset="UTF-8">
        <meta name="viewport" content="width=device-width, initial-scale=1.0">
        <title>Test output</title>
        <style>
        body, html {
            width: 100%;
            margin: 0;
            padding: 0;
            text-align: center;
        }
        img {
            width: 80%;
            object-fit: contain;
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
        }
        .flex > * {
            flex-grow: 1;
            flex-shrink: 0;
            max-width: 100%;
        }
        .button {
            display: flex;
            justify-content: center;
            cursor: pointer;
        }
        </style>
    </head>
    <body>
        <button class="button" id="reRun" title="Rerun test">
            <img src="${webViewSrcs.rerunSvg}"/>
        </button>
        <div class="flex">
            <div>
                <h1>Output</h1>
                <img src="${webViewSrcs.png}"/>
            </div>

            <div>
                <h1>Reference</h1>
                <img src="${webViewSrcs.ref}"/>
            </div>
        </div>

        <h1>Standard output</h1>
        <pre>${escape(stdout)}</pre>

        <h1>Standard error</h1>
        <pre>${escape(stderr)}</pre>

        <script>
            const vscode = acquireVsCodeApi()
            document.querySelector("#reRun").addEventListener("click", () => {
                vscode.postMessage({
                    command: "rerunCmd",
                    testUriString: "${testUri.toString()}",
                })
            })
        </script>
    </body>
    </html>
    `
}

function escape(text) {
    return text.replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

function deactivate() { }

module.exports = { activate, deactivate }
