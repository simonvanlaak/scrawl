// Illustrative diagrams, not a captured customer session. Motion follows the marked moment.
export function HowItWorks() {
  return (
    <section id="how" className="how-section">
      <h2>How it works</h2>
      <div className="feature-cards">
        <article className="feature-card">
          <div className="how-visual moment-visual" role="img" aria-label="A marked moment becomes a numbered frame; a quiet pause does not add a frame">
            <div className="moment-track" aria-hidden="true">
              <span className="moment-tick"><b>1</b><small>mark</small></span>
              <span className="moment-pause"><small>pause</small></span>
              <span className="moment-tick second"><b>2</b><small>click</small></span>
            </div>
          </div>
          <div className="feature-copy">
            <h3>Frames when something happens</h3>
            <p>Marks, clicks and visual changes become frames. A quiet pause does not.</p>
          </div>
        </article>
        <article className="feature-card">
          <div className="how-visual words-visual" role="img" aria-label="A marked frame appears beside the words spoken about that moment">
            <div className="words-frame" aria-hidden="true"><span>1</span><i /><i /><i /></div>
            <div className="words-quote" aria-hidden="true"><span>0:09 · voice</span><strong>“Give this more room.”</strong></div>
          </div>
          <div className="feature-copy">
            <h3>Words stay with the frame</h3>
            <p>Your speech is transcribed on your computer and placed beside the moment it describes.</p>
          </div>
        </article>
        <article className="feature-card">
          <div className="how-visual crop-visual" role="img" aria-label="The full screen gives way to a larger crop of the marked work area">
            <div className="crop-screen" aria-hidden="true"><i /><i /><span className="crop-target"><b>1</b><i /><i /></span></div>
          </div>
          <div className="feature-copy">
            <h3>Cropped to where you worked</h3>
            <p>The full recording stays on disk. Each frame focuses on the window you marked or clicked.</p>
          </div>
        </article>
        <article className="feature-card">
          <div className="how-visual zoom-visual" role="img" aria-label="A marked moment opens into a closer grid of neighboring frames">
            <div className="zoom-grid" aria-hidden="true">
              {Array.from({ length: 9 }, (_, i) => <span key={i} className={i === 4 ? "selected" : ""}><i /><i />{i === 4 && <b>1</b>}</span>)}
            </div>
          </div>
          <div className="feature-copy">
            <h3>A closer look when needed</h3>
            <p>The agent can request nearby frames by time or by a phrase you said.</p>
          </div>
        </article>
      </div>
      <p className="how-caption">Illustrative diagrams. Watch the scripted demo above to see the capture flow.</p>
    </section>
  )
}
