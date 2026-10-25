import Foundation

// Compile the draft-07 vocabulary used by the checked-in Clowder schema.
// Unsupported assertions fail compilation, rather than silently skipping checks.
struct Schema: Sendable {
    indirect enum Rule: Sendable {
        case all([Rule])
        case any([Rule])
        case one([Rule])
        case not(Rule)
        case constant(Bool)
        case type(String)
        case values([Value])
        case length(Int)
        case pattern(NSRegularExpression)
        case minimum(Double)
        case object(required: [String], properties: [String: Rule], additional: Rule, names: Rule, minimum: Int)
        case array(items: Rule, minimum: Int, unique: Bool)
        func accepts(_ value: Value) -> Bool {
            switch self {
            case .all(let rules): return rules.allSatisfy { $0.accepts(value) }
            case .any(let rules): return rules.contains { $0.accepts(value) }
            case .one(let rules): return rules.filter { $0.accepts(value) }.count == 1
            case .not(let rule): return !rule.accepts(value)
            case .constant(let result): return result
            case .type(let type):
                switch type {
                case "string": return value.string != nil
                case "object": return value.object != nil
                case "array": return value.array != nil
                case "boolean": return value.bool != nil
                case "null": return value.isNull
                case "number": return value.number != nil
                case "integer": return value.number.map { $0.rounded() == $0 } ?? false
                default: return false
                }
            case .values(let values): return values.contains(value)
            case .length(let minimum): return value.string.map { $0.unicodeScalars.count >= minimum } ?? true
            case .pattern(let expression):
                guard let text = value.string else { return true }
                return expression.firstMatch(in: text, range: NSRange(text.startIndex..., in: text)) != nil
            case .minimum(let minimum): return value.number.map { $0 >= minimum } ?? true
            case .object(let required, let properties, let additional, let names, let minimum):
                guard let object = value.object else { return true }
                return object.count >= minimum && required.allSatisfy { object[$0] != nil }
                    && object.pairs.allSatisfy { key, value in
                        names.accepts(.string(key)) && (properties[key] ?? additional).accepts(value)
                    }
            case .array(let items, let minimum, let unique):
                guard let array = value.array else { return true }
                if array.count < minimum || !array.allSatisfy({ items.accepts($0) }) { return false }
                if unique { for i in array.indices { if array[..<i].contains(array[i]) { return false } } }
                return true
            }
        }
    }
    static let manifest: Rule = {
        do {
            let value = try Value.parseJSON(Embedded.schema)
            return try compile(value, root: value)
        } catch { preconditionFailure("Invalid embedded schema: \(error)") }
    }()
    static func compile(_ value: Value, root: Value) throws -> Rule {
        if let boolean = value.bool { return .constant(boolean) }
        guard let object = value.object else { throw ClowderError("Invalid schema") }
        if let reference = value["$ref"].string {
            if !reference.hasPrefix("#/definitions/") {
                throw ClowderError("Unsupported schema reference: \(reference)")
            }
            return try compile(root["definitions"][reference.removingPrefix("#/definitions/")], root: root)
        }
        let supported: Set<String> = [
            "$id", "$schema", "definitions", "description", "title", "default", "type", "enum", "allOf", "anyOf",
            "oneOf", "not", "minLength", "pattern", "minimum", "properties", "additionalProperties", "required",
            "propertyNames", "minProperties", "items", "minItems", "uniqueItems",
        ]
        for key in object.keys where !supported.contains(key) {
            throw ClowderError("Unsupported schema keyword: \(key)")
        }
        var rules: [Rule] = []
        if let type = value["type"].string { rules.append(.type(type)) }
        if let types = value["type"].array { rules.append(.any(types.compactMap(\.string).map(Rule.type))) }
        if let values = value["enum"].array { rules.append(.values(values)) }
        for key in ["allOf", "anyOf", "oneOf"] {
            if let values = value[key].array {
                let children = try values.map { try compile($0, root: root) }
                rules.append(key == "allOf" ? .all(children) : key == "anyOf" ? .any(children) : .one(children))
            }
        }
        if let child = object["not"] { rules.append(.not(try compile(child, root: root))) }
        if let minimum = value["minLength"].number { rules.append(.length(Int(minimum))) }
        if let pattern = value["pattern"].string { rules.append(.pattern(try NSRegularExpression(pattern: pattern))) }
        if let minimum = value["minimum"].number { rules.append(.minimum(minimum)) }
        if ["properties", "required", "additionalProperties", "propertyNames", "minProperties"].contains(where: {
            object[$0] != nil
        }) {
            var properties: [String: Rule] = [:]
            for (key, child) in value["properties"].object?.pairs ?? [] {
                properties[key] = try compile(child, root: root)
            }
            rules.append(
                .object(
                    required: value["required"].array?.compactMap(\.string) ?? [], properties: properties,
                    additional: try compile(object["additionalProperties"] ?? .bool(true), root: root),
                    names: try compile(object["propertyNames"] ?? .bool(true), root: root),
                    minimum: Int(value["minProperties"].number ?? 0)))
        }
        if ["items", "minItems", "uniqueItems"].contains(where: { object[$0] != nil }) {
            rules.append(
                .array(
                    items: try compile(object["items"] ?? .bool(true), root: root),
                    minimum: Int(value["minItems"].number ?? 0), unique: value["uniqueItems"].bool ?? false))
        }
        return .all(rules)
    }
}
