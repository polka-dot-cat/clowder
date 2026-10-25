import Foundation

enum Commands {
    static func run(_ args: Arguments) async throws {
        var workspace = Workspace.discover()
        if args.command == "init" {
            workspace = Workspace(root: FileManager.default.currentDirectoryPath, found: true)
            if FS.directory(workspace.repo) {
                guard try FS.entries(workspace.repo).isEmpty else {
                    throw ClowderError("Non-empty directory already exists: \(workspace.repo)")
                }
                try FS.remove(workspace.repo)
            }
            let url = args.one("url")!
            writeOut("Create clowder repo from \(url)\n\n")
            var options = ["clone", "--origin", "origin"]
            if let branch = args.one("branch") { options += ["--branch", branch] }
            try await Git.run(workspace.root, options + [url, ".clowder"])
            try await workspace.link(nil)
            _ = try workspace.manifest()
            return
        }
        if args.command == "link" {
            if let manifest = try? Manifest.load(workspace) { writeOut(manifest.name + "\n\n") }
            writeOut(try await workspace.status())
            return try await workspace.link(args.one("version"))
        }
        if ["repo", "config set"].contains(args.command) {
            writeOut(CLI.help(args.command))
            return
        }
        var config = try Config.load(workspace)
        if args.command.hasPrefix("repo ") { return try await repo(args, workspace) }
        let proto = try config.get("git", "protocol") ?? args.one("protocol")
        let manifest = try Manifest.load(workspace, protocol: proto)
        if args.command == "yaml" {
            writeOut(try await manifest.yaml(resolved: args.flag("resolved")))
            return
        }
        writeOut(manifest.name + "\n\n")
        if args.command.hasPrefix("config") {
            let parts = args.command.components(separatedBy: " ")
            if parts.count >= 2 {
                let clear = parts[1] == "clear"
                if parts.count == 2 {
                    writeOut(" - Clear all config values\n")
                    try config.save(workspace)
                } else {
                    let key = parts[2]
                    if key == "projects" && !clear { _ = try manifest.selected(args.many("projects")) }
                    writeOut(" - \(clear ? "Clear" : "Set") \(key) config value\n")
                    let section = ["jobs", "projects"].contains(key) ? "command" : "git"
                    let value =
                        clear
                        ? nil
                        : key == "rebase"
                            ? "True" : key == "projects" ? args.many(key).joined(separator: ", ") : args.one(key)
                    config.set(section, key, value)
                    try config.save(workspace)
                }
            }
            try Config.printCurrent(workspace)
            return
        }
        if args.command == "save" {
            _ = try await workspace.requireRepo(false)
            let version = args.one("version")!.lowercased().replacingOccurrences(of: "/", with: "-")
            if version == "default" { throw ClowderError("Version name 'default' is not allowed") }
            writeOut(try await workspace.status())
            let yaml = try await manifest.yaml(resolved: true)
            let versions = FS.join(workspace.repo, "versions")
            let path = FS.join(versions, version + ".clowder.yml")
            if FS.present(path) || FS.present(FS.join(versions, version + ".clowder.yaml")) {
                throw ClowderError("Version '\(version)' already exists")
            }
            try FS.mkdir(versions)
            writeOut(" - Save version '\(version)'\n\(path)\n")
            try FS.write(yaml, path)
            return
        }
        let projects = try manifest.selected(
            args.many("projects"), defaults: config.projects(), exclude: args.many("exclude"))
        if args.command == "status" {
            if args.flag("fetch") {
                if await Git.exists(workspace.repo) {
                    try await Git.run(workspace.repo, ["fetch", "--prune", "--tags", "origin"])
                }
                for project in projects where await Git.exists(project.path) {
                    try await Git.run(project.path, ["fetch", "--prune", "--tags", project.remote])
                    if let upstream = project.upstream {
                        try await Git.run(project.path, ["fetch", "--prune", "--tags", upstream.remote])
                    }
                }
            }
            writeOut(try await workspace.status())
            var width = 0
            for project in projects { width = await max(width, project.displayName().count) }
            for project in projects { writeOut(try await project.status(width: width) + "\n") }
            return
        }
        writeOut(try await workspace.status())
        try await Operations.run(args, workspace, config, projects)
    }
    static func repo(_ args: Arguments, _ workspace: Workspace) async throws {
        let path = try await workspace.requireRepo(true)
        if let manifest = try? Manifest.load(workspace) { writeOut(manifest.name + "\n\n") }
        writeOut(try await workspace.status())
        switch args.command {
        case "repo add": try await Git.run(path, ["add"] + args.many("files"))
        case "repo status": try await Git.run(path, ["status", "-vv"])
        case "repo checkout":
            if await Git.dirty(path) {
                writeOut(" - Dirty repo. Please stash, commit, or discard your changes\n")
                try await Git.run(path, ["status", "-vv"])
            } else {
                try await Git.run(path, ["-c", "advice.detachedHead=false", "checkout", args.one("ref")!])
            }
        case "repo commit": try await Git.run(path, ["commit", "-m", args.one("message")!])
        case "repo pull": try await Git.run(path, ["pull", "--no-edit"])
        case "repo push": try await Git.run(path, ["push"])
        case "repo clean": try await Git.run(path, ["clean", "-f"])
        case "repo run":
            let command = args.one("command")!
            writeOut(command + "\n")
            let status = try await ChildProcess.run(
                "sh", ["-c", command], at: path, output: .inherit, interactive: true)
            if status.code != 0 { throw ClowderError("Command exited with status \(status.code)", code: status.code) }
        default: throw ClowderError("Unknown command: \(args.command)")
        }
    }
}
