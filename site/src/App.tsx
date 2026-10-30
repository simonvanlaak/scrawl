import { Demo } from "./demo/VideoDemo"
import { BeliefPond } from "./BeliefPond"
import { FrogPond } from "./FrogPond"
import { GithubStars } from "./GithubStars"

const REPO = "https://github.com/tryapparent/apparent"

export function App() {
  return (
    <div className="pond-page">
      <header className="pond-nav">
        <a className="brand" href="#top" aria-label="Apparent home">
          <img src="/apparent-wordmark.svg" width={149} height={35} decoding="async" alt="apparent" />
        </a>
        <GithubStars />
      </header>
      <main>
        <FrogPond />
        <section className="pond-demo" aria-label="See Apparent in action">
          <h2>See what <span>apparent</span> captures</h2>
          <div className="demo-section" id="demo" aria-label="Product demo">
            <Demo />
          </div>
        </section>
        <BeliefPond />
      </main>
      <footer className="foot">
        <img src="/apparent-wordmark.svg" width={130} height={30} decoding="async" alt="apparent" />
        <a href={REPO}>GitHub</a>
      </footer>
    </div>
  )
}
