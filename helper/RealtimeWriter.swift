// Candidate-only Mac writer. Never publishes, replaces, or reads recording.mp4.
// The ABI and lifecycle are documented in docs/issue-99-mac-writer-abi.md.
import AVFoundation
import CoreImage
import CoreMedia
import CoreVideo
import Foundation

private enum WriterResult: Int32 {
  case ok = 0
  case invalid = 1
  case overloaded = 2
  case timestamp = 3
  case encoder = 4
  case finished = 5
}

private final class RealtimeWriter {
  static let maxPendingSamples = 8
  private let queue = DispatchQueue(label: "dev.apparent.realtime-writer", qos: .utility)
  private let lock = NSLock()
  private let writer: AVAssetWriter
  private let input: AVAssetWriterInput
  private let adaptor: AVAssetWriterInputPixelBufferAdaptor
  private let context = CIContext(options: [.useSoftwareRenderer: false])
  private let width: Int
  private let height: Int
  private var pending = 0
  private var lastQueuedMs: Int64 = -1
  private var lastWrittenMs: Int64 = -1
  private var error: WriterResult = .ok
  private var closed = false
  private var finalQueued = false

  init?(path: String, width: Int, height: Int) {
    guard width > 0, height > 0, width <= 16384, height <= 16384,
      width % 2 == 0, height % 2 == 0,
      width <= 64_000_000 / height,
      !FileManager.default.fileExists(atPath: path) else { return nil }
    self.width = width
    self.height = height
    guard let asset = try? AVAssetWriter(outputURL: URL(fileURLWithPath: path), fileType: .mp4) else {
      return nil
    }
    writer = asset
    input = AVAssetWriterInput(mediaType: .video, outputSettings: [
      AVVideoCodecKey: AVVideoCodecType.h264,
      AVVideoWidthKey: width,
      AVVideoHeightKey: height,
      AVVideoCompressionPropertiesKey: [AVVideoAverageBitRateKey: 2_000_000,
        AVVideoAllowFrameReorderingKey: false],
    ])
    input.expectsMediaDataInRealTime = true
    adaptor = AVAssetWriterInputPixelBufferAdaptor(assetWriterInput: input,
      sourcePixelBufferAttributes: [
        kCVPixelBufferPixelFormatTypeKey as String: kCVPixelFormatType_32BGRA,
        kCVPixelBufferWidthKey as String: width,
        kCVPixelBufferHeightKey as String: height,
      ])
    guard writer.canAdd(input) else { return nil }
    writer.add(input)
    guard writer.startWriting() else { return nil }
    writer.startSession(atSourceTime: .zero)
  }

  // Called only with the lock held. A callback never waits for encoder readiness.
  private func reserve(_ ms: Int64, final: Bool) -> WriterResult {
    if closed { return .finished }
    if finalQueued {
      error = .timestamp
      return error
    }
    if error != .ok { return error }
    if ms < 0 || ms <= lastQueuedMs {
      error = .timestamp
      return error
    }
    if pending >= Self.maxPendingSamples {
      error = .overloaded
      return error
    }
    pending += 1
    lastQueuedMs = ms
    if final { finalQueued = true }
    return .ok
  }

  private func poison(_ result: WriterResult) {
    lock.lock()
    if error == .ok { error = result }
    lock.unlock()
  }

  func enqueue(_ sample: CMSampleBuffer, at ms: Int64) -> Int32 {
    lock.lock()
    // Check image presence before retaining; rejected callbacks cannot leave a green candidate.
    if CMSampleBufferGetImageBuffer(sample) == nil {
      if error == .ok { error = .invalid }
      lock.unlock()
      return WriterResult.invalid.rawValue
    }
    let result = reserve(ms, final: false)
    if result == .ok {
      let retained = Unmanaged.passUnretained(sample).retain()
      queue.async { [self] in
        defer {
          retained.release()
          lock.lock()
          pending -= 1
          lock.unlock()
        }
        guard let image = CMSampleBufferGetImageBuffer(retained.takeUnretainedValue()) else {
          poison(.invalid)
          return
        }
        append(image, at: ms)
      }
    }
    lock.unlock()
    return result.rawValue
  }

