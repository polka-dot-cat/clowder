import Foundation

struct Config: Sendable {
    var sections = Object()
    static func load(_ workspace: Workspace) throws -> Config {
        let path = FS.join(workspace.root, "clowder.config")
        return try parse(FS.file(path) ? FS.read(path) : "")
    }
    static func parse(_ text: String) throws -> Config {
        var result = Config()
        var section = ""
        var previous: (String, Int)?
        var seenSections: Set<String> = []
        var seenKeys: Set<String> = []
        func append(_ key: String, _ line: String) {
            result.set(section, key, (result.sections[section]?[key].string ?? "") + "\n" + line)
        }
        for line in text.lines {
            let trimmed = line.trimmed
            if trimmed.hasPrefix("#") || trimmed.hasPrefix(";") { continue }
            if trimmed.isEmpty {
                if let (key, _) = previous { append(key, "") }
                continue
            }
            let indent = line.prefix(while: \.isWhitespace).count
            if let (key, previousIndent) = previous, indent > previousIndent {
                append(key, trimmed)
                continue
            }
            if trimmed.hasPrefix("["), let end = trimmed.lastIndex(of: "]"),
                end > trimmed.index(after: trimmed.startIndex)
            {
                section = String(trimmed[trimmed.index(after: trimmed.startIndex)..<end])
                if !seenSections.insert(section).inserted && section != "DEFAULT" {
                    throw ClowderError("Duplicate clowder.config section: \(section)")
                }
                if result.sections[section] == nil { result.sections[section] = .object(Object()) }
                previous = nil
            } else if let index = trimmed.firstIndex(where: { $0 == "=" || $0 == ":" }) {
                if section.isEmpty { throw ClowderError("Invalid clowder.config: setting outside a section") }
                let key = String(trimmed[..<index]).trimmed.lowercased()
                if key.isEmpty || !seenKeys.insert(section + "\0" + key).inserted {
                    throw ClowderError("Invalid or duplicate clowder.config key: \(key)")
                }
                result.set(section, key, String(trimmed[trimmed.index(after: index)...]).trimmed)
                previous = (key, indent)
            } else {
                throw ClowderError("Invalid clowder.config line: \(line)")
            }
        }
        for (section, values) in result.sections.pairs {
            for (key, value) in values.object?.pairs ?? [] { result.set(section, key, value.string!.trimEnd) }
        }
        for section in ["git", "command"] where result.sections[section] == nil {
            result.sections[section] = .object(Object())
        }
        return result
    }
    func raw(_ section: String, _ key: String) -> String? {
        sections[section]?[key].string ?? sections["DEFAULT"]?[key].string
    }
    func get(_ section: String, _ key: String) throws -> String? {
        try raw(section, key.lowercased()).map { try interpolate(section, $0, depth: 1) }
    }
    func interpolate(_ section: String, _ value: String, depth: Int) throws -> String {
        if depth > 10 { throw ClowderError("clowder.config interpolation is recursive or exceeds 10 levels") }
        var text = value[...]
        var output = ""
        while let index = text.firstIndex(of: "%") {
            output += text[..<index]
            text = text[text.index(after: index)...]
            if text.hasPrefix("%") {
                output += "%"
                text = text.dropFirst()
            } else if text.hasPrefix("("), let end = text.firstIndex(of: ")"), end > text.index(after: text.startIndex),
                text[text.index(after: end)...].hasPrefix("s")
            {
                let key = String(text[text.index(after: text.startIndex)..<end])
                guard let replacement = raw(section, key.lowercased()) else {
                    throw ClowderError("Unknown clowder.config interpolation key: \(key)")
                }
                output +=
                    try replacement.contains("%") ? interpolate(section, replacement, depth: depth + 1) : replacement
                text = text[text.index(end, offsetBy: 2)...]
            } else {
                throw ClowderError("Invalid clowder.config interpolation; use %% for a literal percent sign")
            }
        }
        return output + text
    }
    mutating func set(_ section: String, _ key: String, _ value: String?) {
        var values = sections[section]?.object ?? Object()
        values[key.lowercased()] = value.map(Value.string)
        sections[section] = .object(values)
    }
    func projects() throws -> [String] {
        try get("command", "projects").map { $0.trimmed.components(separatedBy: ", ") } ?? []
    }
    func jobs() throws -> Int? {
        guard let text = try get("command", "jobs") else { return nil }
        guard let value = Int(text.trimmed.replacingOccurrences(of: "_", with: "")) else {
            throw ClowderError("Invalid jobs setting: \(text)")
        }
        return value
    }
    func rebase() throws -> Bool? {
        guard let value = try get("git", "rebase")?.lowercased() else { return nil }
        if ["true", "yes", "on", "1"].contains(value) { return true }
        if ["false", "no", "off", "0"].contains(value) { return false }
        throw ClowderError("Invalid rebase setting: \(value)")
    }
    func serialized() -> String {
        var text = ""
        let keys = (sections["DEFAULT"] == nil ? [] : ["DEFAULT"]) + sections.keys.filter { $0 != "DEFAULT" }
        for section in keys {
            let values = sections[section]!.object!
            if section == "DEFAULT" && values.count == 0 { continue }
            text += "[\(section)]\n"
            for (key, value) in values.pairs {
                text += "\(key) = \(value.string!.replacingOccurrences(of: "\n", with: "\n\t"))\n"
            }
            text += "\n"
        }
        return text
    }
    func save(_ workspace: Workspace) throws {
        if !workspace.found { throw ClowderError("No clowder config file path found") }
        try FS.write(serialized(), FS.join(workspace.root, "clowder.config"))
    }
    static func printCurrent(_ workspace: Workspace) throws {
        let path = FS.join(workspace.root, "clowder.config")
        if FS.file(path) {
            writeOut("Current config\n\n\(try FS.read(path).trimmed)\n\n")
        } else {
            writeOut(" - No config file found\n")
        }
    }
}
