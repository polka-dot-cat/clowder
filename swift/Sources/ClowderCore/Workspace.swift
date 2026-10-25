import Foundation

#if canImport(Darwin)
    import Darwin
#else
    import Glibc
#endif

enum FS {
    static func join(_ base: String, _ child: String) -> String {
        if child.hasPrefix("/") { return child }
        if child.isEmpty { return base }
        return base.hasSuffix("/") ? base + child : base + "/" + child
    }
    static func parent(_ path: String) -> String { (path as NSString).deletingLastPathComponent }
    static func name(_ path: String) -> String { (path as NSString).lastPathComponent }
    static func present(_ path: String) -> Bool {
        var s = stat()
        return lstat(path, &s) == 0
    }
    static func symlink(_ path: String) -> Bool {
        var s = stat()
        return lstat(path, &s) == 0 && s.st_mode & S_IFMT == S_IFLNK
    }
    static func directory(_ path: String) -> Bool {
        var directory: ObjCBool = false
        return FileManager.default.fileExists(atPath: path, isDirectory: &directory) && directory.boolValue
    }
    static func file(_ path: String) -> Bool { FileManager.default.fileExists(atPath: path) && !directory(path) }
    static func read(_ path: String) throws -> String { try String(contentsOfFile: path, encoding: .utf8) }
    static func write(_ text: String, _ path: String) throws {
        try Data(text.utf8).write(to: URL(fileURLWithPath: path))
    }
    static func remove(_ path: String) throws { try FileManager.default.removeItem(atPath: path) }
    static func mkdir(_ path: String) throws {
        try FileManager.default.createDirectory(atPath: path, withIntermediateDirectories: true)
    }
    static func entries(_ path: String) throws -> [String] { try FileManager.default.contentsOfDirectory(atPath: path) }
    static func normalize(_ path: String) -> String {
        var pieces: [String] = []
        for part in path.split(separator: "/").map(String.init) {
            if part == "." { continue }
            if part == ".." {
                if let last = pieces.last, last != ".." {
                    pieces.removeLast()
                } else if !path.hasPrefix("/") {
                    pieces.append(part)
                }
            } else {
                pieces.append(part)
            }
        }
        return (path.hasPrefix("/") ? "/" : "") + pieces.joined(separator: "/")
    }
    static func resolve(_ path: String, links: Int = 0) throws -> String {
        if path.contains("\0") { throw ClowderError("Paths cannot contain NUL characters") }
        if links > 40 { throw ClowderError("Too many symbolic links in \(path)") }
        if let pointer = realpath(path, nil) {
            defer { free(pointer) }
            return String(cString: pointer)
        }
        let error = errno
        if error != ENOENT { throw ClowderError("Cannot resolve \(path): \(String(cString: strerror(error)))") }
        if let target = try? FileManager.default.destinationOfSymbolicLink(atPath: path) {
            return try resolve(join(parent(path), target), links: links + 1)
        }
        let parent = parent(path)
        if parent.isEmpty || parent == path { throw ClowderError("Cannot resolve \(path)") }
        return try normalize(join(resolve(parent, links: links), name(path)))
    }
}

struct Workspace: Sendable {
    var root: String
    var found: Bool
    var repo: String { FS.join(root, ".clowder") }
    static func discover() -> Workspace {
        let cwd = FileManager.default.currentDirectoryPath
        var path = cwd
        while true {
            if [".clowder", "clowder.yml", "clowder.yaml"].contains(where: { FS.present(FS.join(path, $0)) }) {
                return Workspace(root: path, found: true)
            }
            if path == "/" { break }
            path = FS.parent(path)
        }
        return Workspace(root: cwd, found: false)
    }
    func requireRepo(_ isGit: Bool) async throws -> String {
        if !FS.directory(repo) { throw ClowderError("No .clowder directory found") }
        if isGit, !(await Git.exists(repo)) { throw ClowderError("No .clowder git repository found") }
        return repo
    }
    func manifest() throws -> String {
        let yml = FS.join(root, "clowder.yml")
        let yaml = FS.join(root, "clowder.yaml")
        if FS.present(yml) && FS.present(yaml) {
            throw ClowderError("Found clowder.yml and clowder.yaml files in same directory")
        }
        let path = FS.present(yaml) ? yaml : yml
        if !FS.file(path) { throw ClowderError("\(path) appears to be missing") }
        return path
    }
    func status() async throws -> String {
        if !FS.directory(repo) { return "" }
        var result = ".clowder"
        if await Git.exists(repo) {
            if await Git.dirty(repo) { result += "*" }
            result += try await " " + Git.formattedRef(repo)
        }
        if let path = try? manifest(), FS.symlink(path) {
            let source = try FS.resolve(path)
            result += "\n\(FS.name(path)) -> \(source.removingPrefix(root + "/"))"
        }
        return result + "\n"
    }
    func link(_ version: String?) async throws {
        _ = try await requireRepo(false)
        let directory = version == nil ? repo : FS.join(repo, "versions")
        let stem = version.map { "\($0).clowder" } ?? "clowder"
        var names: Set<String> = []
        for name in (try? FS.entries(FS.join(repo, "versions"))) ?? [] {
            for suffix in [".clowder.yml", ".clowder.yaml"] where name.hasSuffix(suffix) {
                let value = String(name.dropLast(suffix.count))
                if !names.insert(value).inserted { throw ClowderError("Duplicate version found: \(value)") }
            }
        }
        let yml = FS.join(directory, stem + ".yml")
        let yaml = FS.join(directory, stem + ".yaml")
        let source = FS.file(yml) ? yml : yaml
        if !FS.file(source) { throw ClowderError("Symlink source \(yml) appears to be missing") }
        let target = FS.join(root, source.hasSuffix(".yaml") ? "clowder.yaml" : "clowder.yml")
        if FS.present(target) && !FS.symlink(target) {
            throw ClowderError("Found non-symlink file \(target) at target path")
        }
        for name in ["clowder.yml", "clowder.yaml"] {
            let path = FS.join(root, name)
            if FS.symlink(path) { try FS.remove(path) }
        }
        let relative = source.removingPrefix(root + "/")
        writeOut(" - Symlink \(FS.name(target)) -> \(relative)\n")
        try FileManager.default.createSymbolicLink(atPath: target, withDestinationPath: relative)
    }
}
