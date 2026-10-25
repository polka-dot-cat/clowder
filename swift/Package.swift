// swift-tools-version: 6.2
import PackageDescription

let package = Package(
    name: "ClowderSwift",
    platforms: [.macOS(.v13)],
    products: [.executable(name: "clowder", targets: ["ClowderCLI"])],
    dependencies: [.package(url: "https://github.com/jpsim/Yams.git", exact: "6.2.2")],
    targets: [
        .target(name: "CClowder"),
        .target(name: "ClowderCore", dependencies: ["CClowder", "Yams"]),
        .executableTarget(name: "ClowderCLI", dependencies: ["ClowderCore"]),
        .testTarget(name: "ClowderTests", dependencies: ["ClowderCore"]),
    ]
)
