// Native issue #99 writer/validator ABI fixture, linked against the Cargo-built dylib.
// On macOS, after cargo build: xcrun swiftc -swift-version 5 \
//   scripts/tests/mac_writer_fixture.swift -L target/debug -lapparent_realtime_writer \
//   -o "$TMPDIR/mac-writer-fixture"
//   DYLD_LIBRARY_PATH="$PWD/target/debug" "$TMPDIR/mac-writer-fixture" "$TMPDIR/issue99-writer-fixture.mp4"
// Synthetic red stream frames at 0 and 1 s; a fresh green stop pixel at 6 s.
import AVFoundation
import CoreMedia
import CoreVideo
import Foundation

@_silgen_name("apparent_writer_create_v1")
private func createWriter(_ path: UnsafePointer<UInt8>?, _ length: Int,
                          _ width: Int32, _ height: Int32) -> UnsafeMutableRawPointer?
@_silgen_name("apparent_writer_enqueue_sample_v1")
private func enqueueSample(_ handle: UnsafeMutableRawPointer?, _ sample: UnsafeRawPointer?,
                           _ captureMs: Int64) -> Int32
@_silgen_name("apparent_writer_enqueue_final_rgba_v1")
private func enqueueFinal(_ handle: UnsafeMutableRawPointer?, _ rgba: UnsafeRawPointer?,
                          _ width: Int32, _ height: Int32, _ stopMs: Int64) -> Int32
@_silgen_name("apparent_writer_finish_v1")
private func finishWriter(_ handle: UnsafeMutableRawPointer?) -> Int32
@_silgen_name("apparent_writer_destroy_v1")
private func destroyWriter(_ handle: UnsafeMutableRawPointer?)
@_silgen_name("apparent_writer_validate_v1")
private func validateWriter(_ path: UnsafePointer<UInt8>?, _ length: Int,
                            _ rgba: UnsafeRawPointer?, _ width: Int32,
                            _ height: Int32, _ stopMs: Int64) -> Int32

private enum FixtureError: Error {
  case failed(String)
}

private func require(_ condition: @autoclosure () -> Bool, _ message: String) throws {
  if !condition() { throw FixtureError.failed(message) }
}

private let width = 320
private let height = 180

private func pixels(red: Bool) throws -> CVPixelBuffer {
  var output: CVPixelBuffer?
  let status = CVPixelBufferCreate(kCFAllocatorDefault, width, height,
    kCVPixelFormatType_32BGRA,
    [kCVPixelBufferIOSurfacePropertiesKey as String: [String: Any]()] as CFDictionary, &output)
  try require(status == kCVReturnSuccess && output != nil, "could not allocate pixels: \(status)")
  let buffer = output!
  try require(CVPixelBufferLockBaseAddress(buffer, []) == kCVReturnSuccess, "lock failed")
  defer { CVPixelBufferUnlockBaseAddress(buffer, []) }
  guard let base = CVPixelBufferGetBaseAddress(buffer) else {
    throw FixtureError.failed("no pixel data")
  }
  let stride = CVPixelBufferGetBytesPerRow(buffer)
  for y in 0..<height {
    let row = base.advanced(by: y * stride).assumingMemoryBound(to: UInt8.self)
    for x in 0..<width {
      row[x * 4] = 0
      row[x * 4 + 1] = red ? 0 : 255
      row[x * 4 + 2] = red ? 255 : 0
      row[x * 4 + 3] = 255
    }
  }
  return buffer
}

private func sample(_ buffer: CVPixelBuffer, at ms: Int64) throws -> CMSampleBuffer {
  var format: CMVideoFormatDescription?
  let formatStatus = CMVideoFormatDescriptionCreateForImageBuffer(
    allocator: kCFAllocatorDefault, imageBuffer: buffer, formatDescriptionOut: &format)
  try require(formatStatus == noErr && format != nil, "format description failed")
  var timing = CMSampleTimingInfo(duration: .invalid,
    presentationTimeStamp: CMTime(value: ms, timescale: 1000),
    decodeTimeStamp: .invalid)
  var result: CMSampleBuffer?
  let status = CMSampleBufferCreateReadyWithImageBuffer(allocator: kCFAllocatorDefault,
    imageBuffer: buffer, formatDescription: format!, sampleTiming: &timing,
    sampleBufferOut: &result)
  try require(status == noErr && result != nil, "sample buffer failed")
  return result!
}

