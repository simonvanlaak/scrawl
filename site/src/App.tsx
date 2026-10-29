import { Demo } from "./demo/Demo"
import { inkPath } from "./demo/ink"
import { FrogPond } from "./FrogPond"

const REPO = "https://github.com/tryapparent/apparent"

const brushPoints: [number, number, number][] = [
  [5, 15, .5], [15, 14, .5], [28, 13, .5], [42, 14, .5],
  [57, 13, .5], [72, 12, .5], [86, 13, .5], [101, 12, .5], [115, 11, .5],
]
const brushPath = inkPath(brushPoints, 19, .5)

export function App() {
  return (
    <div className="pond-page">
      <header className="pond-nav">
        <a className="brand" href="#top" aria-label="Apparent home">
          <img src="/apparent-wordmark.svg" width={149} height={35} decoding="async" alt="apparent" />
        </a>
      </header>
      <main>
        <FrogPond />
        <section className="pond-demo" aria-label="See Apparent in action">
          <h2>See what Apparent captures</h2>
          <div className="demo-section" id="demo" aria-label="Product demo">
            <Demo />
          </div>
        </section>
        <section className="pond-purpose">
          <h2>We believe work should be <span className="word-highlight">enjoyable<svg className="brush-highlight" viewBox="0 0 120 28" preserveAspectRatio="none" aria-hidden="true" focusable="false"><path d={brushPath} fill="#b7a5d8" /></svg></span><br /> and leave us free for late nights with friends.</h2>
        </section>
      </main>
      <footer className="foot">
        <img src="/apparent-wordmark.svg" width={130} height={30} decoding="async" alt="apparent" />
        <a href={REPO}>GitHub</a>
      </footer>
    </div>
  )
}
