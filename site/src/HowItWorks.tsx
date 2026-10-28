import { useEffect, useLayoutEffect, useRef, useState, useSyncExternalStore } from "react"

// One clock, one example. The scene pauses when it leaves the viewport and stops at the handoff.
const END = 9100
const reducedQuery = typeof window === "undefined" ? null : window.matchMedia("(prefers-reduced-motion: reduce)")
const subscribeReduced = (notify: () => void) => {
  reducedQuery?.addEventListener("change", notify)
  return () => reducedQuery?.removeEventListener("change", notify)
}
const clamp = (n: number) => Math.max(0, Math.min(1, n))
const between = (t: number, start: number, end: number) => clamp((t - start) / (end - start))
const ease = (n: number) => 1 - (1 - n) ** 3

type Box = { x: number; y: number; w: number; h: number }

export function HowItWorks() {
  const section = useRef<HTMLElement>(null)
  const stage = useRef<HTMLDivElement>(null)
  const target = useRef<HTMLDivElement>(null)
  const elapsed = useRef(0)
  const [time, setTime] = useState(0)
  const [visible, setVisible] = useState(false)
  const [playing, setPlaying] = useState(true)
  const [scale, setScale] = useState(1)
  const [box, setBox] = useState<Box>({ x: 274, y: 179, w: 275, h: 91 })
  const reduced = useSyncExternalStore(subscribeReduced, () => !!reducedQuery?.matches, () => false)
  const t = reduced ? END : time

  useLayoutEffect(() => {
    const container = stage.current?.parentElement
    const scene = stage.current
    const mark = target.current
    if (!container || !scene || !mark) return
    const measure = () => {
      setScale(container.clientWidth / 960)
      const a = scene.getBoundingClientRect()
      const b = mark.getBoundingClientRect()
      const ratio = a.width / 960
      setBox({ x: (b.left - a.left) / ratio, y: (b.top - a.top) / ratio, w: b.width / ratio, h: b.height / ratio })
    }
    measure()
    document.fonts?.ready.then(measure)
    const observer = new ResizeObserver(measure)
    observer.observe(container)
    return () => observer.disconnect()
  }, [])

  useEffect(() => {
    if (!section.current) return
    const observer = new IntersectionObserver(([entry]) => setVisible(entry.isIntersecting), { threshold: 0.1 })
    observer.observe(section.current)
    return () => observer.disconnect()
  }, [])

  useEffect(() => {
    if (!visible || !playing || reduced || time >= END) return
    let frame = 0
    let last = performance.now()
    const tick = (now: number) => {
      elapsed.current = Math.min(END, elapsed.current + Math.max(0, now - last))
      last = now
      setTime(elapsed.current)
      if (elapsed.current < END) frame = requestAnimationFrame(tick)
    }
    frame = requestAnimationFrame(tick)
    return () => cancelAnimationFrame(frame)
  }, [visible, playing, reduced, time >= END])

  const replay = () => {
    elapsed.current = 0
    setTime(0)
    setPlaying(true)
  }

  const ink = ease(between(t, 1050, 2150))
  const speech = ease(between(t, 2350, 3200))
  const focus = ease(between(t, 5400, 6750))
  const frameLands = ease(between(t, 6470, 6750))
  const handoff = ease(between(t, 6900, 7750))
  const complete = t >= END
  const cx = box.x + box.w * 0.53
  const cy = box.y + box.h * 0.47
  const rx = box.w * 0.48
  const ry = box.h * 0.64
  const stroke = `M ${cx - rx * 0.8} ${cy - ry * 0.65} C ${cx - rx * 1.4} ${cy + ry * 0.4}, ${cx - rx * 0.45} ${cy + ry * 1.25}, ${cx + rx * 0.5} ${cy + ry * 0.85} C ${cx + rx * 1.5} ${cy + ry * 0.5}, ${cx + rx * 1.2} ${cy - ry * 1.05}, ${cx + rx * 0.15} ${cy - ry * 1.05} C ${cx - rx * 0.3} ${cy - ry * 1.05}, ${cx - rx * 0.65} ${cy - ry * 0.8}, ${cx - rx * 0.8} ${cy - ry * 0.65}`

  return (
    <section id="how" className="how-section" ref={section}>
      <div className="how-intro">
        <div>
          <p className="how-eyebrow">One moment, understood</p>
          <h2>How it works</h2>
          <p>Point at the part you mean. Say what should change. Apparent keeps the picture and your words together for the agent.</p>
        </div>
        <div className="how-controls">
          <button className="how-replay" type="button" onClick={() => setPlaying(!playing)} disabled={reduced || time >= END} aria-label={playing ? "Pause how it works animation" : "Resume how it works animation"}>
            <span aria-hidden="true">{playing ? "Ⅱ" : "▶"}</span> {playing ? "Pause" : "Resume"}
          </button>
          <button className="how-replay" type="button" onClick={replay} disabled={reduced} aria-label={reduced ? "Animation unavailable with reduced motion" : "Replay how it works animation"}>
            <span aria-hidden="true">↺</span> Replay
          </button>
        </div>
      </div>

      <div className="how-stage-viewport" role="img" aria-label="Illustration: a person circles a cramped heading on a sample web page and says to give it more room. A quiet pause adds no frame. Apparent focuses the marked area and prepares the image and words for a manual handoff to an agent.">
        <div className="how-stage" ref={stage} style={{ transform: `scale(${scale})` }} aria-hidden="true">
          <div className="how-scene-label">YOUR SCREEN <span>·</span> A sample page</div>
          <div className="how-browser">
            <div className="how-browser-bar"><span className="how-lights"><i /><i /><i /></span><span>studio.local / work</span><span>↗</span></div>
            <div className="how-browser-body">
              <aside className="how-sidebar"><strong>STUDIO<span>✳</span></strong><span className="on">Overview</span><span>Projects</span><span>Messages</span><small>Workspace</small><span>Settings</span></aside>
              <div className="how-page">
                <div className="how-page-top"><span>Workspace / Overview</span><span>Wednesday, 28 October</span></div>
                <div className="how-dashboard-heading" ref={target}>
                  <div><small>GOOD MORNING, ALEX</small><strong>Your workspace <em>Live</em></strong><p>Everything in motion, all in one place.</p></div>
                </div>
                <div className="how-metrics"><div><small>Active projects</small><strong>08</strong><span>↗ 2 this week</span></div><div><small>Team members</small><strong>24</strong><span>Across 3 teams</span></div><div><small>Next milestone</small><strong>12 Nov</strong><span>Website launch</span></div></div>
                <div className="how-table"><strong>Recent activity</strong><span>Website refresh <i>In progress</i></span><span>Onboarding flow <i>In review</i></span></div>
              </div>
            </div>
          </div>

          <svg className="how-ink-layer" viewBox="0 0 960 540">
            <path d={stroke} fill="none" stroke="#ff3b30" strokeWidth="5" strokeLinecap="round" strokeLinejoin="round" pathLength="100" strokeDasharray="100" strokeDashoffset={100 * (1 - ink)} />
          </svg>
          {t >= 1800 && <span className="how-frame-pin" style={{ left: box.x + box.w - 4, top: box.y - 17, opacity: ease(between(t, 1800, 2200)) }}>1</span>}
          <div className="how-speech" style={{ opacity: speech, transform: `translateY(${(1 - speech) * 12}px)` }}><span className="how-wave">▂▆▃▇▅</span><span>“Give this heading more room.”</span></div>

          <div className="how-focus-line" style={{ opacity: focus }} />
          {focus > 0 && focus < 1 && <div className="how-flying-frame" style={{
            left: box.x + (666 - box.x) * focus,
            top: box.y + (108 - box.y) * focus,
            width: box.w + (240 - box.w) * focus,
            height: box.h + (126 - box.h) * focus,
            opacity: 1 - frameLands,
          }}><small>GOOD MORNING, ALEX</small><strong>Your workspace</strong><span>1</span></div>}
          <div className="how-agent" style={{ opacity: focus, transform: `translateX(${(1 - focus) * 36}px)` }}>
            <div className="how-agent-head"><span>APPARENT <b>→</b> AGENT</span><span>Ready to paste</span></div>
            <div className="how-frame" style={{ opacity: frameLands }}><span className="how-frame-number">1</span><div className="how-frame-content"><small>GOOD MORNING, ALEX</small><strong>Your workspace <em>Live</em></strong><p>Everything in motion, all in one place.</p><svg viewBox="0 0 240 100"><path d="M24 20 C -4 60 24 95 140 90 C 245 90 264 -3 112 8 C 75 10 34 13 24 20" fill="none" stroke="#ff3b30" strokeWidth="3" strokeLinecap="round" /></svg></div></div>
            <div className="how-agent-context" style={{ opacity: handoff }}><small>0:02 · MARK + VOICE</small><p>“Give this heading more room.”</p><div className="how-agent-file">contact-sheet.png <span>↗</span></div><div className="how-agent-file">summary.md <span>↗</span></div></div>
          </div>

          <div className="how-timeline"><span>0:00</span><div className="how-track"><i className="how-track-progress" style={{ width: `${clamp(t / END) * 100}%` }} /><b className="how-timeline-mark">1</b><span className="how-pause-band">quiet pause <small>no new frame</small></span><b className="how-timeline-end">↗</b></div><span>0:09</span></div>
          <div className="how-current">{complete ? "Ready for you to paste" : t < 2200 ? "Mark the change" : t < 4600 ? "Words follow the mark" : t < 5400 ? "Nothing changes during the pause" : t < 6900 ? "Focus on the marked area" : "Picture and words, together"}</div>
        </div>
      </div>
      <div className="how-after"><p><strong>Not another recording to sift through.</strong> A useful frame, the words that belong to it, and the full recording still on disk.</p><p>Illustrative sequence, not a live agent session. Nothing is sent automatically.</p></div>
    </section>
  )
}