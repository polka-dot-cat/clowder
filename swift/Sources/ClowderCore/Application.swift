import Foundation

public enum Application {
    public static func run(_ tokens: [String]) async -> Int32 {
        do {
            try ChildProcess.initialize()
            if ProcessInfo.processInfo.environment["CLOWDER_COMPLETE"] == "probe" {
                writeOut("clowder-native-completion-v1\n")
                return 0
            }
            if ProcessInfo.processInfo.environment["CLOWDER_COMPLETE"] == "1" {
                for candidate in await CLI.complete(tokens) { writeOut(candidate + "\n") }
                return 0
            }
            if let args = try CLI.parse(tokens) { try await Commands.run(args) }
            try ChildProcess.check()
            return 0
        } catch let error as ClowderError {
            writeError(error.description + "\n")
            return error.code
        } catch {
            writeError(String(describing: error) + "\n")
            return 1
        }
    }
}