private func greenRGBA() -> [UInt8] {
  var bytes = [UInt8](repeating: 0, count: width * height * 4)
  for pixel in stride(from: 0, to: bytes.count, by: 4) {
    bytes[pixel + 1] = 255
    bytes[pixel + 3] = 255
  }
  return bytes
}

private func verify(_ url: URL) throws {
  let asset = AVURLAsset(url: url)
  guard let track = asset.tracks(withMediaType: .video).first else {
    throw FixtureError.failed("no video track")
  }
  let reader = try AVAssetReader(asset: asset)
  let output = AVAssetReaderTrackOutput(track: track, outputSettings: [
    kCVPixelBufferPixelFormatTypeKey as String: kCVPixelFormatType_32BGRA,
  ])
  reader.add(output)
  try require(reader.startReading(), "reader failed: \(String(describing: reader.error))")
  var count = 0
  var lastTime: CMTime = .invalid
  var lastGreen = false
  while let frame = output.copyNextSampleBuffer() {
    count += 1
    let time = CMSampleBufferGetPresentationTimeStamp(frame)
    try require(!lastTime.isValid || CMTimeCompare(time, lastTime) > 0, "decoded times regressed")
    lastTime = time
    guard let pixel = CMSampleBufferGetImageBuffer(frame) else {
      throw FixtureError.failed("decoded sample has no image")
    }
    try require(CVPixelBufferLockBaseAddress(pixel, .readOnly) == kCVReturnSuccess, "decode lock failed")
    guard let base = CVPixelBufferGetBaseAddress(pixel) else {
      CVPixelBufferUnlockBaseAddress(pixel, .readOnly)
      throw FixtureError.failed("decoded sample has no pixels")
    }
    let center = base.advanced(by: (height / 2) * CVPixelBufferGetBytesPerRow(pixel) +
      (width / 2) * 4).assumingMemoryBound(to: UInt8.self)
    lastGreen = center[1] > 150 && center[2] < 100
    CVPixelBufferUnlockBaseAddress(pixel, .readOnly)
  }
  try require(reader.status == .completed, "decode failed: \(String(describing: reader.error))")
  try require(count == 3, "expected 3 decoded frames, got \(count)")
  try require(lastTime.isValid && CMTimeGetSeconds(lastTime) >= 5.95,
    "stop frame did not reach 6 seconds")
  try require(lastGreen, "final decoded pixels are not the fresh stop image")
  print("PASS: \(count) decoded frames, final time \(CMTimeGetSeconds(lastTime))s, green stop pixels")
}

do {
  guard CommandLine.arguments.count == 2 else {
    throw FixtureError.failed("usage: mac-writer-fixture OUTPUT.mp4 (must not exist)")
  }
  let url = URL(fileURLWithPath: CommandLine.arguments[1])
  try require(!FileManager.default.fileExists(atPath: url.path), "output already exists")
  let path = Array(url.path.utf8)
  let writer = path.withUnsafeBufferPointer {
    createWriter($0.baseAddress, $0.count, Int32(width), Int32(height))
  }
  try require(writer != nil, "production writer creation failed")
  defer { destroyWriter(writer) }
  for ms in [Int64(0), 1000] {
    let frame = try sample(pixels(red: true), at: ms)
    try require(enqueueSample(writer, Unmanaged.passUnretained(frame).toOpaque(), ms) == 0,
      "production sample enqueue failed at \(ms) ms")
  }
  let final = greenRGBA()
  try final.withUnsafeBytes { raw in
    try require(enqueueFinal(writer, raw.baseAddress, Int32(width), Int32(height), 6000) == 0,
      "production stop image enqueue failed")
  }
  try require(finishWriter(writer) == 0, "production writer finish failed")
  try path.withUnsafeBufferPointer { pathBytes in
    try final.withUnsafeBytes { rgba in
      try require(validateWriter(pathBytes.baseAddress, pathBytes.count, rgba.baseAddress,
        Int32(width), Int32(height), 6000) == 0,
        "production tail validation failed")
    }
  }
  try verify(url)
} catch {
  fputs("FAIL: \(error)\n", stderr)
  exit(1)
}
