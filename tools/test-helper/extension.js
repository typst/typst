const vscode = require('vscode')
const cp = require('child_process')

class TestHelper {
    constructor() {
        /** @type {vscode.Uri?} */ this.sourceUriOfActivePanel = null
        /** @type {Map<vscode.Uri, vscode.WebviewPanel>} */ this.panels = new Map()

        /** @type {vscode.StatusBarItem} */ this.testRunningStatusBarItem =
            vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right)
        this.testRunningStatusBarItem.text = "$(loading~spin) Running"
        this.testRunningStatusBarItem.backgroundColor =
            new vscode.ThemeColor('statusBarItem.warningBackground')
    }

    /**
     * The caller should ensure when this function is called, a text editor is active.
     * Note the fake "editor" for the extension's WebView panel is not one.
     * @returns {vscode.Uri}
     */
    static getActiveDocumentUri() {
        const editor = vscode.window.activeTextEditor
        if (!editor) {
            throw new Error('vscode.window.activeTextEditor is undefined.')
        }
        return editor.document.uri
    }

    /**
     * The caller should ensure when this function is called, a WebView panel is active.
     * @returns {vscode.Uri}
     */
    getSourceUriOfActivePanel() {
        // If this function is invoked when user clicks the button from within a WebView
        // panel, then the active panel is this panel, and sourceUriOfActivePanel is
        // guaranteed to have been updated by that panel's onDidChangeViewState listener.
        if (!this.sourceUriOfActivePanel) {
            throw new Error('sourceUriOfActivePanel is falsy; is there a focused panel?')
        }
        return this.sourceUriOfActivePanel
    }

    /** @param {vscode.Uri} uri */
    static getImageUris(uri) {
        const png = vscode.Uri.file(uri.path
            .replace("tests/typ", "tests/png")
            .replace(".typ", ".png"))

        const ref = vscode.Uri.file(uri.path
            .replace("tests/typ", "tests/ref")
            .replace(".typ", ".png"))

        return {png, ref}
    }

    /**
     * @param {vscode.Uri} uri
     * @param {string} stdout
     * @param {string} stderr
     */
    refreshTestPreviewImpl_(uri, stdout, stderr) {
        const {png, ref} = TestHelper.getImageUris(uri)

        const panel = this.panels.get(uri)
        if (panel && panel.visible) {
            console.log(`Refreshing WebView for ${uri.fsPath}`)
            const webViewSrcs = {
                png: panel.webview.asWebviewUri(png),
                ref: panel.webview.asWebviewUri(ref),
            }
            panel.webview.html = ''

            // Make refresh notable.
            setTimeout(() => {
                if (!panel) {
                    throw new Error('panel to refresh is falsy after waiting')
                }
                panel.webview.html = getWebviewContent(webViewSrcs, stdout, stderr)
            }, 50)
        }
    }

    /** @param {vscode.Uri} uri */
    openTestPreview(uri) {
        if (this.panels.has(uri)) {
            this.panels.get(uri)?.reveal()
            return
        }

        const newPanel = vscode.window.createWebviewPanel(
            'Typst.test-helper.preview',
            uri.path.split('/').pop()?.replace('.typ', '.png') ?? 'Test output',
            vscode.ViewColumn.Beside,
        )
        newPanel.onDidChangeViewState(() => {
            if (newPanel && newPanel.active && newPanel.visible) {
                console.log(`Set sourceUriOfActivePanel to ${uri}`)
                this.sourceUriOfActivePanel = uri
            } else {
                console.log(`Set sourceUriOfActivePanel to null`)
                this.sourceUriOfActivePanel = null
            }
        })
        newPanel.onDidDispose(() => {
            console.log(`Delete panel ${uri}`)
            this.panels.delete(uri)
            if (this.sourceUriOfActivePanel === uri) {
                this.sourceUriOfActivePanel = null
            }
        })
        this.panels.set(uri, newPanel)

        this.refreshTestPreviewImpl_(uri, "", "")
    }

    /** @param {vscode.Uri} uri */
    runTest(uri) {
        const components = uri.fsPath.split(/tests[\/\\]/)
        const dir = components[0]
        const subPath = components[1]

        this.testRunningStatusBarItem.show()
        cp.exec(
            `cargo test --manifest-path ${dir}/Cargo.toml --all --test tests -- ${subPath}`,
            (err, stdout, stderr) => {
                this.testRunningStatusBarItem.hide()
                console.log(`Ran tests ${uri.fsPath}`)
                this.refreshTestPreviewImpl_(uri, stdout, stderr)
            }
        )
    }

    /** @param {vscode.Uri} uri */
    refreshTestPreview(uri) {
        const panel = this.panels.get(uri)
        if (panel) {
            panel.reveal()
            this.refreshTestPreviewImpl_(uri, "", "")
        }
    }

    /** @param {vscode.Uri} uri */
    updateTestReference(uri) {
        const {png, ref} = TestHelper.getImageUris(uri)

        vscode.workspace.fs.copy(png, ref, {overwrite: true})
            .then(() => {
                cp.exec(`oxipng -o max -a ${ref.fsPath}`, (err, stdout, stderr) => {
                    console.log(`Copied to reference file for ${uri.fsPath}`)
                    this.refreshTestPreviewImpl_(uri, stdout, stderr)
                })
            })
    }

    /**
     * @param {vscode.Uri} uri
     * @param {string} webviewSection
     */
    copyFilePathToClipboard(uri, webviewSection) {
        const {png, ref} = TestHelper.getImageUris(uri)
        switch (webviewSection) {
            case 'png':
                vscode.env.clipboard.writeText(png.fsPath)
                break
            case 'ref':
                vscode.env.clipboard.writeText(ref.fsPath)
                break
            default:
                break
        }
    }
}

