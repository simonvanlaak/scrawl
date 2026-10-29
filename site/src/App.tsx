import { Demo } from "./demo/Demo"
import { Copy } from "./Copy"
import { inkPath } from "./demo/ink"
import { LatestDownload } from "./LatestDownload"

const REPO = "https://github.com/tryapparent/apparent"
const RELEASES = `${REPO}/releases`
const INSTALL_PROMPT = `Install the Apparent desktop app from [Apparent releases](${RELEASES}), then launch it.`
const brushPoints: [number, number, number][] = [
  [5, 15, .5], [15, 14, .5], [28, 13, .5], [42, 14, .5],
  [57, 13, .5], [72, 12, .5], [86, 13, .5], [101, 12, .5], [115, 11, .5],
]
const brushPath = inkPath(brushPoints, 19, .5)

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
          <LatestDownload nav />
        </nav>
      </header>
      <main>
        <section className="hero story-hero" id="top">
          <div className="story-intro">
            <h1>We believe work should be <span className="word-highlight">enjoyable<svg className="brush-highlight" viewBox="0 0 120 28" preserveAspectRatio="none" aria-hidden="true" focusable="false"><path d={brushPath} fill="#b7a5d8" /></svg></span><br /> and leave us free for late nights with friends.</h1>
            <p>Visual feedback gets lost in text.</p>
            <p><img className="inline-wordmark" src="/apparent-wordmark.svg" width={104} height={24} alt="Apparent" /> lets you show an agent what to change and see what it made.</p>
            <LatestDownload />
            <div className="agent-install">
              <span className="agent-install-label">Or send this to your agent</span>
              <div className="agent-install-prompt">
                <p>{INSTALL_PROMPT}</p>
                <Copy text={INSTALL_PROMPT} />
              </div>
            </div>
          </div>
        </section>
        <section className="demo-section" id="demo" aria-label="Product demo">
          <Demo />
        </section>
      </main>
      <footer className="foot">
        <img src="/apparent-wordmark.svg" width={130} height={30} decoding="async" alt="apparent" />
        <a href={REPO}>GitHub</a>
      </footer>
    </>
  )
}
