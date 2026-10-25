import Foundation
import Testing

@testable import ClowderCore

struct ManifestTests {
    @Test func inheritanceAndLivePaths() throws {
        let temporary = try TemporaryWorkspace()
        let source = """
            name: sample
            protocol: https
            defaults:
              remote: origin
              source: gitlab
              git: {submodules: recursive, config: {sample.old: yes, sample.keep: 4}}
              upstream: {remote: upstream, source: github}
            clowder:
              tools:
                path: tools
                groups: [utilities]
                defaults:
                  remote: fork
                  git: {config: {sample.old: null}}
                projects:
                - name: owner/tool
                  source: {url: example.org, protocol: ssh}
                  upstream: other/tool
            """
        let manifest = try Manifest.fromValue(temporary.workspace, YAML.parse(source), protocol: "https")
        let project = try #require(manifest.projects.first)
        #expect(project.url == "git@example.org:owner/tool.git")
        #expect(project.remote == "fork")
        #expect(project.relative == "tools/tool")
        #expect(project.git.recursive)
        #expect(project.git.config["sample.old"] == nil)
        #expect(project.git.config["sample.keep"] == .int(4))
        #expect(project.upstream?.url == "https://github.com/other/tool.git")
        #expect(project.groups.contains("utilities"))
        try FS.mkdir(FS.join(temporary.root, "a"))
        try FileManager.default.createSymbolicLink(atPath: FS.join(temporary.root, "b"), withDestinationPath: "a")
        #expect(throws: ClowderError.self) {
            try Manifest.fromValue(
                temporary.workspace, YAML.parse("name: x\nclowder: [{name: one, path: a}, {name: two, path: b}]"))
        }
        try FS.mkdir(FS.join(temporary.root, "physical/child"))
        try FileManager.default.createSymbolicLink(
            atPath: FS.join(temporary.root, "link"), withDestinationPath: "physical/child")
        #expect(
            try FS.resolve(FS.join(temporary.root, "link/../missing"))
                == FS.join(FS.resolve(temporary.root), "physical/missing"))
    }
    @Test(arguments: [
        "name: x\nclowder: []", "name: x\nclowder: [true]", "name: x\nclowder: [{name: a, branch: main, tag: v1}]",
        "name: x\nclowder: [{name: a, path: /absolute}]", "name: x\nclowder: [{name: a, groups: [tools, tools]}]",
        "name: x\nclowder: [{name: a, git: {depth: -1}}]", "name: x\nclowder: [{name: a, extra: 1}]",
        "name: x\nprotocol: file\nclowder: [a]", "name: x\nsources: {}\nclowder: [a]",
    ]) func schemaRejectsInvalidManifests(_ text: String) throws {
        #expect(Schema.manifest.accepts(try YAML.parse(text)) == false)
    }
}
