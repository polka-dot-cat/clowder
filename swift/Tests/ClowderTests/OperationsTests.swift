import Foundation
import Testing

@testable import ClowderCore

struct OperationsTests {
    @Test func rejectsNULBeforeLaunchingProcess() async throws {
        await #expect(throws: ClowderError.self) {
            try await ChildProcess.run("sh", ["-c", "exit 0\0ignored"], at: "/tmp")
        }
        #expect(throws: ClowderError.self) { try FS.resolve("/tmp\0/ignored") }
    }

    func git(_ path: String, _ arguments: [String]) async throws -> String {
        let result = try await ChildProcess.run(
            "git",
            [
                "-c", "commit.gpgsign=false", "-c", "core.hooksPath=/dev/null", "-c", "user.name=Swift Test", "-c",
                "user.email=test@example.invalid",
            ] + arguments, at: path, environment: isolatedGit)
        try #require(result.code == 0, "Git: \(result.errorText)")
        return result.text
    }
    func project(_ root: String, _ name: String) async throws {
        let path = FS.join(root, name)
        try FS.mkdir(path)
        _ = try await git(path, ["init", "--initial-branch=main"])
        try FS.write("original\n", FS.join(path, "tracked.txt"))
        _ = try await git(path, ["add", "tracked.txt"])
        _ = try await git(path, ["commit", "-m", "Initial"])
    }
    @Test func concurrentWorkersAndFailureCodes() async throws {
        let temporary = try TemporaryWorkspace()
        let root = temporary.root
        try FS.write(
            "name: local\ndefaults: {branch: main}\nclowder: [owner/alpha, owner/beta]\n", FS.join(root, "clowder.yml"))
        for name in ["alpha", "beta"] { try await project(root, name) }
        let command =
            #"printf '%s\n' "$PROJECT_NAME|$PROJECT_PATH|$PROJECT_REF" > identity; touch "$CLOWDER_PATH/$(basename "$PROJECT_PATH").ready"; n=0; while [ ! -f "$CLOWDER_PATH/alpha.ready" ] || [ ! -f "$CLOWDER_PATH/beta.ready" ]; do n=$((n+1)); [ "$n" -lt 100 ] || exit 77; sleep 0.02; done"#
        let result = try await ChildProcess.run(
            executable(), ["forall", "-j2", command], at: root, environment: isolatedGit)
        #expect(result.code == 0, "\(result.errorText)")
        for name in ["alpha", "beta"] {
            #expect(
                try FS.read(FS.join(root, name + "/identity"))
                    == "owner/\(name)|\(FS.resolve(root))/\(name)|refs/heads/main\n")
        }
        let failed = try await ChildProcess.run(
            executable(), ["forall", "-j2", "exit 42"], at: root, environment: isolatedGit)
        #expect(failed.code == 42)
        let ignored = try await ChildProcess.run(
            executable(), ["forall", "-ij2", "exit 42"], at: root, environment: isolatedGit)
        #expect(ignored.code == 0)
        let binary = try await ChildProcess.run(
            executable(), ["forall", "-j2", #"printf '\377\000\376'"#], at: root, environment: isolatedGit)
        #expect(binary.code == 0)
        #expect(binary.stdout.filter { $0 == 255 }.count == 2)
    }
    @Test func validatesBeforeMutationAndCompletionIsReadOnly() async throws {
        let temporary = try TemporaryWorkspace()
        let root = temporary.root
        try FS.write(
            "name: local\ndefaults: {branch: main}\nclowder: [owner/alpha, owner/beta]\n", FS.join(root, "clowder.yml"))
        for name in ["alpha", "beta"] { try await project(root, name) }
        try FS.write("changed\n", FS.join(root, "beta/tracked.txt"))
        let start = try await ChildProcess.run(executable(), ["start", "feature"], at: root, environment: isolatedGit)
        #expect(start.code != 0)
        #expect(try await git(FS.join(root, "alpha"), ["branch", "--list", "feature"]).isEmpty)
        let complete = try await ChildProcess.run(
            executable(), ["herd", "owner/"], at: root,
            environment: isolatedGit.merging(["CLOWDER_COMPLETE": "1"]) { _, new in new })
        #expect(complete.code == 0)
        #expect(complete.text == "owner/alpha\nowner/beta")
        #expect(try await git(FS.join(root, "beta"), ["status", "--porcelain"]) == " M tracked.txt")
    }
}
