import Foundation

struct ProjectLog: Sendable {
    var text = ""
    var commandOutput: String?
    var interactive = false
    mutating func line(_ message: String) { text += message + "\n" }
    mutating func flush() throws {
        writeOut(text)
        text = ""
        if let path = commandOutput {
            commandOutput = nil
            defer { try? FS.remove(path) }
            let handle = try FileHandle(forReadingFrom: URL(fileURLWithPath: path))
            defer { try? handle.close() }
            while let data = try handle.read(upToCount: 64 * 1024), !data.isEmpty {
                FileHandle.standardOutput.write(data)
            }
        }
    }
    mutating func git(_ path: String, _ args: [String]) async throws {
        let output = try await Git.output(path, args)
        text += String(decoding: output.stdout, as: UTF8.self) + String(decoding: output.stderr, as: UTF8.self)
        if output.code != 0 { throw ClowderError("Command exited with status \(output.code)", code: output.code) }
    }
    mutating func checkout(_ project: Project, _ reference: String, check: Bool) async throws {
        let reference = reference.removingPrefix("refs/heads/").removingPrefix("refs/tags/")
        if await Git.branch(project.path) == reference {
            line(" - Branch \(reference) already checked out")
            return
        }
        line(" - Checkout branch \(reference)")
        do { try await git(project.path, ["-c", "advice.detachedHead=false", "checkout", reference]) } catch {
            if check { throw error }
            line(" - Failed to checkout")
        }
    }
    mutating func fetch(_ project: Project, _ remote: String, check: Bool) async throws {
        line(" - Fetch \(remote)")
        var args = ["fetch", "--prune", "--tags"]
        if let depth = project.git.depth { args.append("--depth=\(depth)") }
        args.append(remote)
        do { try await git(project.path, args) } catch { if check { throw error } }
    }
    mutating func lfs(_ project: Project) async throws {
        if project.git.lfs {
            line(" - Update git lfs hooks")
            try await git(project.path, ["lfs", "install", "--local"])
            line(" - Pull git lfs objects")
            try await git(project.path, ["lfs", "pull"])
        }
    }
}