  func enqueueFinal(_ rgba: UnsafeRawPointer?, width sourceWidth: Int, height sourceHeight: Int,
                    at ms: Int64) -> Int32 {
    guard let rgba, sourceWidth > 0, sourceHeight > 0,
      sourceWidth <= 16384, sourceHeight <= 16384,
      sourceWidth <= 64_000_000 / sourceHeight else {
      poison(.invalid)
      return WriterResult.invalid.rawValue
    }
    let bytes = sourceWidth * sourceHeight * 4
    lock.lock()
    let result = reserve(ms, final: true)
    lock.unlock()
    if result == .ok {
      // Earlier sample callbacks enqueue while holding the lock. Once final is
      // reserved, later samples are refused. Copy outside the callback lock.
      // The Rust screenshot allocation may disappear when this call returns.
      let pixels = Data(bytes: rgba, count: bytes)
      queue.async { [self] in
        defer {
          lock.lock()
          pending -= 1
          lock.unlock()
        }
        guard let source = makeBuffer(width: sourceWidth, height: sourceHeight) else {
          poison(.encoder)
          return
        }
        guard CVPixelBufferLockBaseAddress(source, []) == kCVReturnSuccess else {
          poison(.encoder)
          return
        }
        if let base = CVPixelBufferGetBaseAddress(source) {
          pixels.withUnsafeBytes { raw in
            if let bytes = raw.baseAddress {
              for y in 0..<sourceHeight {
                let row = base.advanced(by: y * CVPixelBufferGetBytesPerRow(source))
                  .assumingMemoryBound(to: UInt8.self)
                let src = bytes.advanced(by: y * sourceWidth * 4).assumingMemoryBound(to: UInt8.self)
                for x in 0..<sourceWidth {
                  row[x * 4] = src[x * 4 + 2] // RGBA -> BGRA
                  row[x * 4 + 1] = src[x * 4 + 1]
                  row[x * 4 + 2] = src[x * 4]
                  row[x * 4 + 3] = src[x * 4 + 3]
                }
              }
            }
          }
        } else {
          poison(.encoder)
        }
        CVPixelBufferUnlockBaseAddress(source, [])
        append(source, at: ms)
      }
    }
    return result.rawValue
  }

  private func makeBuffer(width: Int, height: Int) -> CVPixelBuffer? {
    var output: CVPixelBuffer?
    let status = CVPixelBufferCreate(kCFAllocatorDefault, width, height,
      kCVPixelFormatType_32BGRA,
      [kCVPixelBufferIOSurfacePropertiesKey as String: [String: Any]()] as CFDictionary,
      &output)
    return status == kCVReturnSuccess ? output : nil
  }

  private func append(_ source: CVPixelBuffer, at ms: Int64) {
    lock.lock()
    let failed = error != .ok
    lock.unlock()
    if failed { return }
    guard ms > lastWrittenMs, writer.status == .writing,
      let canvas = makeBuffer(width: width, height: height) else {
      poison(.encoder)
      return
    }
    // Render only the supplied frame into the fixed canvas. Target selection and
    // exclusion are Rust's responsibility; this writer never reopens a display.
    let frame = CIImage(cvPixelBuffer: source)
    let resized = frame.transformed(by: CGAffineTransform(
      scaleX: CGFloat(width) / CGFloat(CVPixelBufferGetWidth(source)),
      y: CGFloat(height) / CGFloat(CVPixelBufferGetHeight(source))))
    context.render(resized, to: canvas, bounds: CGRect(x: 0, y: 0,
      width: CGFloat(width), height: CGFloat(height)),
      colorSpace: CGColorSpaceCreateDeviceRGB())
    let deadline = Date().addingTimeInterval(2)
    while !input.isReadyForMoreMediaData && writer.status == .writing && Date() < deadline {
      Thread.sleep(forTimeInterval: 0.005) // encoder queue only, never SCK callback
    }
    guard input.isReadyForMoreMediaData,
      adaptor.append(canvas, withPresentationTime: CMTime(value: ms, timescale: 1000)) else {
      poison(.encoder)
      return
    }
    lastWrittenMs = ms
  }

  func cancelIfOpen() {
    lock.lock()
    let shouldCancel = !closed
    closed = true
    lock.unlock()
    if shouldCancel { writer.cancelWriting() }
  }

  func finish() -> Int32 {
    lock.lock()
    if closed {
      lock.unlock()
      return WriterResult.finished.rawValue
    }
    closed = true
    lock.unlock()
    // Caller must first stop/drain SCK callbacks. sync is the sample/final ordering barrier.
    return queue.sync {
      // With sparse window frames, AVAssetWriter can give the last packet the
      // preceding multi-second gap as its duration. Bound the session to the
      // actual final image instead of exporting a movie that runs long.
      if lastWrittenMs >= 0 {
        writer.endSession(atSourceTime: CMTime(value: lastWrittenMs + 17, timescale: 1000))
      }
      input.markAsFinished()
      let done = DispatchSemaphore(value: 0)
      writer.finishWriting { done.signal() }
      let completed = done.wait(timeout: .now() + 15) == .success
      lock.lock()
      let result = error
      let hasFinal = finalQueued
      lock.unlock()
      if !completed || writer.status != .completed { return WriterResult.encoder.rawValue }
      if result != .ok { return result.rawValue }
      return hasFinal ? WriterResult.ok.rawValue : WriterResult.invalid.rawValue
    }
  }
}

