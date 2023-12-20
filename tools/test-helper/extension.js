const vscode = require('vscode')
const cp = require('child_process')

/**
 * @param {vscode.ExtensionContext} context
 */
function activate(context) {
    let /** @type {vscode.Uri?} */ sourceUriOfActivePanel = null
    function getSourceUriOfActivePanel() {
        // If this function is invoked when user clicks the button from within a WebView
        // panel, then the active panel is this panel, and sourceUriOfActivePanel is
        // guaranteed to have been updated by that panel's onDidChangeViewState listener.
        if (!sourceUriOfActivePanel) {
            throw new Error('sourceUriOfActivePanel is falsy; is there a focused panel?')
        }
        return sourceUriOfActivePanel
    }

    let /** @type {Map<vscode.Uri, vscode.WebviewPanel>} */ panels = new Map()

    const cmdStatusBar = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right)
    cmdStatusBar.tooltip = "Typst test-helper"
    cmdStatusBar.backgroundColor = new vscode.ThemeColor('statusBarItem.warningBackground')

    /**
     * @param {vscode.Uri} uri
     * @param {string} stdout
     * @param {string} stderr
     */
    function refreshPanel(uri, stdout, stderr) {
        const { pngPath, refPath } = getImageUris(uri)

        const panel = panels.get(uri)
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
                    throw new Error('panel to refresh is falsy after waiting')
                }
                panel.title = getWebviewPanelTabTitle(uri)
                panel.webview.html = getWebviewContent(
                    uri, webViewSrcs, stdout, stderr)
            }, 50)
        }
    }

    /** @param {vscode.Uri} uri */
    function runCmdImpl(uri) {
        const components = uri.fsPath.split(/tests[\/\\]/)
        const dir = components[0]
        const subPath = components[1]

        cmdStatusBar.text = "$(loading~spin) Running"
        cmdStatusBar.show()
        cp.exec(
            `cargo test --manifest-path ${dir}/Cargo.toml --all --test tests -- ${subPath}`,
            (err, stdout, stderr) => {
                cmdStatusBar.hide()
                console.log(`Ran tests ${uri.fsPath}`)
                refreshPanel(uri, stdout, stderr)
            }
        )
    }

    /** @param {vscode.Uri} uri */
    function refreshCmdImpl(uri) {
        const panel = panels.get(uri)
        if (panel) {
            panel.reveal()
            refreshPanel(uri, "", "")
        }
    }

    /** @param {vscode.Uri} uri */
    function updateCmdImpl(uri) {
        const { pngPath, refPath } = getImageUris(uri)

        vscode.workspace.fs.copy(pngPath, refPath, { overwrite: true })
            .then(() => {
                cp.exec(`oxipng -o max -a ${refPath.fsPath}`, (err, stdout, stderr) => {
                    console.log(`Copied to reference file for ${uri.fsPath}`)
                    refreshPanel(uri, stdout, stderr)
                })
            })
    }

    const openCmd = vscode.commands.registerCommand("Typst.test-helper.open", () => {
        const uri = getActiveDocumentUri()
        if (panels.has(uri)) {
            panels.get(uri)?.reveal()
            return
        }
        const newPanel = vscode.window.createWebviewPanel(
            'Typst.test-helper.preview',
            getWebviewPanelTabTitle(uri),
            vscode.ViewColumn.Beside,
        )
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
            panels.delete(uri)
            if (sourceUriOfActivePanel === uri) {
                sourceUriOfActivePanel = null
            }
        })
        panels.set(uri, newPanel)

        refreshPanel(uri, "", "")
    })

    const refreshFromSourceCmd = vscode.commands.registerCommand(
        "Typst.test-helper.refreshFromSource", () => {
            refreshCmdImpl(getActiveDocumentUri())
        })

    const refreshFromPreviewCmd = vscode.commands.registerCommand(
        "Typst.test-helper.refreshFromPreview", () => {
            refreshCmdImpl(getSourceUriOfActivePanel())
        })

    const runFromSourceCmd = vscode.commands.registerCommand(
        "Typst.test-helper.runFromSource", () => {
            runCmdImpl(getActiveDocumentUri())
        })

    const runFromPreviewCmd = vscode.commands.registerCommand(
        "Typst.test-helper.runFromPreview", () => {
            runCmdImpl(getSourceUriOfActivePanel())
        })

    const updateFromSourceCmd = vscode.commands.registerCommand(
        "Typst.test-helper.updateFromSource", () => {
            updateCmdImpl(getActiveDocumentUri())
        })

    const updateFromPreviewCmd = vscode.commands.registerCommand(
        "Typst.test-helper.updateFromPreview", () => {
            updateCmdImpl(getSourceUriOfActivePanel())
        })

    const copyImageFilePathCmd = vscode.commands.registerCommand(
        "Typst.test-helper.copyImageFilePathFromPreviewContext", (e) => {
            const { pngPath, refPath } = getImageUris(getSourceUriOfActivePanel())
            switch (e.webviewSection) {
                case 'png':
                    vscode.env.clipboard.writeText(pngPath.fsPath)
                    break
                case 'ref':
                    vscode.env.clipboard.writeText(refPath.fsPath)
                    break
                default:
                    break
            }
        })

    context.subscriptions.push(openCmd)
    context.subscriptions.push(refreshFromSourceCmd)
    context.subscriptions.push(refreshFromPreviewCmd)
    context.subscriptions.push(runFromSourceCmd)
    context.subscriptions.push(runFromPreviewCmd)
    context.subscriptions.push(updateFromSourceCmd)
    context.subscriptions.push(updateFromPreviewCmd)
    context.subscriptions.push(copyImageFilePathCmd)

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

function getImageUris(uri) {
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
        <div class="flex" data-vscode-context='{"preventDefaultContextMenuItems": true}'>
            <div>
                <h1>Output</h1>
                <img data-vscode-context='{"webviewSection":"png"}' src="${webViewSrcs.png}"/>
            </div>

            <div>
                <h1>Reference</h1>
                <img data-vscode-context='{"webviewSection":"ref"}' src="${webViewSrcs.ref}"/>
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
    return text.replace(/</g, "&lt;").replace(/>/g, "&gt;")
}

function deactivate() { }

module.exports = { activate, deactivate }