enum Operations {
    static func run(_ args: Arguments, _ workspace: Workspace, _ config: Config, _ projects: [Project]) async throws {
        let command = args.command
        if ["herd", "reset", "prune", "start"].contains(command) {
            for project in projects { try await Git.validate(project.path, allowMissing: true) }
        }
        if (command == "start" && args.flag("tracking"))
            || (command == "prune" && (args.flag("all") || args.flag("remote")))
        {
            for project in projects where await Git.exists(project.path) {
                _ = try await Git.read(project.path, ["ls-remote", "--heads", project.remote])
            }
        }
        var jobs = try ["herd", "reset", "forall"].contains(command) ? config.jobs() ?? args.number("jobs") ?? 1 : 1
        if jobs <= 0 { jobs = 4 }
        let rebase = try config.rebase() ?? args.flag("rebase")
        if jobs == 1 {
            for project in projects {
                var (log, error) = await execute(args, workspace, project, rebase, interactive: true)
                try log.flush()
                if let error { throw error }
            }
        } else {
            // Bounded structured tasks; each result retains its project order.
            let results = await withTaskGroup(of: (Int, ProjectLog, ClowderError?).self) { group in
                var next = 0
                var results: [(ProjectLog, ClowderError?)?] = Array(repeating: nil, count: projects.count)
                for index in 0..<min(jobs, projects.count) {
                    next += 1
                    group.addTask {
                        let (log, error) = await execute(args, workspace, projects[index], rebase, interactive: false)
                        return (index, log, error)
                    }
                }
                while let (index, log, error) = await group.next() {
                    results[index] = (log, error)
                    if next < projects.count {
                        let index = next
                        next += 1
                        group.addTask {
                            let (log, error) = await execute(
                                args, workspace, projects[index], rebase, interactive: false)
                            return (index, log, error)
                        }
                    }
                }
                return results.compactMap { $0 }
            }
            var firstError: ClowderError?
            for (var log, error) in results {
                try log.flush()
                if firstError == nil { firstError = error }
            }
            if let firstError { throw firstError }
        }
    }
    static func execute(
        _ args: Arguments, _ workspace: Workspace, _ project: Project, _ rebase: Bool, interactive: Bool
    ) async -> (ProjectLog, ClowderError?) {
        var log = ProjectLog(interactive: interactive)
        do {
            try await perform(args, workspace, project, rebase, &log)
            return (log, nil)
        } catch {
            let code = (error as? ClowderError)?.code ?? 1
            return (log, ClowderError("Project \(project.name): \(error)", code: code))
        }
    }
    static func perform(
        _ args: Arguments, _ workspace: Workspace, _ project: Project, _ rebase: Bool, _ log: inout ProjectLog
    ) async throws {
        try ChildProcess.check()
        log.line(try await project.status())
        if args.command != "herd", !(await Git.exists(project.path)) {
            log.line(" - Project missing\n")
            return
        }
        let path = project.path
        switch args.command {
        case "herd": try await herd(args, project, rebase, &log)
        case "checkout":
            try await log.checkout(project, args.one("branch")!, check: false)
            try await log.lfs(project)
        case "forall":
            let command = args.one("command")!
            log.line(command)
            let env: [String: String?] = [
                "CLOWDER_PATH": workspace.root, "PROJECT_PATH": path, "PROJECT_NAME": project.name,
                "PROJECT_REMOTE": project.remote, "PROJECT_REF": await project.defaultRef(),
                "UPSTREAM_NAME": project.upstream?.name, "UPSTREAM_REMOTE": project.upstream?.remote,
            ]
            let output: ChildProcess.Output
            if log.interactive {
                try log.flush()
                output = .inherit
            } else {
                let file = FS.join(NSTemporaryDirectory(), "clowder-swift-forall-" + UUID().uuidString)
                log.commandOutput = file
                output = .combined(file)
            }
            let status = try await ChildProcess.run(
                "sh", ["-c", command], at: path, environment: env, output: output, interactive: log.interactive)
            if status.code != 0 && !args.flag("ignore_errors") {
                throw ClowderError("Command exited with status \(status.code)", code: status.code)
            }
        case "branch":
            if args.flag("remote") || args.flag("all") {
                try await log.fetch(project, project.remote, check: false)
                if let upstream = project.upstream { try await log.fetch(project, upstream.remote, check: false) }
            }
            if !args.flag("remote") {
                let branches = try await Git.read(path, ["for-each-ref", "--format=%(refname:short)", "refs/heads"])
                let current = await Git.branch(path)
                for branch in branches.lines { log.line(current == branch ? "* " + branch : branch) }
            }
        case "diff": try await log.git(path, ["status", "-vv"])
        case "stash":
            if await Git.dirty(path) {
                log.line(" - Stash current changes")
                try await log.git(path, ["stash"])
            } else {
                log.line(" - No changes to stash")
            }
        case "clean":
            let all = args.flag("all")
            var flags = "-f"
            if all || args.flag("untracked_directories") { flags += "d" }
            if all || args.flag("force") { flags += "f" }
            if !all && args.flag("ignored") { flags += "X" }
            if all || args.flag("untracked_files") { flags += "x" }
            log.line(" - Clean repo")
            try await log.git(path, ["clean", flags])
            log.line(" - Reset repo to HEAD")
            try await log.git(path, ["reset", "--hard", "HEAD"])
            let directory = try await FS.join(path, Git.read(path, ["rev-parse", "--git-dir"]))
            if FS.present(FS.join(directory, "rebase-merge")) || FS.present(FS.join(directory, "rebase-apply")) {
                try await log.git(path, ["rebase", "--abort"])
            }
            if args.flag("submodules") && !all { throw ClowderError("Cleaning submodules is not implemented") }
        case "reset":
            try await log.fetch(project, project.remote, check: false)
            if project.tag == nil && project.commit == nil {
                let branch = await project.defaultBranch()
                if !(await Git.refExists(path, "refs/heads/" + branch)) {
                    try await log.git(path, ["branch", "--track", branch, "HEAD"])
                    return
                }
                try await log.checkout(project, branch, check: true)
                let upstream = project.remote + "/" + branch
                if !(await Git.refExists(path, "refs/remotes/" + upstream)) {
                    throw ClowderError("No existing upstream branch \(upstream)")
                }
                log.line(" - Reset branch \(branch) to \(project.remote) \(branch)")
                log.line(" - Reset repo to \(upstream)")
                try await log.git(path, ["reset", "--hard", upstream])
            }
            try await log.lfs(project)
        case "start":
            let branch = args.one("branch")!
            if !(await Git.refExists(path, "refs/heads/" + branch)) {
                log.line(" - Create local branch \(branch)")
                try await log.git(path, ["branch", "--no-track", branch, "HEAD"])
            }
            try await log.checkout(project, branch, check: true)
            if args.flag("tracking") {
                try await log.fetch(project, project.remote, check: true)
                let remote = project.remote + "/" + branch
                if !(await Git.refExists(path, "refs/remotes/" + remote)) {
                    log.line(" - Create remote branch \(branch)")
                    try await log.git(path, ["push", project.remote, "refs/heads/\(branch):refs/heads/\(branch)"])
                    try await log.fetch(project, project.remote, check: true)
                }
                try await log.git(path, ["branch", "--set-upstream-to", remote, branch])
            }
        case "prune":
            let branch = args.one("branch")!
            let local = !args.flag("remote")
            let remote = args.flag("remote") || args.flag("all")
            if local, await Git.branch(path) == branch {
                try await log.checkout(project, project.defaultRef(), check: false)
            }
            if local, await Git.refExists(path, "refs/heads/" + branch) {
                log.line(" - Delete local branch \(branch)")
                try await log.git(path, ["branch", args.flag("force") ? "-D" : "-d", branch])
            }
            if remote, await Git.refExists(path, "refs/remotes/\(project.remote)/\(branch)") {
                log.line(" - Delete remote branch \(branch)")
                try await log.git(path, ["push", project.remote, "--delete", branch])
            }
        default: throw ClowderError("Unknown command: \(args.command)")
        }
    }
    static func herd(_ args: Arguments, _ project: Project, _ rebase: Bool, _ log: inout ProjectLog) async throws {
        let path = project.path
        let initial = await !Git.exists(project.path)
        if initial {
            if FS.present(path) {
                if try !FS.entries(path).isEmpty { throw ClowderError("Non-empty directory already exists: \(path)") }
                try FS.remove(path)
            }
            try FS.mkdir(FS.parent(path))
            var cloneBranch = project.branch
            if let branch = args.one("branch") {
                let refs = try await Git.read(
                    FS.parent(path), ["ls-remote", "--heads", project.url, "refs/heads/" + branch])
                if !refs.isEmpty { cloneBranch = branch }
            }
            var flags = ["clone", "--origin", project.remote]
            if let cloneBranch { flags += ["--branch", cloneBranch] }
            if let depth = args.number("depth").map(String.init) ?? project.git.depth.map(String.init) {
                flags += ["--depth", depth]
            }
            flags += [project.url, path]
            log.line(" - Clone repo")
            try await log.git(FS.parent(path), flags)
            if let tag = args.one("tag") {
                if await Git.refExists(path, "refs/tags/" + tag) { try await log.checkout(project, tag, check: true) }
            } else if let reference = project.tag ?? project.commit {
                try await log.checkout(project, reference, check: true)
            }
        }
        log.line(" - Update git herd alias")
        let selector = project.relative.replacingOccurrences(of: "'", with: "'\\''")
        let alias = "!f() { clowder herd \"$@\" -- '\(selector)'; }; f"
        try await log.git(path, ["config", "--local", "alias.herd", alias])
        for (key, value) in project.git.config.pairs {
            try await log.git(path, ["config", "--local", key, value.scalar])
        }
        if !initial {
            if !(await Git.succeeds(path, ["remote", "get-url", project.remote])) {
                try await log.git(path, ["remote", "add", project.remote, project.url])
            }
            try await log.fetch(project, project.remote, check: true)
            if let reference = project.tag ?? project.commit {
                try await log.checkout(project, reference, check: true)
            } else {
                try await herdBranch(project, project.defaultBranch(), create: true, rebase: rebase, &log)
            }
            if let branch = args.one("branch") {
                try await herdBranch(project, branch, create: false, rebase: rebase, &log)
            } else if let tag = args.one("tag"), await Git.refExists(path, "refs/tags/" + tag) {
                try await log.checkout(project, tag, check: true)
            }
        }
        try await log.lfs(project)
        if let upstream = project.upstream {
            log.line(upstream.name)
            if !(await Git.succeeds(path, ["remote", "get-url", upstream.remote])) {
                try await log.git(path, ["remote", "add", upstream.remote, upstream.url])
            }
            try await log.fetch(project, upstream.remote, check: true)
        }
        if project.git.recursive {
            log.line(" - Update submodules")
            var flags = ["submodule", "update", "--init", "--recursive", "--checkout"]
            if let depth = project.git.depth { flags.append("--depth=\(depth)") }
            try await log.git(path, flags)
        }
    }
    static func herdBranch(_ project: Project, _ branch: String, create: Bool, rebase: Bool, _ log: inout ProjectLog)
        async throws
    {
        let path = project.path
        let upstream = project.remote + "/" + branch
        let hasUpstream = await Git.refExists(path, "refs/remotes/" + upstream)
        if !(await Git.refExists(path, "refs/heads/" + branch)) {
            if hasUpstream {
                try await log.git(path, ["checkout", "--track", upstream])
                return
            }
            if create { try await log.git(path, ["branch", "--track", branch, "HEAD"]) }
            return
        }
        try await log.checkout(project, branch, check: create)
        if !(await Git.refExists(path, branch + "@{upstream}")) {
            try await log.git(path, ["branch", "--set-upstream-to", upstream, branch])
        }
        log.line(" - Pull\(rebase ? " with rebase" : "") from \(project.remote) \(branch)")
        var flags = ["pull", "--no-edit"]
        if rebase { flags.append("--rebase") }
        try await log.git(path, flags + [project.remote, branch])
    }
}
