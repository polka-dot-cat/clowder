import Foundation

struct Arguments: Sendable {
    var command = ""
    var values: [String: [String]] = [:]
    var debug = false
    func one(_ key: String) -> String? { values[key]?.first }
    func many(_ key: String) -> [String] { values[key] ?? [] }
    func flag(_ key: String) -> Bool { one(key) == "true" }
    func number(_ key: String) -> Int? { one(key).flatMap(Int.init) }
}
enum CLI {
    struct Action: Decodable, Sendable {
        var name: String, flags: [String], nargs: Value, boolean: Bool, integer: Bool
        var `default`: Value
        var choices: [String]?, metavar: String?
    }
    struct Spec: Decodable, Sendable {
        var help: String, usage: String, actions: [Action], children: [String], exclusive: [[String]]
    }
    static let specs = try! JSONDecoder().decode([String: Spec].self, from: Data(Embedded.grammar.utf8))
    static func help(_ command: String) -> String {
        var help = specs[command]!.help
        if let manifest = try? Manifest.load(Workspace.discover()),
            let line = help.lines.first(where: { $0.hasPrefix("  <project|group>") })
        {
            let table = projectTable(manifest)
            if !table.isEmpty {
                let replacement =
                    line + ":\n                        \n"
                    + table.map { "                        " + $0 + "\n" }.joined()
                help = help.replacingOccurrences(of: line + "\n", with: replacement)
            }
        }
        return help
    }
    static func projectTable(_ manifest: Manifest) -> [String] {
        func sorted(_ values: [String]) -> [String] {
            Set(values).sorted().sorted { $0.lowercased() < $1.lowercased() }
        }
        let names = sorted(manifest.projects.map(\.name))
        let paths = sorted(manifest.projects.map(\.relative))
        let upstreams = sorted(manifest.projects.compactMap { $0.upstream?.name })
        let groups = sorted(
            manifest.projects.flatMap { Array($0.groups) }.filter {
                !names.contains($0) && !paths.contains($0) && !upstreams.contains($0)
            })
        if names.isEmpty || paths.isEmpty || groups.isEmpty { return [] }
        func width(_ values: [String]) -> Int { values.map(\.count).max() ?? 1 }
        func rows(_ columns: [([String], Int)], _ titles: [String]) -> [String] {
            var rows = [
                zip(columns, titles).map { $1.padded($0.1) }.joined(),
                zip(columns, titles).map { String(repeating: "-", count: $1.count).padded($0.1) }.joined(),
            ]
            for index in 0..<(columns.map { $0.0.count }.max() ?? 0) {
                rows.append(columns.map { (index < $0.0.count ? $0.0[index] : "").padded($0.1) }.joined())
            }
            return rows
        }
        if upstreams.isEmpty {
            return rows(
                [(names, width(names) + 2), (paths, width(paths) + 2), (groups, width(groups))],
                ["Project Names", "Project Paths", "Project Groups"])
        }
        let nameWidth = max(width(names), width(upstreams)) + 2
        let otherWidth = max(width(paths), width(groups))
        return rows([(names, nameWidth), (paths, otherWidth)], ["Project Names", "Project Paths"]) + [""]
            + rows([(upstreams, nameWidth), (groups, otherWidth)], ["Upstream Names", "Project Groups"])
    }
    static func option(_ token: String) -> Bool { token.hasPrefix("-") && token != "-" && Int64(token) == nil }
    static func error(_ command: String, _ message: String) -> ClowderError {
        ClowderError(
            specs[command]!.usage.trimEnd + "\nclowder" + (command.isEmpty ? "" : " " + command) + ": error: "
                + message, code: 2)
    }
    static func validate(_ action: Action, _ value: String, _ command: String) throws {
        if action.integer && Int64(value) == nil {
            throw error(command, "argument \(action.name): invalid int value: '\(value)'")
        }
        if let choices = action.choices, !choices.contains(value) {
            throw error(
                command,
                "argument \(action.name): invalid choice: '\(value)' (choose from \(choices.map { "'\($0)'" }.joined(separator: ", ")))"
            )
        }
    }
    static func parse(_ tokens: [String]) throws -> Arguments? {
        var pending = tokens
        var args = Arguments()
        var positional: [String] = []
        var seen: [String] = []
        var optionsEnded = false
        while !pending.isEmpty {
            var token = pending.removeFirst()
            let spec = specs[args.command]!
            if !optionsEnded {
                if token.hasPrefix("--") && token != "--" {
                    let pieces = token.split(separator: "=", maxSplits: 1, omittingEmptySubsequences: false).map(
                        String.init)
                    let flag = pieces[0]
                    let suffix = pieces.count == 2 ? pieces[1] : nil
                    var flags = spec.actions.flatMap(\.flags) + ["--help"]
                    if args.command.isEmpty { flags.append("--version") }
                    if !flags.contains(flag) {
                        let matches = flags.filter { $0.hasPrefix(flag) }
                        if matches.count > 1 {
                            throw error(
                                args.command, "ambiguous option: \(flag) could match \(matches.joined(separator: ", "))"
                            )
                        }
                        if let resolved = matches.first { token = resolved + (suffix.map { "=" + $0 } ?? "") }
                    }
                }
                if token == "--" {
                    optionsEnded = true
                    continue
                }
                if token == "-h" || token == "--help" {
                    writeOut(help(args.command))
                    return nil
                }
                if args.command.isEmpty && ["-v", "--version"].contains(token) {
                    writeOut("clowder version 4.0b7\n")
                    return nil
                }
                if !spec.children.isEmpty && !option(token) && positional.isEmpty {
                    if !spec.children.contains(token) { throw error(args.command, "invalid command '\(token)'") }
                    args.command = args.command.isEmpty ? token : args.command + " " + token
                    seen = []
                    continue
                }
                if option(token) {
                    var attached: String?
                    if token.hasPrefix("--") {
                        if let equal = token.firstIndex(of: "=") {
                            attached = String(token[token.index(after: equal)...])
                            token = String(token[..<equal])
                        }
                    } else if token.utf8.count > 2 && token.unicodeScalars.allSatisfy(\.isASCII) {
                        let flag = String(token.prefix(2))
                        if let action = spec.actions.first(where: { $0.flags.contains(flag) }) {
                            let rest = String(token.dropFirst(2))
                            if action.boolean { pending.insert("-" + rest, at: 0) } else { attached = rest }
                            token = flag
                        }
                    }
                    guard let action = spec.actions.first(where: { $0.flags.contains(token) }) else {
                        throw error(args.command, "unrecognized arguments: \(token)")
                    }
                    seen.append(action.name)
                    if action.boolean {
                        if attached != nil { throw error(args.command, "argument \(token): ignored explicit argument") }
                        args.values[action.name] = ["true"]
                        if action.name == "debug" { args.debug = true }
                    } else {
                        var values = attached.map { [$0] } ?? []
                        let multiple = ["+", "*"].contains(action.nargs.string ?? "")
                        while let value = pending.first {
                            if option(value) || (!multiple && !values.isEmpty) { break }
                            values.append(pending.removeFirst())
                        }
                        if values.isEmpty && action.nargs != .string("*") {
                            throw error(args.command, "argument \(token): expected one argument")
                        }
                        for value in values { try validate(action, value, args.command) }
                        args.values[action.name] = values
                    }
                    continue
                }
            }
            positional.append(token)
        }
        let spec = specs[args.command]!
        if args.command.isEmpty {
            writeError(spec.help)
            return nil
        }
        for group in spec.exclusive {
            let used = group.filter { seen.contains($0) }
            if used.count > 1 { throw error(args.command, "argument \(used[1]): not allowed with argument \(used[0])") }
        }
        for action in spec.actions where action.flags.isEmpty {
            let multiple = ["+", "*"].contains(action.nargs.string ?? "")
            let optional = ["?", "*"].contains(action.nargs.string ?? "")
            let values: [String]
            if multiple {
                values = positional
                positional = []
            } else {
                values = positional.isEmpty ? [] : [positional.removeFirst()]
            }
            if values.isEmpty {
                if !optional {
                    throw error(args.command, "the following arguments are required: \(action.metavar ?? action.name)")
                }
                if let value = action.default.string { args.values[action.name] = [value] }
            } else {
                for value in values { try validate(action, value, args.command) }
                args.values[action.name] = values
            }
        }
        if !positional.isEmpty {
            throw error(args.command, "unrecognized arguments: \(positional.joined(separator: " "))")
        }
        return args
    }
}
