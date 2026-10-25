import Foundation

extension CLI {
    static func completionAction(_ spec: Spec, _ flag: String) -> Action? {
        if let exact = spec.actions.first(where: { $0.flags.contains(flag) }) { return exact }
        let matches = spec.actions.filter { flag.hasPrefix("--") && $0.flags.contains(where: { $0.hasPrefix(flag) }) }
        return matches.count == 1 ? matches[0] : nil
    }
    static func argumentValues(_ command: String, _ argument: Action) async -> [String] {
        if let choices = argument.choices { return choices }
        let workspace = Workspace.discover()
        if argument.name == "version" && command == "link" {
            return ((try? FS.entries(FS.join(workspace.repo, "versions"))) ?? []).compactMap { name in
                for suffix in [".clowder.yml", ".clowder.yaml"] where name.hasSuffix(suffix) {
                    return String(name.dropLast(suffix.count))
                }
                return nil
            }
        }
        let manifest = try? Manifest.load(workspace)
        switch argument.name {
        case "projects": return (manifest?.projects.flatMap { Array($0.groups) } ?? []) + ["default"]
        case "exclude": return manifest?.projects.map(\.name) ?? []
        case "branch", "tag", "ref":
            let paths = command.hasPrefix("repo ") ? [workspace.repo] : manifest?.projects.map(\.path) ?? []
            let refs =
                argument.name == "tag"
                ? ["refs/tags"]
                : argument.name == "branch" ? ["refs/heads"] : ["refs/heads", "refs/tags", "refs/remotes"]
            var values: [String] = []
            for path in paths where await Git.exists(path) {
                if let text = try? await Git.read(path, ["for-each-ref", "--format=%(refname:short)"] + refs) {
                    values += text.lines
                }
            }
            return values
        default: return []
        }
    }
    static func complete(_ tokens: [String]) async -> [String] {
        let current = tokens.last ?? ""
        let previous = tokens.dropLast()
        var command = ""
        var pending: Action?
        var positionals = 0
        var optionsEnded = false
        for token in previous {
            let spec = specs[command]!
            if let argument = pending {
                if !option(token) {
                    if argument.nargs != .string("*") && argument.nargs != .string("+") { pending = nil }
                    continue
                }
                pending = nil
            }
            if !optionsEnded && token == "--" {
                optionsEnded = true
            } else if !optionsEnded && option(token) {
                if token.hasPrefix("--") {
                    let flag = String(token.split(separator: "=", maxSplits: 1, omittingEmptySubsequences: false)[0])
                    pending = completionAction(spec, flag)
                    if pending?.boolean == true || token.contains("=") { pending = nil }
                } else {
                    for (index, character) in token.dropFirst().enumerated() {
                        guard let argument = completionAction(spec, "-" + String(character)) else { break }
                        if !argument.boolean {
                            if index + 2 == token.count { pending = argument }
                            break
                        }
                    }
                }
            } else if !optionsEnded && spec.children.contains(token) && positionals == 0 {
                command = command.isEmpty ? token : command + " " + token
            } else {
                positionals += 1
            }
        }
        let spec = specs[command]!
        let values: [String]
        if !optionsEnded && current.hasPrefix("--"), let equal = current.firstIndex(of: "=") {
            let flag = String(current[..<equal])
            let prefix = String(current[current.index(after: equal)...])
            if let argument = completionAction(spec, flag) {
                values = await argumentValues(command, argument).filter { $0.hasPrefix(prefix) }.map { flag + "=" + $0 }
            } else {
                values = []
            }
        } else if !optionsEnded && current.hasPrefix("-") {
            values = spec.actions.flatMap(\.flags) + ["--help", "-h"] + (command.isEmpty ? ["--version"] : [])
        } else if let pending {
            values = await argumentValues(command, pending)
        } else if !spec.children.isEmpty {
            values = spec.children
        } else {
            var argument: Action?
            for candidate in spec.actions where candidate.flags.isEmpty {
                if ["*", "+"].contains(candidate.nargs.string ?? "") || positionals == 0 {
                    argument = candidate
                    break
                }
                positionals -= 1
            }
            if let argument { values = await argumentValues(command, argument) } else { values = [] }
        }
        return Set(values.filter { $0.hasPrefix(current) }).sorted()
    }
}
