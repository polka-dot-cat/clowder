import Foundation
import Testing

@testable import ClowderCore

struct CLITests {
    @Test func capturedOutput() async throws {
        let reference = try fixture("output-compatibility.json")
        for test in reference["cases"].array! {
            let temporary = try TemporaryWorkspace()
            try FS.write(reference["manifest"].string!, FS.join(temporary.root, "clowder.yml"))
            let arguments = test["args"].array!.compactMap(\.string)
            let output = try await ChildProcess.run(
                executable(), arguments, at: temporary.root, environment: isolatedGit)
            #expect(output.code == 0, "\(arguments): \(output.errorText)")
            #expect(output.stderr.isEmpty, "\(arguments): \(output.errorText)")
            #expect(String(decoding: output.stdout, as: UTF8.self) == test["stdout"].string, "Arguments: \(arguments)")
        }
    }
    @Test func argumentGrammar() throws {
        let args = try #require(try CLI.parse(["herd", "--reb", "-j4", "--", "-project"]))
        #expect(args.command == "herd")
        #expect(args.flag("rebase"))
        #expect(args.number("jobs") == 4)
        #expect(args.many("projects") == ["-project"])
        #expect(throws: ClowderError.self) { try CLI.parse(["herd", "--branch", "main", "--tag", "v1"]) }
        #expect(throws: ClowderError.self) { try CLI.parse(["herd", "--jobs", "no"]) }
    }
}