@_cdecl("apparent_writer_create_v1")
public func apparent_writer_create_v1(_ path: UnsafePointer<UInt8>?, _ length: Int,
                                      _ width: Int32, _ height: Int32) -> UnsafeMutableRawPointer? {
  guard let path, length > 0, length <= 4096,
    let name = String(bytes: UnsafeBufferPointer(start: path, count: length), encoding: .utf8),
    name.hasPrefix("/"),
    let session = RealtimeWriter(path: name, width: Int(width), height: Int(height)) else { return nil }
  return Unmanaged.passRetained(session).toOpaque()
}

@_cdecl("apparent_writer_enqueue_sample_v1")
public func apparent_writer_enqueue_sample_v1(_ handle: UnsafeMutableRawPointer?,
                                               _ borrowedSample: UnsafeRawPointer?,
                                               _ captureMs: Int64) -> Int32 {
  guard let handle, let borrowedSample else { return WriterResult.invalid.rawValue }
  let session = Unmanaged<RealtimeWriter>.fromOpaque(handle).takeUnretainedValue()
  let sample = Unmanaged<CMSampleBuffer>.fromOpaque(borrowedSample).takeUnretainedValue()
  return session.enqueue(sample, at: captureMs)
}

@_cdecl("apparent_writer_enqueue_final_rgba_v1")
public func apparent_writer_enqueue_final_rgba_v1(_ handle: UnsafeMutableRawPointer?,
                                                   _ rgba: UnsafeRawPointer?,
                                                   _ width: Int32, _ height: Int32,
                                                   _ ptsMs: Int64) -> Int32 {
  guard let handle else { return WriterResult.invalid.rawValue }
  let session = Unmanaged<RealtimeWriter>.fromOpaque(handle).takeUnretainedValue()
  return session.enqueueFinal(rgba, width: Int(width), height: Int(height), at: ptsMs)
}

@_cdecl("apparent_writer_finish_v1")
public func apparent_writer_finish_v1(_ handle: UnsafeMutableRawPointer?) -> Int32 {
  guard let handle else { return WriterResult.invalid.rawValue }
  return Unmanaged<RealtimeWriter>.fromOpaque(handle).takeUnretainedValue().finish()
}

@_cdecl("apparent_writer_destroy_v1")
public func apparent_writer_destroy_v1(_ handle: UnsafeMutableRawPointer?) {
  guard let handle else { return }
  let session = Unmanaged<RealtimeWriter>.fromOpaque(handle).takeRetainedValue()
  // Destruction must not wait for the serial encoder queue (which can be
  // stalled in append or finish). A failed candidate is discarded by Rust.
  session.cancelIfOpen()
}

