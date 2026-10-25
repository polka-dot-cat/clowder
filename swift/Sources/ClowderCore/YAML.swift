import CYaml
import Foundation
import Yams

enum YAML {
    enum Event {
        case scalar(String, Bool, String?, String?)
        case sequence(String?, String?)
        case mapping(String?, String?)
        case alias(String)
        case endSequence, endMapping, document, endDocument, stream
    }
    static func parse(_ text: String) throws -> Value {
        var parser = yaml_parser_t()
        guard yaml_parser_initialize(&parser) != 0 else { throw ClowderError("Cannot initialize YAML parser") }
        defer { yaml_parser_delete(&parser) }
        let bytes = Array(text.utf8)
        return try bytes.withUnsafeBufferPointer { buffer in
            yaml_parser_set_input_string(&parser, buffer.baseAddress, buffer.count)
            var events: [Event] = []
            var anchors: Set<String> = []
            func string(_ pointer: UnsafePointer<UInt8>?) -> String? { pointer.map { String(cString: $0) } }
            while true {
                var event = yaml_event_t()
                guard yaml_parser_parse(&parser, &event) != 0 else {
                    throw ClowderError(
                        "Invalid YAML: \(parser.problem.map { String(cString: $0) } ?? "syntax error") at line \(parser.problem_mark.line + 1)"
                    )
                }
                defer { yaml_event_delete(&event) }
                var anchor: String?
                switch event.type {
                case YAML_STREAM_END_EVENT: break
                case YAML_DOCUMENT_START_EVENT: events.append(.document)
                case YAML_DOCUMENT_END_EVENT: events.append(.endDocument)
                case YAML_SCALAR_EVENT:
                    anchor = string(event.data.scalar.anchor)
                    events.append(
                        .scalar(
                            String(
                                decoding: UnsafeBufferPointer(
                                    start: event.data.scalar.value, count: event.data.scalar.length), as: UTF8.self),
                            event.data.scalar.style != YAML_PLAIN_SCALAR_STYLE, string(event.data.scalar.tag), anchor))
                case YAML_SEQUENCE_START_EVENT:
                    anchor = string(event.data.sequence_start.anchor)
                    events.append(.sequence(string(event.data.sequence_start.tag), anchor))
                case YAML_MAPPING_START_EVENT:
                    anchor = string(event.data.mapping_start.anchor)
                    events.append(.mapping(string(event.data.mapping_start.tag), anchor))
                case YAML_ALIAS_EVENT: events.append(.alias(string(event.data.alias.anchor)!))
                case YAML_SEQUENCE_END_EVENT: events.append(.endSequence)
                case YAML_MAPPING_END_EVENT: events.append(.endMapping)
                default: break
                }
                if let anchor, !anchors.insert(anchor).inserted {
                    throw ClowderError("Duplicate YAML anchor \(anchor)")
                }
                if event.type == YAML_STREAM_END_EVENT { break }
            }
            if events.filter({ if case .document = $0 { true } else { false } }).count > 1 {
                throw ClowderError("Expected one YAML document")
            }
            if events.isEmpty { return .null }
            var builder = Builder(events: events, index: 1)
            return try builder.value(0)
        }
    }
    struct Builder {
        var events: [Event]
        var index: Int
        var anchors: [String: Value] = [:]
        var aliasBytes = 0
        mutating func value(_ depth: Int) throws -> Value {
            if depth > 128 { throw ClowderError("YAML nesting is too deep") }
            guard index < events.count else { throw ClowderError("Incomplete YAML value") }
            let event = events[index]
            index += 1
            var anchor: String?
            var result: Value
            switch event {
            case .scalar(let text, let quoted, let tag, let name):
                anchor = name
                result = try scalar(text, quoted: quoted, tag: tag)
            case .alias(let name):
                guard let value = anchors[name] else {
                    throw ClowderError("Undefined or recursive YAML alias: \(name)")
                }
                aliasBytes += try size(value, limit: 64 * 1024 * 1024 - aliasBytes, depth: depth)
                result = value
            case .sequence(let tag, let name):
                try checkTag(tag, "seq")
                anchor = name
                var array: [Value] = []
                while index < events.count {
                    if case .endSequence = events[index] {
                        index += 1
                        break
                    }
                    array.append(try value(depth + 1))
                }
                result = .array(array)
            case .mapping(let tag, let name):
                try checkTag(tag, "map")
                anchor = name
                var merged = Object()
                var explicit: [(String, Value)] = []
                while index < events.count {
                    if case .endMapping = events[index] {
                        index += 1
                        break
                    }
                    var merge = false
                    var key: String?
                    if case .scalar(let text, let quoted, let tag, _) = events[index] {
                        merge = (text == "<<" && !quoted && tag == nil) || tag == "tag:yaml.org,2002:merge"
                        if text == "=" && !quoted && tag == nil {
                            key = "="
                            index += 1
                        }
                    }
                    if merge {
                        index += 1
                        let incoming = try value(depth + 1)
                        let maps = incoming.array.map { Array($0.reversed()) } ?? [incoming]
                        for map in maps {
                            guard let object = map.object else {
                                throw ClowderError("YAML merge entries must be mappings")
                            }
                            for (key, value) in object.pairs { merged[key] = value }
                        }
                    } else {
                        if key == nil { key = try value(depth + 1).string }
                        guard let key else { throw ClowderError("Manifest mapping keys must be strings") }
                        explicit.append((key, try value(depth + 1)))
                    }
                }
                for (key, value) in explicit { merged[key] = value }
                result = .object(merged)
            default: throw ClowderError("Expected a YAML value")
            }
            if let anchor { anchors[anchor] = result }
            return result
        }
        func size(_ value: Value, limit: Int, depth: Int) throws -> Int {
            if depth > 128 { throw ClowderError("YAML nesting is too deep") }
            if limit < 0 { throw ClowderError("YAML alias expansion exceeds 64 MiB") }
            var count = 2
            switch value {
            case .array(let array):
                for value in array { count += try size(value, limit: limit - count, depth: depth + 1) + 1 }
            case .object(let object):
                for (key, value) in object.pairs {
                    count += key.utf8.count + 4 + (try size(value, limit: limit - count, depth: depth + 1))
                }
            default: count += value.scalar.utf8.count
            }
            if count > limit { throw ClowderError("YAML alias expansion exceeds 64 MiB") }
            return count
        }
    }
    static func checkTag(_ tag: String?, _ expected: String) throws {
        if let tag, tag != "!", tag != "tag:yaml.org,2002:" + expected {
            throw ClowderError("Unsupported YAML tag \(tag)")
        }
    }
    static let patterns = [
        #"^(?:[-+]?0b[0-1_]+|[-+]?0[0-7_]+|[-+]?(?:0|[1-9][0-9_]*)|[-+]?0x[0-9a-fA-F_]+|[-+]?[1-9][0-9_]*(?::[0-5]?[0-9])+)$"#,
        #"^(?:[-+]?[0-9][0-9_]*\.[0-9_]*(?:[eE][-+][0-9]+)?|\.[0-9][0-9_]*(?:[eE][-+][0-9]+)?|[-+]?[0-9][0-9_]*(?::[0-5]?[0-9])+\.[0-9_]*|[-+]?\.(?:inf|Inf|INF)|\.(?:nan|NaN|NAN))$"#,
        #"^(?:[0-9]{4}-[0-9]{2}-[0-9]{2}|[0-9]{4}-[0-9]{1,2}-[0-9]{1,2}(?:[Tt]|[ \t]+)[0-9]{1,2}:[0-9]{2}:[0-9]{2}(?:\.[0-9]*)?(?:[ \t]*(?:Z|[-+][0-9]{1,2}(?::[0-9]{2})?))?)$"#,
    ].map { try! NSRegularExpression(pattern: $0) }
    static func matches(_ pattern: Int, _ text: String) -> Bool {
        patterns[pattern].firstMatch(in: text, range: NSRange(text.startIndex..., in: text)) != nil
    }
    static func scalar(_ text: String, quoted: Bool = false, tag: String? = nil) throws -> Value {
        let explicit: String?
        if let tag, tag != "!" {
            guard tag.hasPrefix("tag:yaml.org,2002:") else { throw ClowderError("Unsupported YAML tag \(tag)") }
            explicit = tag.removingPrefix("tag:yaml.org,2002:")
        } else {
            explicit = nil
        }
        if explicit == "str" || (explicit == nil && quoted) { return .string(text) }
        if explicit == "null" || (explicit == nil && ["", "~", "null", "Null", "NULL"].contains(text)) { return .null }
        let booleanText = explicit == "bool" ? text.lowercased() : text
        let positive = ["yes", "Yes", "YES", "true", "True", "TRUE", "on", "On", "ON"].contains(booleanText)
        let negative = ["no", "No", "NO", "false", "False", "FALSE", "off", "Off", "OFF"].contains(booleanText)
        if explicit == "bool" || (explicit == nil && (positive || negative)) {
            if !positive && !negative { throw ClowderError("Invalid YAML boolean: \(text)") }
            return .bool(positive)
        }
        if explicit == "int" || (explicit == nil && matches(0, text)) {
            var digits = text.replacingOccurrences(of: "_", with: "")
            var negative = digits.hasPrefix("-")
            if digits.hasPrefix("-") || digits.hasPrefix("+") { digits.removeFirst() }
            var number: UInt64?
            if digits.contains(":") {
                var result: UInt64 = 0
                for part in digits.split(separator: ":", omittingEmptySubsequences: false) {
                    guard let part = UInt64(part) else { throw ClowderError("Invalid YAML integer") }
                    let (product, overflow) = result.multipliedReportingOverflow(by: 60)
                    let (sum, additionOverflow) = product.addingReportingOverflow(part)
                    if overflow || additionOverflow { throw ClowderError("YAML integer is out of range") }
                    result = sum
                }
                number = result
            } else {
                var radix = 10
                if digits.hasPrefix("0b") {
                    radix = 2
                    digits = String(digits.dropFirst(2))
                } else if digits.hasPrefix("0x") {
                    radix = 16
                    digits = String(digits.dropFirst(2))
                } else if digits.count > 1 && digits.hasPrefix("0") {
                    radix = 8
                    digits = String(digits.dropFirst(digits.hasPrefix("0o") ? 2 : 1))
                }
                if digits.hasPrefix("-") {
                    negative.toggle()
                    digits.removeFirst()
                }
                number = UInt64(digits, radix: radix)
            }
            guard let number else { throw ClowderError("YAML integer is out of range") }
            if negative {
                if number == UInt64(Int64.max) + 1 { return .int(Int64.min) }
                guard let signed = Int64(exactly: number) else { throw ClowderError("YAML integer is out of range") }
                return .int(-signed)
            }
            return Int64(exactly: number).map(Value.int) ?? .uint(number)
        }
        if explicit == "float" || (explicit == nil && matches(1, text)) {
            let digits = text.replacingOccurrences(of: "_", with: "")
            let value: Double?
            if digits.contains(":") {
                let sign = digits.hasPrefix("-") ? -1.0 : 1.0
                let parts = digits.removingPrefix("-").removingPrefix("+").split(
                    separator: ":", omittingEmptySubsequences: false)
                var result = 0.0
                for part in parts {
                    guard let part = Double(part) else { throw ClowderError("Invalid YAML float") }
                    result = result * 60 + part
                }
                value = result * sign
            } else {
                value = Double(digits)
            }
            guard let value, value.isFinite else { throw ClowderError("Manifest numbers must be finite") }
            return .double(value)
        }
        if explicit == "timestamp" || (explicit == nil && matches(2, text)) {
            throw ClowderError("YAML timestamps are not manifest values; quote '\(text)' to use it as a string")
        }
        if let explicit { throw ClowderError("Unsupported YAML scalar tag: \(explicit)") }
        if ["<<", "="].contains(text) {
            throw ClowderError("YAML value '\(text)' must be quoted to use it as a string")
        }
        return .string(text)
    }
    static func stringify(_ value: Value) throws -> String {
        func node(_ value: Value) -> Node {
            switch value {
            case .object(let object):
                return .mapping(.init(object.pairs.map { (node(.string($0.0)), node($0.1)) }, Tag(.map), .block))
            case .array(let array): return .sequence(.init(array.map(node), Tag(.seq), .block))
            case .string(let text):
                let plain = (try? scalar(text)) == .string(text)
                return .scalar(.init(text, Tag(.str), plain ? .any : .singleQuoted))
            case .null: return .scalar(.init("null", Tag(.null), .plain))
            case .bool: return .scalar(.init(value.scalar, Tag(.bool), .plain))
            case .int, .uint: return .scalar(.init(value.scalar, Tag(.int), .plain))
            case .double: return .scalar(.init(value.scalar, Tag(.float), .plain))
            }
        }
        return try Yams.serialize(node: node(value), indent: 2, width: -1, allowUnicode: true)
    }
}
