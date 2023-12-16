const vscode = require('vscode')
const cp = require('child_process')

/**
 * @param {vscode.ExtensionContext} context
 */
function activate(context) {
    let /** @type {vscode.WebviewPanel?} */ panel = null

    function refreshPanel(stdout, stderr) {
        const uri = getActiveDocumentUri()
        const { pngPath, refPath } = getPaths(uri)

        if (panel && panel.visible) {
            console.log('Refreshing WebView')
            const pngSrc = panel.webview.asWebviewUri(pngPath)
            const refSrc = panel.webview.asWebviewUri(refPath)
            panel.webview.html = ''

            // Make refresh notable.
            setTimeout(() => {
                if (!panel) {
                    throw new Error('state.panel is falsy')
                }
                panel.title = getTestOutputTabTitle(uri)
                panel.webview.html = getWebviewContent(pngSrc, refSrc, stdout, stderr)
            }, 50)
        }
    }

    const openCmd = vscode.commands.registerCommand("ShortcutMenuBar.testOpen", () => {
        panel = vscode.window.createWebviewPanel(
            'testOutput',
            getTestOutputTabTitle(getActiveDocumentUri()),
            vscode.ViewColumn.Beside,
            {}
        )

        refreshPanel("", "")
    })

    const refreshCmd = vscode.commands.registerCommand("ShortcutMenuBar.testRefresh", () => {
        refreshPanel("", "")
    })

    const cmdStatusBar = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right)
    cmdStatusBar.tooltip = "Typst test-helper is running test..."
    cmdStatusBar.backgroundColor = new vscode.ThemeColor('statusBarItem.warningBackground')
    const rerunCmd = vscode.commands.registerCommand("ShortcutMenuBar.testRerun", () => {
        const uri = getActiveDocumentUri()
        const components = uri.fsPath.split(/tests[\/\\]/)
        const dir = components[0]
        const subPath = components[1]

        cmdStatusBar.text = "$(loading~spin) Running test"
        cmdStatusBar.show();
        cp.exec(
            `cargo test --manifest-path ${dir}/Cargo.toml --all --test tests -- ${subPath}`,
            (err, stdout, stderr) => {
                cmdStatusBar.hide()
                console.log('Ran tests')
                refreshPanel(stdout, stderr)
            }
        )
    })

    const updateCmd = vscode.commands.registerCommand("ShortcutMenuBar.testUpdate", () => {
        const uri = getActiveDocumentUri()
        const { pngPath, refPath } = getPaths(uri)

        vscode.window
            .showInformationMessage(
                "Are you sure you want to update the reference image for "
                + `${uri.path.split('/').pop()}?`,
                'Yes', 'Cancel')
            .then(answer => {
                if (answer === 'Yes') {
                    vscode.workspace.fs.copy(pngPath, refPath, { overwrite: true })
                        .then(() => {
                            console.log('Copied to reference file')
                            cp.exec(`oxipng -o max -a ${refPath.fsPath}`, (err, stdout, stderr) => {
                                refreshPanel(stdout, stderr)
                            })
                        })
                }
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

    return { pngPath, refPath }
}

function getWebviewContent(pngSrc, refSrc, stdout, stderr) {
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
                <img src="${pngSrc}"/>
            </div>

            <div>
                <h1>Reference</h1>
                <img src="${refSrc}"/>
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
