import CClowder
import Foundation

#if canImport(Darwin)
    import Darwin
#else
    import Glibc
#endif

struct ProcessResult: Sendable {
    var code: Int32
    var stdout: Data
    var stderr: Data
    var text: String { String(decoding: stdout, as: UTF8.self).trimEnd }
    var errorText: String { String(decoding: stderr, as: UTF8.self).trimEnd }
}

enum ChildProcess {
    enum Output: Sendable {
        case capture, inherit
        case combined(String)
    }
    static func initialize() throws {
        let error = clowder_install_signals()
        if error != 0 { throw ClowderError("Cannot install signal handlers: \(String(cString: strerror(error)))") }
    }
    static func check() throws {
        let signal = clowder_cancelled()
        if signal != 0 { throw ClowderError("Command exited with status \(128 + signal)", code: 128 + signal) }
    }
    static func run(
        _ program: String, _ arguments: [String], at directory: String,
        environment: [String: String?] = [:], output: Output = .capture,
        interactive: Bool = false
    ) async throws -> ProcessResult {
        try check()
        // Blocking POSIX wait and file I/O stay off Swift's cooperative executor.
        // All descriptors and C buffers belong to this one closure.
        return try await withCheckedThrowingContinuation { continuation in
            DispatchQueue.global().async {
                do {
                    continuation.resume(
                        returning: try runBlocking(program, arguments, directory, environment, output, interactive))
                } catch { continuation.resume(throwing: error) }
            }
        }
    }
    private static func runBlocking(
        _ program: String, _ arguments: [String], _ directory: String,
        _ overrides: [String: String?], _ output: Output, _ interactive: Bool
    ) throws -> ProcessResult {
        var environment = ProcessInfo.processInfo.environment
        for (key, value) in overrides { environment[key] = value }
        if ([program, directory] + arguments + Array(environment.keys) + Array(environment.values)).contains(where: {
            $0.contains("\0")
        }) {
            throw ClowderError("Process arguments and environment cannot contain NUL characters")
        }
        let argv = ([program] + arguments).map { strdup($0) } + [nil]
        let envp = environment.map { strdup("\($0.key)=\($0.value)") } + [nil]
        defer {
            for pointer in argv { free(pointer) }
            for pointer in envp { free(pointer) }
        }
        var handles: [FileHandle] = []
        defer { for handle in handles { try? handle.close() } }
        var input: Int32 = -1
        var out: Int32 = -1
        var err: Int32 = -1
        var outFile: FileHandle?
        var errFile: FileHandle?
        switch output {
        case .capture:
            outFile = try temporaryHandle()
            errFile = try temporaryHandle()
            handles += [outFile!, errFile!]
            out = outFile!.fileDescriptor
            err = errFile!.fileDescriptor
            input = open("/dev/null", O_RDONLY | O_CLOEXEC)
        case .combined(let path):
            let fd = open(path, O_WRONLY | O_CREAT | O_EXCL | O_CLOEXEC, 0o600)
            if fd < 0 { throw ClowderError("Cannot create command log: \(path)") }
            handles.append(FileHandle(fileDescriptor: fd, closeOnDealloc: false))
            out = fd
            err = fd
            input = open("/dev/null", O_RDONLY | O_CLOEXEC)
        case .inherit: break
        }
        defer { if input >= 0 { close(input) } }
        var code: Int32 = 1
        let error = clowder_run(program, argv, envp, directory, input, out, err, interactive ? 1 : 0, &code)
        if error != 0 {
            throw ClowderError("Failed to run \(program) in \(directory): \(String(cString: strerror(error)))")
        }
        try check()
        func read(_ handle: FileHandle?) throws -> Data {
            guard let handle else { return Data() }
            try handle.seek(toOffset: 0)
            return try handle.readToEnd() ?? Data()
        }
        return try ProcessResult(code: code, stdout: read(outFile), stderr: read(errFile))
    }
    private static func temporaryHandle() throws -> FileHandle {
        var template = Array((NSTemporaryDirectory() + "clowder-swift-XXXXXX").utf8CString)
        let fd = mkstemp(&template)
        if fd < 0 { throw ClowderError("Cannot create temporary output file") }
        template.withUnsafeBufferPointer { _ = unlink($0.baseAddress!) }
        _ = fcntl(fd, F_SETFD, FD_CLOEXEC)
        return FileHandle(fileDescriptor: fd, closeOnDealloc: false)
    }
}
