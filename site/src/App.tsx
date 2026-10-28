import { Demo } from "./demo/Demo"
import { Copy } from "./Copy"

const REPO = "https://github.com/tryapparent/apparent"
const RELEASES = `${REPO}/releases`
const INSTALL_PROMPT = `Install the Apparent desktop app from ${RELEASES}, then launch it.`

export function App() {
  return (
    <>
      <header className="nav">
        <a className="brand" href="#top" aria-label="Apparent home">
          <img src="/apparent-wordmark.svg" width={148} height={35} decoding="async" alt="apparent" />
        </a>
        <nav>
          <a href="#demo">Demo</a>
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
            <div className="agent-install">
              <span className="agent-install-label">Or send this to your agent</span>
              <div className="agent-install-prompt">
                <p>{INSTALL_PROMPT}</p>
                <Copy text={INSTALL_PROMPT} />
              </div>
            </div>
            <p className="release-note">Preview downloads on GitHub are still Scrawl-branded. Apparent does not support Windows.</p>
          </div>
        </section>
        <section className="demo-section" id="demo" aria-label="Product demo">
          <Demo />
        </section>
      </main>
      <footer className="foot">
        <img src="/apparent-wordmark.svg" width={130} height={30} decoding="async" alt="apparent" />
        <span>Make what you mean apparent.</span>
        <a href={REPO}>GitHub</a>
      </footer>
    </>
  )
}
