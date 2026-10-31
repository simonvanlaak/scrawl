@preconcurrency import AVFoundation
import Speech

/// `transcribe-stream`: transcribes a recording while it's made. Reads 16 kHz mono 16-bit
/// little-endian PCM on stdin from the recording's first sample (the samples `audio.wav` will
/// hold), and writes JSON lines: `{"ready":true,"locale","engine"}` once the model is loaded, one
/// `{"segment":{t0,t1,text},"words":[…]}` per finalized result, and after stdin closes and the
/// tail is finalized `{"done":true,"samples":N}`. Times use the original source clock;
/// finite chunks can change recognition/alignment versus a whole-file pass. It never downloads:
/// a model that isn't installed
/// fails at once, and Apparent transcribes the finished recording instead.
/// Before macOS 26 there is no SpeechAnalyzer to stream to, so it fails at once the same way.
enum LiveTranscriber {
  static func run(preferred: String? = nil) async throws {
    #if compiler(>=6.2)
    if #available(macOS 26, *) { return try await stream(preferred: preferred) }
    #endif
    throw HelperError("live transcription needs macOS 26's SpeechAnalyzer")
  }

  #if compiler(>=6.2)
  @available(macOS 26, *)
  private static func stream(preferred: String?) async throws {
    guard let choice = await SpeechModel.choose(preferred: preferred) else {
      throw HelperError("no speech model is available on this Mac")
    }
    let module = choice.module()
    guard await AssetInventory.status(forModules: [module]) == .installed else {
      throw HelperError("the speech model for \(choice.locale.identifier) isn't installed")
    }

    let budget = PendingPCM()
    let (chunks, input) = AsyncThrowingStream<PCMChunk, Error>.makeStream()
    // Worker errors bypass a blocking stdin read. Rust owns hard process-group cleanup.
    let (events, completion) = AsyncThrowingStream<LiveProgress, Error>.makeStream()
    let worker = Task {
      do {
        try await consumeChunks(chunks, budget: budget) { chunk in
          try await transcribeChunk(chunk, choice: choice)
        }
        completion.yield(.chunksDrained)
      } catch { completion.finish(throwing: error) }
    }
    emit(["ready": true, "locale": choice.locale.identifier, "engine": choice.name])
    let reader = Task.detached {
      do {
        var owner = PCMChunker(budget: budget)
        while let bytes = try FileHandle.standardInput.read(upToCount: 1600), !bytes.isEmpty {
          try Task.checkCancellation()
          try owner.append(bytes) { chunk in
            guard case .enqueued = input.yield(chunk) else { throw HelperError("chunk input closed") }
          }
        }
        try Task.checkCancellation()
        try owner.finish { chunk in
          guard case .enqueued = input.yield(chunk) else { throw HelperError("chunk input closed") }
        }
        input.finish()
        completion.yield(.samples(owner.total))
      } catch { input.finish(throwing: error); completion.finish(throwing: error) }
    }
    defer {
      input.finish(); completion.finish(); worker.cancel(); reader.cancel()
    }
    let total = try await finishLiveInput(events)
    emit(["done": true, "samples": total])
  }

