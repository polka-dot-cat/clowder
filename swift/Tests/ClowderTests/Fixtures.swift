import Foundation

@testable import ClowderCore

let repository = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
    .deletingLastPathComponent().deletingLastPathComponent().path
func fixture(_ name: String) throws -> Value {
    try Value.parseJSON(FS.read(FS.join(repository, "tests/data/rust/" + name)))
}
final class TemporaryWorkspace {
    let root = FS.join(NSTemporaryDirectory(), "clowder-swift-test-" + UUID().uuidString)
    var workspace: Workspace { Workspace(root: root, found: true) }
    init() throws { try FS.mkdir(root) }
    deinit { try? FS.remove(root) }
}
func executable() -> String {
    if let value = ProcessInfo.processInfo.environment["CLOWDER_SWIFT_EXECUTABLE"] { return value }
    #if DEBUG
        return FS.join(repository, "swift/.build/debug/clowder")
    #else
        return FS.join(repository, "swift/.build/release/clowder")
    #endif
}
let isolatedGit: [String: String?] = [
    "GIT_CONFIG_GLOBAL": "/dev/null", "GIT_CONFIG_SYSTEM": "/dev/null", "GIT_TERMINAL_PROMPT": "0",
    "CLOWDER_COMPLETE": nil,
]
