const vscode = require('vscode')
const cp = require('child_process')

/**
 * @param {vscode.ExtensionContext} context
 */
function activate(context) {
    let /** @type {vscode.Uri?} */ sourceUriOfActivePanel = null
    let /** @type {Map<string, vscode.WebviewPanel>} */ panels = new Map();

    const cmdStatusBar = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right)
    cmdStatusBar.tooltip = "Typst test-helper"
    cmdStatusBar.backgroundColor = new vscode.ThemeColor('statusBarItem.warningBackground')

    /**
     * @param {vscode.Uri} uri
     * @param {string} stdout
     * @param {string} stderr
     */
    function refreshPanel(uri, stdout, stderr) {
        const { pngPath, refPath } = getPaths(uri)

        const panel = panels.get(uri.toString())
        if (panel && panel.visible) {
            console.log(`Refreshing WebView for ${uri.fsPath}`)
            const webViewSrcs = {
                png: panel.webview.asWebviewUri(pngPath),
                ref: panel.webview.asWebviewUri(refPath),
            }
            panel.webview.html = ''

            // Make refresh notable.
            setTimeout(() => {
                if (!panel) {
                    throw new Error('state.panel is falsy')
                }
                panel.title = getWebviewPanelTabTitle(uri)
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
                console.log(`Ran tests ${uri.fsPath}`)
                refreshPanel(uri, stdout, stderr)
            }
        )
    }

    const openCmd = vscode.commands.registerCommand("ShortcutMenuBar.testOpen", () => {
        const uri = getActiveDocumentUri()
        const newPanel = vscode.window.createWebviewPanel(
            'typst.TestHelperOutputPreview',
            getWebviewPanelTabTitle(uri),
            vscode.ViewColumn.Beside,
            { enableScripts: true },
        )
        newPanel.webview.onDidReceiveMessage(
            /** @param {{command: string, testUriString: string}} message */
            message => {
                console.log(JSON.stringify(message))
                switch (message.command) {
                    case 'rerunCmd':
                        rerunCmdImpl(vscode.Uri.parse(message.testUriString));
                        return;
                }
            });
        newPanel.onDidChangeViewState(() => {
            if (newPanel && newPanel.active && newPanel.visible) {
                console.log(`Set sourceUriOfActivePanel to ${uri}`)
                sourceUriOfActivePanel = uri
            } else {
                console.log(`Set sourceUriOfActivePanel to null`)
                sourceUriOfActivePanel = null
            }
        })
        newPanel.onDidDispose(() => {
            console.log(`Delete panel ${uri}`)
            panels.delete(uri.toString())
            if (sourceUriOfActivePanel === uri) {
                sourceUriOfActivePanel = null
            }
        })
        panels.set(uri.toString(), newPanel)

        refreshPanel(uri, "", "")
    })

    const refreshCmd = vscode.commands.registerCommand("ShortcutMenuBar.testRefresh", () => {
        const uri = getActiveDocumentUri()
        const panel = panels.get(uri.toString())
        if (panel) {
            if (!panel.visible) {
                panel.reveal()
            }
            refreshPanel(getActiveDocumentUri(), "", "")
        }
    })

    const rerunFromSourceCmd = vscode.commands.registerCommand(
        "ShortcutMenuBar.testRerunFromSource", () => {
            rerunCmdImpl(getActiveDocumentUri())
        })

    const rerunFromPreviewCmd = vscode.commands.registerCommand(
        "ShortcutMenuBar.testRerunFromPreview", () => {
            // The command is invoked when user clicks the button from within a WebView
            // panel, so the active panel is this panel, and sourceUriOfActivePanel will
            // be updated by that panel's onDidChangeViewState listener.
            if (!sourceUriOfActivePanel) {
                throw new Error('sourceUriOfActivePanel is falsy')
            }
            rerunCmdImpl(sourceUriOfActivePanel)
        })

    const updateCmd = vscode.commands.registerCommand("ShortcutMenuBar.testUpdate", () => {
        const uri = getActiveDocumentUri()
        const { pngPath, refPath } = getPaths(uri)

        vscode.workspace.fs.copy(pngPath, refPath, { overwrite: true })
            .then(() => {
                cp.exec(`oxipng -o max -a ${refPath.fsPath}`, (err, stdout, stderr) => {
                    console.log(`Copied to reference file for ${uri.fsPath}`)
                    refreshPanel(uri, stdout, stderr)
                })
            })
    })

    context.subscriptions.push(openCmd)
    context.subscriptions.push(refreshCmd)
    context.subscriptions.push(rerunFromSourceCmd)
    context.subscriptions.push(rerunFromPreviewCmd)
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
function getWebviewPanelTabTitle(uri) {
    return uri.path.split('/').pop()?.replace('.typ', '.png') ?? 'Test output'
}

function getPaths(uri) {
    const pngPath = vscode.Uri.file(uri.path
        .replace("tests/typ", "tests/png")
        .replace(".typ", ".png"))

    const refPath = vscode.Uri.file(uri.path
        .replace("tests/typ", "tests/ref")
        .replace(".typ", ".png"))

    return { pngPath, refPath }
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
        </style>
    </head>
    <body>
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
    </body>
    </html>
    `
}

function escape(text) {
    return text.replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

function deactivate() { }

module.exports = { activate, deactivate }
