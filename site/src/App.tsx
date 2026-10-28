import { Demo } from "./demo/Demo"
import { HowItWorks } from "./HowItWorks"
import { IPhoneSection } from "./IPhone"

const REPO = "https://github.com/tryapparent/apparent"

const CONTROLS: [string[], string][] = [
  [["⌥", "⇧", "S"], "Start or finish"],
  [["⇧"], "Hold and drag to draw"],
  [["⇧", "click"], "Drop a numbered pin"],
  [["⌘", "Z"], "Undo"],
  [["esc"], "Cancel"],
  [["⌥", "⇧", "V"], "Paste the latest capture"],
  [["M"], "Flag a moment on iPhone"],
]

export function App() {
  return (
    <>
      <header className="nav">
        <a className="brand" href="#top" aria-label="Apparent home">
          <img src="/apparent-wordmark.svg" width={148} height={35} decoding="async" alt="apparent" />
        </a>
        <nav>
          <a href="#demo">Demo</a>
          <a href="#iphone">iPhone</a>
          <a href="#how">How it works</a>
          <a href={REPO}>GitHub</a>
          <a className="nav-dl" href="#availability">Get the app</a>
        </nav>
      </header>
      <main>
        <section className="hero" id="top">
          <p className="hero-eyebrow">Screen · voice · marks → agent context</p>
          <h1>Show the problem.<br /><em>Keep the context.</em></h1>
          <p className="lede">
            Press <kbd>⌥</kbd><kbd>⇧</kbd><kbd>S</kbd>, talk through what happened, and draw on the screen.
            Apparent gives your coding agent the evidence instead of another paragraph of guesswork.
          </p>
          <a className="btn-primary" href="#demo">See how it works <span aria-hidden="true">↗</span></a>
          <p className="hero-note">A recording becomes keyframes, a timed transcript, and a compact bundle you can paste.</p>
        </section>
        <section className="demo-section" id="demo" aria-label="Product demo">
          <div className="demo-film">
            <div className="demo-film-intro">
              <div>
                <p className="demo-film-kicker">New recording · 21 seconds</p>
                <h2>See the Pi handoff</h2>
              </div>
              <p>A scripted walkthrough of the capture, marks, and Pi handoff. The terminal sequence is illustrative, not a live agent session.</p>
            </div>
            <video controls playsInline preload="metadata" poster="/apparent-demo-pi-poster.jpg?v=525a5c3" aria-label="Apparent capture and Pi handoff walkthrough">
              <source src="/apparent-demo-pi.mp4?v=silent-20261028" type="video/mp4" />
              Your browser does not support this video.
            </video>
          </div>
          <Demo />
        </section>
        <IPhoneSection />
        <HowItWorks />
        <section className="split">
          <div><h2>Controls</h2></div>
          <ul className="shortcuts">
            {CONTROLS.map(([keys, label]) => (
              <li key={label}>
                <span className="combo">{keys.map((k) => <kbd key={k}>{k}</kbd>)}</span>
                <span>{label}</span>
              </li>
            ))}
          </ul>
        </section>
        <section id="availability" className="split availability">
          <div><h2>Get Apparent</h2></div>
          <div className="availability-copy">
            <p>The public demo is ready. A separately signed Apparent installer is not yet available, so this page does not send you to someone else’s release.</p>
            <a className="btn-primary" href={REPO}>View the source on GitHub <span aria-hidden="true">↗</span></a>
            <p className="availability-note">The source builds the <code>apparent</code> command and writes <code>.apparent</code> captures. No branded installer is published yet.</p>
          </div>
        </section>
      </main>
      <footer className="foot">
        <img src="/apparent-wordmark.svg" width={130} height={30} decoding="async" alt="apparent" />
        <span>Show, don’t explain.</span>
        <a href={REPO}>GitHub</a>
      </footer>
    </>
  )
}
