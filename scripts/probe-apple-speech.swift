import Foundation
import Speech

// Read capabilities only: no audio, model downloads, permissions or app setup.
@main
struct AppleSpeechProbe {
  static func main() async throws {
    if #available(macOS 26, *) {
      let speechLocales = await SpeechTranscriber.supportedLocales
      let dictationLocales = await DictationTranscriber.supportedLocales
      let report: [String: Any] = [
        "os": ProcessInfo.processInfo.operatingSystemVersionString,
        "speechTranscriberAvailable": SpeechTranscriber.isAvailable,
        "speechTranscriberLocales": speechLocales.map(\.identifier).sorted(),

        "dictationTranscriberLocales": dictationLocales.map(\.identifier).sorted(),
        "scope": "Capability discovery only; not recognition or installed-app acceptance"
      ]
      let data = try JSONSerialization.data(withJSONObject: report, options: [.prettyPrinted, .sortedKeys])
      FileHandle.standardOutput.write(data)
      FileHandle.standardOutput.write(Data("\n".utf8))
    } else {
      throw NSError(domain: "AppleSpeechProbe", code: 1,
                    userInfo: [NSLocalizedDescriptionKey: "macOS 26 or later required"])
    }
  }
}
