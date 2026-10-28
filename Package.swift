// swift-tools-version: 6.2
import PackageDescription

let package = Package(
  name: "ApparentKit",
  // macOS only so `swift test` can run the platform-independent parts (the outbox) on a Mac;
  // everything UIKit is `#if os(iOS)`.
  platforms: [.iOS(.v18), .macOS(.v15)],
  products: [
    .library(name: "ApparentKit", targets: ["ApparentKit"]),
  ],
  targets: [
    .target(
      name: "ApparentKit",
      path: "Sources/ApparentKit",
      swiftSettings: [.swiftLanguageMode(.v5)]
    ),
    .testTarget(
      name: "ApparentKitTests",
      dependencies: ["ApparentKit"],
      path: "Tests/ApparentKitTests",
      swiftSettings: [.swiftLanguageMode(.v5)]
    ),
  ]
)
