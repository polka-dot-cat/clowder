import ClowderCore

#if canImport(Darwin)
    import Darwin
#else
    import Glibc
#endif

@main
struct Clowder {
    static func main() async { exit(await Application.run(Array(CommandLine.arguments.dropFirst()))) }
}
