const vscode = require('vscode')
const cp = require('child_process')
const {clearInterval} = require('timers')

class Handler {
    constructor() {
        /** @type {vscode.Uri?} */ this.sourceUriOfActivePanel = null
        /** @type {Map<vscode.Uri, vscode.WebviewPanel>} */ this.panels = new Map()

        /** @type {vscode.StatusBarItem} */ this.testRunningStatusBarItem =
            vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right)
        this.testRunningStatusBarItem.text = "$(loading~spin) Running"
        this.testRunningStatusBarItem.backgroundColor =
            new vscode.ThemeColor('statusBarItem.warningBackground')
        this.testRunningStatusBarItem.tooltip =
            "test-helper rebuilds crates if necessary, so it may take some time."
        this.testRunningStatusBarItem.command =
            "Typst.test-helper.showTestProgress"
        /** @type {string|undefined} */ this.testRunningLatestMessage = undefined
        this.testRunningProgressShown = false

        Handler.enableRunTestButton_(true)
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

    /** @param {boolean} enable */
    static enableRunTestButton_(enable) {
        // Need to flip the value here, i.e. "disableRunTestButton" rather than
        // "enableRunTestButton", because default values of custom context keys
        // before extension activation are falsy. The extension isn't activated
        // until a button is clicked.
        //
        // Note: at the time of this writing, VSCode doesn't support activating
        // on path patterns. Alternatively one may try activating the extension
        // using the activation event "onLanguage:typst", but this idea in fact
        // doesn't work perperly as we would like, since (a) we do not want the
        // extension to be enabled on every Typst file, e.g. the thesis you are
        // working on, and (b) VSCode does not know the language ID "typst" out
        // of box.
        vscode.commands.executeCommand(
            "setContext", "Typst.test-helper.disableRunTestButton", !enable)
    }

    /**
     * @param {vscode.Uri} uri
     * @param {string} stdout
     * @param {string} stderr
     */
    refreshTestPreviewImpl_(uri, stdout, stderr) {
        const {png, ref} = Handler.getImageUris(uri)

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
            {enableFindWidget: true},
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

    showTestProgress() {
        if (this.testRunningLatestMessage === undefined
            || this.testRunningProgressShown) {
            return
        }
        this.testRunningProgressShown = true
        vscode.window.withProgress({
            location: vscode.ProgressLocation.Notification,
            title: "test-helper",
        }, (progress) => {
            /** @type {!Promise<void>} */
            const closeWhenResolved = new Promise((resolve) => {
                // This progress bar intends to relieve the developer's doubt
                // during the possibly long waiting because of the rebuilding
                // phase before actually running the test. Therefore, a naive
                // polling (updates every few millisec) should be sufficient.
                const timerId = setInterval(() => {
                    if (this.testRunningLatestMessage === undefined) {
                        this.testRunningProgressShown = false
                        clearInterval(timerId)
                        resolve()
                    }
                    progress.report({message: this.testRunningLatestMessage})
                }, 100)
            })

            return closeWhenResolved
        })
    }

    /** @param {vscode.Uri} uri */
    runTest(uri) {
        const components = uri.fsPath.split(/tests[\/\\]/)
        const [dir, subPath] = components

        Handler.enableRunTestButton_(false)
        this.testRunningStatusBarItem.show()

        const proc = cp.spawn(
            "cargo",
            ["test", "--manifest-path", `${dir}/Cargo.toml`, "--workspace", "--test", "tests", "--", `${subPath}`])
        let outs = {stdout: "", stderr: ""}
        if (!proc.stdout || !proc.stderr) {
            throw new Error('Child process was not spawned successfully.')
        }
        proc.stdout.setEncoding('utf8')
        proc.stdout.on("data", (data) => {
            outs.stdout += data.toString()
        })
        proc.stderr.setEncoding('utf8')
        proc.stderr.on("data", (data) => {
            let s = data.toString()
            outs.stderr += s
            s = s.replace(/\(.+?\)/, "")
            this.testRunningLatestMessage = s.length > 50 ? (s.slice(0, 50) + "...") : s
        })
        proc.on("close", (exitCode) => {
            Handler.enableRunTestButton_(true)
            this.testRunningStatusBarItem.hide()
            this.testRunningLatestMessage = undefined
            console.log(`Ran tests ${uri.fsPath}, exit = ${exitCode}`)
            this.refreshTestPreviewImpl_(uri, outs.stdout, outs.stderr)
        })
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
        const {png, ref} = Handler.getImageUris(uri)

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
        const {png, ref} = Handler.getImageUris(uri)
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
    const handler = new Handler()
    context.subscriptions.push(handler.testRunningStatusBarItem)

    context.subscriptions.push(vscode.commands.registerCommand(
        "Typst.test-helper.showTestProgress", () => {
            handler.showTestProgress()
        }))
    context.subscriptions.push(vscode.commands.registerCommand(
        "Typst.test-helper.openFromSource", () => {
            handler.openTestPreview(Handler.getActiveDocumentUri())
        }))
    context.subscriptions.push(vscode.commands.registerCommand(
        "Typst.test-helper.refreshFromSource", () => {
            handler.refreshTestPreview(Handler.getActiveDocumentUri())
        }))
    context.subscriptions.push(vscode.commands.registerCommand(
        "Typst.test-helper.refreshFromPreview", () => {
            handler.refreshTestPreview(handler.getSourceUriOfActivePanel())
        }))
    context.subscriptions.push(vscode.commands.registerCommand(
        "Typst.test-helper.runFromSource", () => {
            handler.runTest(Handler.getActiveDocumentUri())
        }))
    context.subscriptions.push(vscode.commands.registerCommand(
        "Typst.test-helper.runFromPreview", () => {
            handler.runTest(handler.getSourceUriOfActivePanel())
        }))
    context.subscriptions.push(vscode.commands.registerCommand(
        "Typst.test-helper.updateFromSource", () => {
            handler.updateTestReference(Handler.getActiveDocumentUri())
        }))
    context.subscriptions.push(vscode.commands.registerCommand(
        "Typst.test-helper.updateFromPreview", () => {
            handler.updateTestReference(handler.getSourceUriOfActivePanel())
        }))
    context.subscriptions.push(vscode.commands.registerCommand(
        // The context menu (the "right-click menu") in the preview tab.
        "Typst.test-helper.copyImageFilePathFromPreviewContext", (e) => {
            handler.copyFilePathToClipboard(
                handler.getSourceUriOfActivePanel(), e.webviewSection)
        }))
}

/**
 * @param {{png: vscode.Uri, ref: vscode.Uri}} webViewSrcs
 * @param {string} stdout
 * @param {string} stderr
 * @returns {string}
 */
function getWebviewContent(webViewSrcs, stdout, stderr) {
    const escape = (/**@type{string}*/text) => text.replace(/</g, "&lt;").replace(/>/g, "&gt;")
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
