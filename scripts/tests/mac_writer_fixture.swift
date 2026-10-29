// Standalone issue #99 codec fixture. Not linked into Apparent.
// On macOS: xcrun swiftc -swift-version 5 scripts/tests/mac_writer_fixture.swift -o /tmp/mac-writer-fixture
//           /tmp/mac-writer-fixture /tmp/issue99-writer-fixture.mp4
// Synthetic red stream frames at 0 and 1 s; a fresh green stop pixel at 6 s.
import AVFoundation
import CoreMedia
import CoreVideo
import Foundation

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

// One writer/input/adaptor for both stream samples and the final screenshot.
// This fixture writes synchronously and holds at most one pending pixel buffer.
// A production SCK callback must enqueue a retained buffer instead of waiting here.
private final class FixtureWriter {
  let writer: AVAssetWriter
  let input: AVAssetWriterInput
  let adaptor: AVAssetWriterInputPixelBufferAdaptor
  private var lastMs: Int64 = -1

  init(url: URL) throws {
    writer = try AVAssetWriter(outputURL: url, fileType: .mp4)
    input = AVAssetWriterInput(mediaType: .video, outputSettings: [
      AVVideoCodecKey: AVVideoCodecType.h264,
      AVVideoWidthKey: width,
      AVVideoHeightKey: height,
      AVVideoCompressionPropertiesKey: [AVVideoAverageBitRateKey: 800_000,
        AVVideoAllowFrameReorderingKey: false],
    ])
    input.expectsMediaDataInRealTime = true
    adaptor = AVAssetWriterInputPixelBufferAdaptor(assetWriterInput: input,
      sourcePixelBufferAttributes: [
        kCVPixelBufferPixelFormatTypeKey as String: kCVPixelFormatType_32BGRA,
        kCVPixelBufferWidthKey as String: width,
        kCVPixelBufferHeightKey as String: height,
      ])
    writer.add(input)
    try require(writer.startWriting(), "writer start failed: \(String(describing: writer.error))")
    writer.startSession(atSourceTime: .zero)
  }

  func stream(_ frame: CMSampleBuffer) throws {
    guard let buffer = CMSampleBufferGetImageBuffer(frame) else {
      throw FixtureError.failed("stream sample has no image")
    }
    let time = CMSampleBufferGetPresentationTimeStamp(frame)
    try append(buffer, at: time)
  }

  func stopPixels(_ buffer: CVPixelBuffer, at time: CMTime) throws {
    try append(buffer, at: time)
  }

  private func append(_ buffer: CVPixelBuffer, at time: CMTime) throws {
    try require(time.isValid && !time.isIndefinite, "invalid frame time")
    try require(CVPixelBufferGetWidth(buffer) == width && CVPixelBufferGetHeight(buffer) == height,
      "canvas changed")
    let ms = CMTimeConvertScale(time, timescale: 1000, method: .roundTowardZero).value
    try require(ms > lastMs, "timestamps must increase")
    let deadline = Date().addingTimeInterval(5)
    while !input.isReadyForMoreMediaData && writer.status == .writing && Date() < deadline {
      Thread.sleep(forTimeInterval: 0.005)
    }
    try require(input.isReadyForMoreMediaData, "encoder stalled: \(String(describing: writer.error))")
    try require(adaptor.append(buffer, withPresentationTime: time),
      "append failed: \(String(describing: writer.error))")
    lastMs = ms
  }

  func finish() throws {
    input.markAsFinished()
    let done = DispatchSemaphore(value: 0)
    writer.finishWriting { done.signal() }
    try require(done.wait(timeout: .now() + 15) == .success, "writer finish timed out")
    try require(writer.status == .completed, "writer finish failed: \(String(describing: writer.error))")
  }
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
  let writer = try FixtureWriter(url: url)
  try writer.stream(sample(pixels(red: true), at: 0))
  try writer.stream(sample(pixels(red: true), at: 1000))
  try writer.stopPixels(pixels(red: false), at: CMTime(value: 6000, timescale: 1000))
  try writer.finish()
  try verify(url)
} catch {
  fputs("FAIL: \(error)\n", stderr)
  exit(1)
}