  @available(macOS 26, *)
  private static func transcribeChunk(_ chunk: PCMChunk, choice: SpeechChoice) async throws {
    try Task.checkCancellation()
    let module = choice.module()
    guard await AssetInventory.status(forModules: [module]) == .installed else {
      throw HelperError("chunk speech model is not installed")
    }
    let pcm = AVAudioFormat(commonFormat: .pcmFormatInt16, sampleRate: 16000, channels: 1, interleaved: false)!
    let format = await SpeechAnalyzer.bestAvailableAudioFormat(compatibleWith: [module]) ?? pcm
    let analyzer = SpeechAnalyzer(modules: [module], options: SpeechAnalyzer.Options(priority: .userInitiated, modelRetention: .whileInUse))
    let (inputs, continuation) = AsyncThrowingStream<AnalyzerInput, Error>.makeStream()
    let (events, completion) = AsyncThrowingStream<ChunkProgress, Error>.makeStream()
    let results = Task {
      do {
        func collect(_ attributed: AttributedString, _ range: CMTimeRange) throws {
          let text = String(attributed.characters).trimmingCharacters(in: .whitespacesAndNewlines)
          guard !text.isEmpty else { return }
          let times = try sourceTimes(range, chunk: chunk)
          var words: [[String: Any]] = []
          for run in attributed.runs {
            guard let local = run.audioTimeRange else { continue }
            let word = String(attributed[run.range].characters).trimmingCharacters(in: .whitespacesAndNewlines)
            guard !word.isEmpty else { continue }
            let wordTimes = try sourceTimes(local, chunk: chunk)
            words.append(["t0": wordTimes.0, "t1": wordTimes.1, "text": word])
          }
          emit(["segment": ["t0": times.0, "t1": times.1, "text": text], "words": words])
        }
        if let transcriber = module as? SpeechTranscriber {
          for try await result in transcriber.results { try collect(result.text, result.range) }
        } else if let transcriber = module as? DictationTranscriber {
          for try await result in transcriber.results where result.isFinal { try collect(result.text, result.range) }
        } else { throw HelperError("unsupported live speech module") }
        completion.yield(.resultsDrained)
      } catch { completion.finish(throwing: error) }
    }
    var feeder: Task<Void, Never>?
    defer { continuation.finish(); completion.finish(); results.cancel(); feeder?.cancel() }
    do {
      try await analyzer.prepareToAnalyze(in: format)
      try Task.checkCancellation()
      // start returns after attaching input, NOT after input consumption.
      try await analyzer.start(inputSequence: inputs)
      feeder = Task.detached {
        do {
          try convertPCM(chunk.bytes, source: pcm, output: format) { buffer in
            guard case .enqueued = continuation.yield(AnalyzerInput(buffer: buffer)) else {
              throw HelperError("chunk audio input closed")
            }
          }
          continuation.finish()
        } catch { continuation.finish(throwing: error); completion.finish(throwing: error) }
      }
      let terminal = Task {
        do {
          try await analyzer.finalizeAndFinishThroughEndOfInput()
          completion.yield(.terminalFinished)
        } catch { completion.finish(throwing: error) }
      }
      defer { terminal.cancel() }
      try await finishChunk(events)
    } catch {
      // Never join an uncooperative native task; helper exit/Rust hard cleanup bounds it.
      Task { await analyzer.cancelAndFinishNow() }
      throw error
    }
  }

  private static func sourceTimes(_ range: CMTimeRange, chunk: PCMChunk) throws -> (Int, Int) {
    let end = CMTimeRangeGetEnd(range)
    let duration = CMTime(value: Int64(chunk.bytes.count / 2), timescale: 16000)
    guard range.isValid, range.start.isNumeric, end.isNumeric,
          CMTimeCompare(range.start, .zero) >= 0,
          CMTimeCompare(end, range.start) >= 0, CMTimeCompare(end, duration) <= 0 else {
      throw HelperError("invalid chunk-local speech range")
    }
    let origin = CMTime(value: Int64(chunk.origin), timescale: 16000)
    let startMs = (CMTimeAdd(origin, range.start).seconds * 1000).rounded()
    let endMs = (CMTimeAdd(origin, end).seconds * 1000).rounded()
    guard startMs.isFinite, endMs.isFinite, startMs >= 0, endMs < Double(Int.max) else {
      throw HelperError("invalid absolute speech range")
    }
    return (Int(startMs), Int(endMs))
  }
  #endif
}

#if compiler(>=6.2)
private enum LiveProgress { case samples(Int), chunksDrained }
private enum ChunkProgress { case resultsDrained, terminalFinished }

