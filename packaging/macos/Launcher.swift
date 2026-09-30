// Kowalski.app: starts the kowalski server that sits next to this launcher, opens the browser
// once it answers, and stops the server when the app quits (Dock, ⌘Q, logout). Clicking the Dock
// icon opens the browser again. If kowalski is already running, the app only opens the browser.
import AppKit

// Mirrors kowalski_core::config::DEFAULT_API_BIND, the server default this app starts with.
let appURL = URL(string: "http://127.0.0.1:3456/")!
let healthURL = URL(string: "http://127.0.0.1:3456/api/health")!

final class Launcher: NSObject, NSApplicationDelegate {
    private var server: Process?
    private let logFile = FileManager.default.homeDirectoryForCurrentUser
        .appendingPathComponent("Library/Logs/Kowalski/kowalski.log")

    func applicationDidFinishLaunching(_ note: Notification) {
        buildMenu()
        if isUp() {
            openBrowser()
            return
        }
        startServer()
        waitThenOpen(tries: 60)
    }

    func applicationShouldHandleReopen(_ app: NSApplication, hasVisibleWindows: Bool) -> Bool {
        openBrowser()
        return false
    }

    func applicationWillTerminate(_ note: Notification) {
        guard let server, server.isRunning else { return }
        server.terminate()
        let deadline = Date().addingTimeInterval(5)
        while server.isRunning && Date() < deadline { usleep(100_000) }
        if server.isRunning { kill(server.processIdentifier, SIGKILL) }
    }

    func applicationDockMenu(_ sender: NSApplication) -> NSMenu? {
        let menu = NSMenu()
        menu.addItem(withTitle: "Open Kowalski", action: #selector(openBrowser), keyEquivalent: "")
        menu.addItem(withTitle: "Show Logs", action: #selector(showLogs), keyEquivalent: "")
        return menu
    }

    private func buildMenu() {
        let main = NSMenu()
        let appItem = NSMenuItem()
        main.addItem(appItem)
        let appMenu = NSMenu()
        appMenu.addItem(withTitle: "Open Kowalski", action: #selector(openBrowser), keyEquivalent: "o")
        appMenu.addItem(withTitle: "Show Logs", action: #selector(showLogs), keyEquivalent: "l")
        appMenu.addItem(.separator())
        appMenu.addItem(withTitle: "Quit Kowalski", action: #selector(NSApplication.terminate(_:)), keyEquivalent: "q")
        appItem.submenu = appMenu
        NSApp.mainMenu = main
    }

    private func startServer() {
        let fm = FileManager.default
        // Started from Finder the working directory is "/"; kowalski keeps its first-run files
        // beside its config, so run it in the config folder.
        let home = fm.homeDirectoryForCurrentUser.appendingPathComponent(".config/kowalski")
        try? fm.createDirectory(at: home, withIntermediateDirectories: true)
        try? fm.createDirectory(at: logFile.deletingLastPathComponent(), withIntermediateDirectories: true)
        if !fm.fileExists(atPath: logFile.path) { fm.createFile(atPath: logFile.path, contents: nil) }
        let log = try? FileHandle(forWritingTo: logFile)
        log?.seekToEndOfFile()

        let exe = Bundle.main.bundleURL.appendingPathComponent("Contents/MacOS/kowalski")
        let p = Process()
        p.executableURL = exe
        p.arguments = ["--no-open"]
        p.currentDirectoryURL = home
        p.standardOutput = log
        p.standardError = log
        p.terminationHandler = { proc in
            DispatchQueue.main.async {
                // A restart from Setup replaces the process in place (same pid), so only a real
                // exit lands here.
                if NSApp.isRunning, proc.terminationReason == .exit, proc.terminationStatus != 0 {
                    let alert = NSAlert()
                    alert.messageText = "Kowalski stopped"
                    alert.informativeText = "The server exited with status \(proc.terminationStatus). The log has the reason."
                    alert.addButton(withTitle: "Show Logs")
                    alert.addButton(withTitle: "Quit")
                    if alert.runModal() == .alertFirstButtonReturn { self.showLogs() }
                }
                NSApp.terminate(nil)
            }
        }
        do {
            try p.run()
            server = p
        } catch {
            let alert = NSAlert()
            alert.messageText = "Kowalski could not start"
            alert.informativeText = error.localizedDescription
            alert.runModal()
            NSApp.terminate(nil)
        }
    }

    private func waitThenOpen(tries: Int) {
        if isUp() {
            openBrowser()
            return
        }
        guard tries > 0, server?.isRunning ?? false else { return }
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) { self.waitThenOpen(tries: tries - 1) }
    }

    private func isUp() -> Bool {
        var req = URLRequest(url: healthURL)
        req.timeoutInterval = 1
        let done = DispatchSemaphore(value: 0)
        var ok = false
        URLSession.shared.dataTask(with: req) { _, resp, _ in
            ok = (resp as? HTTPURLResponse)?.statusCode == 200
            done.signal()
        }.resume()
        _ = done.wait(timeout: .now() + 2)
        return ok
    }

    @objc func openBrowser() {
        NSWorkspace.shared.open(appURL)
    }

    @objc func showLogs() {
        NSWorkspace.shared.activateFileViewerSelecting([logFile])
    }
}

let app = NSApplication.shared
let launcher = Launcher()
app.delegate = launcher
app.setActivationPolicy(.regular)
app.run()
