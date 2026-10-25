import Foundation

struct ClowderError: Error, CustomStringConvertible, Sendable {
    var description: String
    var code: Int32 = 1
    init(_ message: String, code: Int32 = 1) {
        description = message
        self.code = code
    }
}

struct Object: Sendable, Equatable, ExpressibleByDictionaryLiteral {
    private(set) var keys: [String] = []
    private var values: [String: Value] = [:]
    init() {}
    init(dictionaryLiteral elements: (String, Value)...) {
        for (key, value) in elements { self[key] = value }
    }
    subscript(_ key: String) -> Value? {
        get { values[key] }
        set {
            if let value = newValue {
                if values[key] == nil { keys.append(key) }
                values[key] = value
            } else {
                values.removeValue(forKey: key)
                keys.removeAll { $0 == key }
            }
        }
    }
    var pairs: [(String, Value)] { keys.map { ($0, values[$0]!) } }
    var count: Int { keys.count }
    static func == (a: Object, b: Object) -> Bool { a.values == b.values }
}

indirect enum Value: Sendable, Equatable, Decodable {
    case null
    case bool(Bool)
    case int(Int64)
    case uint(UInt64)
    case double(Double)
    case string(String)
    case array([Value])
    case object(Object)
    init(from decoder: any Decoder) throws {
        let c = try decoder.singleValueContainer()
        if c.decodeNil() {
            self = .null
        } else if let v = try? c.decode(Bool.self) {
            self = .bool(v)
        } else if let v = try? c.decode(Int64.self) {
            self = .int(v)
        } else if let v = try? c.decode(UInt64.self) {
            self = .uint(v)
        } else if let v = try? c.decode(Double.self) {
            self = .double(v)
        } else if let v = try? c.decode(String.self) {
            self = .string(v)
        } else if let v = try? c.decode([Value].self) {
            self = .array(v)
        } else {
            let values = try c.decode([String: Value].self)
            var object = Object()
            for key in values.keys.sorted() { object[key] = values[key] }
            self = .object(object)
        }
    }
    subscript(_ key: String) -> Value { object?[key] ?? .null }
    var string: String? { if case .string(let v) = self { v } else { nil } }
    var array: [Value]? { if case .array(let v) = self { v } else { nil } }
    var object: Object? { if case .object(let v) = self { v } else { nil } }
    var bool: Bool? { if case .bool(let v) = self { v } else { nil } }
    var number: Double? {
        switch self {
        case .int(let v): Double(v)
        case .uint(let v): Double(v)
        case .double(let v): v
        default: nil
        }
    }
    var isNull: Bool { self == .null }
    var scalar: String {
        switch self {
        case .string(let v): v
        case .bool(let v): v ? "true" : "false"
        case .int(let v): String(v)
        case .uint(let v): String(v)
        case .double(let v): String(v)
        case .null: "null"
        default: ""
        }
    }
    static func == (a: Value, b: Value) -> Bool {
        switch (a, b) {
        case (.null, .null): true
        case (.bool(let a), .bool(let b)): a == b
        case (.int(let a), .int(let b)): a == b
        case (.uint(let a), .uint(let b)): a == b
        case (.int(let a), .uint(let b)), (.uint(let b), .int(let a)): UInt64(exactly: a) == b
        case (.double(let a), .double(let b)): a == b
        case (.double(let a), .int(let b)), (.int(let b), .double(let a)): Int64(exactly: a) == b
        case (.double(let a), .uint(let b)), (.uint(let b), .double(let a)): UInt64(exactly: a) == b
        case (.string(let a), .string(let b)): a == b
        case (.array(let a), .array(let b)): a == b
        case (.object(let a), .object(let b)): a == b
        default: false
        }
    }
    static func parseJSON(_ text: String) throws -> Value {
        try JSONDecoder().decode(Value.self, from: Data(text.utf8))
    }
}

func writeOut(_ text: String) { FileHandle.standardOutput.write(Data(text.utf8)) }
func writeError(_ text: String) { FileHandle.standardError.write(Data(text.utf8)) }
extension String {
    var trimmed: String { trimmingCharacters(in: .whitespacesAndNewlines) }
    var trimEnd: String { String(reversed().drop(while: { $0.isWhitespace }).reversed()) }
    var lines: [String] {
        utf8.split(separator: 10, omittingEmptySubsequences: false).map { String(decoding: $0, as: UTF8.self) }
            .dropLast(hasSuffix("\n") ? 1 : 0).map { $0.hasSuffix("\r") ? String($0.dropLast()) : $0 }
    }
    func padded(_ width: Int) -> String { self + String(repeating: " ", count: max(0, width - count)) }
    func removingPrefix(_ prefix: String) -> String { hasPrefix(prefix) ? String(dropFirst(prefix.count)) : self }
}
