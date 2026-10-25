import Foundation
import Testing

@testable import ClowderCore

struct ParserTests {
    @Test func yamlReferenceCorpus() throws {
        for name in ["yaml-compatibility.json", "yaml-generated.json"] {
            for test in try fixture(name)["cases"].array! {
                let text = test["yaml"].string!
                if test["error"] == .bool(true) {
                    #expect(throws: (any Error).self, "Accepted invalid YAML: \(text)") { try YAML.parse(text) }
                } else {
                    let value: Value
                    do { value = try YAML.parse(text) } catch {
                        Issue.record("YAML: \(text), error: \(error)")
                        continue
                    }
                    #expect(value == test["value"], "YAML: \(text)")
                    #expect(try YAML.parse(YAML.stringify(value)) == value, "Round trip: \(text)")
                }
            }
        }
    }
    @Test func configReferenceCorpus() throws {
        for name in ["config-compatibility.json", "config-generated.json"] {
            for test in try fixture(name)["cases"].array! {
                let text = test["ini"].string!
                if test["parse_error"] == .bool(true) {
                    #expect(throws: (any Error).self, "Accepted invalid INI: \(text)") { try Config.parse(text) }
                    continue
                }
                let config = try Config.parse(text)
                for query in test["queries"].array! {
                    let section = query["section"].string!
                    let key = query["key"].string!
                    if query["error"] == .bool(true) {
                        #expect(throws: (any Error).self, "INI: \(text), \(section).\(key)") {
                            try config.get(section, key)
                        }
                    } else {
                        #expect(
                            try config.get(section, key) == query["value"].string, "INI: \(text), \(section).\(key)")
                    }
                }
                #expect(config.serialized() == test["saved"].string, "INI: \(text)")
            }
        }
    }
    @Test func rejectsDeepOrExpandingYAML() throws {
        #expect(throws: (any Error).self) {
            try YAML.parse(String(repeating: "[", count: 130) + "0" + String(repeating: "]", count: 130))
        }
        var text = "v0: &v0 [a, b, c, d]\n"
        for index in 1...30 { text += "v\(index): &v\(index) [*v\(index - 1), *v\(index - 1)]\n" }
        #expect(throws: (any Error).self) { try YAML.parse(text) }
        var nested = "v0: &v0 [a]\n"
        for index in 1...150 { nested += "v\(index): &v\(index) [*v\(index - 1)]\n" }
        #expect(throws: (any Error).self) { try YAML.parse(nested) }
    }
}