/** @param {vscode.ExtensionContext} context */
function activate(context) {
    const manager = new TestHelper();
    context.subscriptions.push(manager.testRunningStatusBarItem)

    context.subscriptions.push(vscode.commands.registerCommand(
        "Typst.test-helper.openFromSource", () => {
            manager.openTestPreview(TestHelper.getActiveDocumentUri())
        }))
    context.subscriptions.push(vscode.commands.registerCommand(
        "Typst.test-helper.refreshFromSource", () => {
            manager.refreshTestPreview(TestHelper.getActiveDocumentUri())
        }))
    context.subscriptions.push(vscode.commands.registerCommand(
        "Typst.test-helper.refreshFromPreview", () => {
            manager.refreshTestPreview(manager.getSourceUriOfActivePanel())
        }))
    context.subscriptions.push(vscode.commands.registerCommand(
        "Typst.test-helper.runFromSource", () => {
            manager.runTest(TestHelper.getActiveDocumentUri())
        }))
    context.subscriptions.push(vscode.commands.registerCommand(
        "Typst.test-helper.runFromPreview", () => {
            manager.runTest(manager.getSourceUriOfActivePanel())
        }))
    context.subscriptions.push(vscode.commands.registerCommand(
        "Typst.test-helper.updateFromSource", () => {
            manager.updateTestReference(TestHelper.getActiveDocumentUri())
        }))
    context.subscriptions.push(vscode.commands.registerCommand(
        "Typst.test-helper.updateFromPreview", () => {
            manager.updateTestReference(manager.getSourceUriOfActivePanel())
        }))
    // Context menu: the drop-down menu after right-click.
    context.subscriptions.push(vscode.commands.registerCommand(
        "Typst.test-helper.copyImageFilePathFromPreviewContext", (e) => {
            manager.copyFilePathToClipboard(
                manager.getSourceUriOfActivePanel(), e.webviewSection)
        }))
}

/**
 * @param {{png: vscode.Uri, ref: vscode.Uri}} webViewSrcs
 * @param {string} stdout
 * @param {string} stderr
 * @returns {string}
 */
function getWebviewContent(webViewSrcs, stdout, stderr) {
    const escape = (text) => text.replace(/</g, "&lt;").replace(/>/g, "&gt;")
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

function deactivate() {}

module.exports = {activate, deactivate}
