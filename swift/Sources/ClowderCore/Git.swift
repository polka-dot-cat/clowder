import Foundation

enum Git {
    static func output(_ path: String, _ args: [String]) async throws -> ProcessResult {
        try await ChildProcess.run("git", args, at: path)
    }
    static func read(_ path: String, _ args: [String]) async throws -> String {
        let result = try await output(path, args)
        if result.code != 0 { throw ClowderError(result.errorText) }
        return result.text
    }
    static func run(_ path: String, _ args: [String]) async throws {
        let result = try await output(path, args)
        FileHandle.standardOutput.write(result.stdout)
        FileHandle.standardError.write(result.stderr)
        if result.code != 0 { throw ClowderError("Git exited with status \(result.code)") }
    }
    static func succeeds(_ path: String, _ args: [String]) async -> Bool { (try? await output(path, args).code) == 0 }
    static func exists(_ path: String) async -> Bool {
        guard FS.present(FS.join(path, ".git")) else { return false }
        return await succeeds(path, ["rev-parse", "--verify", "HEAD"])
    }
    static func dirty(_ path: String) async -> Bool {
        guard let changes = try? await read(path, ["status", "--porcelain", "--untracked-files=no"]) else {
            return false
        }
        return !changes.isEmpty
    }
    static func validate(_ path: String, allowMissing: Bool) async throws {
        if !(await exists(path)) {
            if allowMissing { return }
            throw ClowderError("Project missing: \(path). First run clowder herd")
        }
        let changes = try await read(path, ["status", "--porcelain"])
        let directory = try await FS.join(path, read(path, ["rev-parse", "--git-dir"]))
        if !changes.isEmpty || FS.present(FS.join(directory, "rebase-merge"))
            || FS.present(FS.join(directory, "rebase-apply"))
        {
            throw ClowderError("\(path)\n\(changes)\nDirty repo. Please stash, commit, or discard your changes")
        }
        if FS.file(FS.join(path, ".gitmodules")),
            let entries = try? await read(
                path, ["config", "--file", ".gitmodules", "--get-regexp", #"^submodule\..*\.path$"#])
        {
            for entry in entries.lines {
                if let space = entry.firstIndex(of: " ") {
                    try await validate(
                        FS.join(path, String(entry[entry.index(after: space)...])), allowMissing: allowMissing)
                }
            }
        }
    }
    static func branch(_ path: String) async -> String? {
        try? await read(path, ["symbolic-ref", "--quiet", "--short", "HEAD"])
    }
    static func formattedRef(_ path: String) async throws -> String {
        guard let branch = await branch(path) else { return try await "[HEAD @ \(read(path, ["rev-parse", "HEAD"]))]" }
        var result = "[\(branch)]"
        if let counts = try? await read(path, ["rev-list", "--left-right", "--count", "HEAD...@{upstream}"]) {
            let numbers = counts.split(whereSeparator: \.isWhitespace)
            if numbers.count == 2 && numbers != ["0", "0"] { result += "(+\(numbers[0])/-\(numbers[1]))" }
        }
        return result
    }
    static func refExists(_ path: String, _ reference: String) async -> Bool {
        await succeeds(path, ["rev-parse", "--verify", "--quiet", reference])
    }
    static func defaultBranch(_ path: String, remote: String, url: String) async -> String {
        if await exists(path),
            let reference = try? await read(path, ["symbolic-ref", "--short", "refs/remotes/\(remote)/HEAD"])
        {
            return reference.removingPrefix(remote + "/")
        }
        let cwd = FS.directory(path) ? path : FileManager.default.currentDirectoryPath
        if let refs = try? await read(cwd, ["ls-remote", "--symref", url, "HEAD"]) {
            for line in refs.lines where line.hasPrefix("ref: refs/heads/") {
                return String(
                    line.removingPrefix("ref: refs/heads/").split(whereSeparator: \.isWhitespace).first ?? "master")
            }
        }
        return "master"
    }
}
