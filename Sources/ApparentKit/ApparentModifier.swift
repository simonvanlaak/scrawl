#if os(iOS)
  import SwiftUI
  import UIKit

  public enum ApparentTrigger: Equatable, Sendable {
    /// Hold three fingers on the screen for half a second to start or stop. On paired devices this
    /// also turns off iOS's three-finger text-editing gestures in the app (see
    /// `Apparent.suppressesSystemEditingGestures`), which would otherwise open the Undo · Cut · Copy
    /// bar when a text field is focused.
    case threeFingerLongPress
    /// No gesture: call `Apparent.shared.start()` and `stop()` yourself.
    case none
  }

  public extension View {
    /// Adds Apparent to this window: once configured on the device, touches are watched (for the
    /// timeline) and the trigger starts and stops recordings. Unconfigured devices
    /// stay completely inert (no gesture recognizers, overlay window, or network calls).
    ///
    /// This modifier also registers `.onOpenURL` so `<scheme>://apparent?url=...&token=...` and
    /// `<scheme>://apparent?reset=1` setup links (plus the legacy `tether` host) work automatically.
    /// In SwiftUI, `.onOpenURL` accumulates handlers via a preference and invokes every registered
    /// closure for each incoming URL rather than consuming the event at the first modifier, so a
    /// host app's own `.onOpenURL` continues to receive its links. Host apps that also want to
    /// short-circuit their own URL router on Apparent links can check `if !Apparent.handleSetupURL(url) { ... }`.
    func apparent(trigger: ApparentTrigger = .threeFingerLongPress, isEnabled: Bool = true) -> some View {
      background(ApparentInstaller(trigger: trigger, isEnabled: isEnabled))
        .onOpenURL { url in
          if isEnabled {
            _ = Apparent.handleSetupURL(url)
          }
        }
    }
  }

  private struct ApparentInstaller: UIViewRepresentable {
    let trigger: ApparentTrigger
    let isEnabled: Bool

    func makeUIView(context _: Context) -> InstallerView {
      InstallerView(trigger: trigger, isEnabled: isEnabled)
    }

    func updateUIView(_ uiView: InstallerView, context _: Context) {
      uiView.update(trigger: trigger, isEnabled: isEnabled)
    }
  }

  /// Attaches the touch observer and the trigger to its window only when enabled and configured.
  final class InstallerView: UIView {
    private var trigger: ApparentTrigger
    private var isEnabled: Bool
    private weak var installedOn: UIWindow?
    private var installedTrigger: ApparentTrigger?
    private var touchObserver: TouchObserver?
    private var triggerRecognizer: UILongPressGestureRecognizer?

    init(trigger: ApparentTrigger, isEnabled: Bool) {
      self.trigger = trigger
      self.isEnabled = isEnabled
      super.init(frame: .zero)
      isUserInteractionEnabled = false
      NotificationCenter.default.addObserver(
        self,
        selector: #selector(configurationDidChange),
        name: Apparent.configurationDidChangeNotification,
        object: nil
      )
    }

    @available(*, unavailable)
    required init?(coder _: NSCoder) { fatalError() }

    override func didMoveToWindow() {
      super.didMoveToWindow()
      syncInstall()
    }

    func update(trigger: ApparentTrigger, isEnabled: Bool) {
      self.trigger = trigger
      self.isEnabled = isEnabled
      syncInstall()
    }

    @objc private func configurationDidChange() {
      syncInstall()
    }

    private func syncInstall() {
      guard isEnabled, Apparent.isConfigured, let window else {
        uninstall()
        return
      }
      if window === installedOn, installedTrigger == trigger {
        return
      }
      uninstall()
      installedOn = window
      installedTrigger = trigger
      let observer = TouchObserver()
      window.addGestureRecognizer(observer)
      touchObserver = observer
      if case .threeFingerLongPress = trigger {
        let press = UILongPressGestureRecognizer(target: self, action: #selector(triggered(_:)))
        press.numberOfTouchesRequired = 3
        press.minimumPressDuration = 0.5
        press.cancelsTouchesInView = false
        window.addGestureRecognizer(press)
        triggerRecognizer = press
        Apparent.shared.threeFingerTriggers += 1
      }
    }

    private func uninstall() {
      if let installedOn {
        if let touchObserver {
          installedOn.removeGestureRecognizer(touchObserver)
        }
        if let triggerRecognizer {
          installedOn.removeGestureRecognizer(triggerRecognizer)
        }
      }
      if triggerRecognizer != nil {
        Apparent.shared.threeFingerTriggers -= 1
      }
      touchObserver = nil
      triggerRecognizer = nil
      installedOn = nil
      installedTrigger = nil
    }

    @objc private func triggered(_ press: UILongPressGestureRecognizer) {
      guard press.state == .began, Apparent.isConfigured else { return }
      Apparent.shared.triggerRecognized()
    }
  }

  /// Sees every touch in the window without taking part: it never recognizes, never cancels or
  /// delays touches, and works alongside every other gesture.
  final class TouchObserver: UIGestureRecognizer, UIGestureRecognizerDelegate {
    private var fingers = Set<ObjectIdentifier>()
    override init(target: Any?, action: Selector?) {
      super.init(target: target, action: action)
      cancelsTouchesInView = false
      delaysTouchesBegan = false
      delaysTouchesEnded = false
      delegate = self
    }

    convenience init() { self.init(target: nil, action: nil) }

    override func touchesBegan(_ touches: Set<UITouch>, with event: UIEvent) { forward(touches, .began, event) }
    override func touchesMoved(_ touches: Set<UITouch>, with event: UIEvent) { forward(touches, .moved, event) }
    override func touchesEnded(_ touches: Set<UITouch>, with event: UIEvent) { forward(touches, .ended, event) }
    override func touchesCancelled(_ touches: Set<UITouch>, with event: UIEvent) { forward(touches, .cancelled, event) }

    /// Called once every finger has lifted (or the system took the touches).
    override func reset() {
      super.reset()
      fingers.removeAll()
      Apparent.shared.touchesReset()
    }

    private func forward(_ touches: Set<UITouch>, _ phase: UITouch.Phase, _: UIEvent) {
      guard let window = view as? UIWindow else { return }
      if phase == .began {
        fingers.formUnion(touches.map(ObjectIdentifier.init))
        if fingers.count == 3, Apparent.shared.threeFingerTriggers > 0 { Apparent.shared.prepareToStart() }
      } else if phase == .ended || phase == .cancelled {
        fingers.subtract(touches.map(ObjectIdentifier.init))
      }
      for touch in touches {
        Apparent.shared.record(touch: touch, in: window, phase: phase)
      }
    }

    func gestureRecognizer(_: UIGestureRecognizer, shouldRecognizeSimultaneouslyWith _: UIGestureRecognizer) -> Bool { true }
  }
#endif