/// Both owners must succeed before done. Errors bypass success even
/// when the reader is still blocked. A request's return is not result delivery.
private func finishLiveInput(_ events: AsyncThrowingStream<LiveProgress, Error>) async throws -> Int {
  var total: Int?
  var chunksDrained = false
  for try await event in events {
    try Task.checkCancellation()
    switch event {
    case .samples(let count): total = count
    case .chunksDrained: chunksDrained = true
    }
    if let total, chunksDrained { return total }
  }
  throw HelperError("live transcription ended without completing its input")
}

private func finishChunk(_ events: AsyncThrowingStream<ChunkProgress, Error>) async throws {
  var drained = false; var finished = false
  for try await event in events {
    try Task.checkCancellation()
    switch event {
    case .resultsDrained: drained = true
    case .terminalFinished: finished = true
    }
    if drained && finished { return }
  }
  throw HelperError("chunk ended without results and terminal drain")
}

/// Fixed 50 ms windows, independent of pipe reads. The 0.005 normalized RMS floor
/// matches audio::speech_islands' lower bound; 150 ms rounds Hex's 120 ms blip
/// floor up to full windows. 700 ms quiet matches Linux's pause policy, without
/// its three-second chunk minimum (short Dictation phrases must finalize too).
/// This is an energy admission rule, not a claim that noise is intelligible speech.
private struct PausePolicy {
  private var count = 0
  private var energy = 0.0
  private var active = 0
  private var quiet = 0
  private var armed = false

  mutating func append(_ sample: Int16) -> Bool {
    let value = Double(sample) / 32768
    energy += value * value
    count += 1
    guard count == 800 else { return false }
    let voiced = energy / 800 >= 0.005 * 0.005
    count = 0; energy = 0
    if voiced {
      active = min(3, active + 1); quiet = 0
      if active == 3 { armed = true }
    } else {
      active = 0
      quiet = min(14, quiet + 1)
      if armed && quiet == 14 { armed = false; return true }
    }
    return false
  }
}

private struct PCMChunk: Sendable {
  let origin: Int
  let bytes: Data
}

/// Count source bytes across active, queued AND in-flight chunks. An odd byte also
/// consumes budget. No blocking backpressure or speech-dropping queue policy.
private final class PendingPCM: @unchecked Sendable {
  private let lock = NSLock()
  private var bytes = 0
  private let limit: Int
  init(limitSamples: Int = 960000) { limit = limitSamples * 2 }
  var ownedBytes: Int { lock.lock(); defer { lock.unlock() }; return bytes }
  func reserve(_ count: Int) throws {
    lock.lock(); defer { lock.unlock() }
    let (next, overflow) = bytes.addingReportingOverflow(count)
    guard count >= 0, !overflow, next <= limit else { throw HelperError("live PCM budget exceeded") }
    bytes = next
  }
  func release(_ count: Int) throws {
    lock.lock(); defer { lock.unlock() }
    guard count >= 0, count <= bytes else { throw HelperError("invalid live PCM ownership release") }
    bytes -= count
  }
}

private struct PCMChunker {
  let budget: PendingPCM
  private var policy = PausePolicy()
  private var active = Data()
  private var lowByte: UInt8?
  private var origin = 0
  private(set) var total = 0

  mutating func append(_ bytes: Data, enqueue: (PCMChunk) throws -> Void) throws {
    for byte in bytes {
      try Task.checkCancellation()
      try budget.reserve(1)
      active.append(byte)
      guard let low = lowByte else { lowByte = byte; continue }
      lowByte = nil
      let (next, overflow) = total.addingReportingOverflow(1)
      guard !overflow else { throw HelperError("live source clock overflow") }
      total = next
      if policy.append(Int16(bitPattern: UInt16(low) | UInt16(byte) << 8)) {
        // Submit the exact cut before inspecting any later sample in this read.
        try cut(enqueue)
      }
    }
  }
  mutating func finish(enqueue: (PCMChunk) throws -> Void) throws {
    guard lowByte == nil else { throw HelperError("incomplete PCM sample at EOF") }
    if !active.isEmpty { try cut(enqueue) }
  }
  private mutating func cut(_ enqueue: (PCMChunk) throws -> Void) throws {
    try enqueue(PCMChunk(origin: origin, bytes: active))
    active = Data(); origin = total
  }
}

