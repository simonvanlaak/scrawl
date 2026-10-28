import { Demo } from "./demo/Demo"
import { HowItWorks } from "./HowItWorks"

const REPO = "https://github.com/tryapparent/apparent"
const RELEASES = `${REPO}/releases`

export function App() {
  return (
    <>
      <header className="nav">
        <a className="brand" href="#top" aria-label="Apparent home">
          <img src="/apparent-wordmark.svg" width={148} height={35} decoding="async" alt="apparent" />
        </a>
        <nav>
          <a href="#demo">Demo</a>
          <a href="#how">How it works</a>
          <a href={REPO}>GitHub</a>
          <a className="nav-dl" href={RELEASES}>Get the app</a>
        </nav>
      </header>
      <main>
        <section className="hero story-hero" id="top">
          <div className="story-intro">
            <h1>Make what you mean <em>apparent.</em></h1>
            <p>Work should be enjoyable. When a design needs changing, show your agent what you mean.</p>
            <a className="btn-primary" href={RELEASES}>Get the app <span aria-hidden="true">↗</span></a>
            <p className="release-note">Preview downloads on GitHub are still Scrawl-branded. Windows releases are paused; older Windows builds are unsupported.</p>
          </div>
        </section>
        <section className="demo-section" id="demo" aria-label="Product demo">
          <div className="demo-film" id="demo-film">
            <div className="demo-film-intro">
              <div>
                <h2>See the handoff</h2>
              </div>
              <p>21-second scripted walkthrough. The terminal sequence is illustrative, not a live agent session.</p>
            </div>
            <video controls playsInline preload="metadata" poster="/apparent-demo-pi-poster.jpg?v=525a5c3" aria-label="Apparent capture and Pi handoff walkthrough">
              <source src="/apparent-demo-pi.mp4?v=silent-20261028" type="video/mp4" />
              Your browser does not support this video.
            </video>
          </div>
          <Demo />
        </section>
        <HowItWorks />
      </main>
      <footer className="foot">
        <img src="/apparent-wordmark.svg" width={130} height={30} decoding="async" alt="apparent" />
        <span>Make what you mean apparent.</span>
        <a href={REPO}>GitHub</a>
      </footer>
    </>
  )
}