// Read both compressed packet timing and decoded pixels, in a bounded tail. Duration
// alone is not proof of an encoded endpoint: an idle movie may contain one old frame.
private func validateTail(_ path: String, _ rgba: UnsafeRawPointer,
                          _ width: Int, _ height: Int, _ stopMs: Int64) -> Bool {
  guard width > 0, height > 0, width <= 16384, height <= 16384,
    width <= 64_000_000 / height, stopMs >= 0 else { return false }
  let asset = AVURLAsset(url: URL(fileURLWithPath: path))
  guard let track = asset.tracks(withMediaType: .video).first else { return false }
  let stop = Double(stopMs) / 1000
  let start = max(0, stop - 2)
  let range = CMTimeRange(start: CMTime(seconds: start, preferredTimescale: 1000),
    duration: CMTime(seconds: min(3, stop - start + 0.5), preferredTimescale: 1000))
  let deadline = Date().addingTimeInterval(8)
  func read(_ settings: [String: Any]?, _ slice: CMTimeRange,
            _ check: (CMSampleBuffer) -> Bool) -> Bool {
    guard let reader = try? AVAssetReader(asset: asset) else { return false }
    let output = AVAssetReaderTrackOutput(track: track, outputSettings: settings)
    output.alwaysCopiesSampleData = false
    guard reader.canAdd(output) else { return false }
    reader.add(output)
    reader.timeRange = slice
    guard reader.startReading() else { return false }
    var count = 0
    while Date() < deadline && count < 180 {
      guard let sample = output.copyNextSampleBuffer() else {
        let ok = reader.status == .completed && count >= 1
        reader.cancelReading()
        return ok
      }
      // AVAssetReader can emit a timing-only marker before the first coded
      // picture. It has no packet DTS or bytes to validate.
      if settings == nil && CMSampleBufferGetNumSamples(sample) == 0 &&
          CMSampleBufferGetTotalSampleSize(sample) == 0 { continue }
      count += 1
      if !check(sample) { reader.cancelReading(); return false }
    }
    reader.cancelReading()
    return false // A busy or stalled tail is inconclusive, never publication authority.
  }
  var lastDts = -Double.infinity
  var packets = 0
  func checkPacket(_ sample: CMSampleBuffer) -> Bool {
    let dts = CMTimeGetSeconds(CMSampleBufferGetDecodeTimeStamp(sample))
    let pts = CMTimeGetSeconds(CMSampleBufferGetPresentationTimeStamp(sample))
    guard dts.isFinite, pts.isFinite, dts > lastDts, pts >= dts,
      dts <= stop + 0.5 else { return false }
    lastDts = dts
    packets += 1
    return true
  }
  // A static SCK window may produce only an initial packet and the explicit
  // stop packet. Reading the last two seconds alone would see just one packet.
  // Check an early bounded slice too. Each reader proves its own packet order;
  // the compressed tail reader may replay packets from an earlier keyframe.
  if stop > 3 {
    let early = CMTimeRange(start: .zero,
      duration: CMTime(seconds: min(2, start - 0.1), preferredTimescale: 1000))
    guard read(nil, early, checkPacket) else { return false }
  }
  // A compressed AVAssetReader tail seek starts decoding at an earlier keyframe,
  // so its first packet can precede packets already read in the early slice.
  // Each reader's own packet order must still increase strictly.
  lastDts = -.infinity
  guard read(nil, range, checkPacket), packets >= 2,
    abs(lastDts - stop) <= 0.35 else { return false }

  var lastPixelBuffer: CVPixelBuffer?
  var lastPts = -Double.infinity
  guard read([kCVPixelBufferPixelFormatTypeKey as String: kCVPixelFormatType_32BGRA], range, { sample in
    let pts = CMTimeGetSeconds(CMSampleBufferGetPresentationTimeStamp(sample))
    guard pts.isFinite, pts > lastPts, pts <= stop + 0.5,
      let pixel = CMSampleBufferGetImageBuffer(sample) else { return false }
    lastPts = pts
    lastPixelBuffer = pixel
    return true
  }), abs(lastPts - stop) <= 0.35, let pixel = lastPixelBuffer,
    CVPixelBufferGetPixelFormatType(pixel) == kCVPixelFormatType_32BGRA,
    CVPixelBufferLockBaseAddress(pixel, .readOnly) == kCVReturnSuccess else { return false }
  defer { CVPixelBufferUnlockBaseAddress(pixel, .readOnly) }
  guard let base = CVPixelBufferGetBaseAddress(pixel) else { return false }
  let cw = CVPixelBufferGetWidth(pixel), ch = CVPixelBufferGetHeight(pixel)
  guard cw > 0, ch > 0 else { return false }
  let stride = CVPixelBufferGetBytesPerRow(pixel)
  let expected = rgba.assumingMemoryBound(to: UInt8.self)
  let actual = base.assumingMemoryBound(to: UInt8.self)
  var error = 0
  var compared = 0
  // Centre of each cell avoids resize-edge sampling. H.264 is lossy: compare
  // RGB on a grid, allowing codec noise but not an unrelated end frame.
  for y in 1..<20 {
    for x in 1..<20 {
      let px = x * cw / 20, py = y * ch / 20
      let sx = min(width - 1, x * width / 20), sy = min(height - 1, y * height / 20)
      let dst = py * stride + px * 4, src = (sy * width + sx) * 4
      for c in 0..<3 {
        let sourceChannel = c == 0 ? 2 : (c == 2 ? 0 : 1)
        error += abs(Int(actual[dst + c]) - Int(expected[src + sourceChannel]))
        compared += 1
      }
    }
  }
  return compared > 0 && error / compared < 22
}

@_cdecl("apparent_writer_validate_v1")
public func apparent_writer_validate_v1(_ path: UnsafePointer<UInt8>?, _ length: Int,
                                        _ rgba: UnsafeRawPointer?, _ width: Int32,
                                        _ height: Int32, _ stopMs: Int64) -> Int32 {
  guard let path, let rgba, length > 0, length <= 4096,
    let name = String(bytes: UnsafeBufferPointer(start: path, count: length), encoding: .utf8),
    name.hasPrefix("/") else { return 1 }
  return validateTail(name, rgba, Int(width), Int(height), stopMs) ? 0 : 4
}
