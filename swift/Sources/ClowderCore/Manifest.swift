import Foundation

struct Upstream: Sendable {
    var name: String
    var remote: String
    var url: String
}
struct GitSettings: Sendable {
    var submodules = false, recursive = false, lfs = false
    var depth: UInt64?
    var config = Object()
}
struct Project: Sendable {
    var name: String, relative: String, path: String, remote: String, url: String
    var branch: String?, tag: String?, commit: String?
    var upstream: Upstream?
    var groups: Set<String>
    var git: GitSettings
    func defaultBranch() async -> String {
        if let branch { return branch }
        return await Git.defaultBranch(path, remote: remote, url: url)
    }
    func defaultRef() async -> String {
        if let branch { return "refs/heads/" + branch }
        if let tag { return "refs/tags/" + tag }
        if let commit { return commit }
        return await "refs/heads/" + defaultBranch()
    }
    func displayName() async -> String {
        guard await Git.exists(path) else { return relative }
        return await name + (Git.dirty(path) ? "*" : "")
    }
    func status(width: Int? = nil) async throws -> String {
        let display = await displayName()
        let name = width.map { display.padded($0) } ?? display
        guard await Git.exists(path) else { return width == nil ? name : name + " -" }
        return try await name + " " + Git.formattedRef(path)
    }
}
struct Manifest: Sendable {
    var name: String
    var raw: Value
    var projects: [Project]
    static func load(_ workspace: Workspace, protocol override: String? = nil) throws -> Manifest {
        try fromValue(workspace, YAML.parse(FS.read(workspace.manifest())), protocol: override)
    }
    static func fromValue(_ workspace: Workspace, _ raw: Value, protocol override: String? = nil) throws -> Manifest {
        guard Schema.manifest.accepts(raw) else {
            throw ClowderError("Invalid clowder YAML: manifest does not match clowder.schema.json")
        }
        var sources: [String: Value] = [
            "github": .object(["url": .string("github.com")]), "gitlab": .object(["url": .string("gitlab.com")]),
            "bitbucket": .object(["url": .string("bitbucket.org")]),
        ]
        for (key, value) in raw["sources"].object?.pairs ?? [] { sources[key] = value }
        let defaults = raw["defaults"]
        func inherited(_ key: String, _ values: Value...) -> Value? { values.map { $0[key] }.first { !$0.isNull } }
        func sourceURL(_ source: Value?, _ name: String) throws -> String {
            let source = source ?? .string("github")
            let definition: Value
            if let alias = source.string {
                guard let value = sources[alias] else { throw ClowderError("Unknown source: \(alias)") }
                definition = value
            } else {
                definition = source
            }
            guard let host = definition["url"].string else { throw ClowderError("Source is missing its URL") }
            let proto = definition["protocol"].string ?? override ?? raw["protocol"].string ?? "ssh"
            switch proto {
            case "ssh": return "git@\(host):\(name).git"
            case "https": return "https://\(host)/\(name).git"
            default: throw ClowderError("Unknown protocol: \(proto)")
            }
        }
        var entries: [(Value, String?, Value)] = []
        if let projects = raw["clowder"].array {
            entries = projects.map { ($0, nil, .null) }
        } else {
            for (name, section) in raw["clowder"].object?.pairs ?? [] {
                entries += (section.array ?? section["projects"].array ?? []).map { ($0, name, section) }
            }
        }
        var projects: [Project] = []
        var paths: Set<String> = []
        for (project, sectionName, section) in entries {
            let name = project.string ?? project["name"].string!
            let sectionDefaults = section["defaults"]
            func property(_ key: String) -> Value? { inherited(key, project, sectionDefaults, defaults) }
            let pathName = project["path"].string ?? name.components(separatedBy: "/").last!
            let relative = relativePath(section["path"].string ?? "", pathName)
            let path = try FS.resolve(FS.join(workspace.root, relative))
            if !paths.insert(path).inserted { throw ClowderError("Multiple projects with path '\(path)'") }
            let remote = property("remote")?.string ?? "origin"
            let url = try sourceURL(property("source"), name)
            var groups: Set<String> = ["all", name, relative]
            if let sectionName { groups.insert(sectionName) }
            for value in [project, section] { groups.formUnion(value["groups"].array?.compactMap(\.string) ?? []) }
            if groups.contains("notdefault") { groups.remove("all") }
            var settings = GitSettings()
            for git in [defaults["git"], sectionDefaults["git"], project["git"]] {
                if let submodules = git.object?["submodules"] {
                    settings.submodules = submodules == .bool(true) || submodules == .string("recursive")
                    settings.recursive = submodules == .string("recursive")
                }
                if let lfs = git["lfs"].bool { settings.lfs = lfs }
                if case .uint(let depth) = git["depth"] { settings.depth = depth }
                if case .int(let depth) = git["depth"], depth >= 0 { settings.depth = UInt64(depth) }
                for (key, value) in git["config"].object?.pairs ?? [] {
                    settings.config[key] = value.isNull ? nil : value
                }
            }
            let branch = property("branch")?.string
            let tag = branch == nil ? property("tag")?.string : nil
            let commit = branch == nil && tag == nil ? property("commit")?.string : nil
            var upstream: Upstream?
            if let value = project.object?["upstream"] {
                let name = value.string ?? value["name"].string!
                func property(_ key: String) -> Value? {
                    inherited(key, value, sectionDefaults["upstream"], defaults["upstream"])
                }
                let upstreamRemote = property("remote")?.string ?? "upstream"
                if upstreamRemote == remote {
                    throw ClowderError("Upstream '\(name)' and project have same remote name '\(remote)'")
                }
                upstream = try Upstream(name: name, remote: upstreamRemote, url: sourceURL(property("source"), name))
            }
            projects.append(
                Project(
                    name: name, relative: relative, path: path, remote: remote, url: url, branch: branch, tag: tag,
                    commit: commit, upstream: upstream, groups: groups, git: settings))
        }
        projects.sort { $0.name.lowercased() < $1.name.lowercased() }
        return Manifest(name: raw["name"].string!, raw: raw, projects: projects)
    }
    static func relativePath(_ section: String, _ project: String) -> String {
        FS.join(section, project).split(separator: "/").filter { $0 != "." }.joined(separator: "/")
    }
    func selected(_ names: [String], defaults: [String] = [], exclude: [String] = []) throws -> [Project] {
        var names = names.isEmpty || names == ["default"] ? defaults : names
        if names.isEmpty { names = ["all"] }
        for name in names where !projects.contains(where: { $0.groups.contains(name) }) {
            throw ClowderError("Unknown project or group: \(name)")
        }
        return projects.filter { !exclude.contains($0.name) && !Set(names).isDisjoint(with: $0.groups) }
    }
    func yaml(resolved: Bool) async throws -> String {
        if resolved { for project in projects { try await Git.validate(project.path, allowMissing: false) } }
        func projectYAML(_ value: Value, sectionPath: String) async throws -> Value {
            if value.string != nil && !resolved { return value }
            let name = value.string ?? value["name"].string!
            var result: Object = ["name": .string(name)]
            if let path = value.object?["path"] { result["path"] = path }
            if resolved {
                let relative = Self.relativePath(
                    sectionPath, value["path"].string ?? name.components(separatedBy: "/").last!)
                guard let project = projects.first(where: { $0.name == name && $0.relative == relative }) else {
                    throw ClowderError("Cannot resolve project \(name)")
                }
                result["commit"] = try await .string(Git.read(project.path, ["rev-parse", "HEAD"]))
            } else {
                for key in ["branch", "tag", "commit"] { result[key] = value.object?[key] }
            }
            for key in ["groups", "remote", "source", "git", "upstream"] {
                if let value = value.object?[key] { result[key] = Self.canonical(key, value) }
            }
            return .object(result)
        }
        var root = raw.object!
        if let array = raw["clowder"].array {
            var values: [Value] = []
            for project in array { values.append(try await projectYAML(project, sectionPath: "")) }
            root["clowder"] = .array(values)
        } else {
            var sections = Object()
            for (name, section) in raw["clowder"].object!.pairs {
                var values: [Value] = []
                for project in section.array ?? section["projects"].array! {
                    values.append(try await projectYAML(project, sectionPath: section["path"].string ?? ""))
                }
                if var object = section.object {
                    object["projects"] = .array(values)
                    sections[name] = Self.canonical("section", .object(object))
                } else {
                    sections[name] = .array(values)
                }
            }
            root["clowder"] = .object(sections)
        }
        return try YAML.stringify(Self.canonical("root", .object(root)))
    }
    static func canonical(_ kind: String, _ value: Value) -> Value {
        guard let object = value.object else { return value }
        let orders: [String: [String]] = [
            "root": ["name", "protocol", "sources", "defaults", "clowder"],
            "defaults": ["branch", "tag", "commit", "remote", "source", "git", "upstream"],
            "git": ["submodules", "lfs", "depth", "config"], "source": ["url", "protocol"],
            "section": ["path", "groups", "defaults", "projects"], "upstream": ["name", "remote", "source"],
        ]
        guard let keys = orders[kind] else { return value }
        var result = Object()
        for key in keys {
            guard let value = object[key] else { continue }
            if key == "sources", let sources = value.object {
                var mapped = Object()
                for (name, source) in sources.pairs { mapped[name] = canonical("source", source) }
                result[key] = .object(mapped)
            } else {
                result[key] = canonical(key, value)
            }
        }
        return .object(result)
    }
}
