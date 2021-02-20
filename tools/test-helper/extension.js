const vscode = require('vscode')
const cp = require('child_process')

function activate(context) {
    let panel = null

    function refreshPanel() {
        const uri = vscode.window.activeTextEditor.document.uri
        const { pngPath, refPath } = getPaths(uri)

        if (panel && panel.visible) {
            console.log('Refreshing WebView')
            const pngSrc = panel.webview.asWebviewUri(pngPath)
            const refSrc = panel.webview.asWebviewUri(refPath)
            panel.webview.html = ''
            panel.webview.html = getWebviewContent(pngSrc, refSrc)
        }
    }

    const openCmd = vscode.commands.registerCommand("ShortcutMenuBar.openTestOutput", () => {
        panel = vscode.window.createWebviewPanel(
            'testOutput',
            'Test output',
            vscode.ViewColumn.Beside,
            {}
        )

        refreshPanel()
    })

    const refreshCmd = vscode.commands.registerCommand("ShortcutMenuBar.refreshTestOutput", () => {
        const uri = vscode.window.activeTextEditor.document.uri
        const components = uri.fsPath.split('tests')
        const dir = components[0]
        const subPath = components[1]

        cp.exec(
            `cargo test --manifest-path ${dir}/Cargo.toml --test typeset ${subPath}`,
            (err, stdout, stderr) => {
                console.log(stdout)
                console.log(stderr)
                refreshPanel()
            }
        )
    })

    const approveCmd = vscode.commands.registerCommand("ShortcutMenuBar.approveTestOutput", () => {
        const uri = vscode.window.activeTextEditor.document.uri
        const { pngPath, refPath } = getPaths(uri)

        vscode.workspace.fs.copy(pngPath, refPath, { overwrite: true }).then(() => {
            console.log('Copied to reference file')
            cp.exec(`oxipng -o max -a ${refPath.fsPath}`, (err, stdout, stderr) => {
                console.log(stdout)
                console.log(stderr)
                refreshPanel()
            })
        })
    })

    context.subscriptions.push(openCmd)
    context.subscriptions.push(refreshCmd)
    context.subscriptions.push(approveCmd)
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

function getWebviewContent(pngSrc, refSrc) {
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
            text-align: center;
            margin: 0;
            padding: 0;
        }
        img {
            width: 80%;
            max-height: 40vh;
            object-fit: contain;
        }
        </style>
    </head>
    <body>
        <h1>Output image</h1>
        <img src="${pngSrc}"/>

        <h1>Reference image</h1>
        <img src="${refSrc}"/>
    </body>
    </html>
    `
}

function deactivate() {}

module.exports = { activate, deactivate }