private func consumeChunks(_ chunks: AsyncThrowingStream<PCMChunk, Error>, budget: PendingPCM,
                           run: (PCMChunk) async throws -> Void) async throws {
  for try await chunk in chunks {
    try Task.checkCancellation()
    try await run(chunk)
    try Task.checkCancellation()
    // run includes terminal finalization AND complete result consumption.
    try budget.release(chunk.bytes.count)
  }
  try Task.checkCancellation()
}

/// Finite real source PCM, pull converted until TRUE EOS. Never reset, pad or
/// truncate by a guessed conversion ratio. Output buffers have independent storage.
private func convertPCM(_ bytes: Data, source: AVAudioFormat, output: AVAudioFormat,
                        yield: (AVAudioPCMBuffer) throws -> Void) throws {
  guard bytes.count % 2 == 0 else { throw HelperError("incomplete converter source") }
  var offset = 0
  func buffer(_ count: Int) throws -> AVAudioPCMBuffer {
    guard count > 0, let input = AVAudioPCMBuffer(pcmFormat: source, frameCapacity: AVAudioFrameCount(count)),
          let channel = input.int16ChannelData else { throw HelperError("can't allocate source PCM") }
    input.frameLength = AVAudioFrameCount(count)
    for i in 0..<count {
      let at = (offset + i) * 2
      channel[0][i] = Int16(bitPattern: UInt16(bytes[at]) | UInt16(bytes[at + 1]) << 8)
    }
    offset += count
    return input
  }
  if source == output {
    while offset < bytes.count / 2 {
      try Task.checkCancellation()
      try yield(buffer(min(800, bytes.count / 2 - offset)))
    }
    return
  }
  guard let converter = AVAudioConverter(from: source, to: output) else { throw HelperError("can't convert chunk PCM") }
  var stalled = 0
  var suppliedEOS = false
  for _ in 0..<16384 {
    try Task.checkCancellation()
    guard let out = AVAudioPCMBuffer(pcmFormat: output, frameCapacity: 4096) else {
      throw HelperError("can't allocate converted PCM")
    }
    let before = offset
    var failure: Error?
    var error: NSError?

    var callbacks = 0
    let status = converter.convert(to: out, error: &error) { requested, state in
      callbacks += 1
      if callbacks > 4096 || requested == 0 || Task.isCancelled {
        failure = HelperError("converter callback failed or cancelled")
        state.pointee = .endOfStream; return nil
      }
      guard offset < bytes.count / 2 else {
        suppliedEOS = true; state.pointee = .endOfStream; return nil
      }
      do {
        let input = try buffer(min(Int(requested), bytes.count / 2 - offset))
        state.pointee = .haveData; return input
      } catch { failure = error; state.pointee = .endOfStream; return nil }
    }
    if let error { throw error }
    if let failure { throw failure }
    switch status {
    case .error: throw HelperError("converter error without NSError")
    case .haveData, .inputRanDry, .endOfStream: break
    @unknown default: throw HelperError("unknown converter output status")
    }
    if out.frameLength > 0 { try yield(out) }
    if status == .endOfStream {
      guard suppliedEOS, offset == bytes.count / 2 else { throw HelperError("premature converter EOS") }
      return
    }
    // A zero-output priming call is legal, but a wedged converter is not.
    stalled = offset == before && out.frameLength == 0 ? stalled + 1 : 0
    if stalled >= 8 { throw HelperError("converter made no progress through EOS") }
  }
  throw HelperError("converter exceeded finite call budget")
}
#endif
