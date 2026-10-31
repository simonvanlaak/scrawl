import { StrictMode } from "react"
import { createRoot, hydrateRoot } from "react-dom/client"
import { App } from "./App"
import { Hosted, isHostedPath } from "./hosted/Hosted"
import "./styles.css"
import "./pond-hero.css"
import "./cta.css"

const root = document.getElementById("root")!
const app = (
  <StrictMode>
    {isHostedPath(window.location.pathname) ? <Hosted devicePage={window.location.pathname.replace(/\/$/, "") === "/hosted/device"} /> : <App />}
  </StrictMode>
)

// scripts/prerender.ts fills #root at build time; the dev server serves it empty.
if (root.firstElementChild) hydrateRoot(root, app)
else createRoot(root).render(app)
