import { useEffect, useRef, useState } from "react"
import * as api from "./api"
import "./hosted.css"
import { SharedLinks } from "./SharedLinks"

export function isHostedPath(path: string) { return /^\/hosted(?:\/(?:device|shares))?\/?$/.test(path) }
export function Hosted({ devicePage = false, sharesPage = false }: { devicePage?: boolean; sharesPage?: boolean }) {
  return sharesPage ? <SharedLinks /> : <Account devicePage={devicePage} />
}
function Account({ devicePage }: { devicePage: boolean }) {
  const [account, setAccount] = useState<api.Session | null>(null)
  const [request, setRequest] = useState<api.Device | null>(null)
  const [code, setCode] = useState("")
  const [error, setError] = useState("")
  const [loading, setLoading] = useState(true)
  const [busy, setBusy] = useState(false)
  const [interested, setInterested] = useState(false)
  const guard = useRef(false)
  const generation = useRef(0)
  async function load() {
    const current = ++generation.current
    setLoading(true); setError(""); setRequest(null)
    try {
      const result = await api.session()
      if (current !== generation.current) return
      setAccount(result)
      if (result.authenticated) {
        setInterested(result.waitlist.interestedAtFiveDollars)
        const value = new URLSearchParams(window.location.search).get("code") ?? ""
        setCode(value)
        if (devicePage && result.beta.access && value) {
          const preview = await api.device(value)
          if (current === generation.current) setRequest(preview)
        }
      } else if (devicePage) setCode(new URLSearchParams(window.location.search).get("code") ?? "")
    } catch (e) { if (current === generation.current) setError(message(e)) }
    finally { if (current === generation.current) setLoading(false) }
  }
  useEffect(() => { void load(); return () => { generation.current++ } }, [])
  async function act(action: () => Promise<void>) {
    if (guard.current) return
    guard.current = true; setBusy(true); setError("")
    try { await action() } catch (e) { setError(message(e)) }
    finally { guard.current = false; setBusy(false) }
  }
  const authenticated = account?.authenticated ? account : null
  const expired = request ? request.status === "expired" || (request.expiresAt ? Date.parse(request.expiresAt) <= Date.now() : false) : false
  return <div className="pond-page hosted-page">
    <header className="pond-nav"><a href="/" aria-label="Apparent home"><img src="/apparent-wordmark.svg" width="149" height="35" alt="apparent" /></a><a className="hosted-account-link" href="/hosted/">Account</a></header>
    <main className="hosted-main">
      <p className="hosted-eyebrow">Hosted Apparent · beta</p>
      <h1>{devicePage ? "Connect a device" : "Your hosted account"}</h1>
      <p className="hosted-intro">{devicePage ? "Review the permission before you connect your app or agent." : "Keep captures available to your signed-in app and authorized agents."}</p>
      <section className="hosted-panel" aria-label={devicePage ? "Device approval" : "Account status"} aria-busy={loading || busy}>
        {loading && <p role="status">Checking your account…</p>}
        {error && <div role="alert" className="hosted-error"><p>{error}</p><button disabled={busy || loading} onClick={() => void load()}>Reload account</button></div>}
        {!loading && !account && !error && <p>Account status is not available.</p>}
        {!loading && account && !account.authenticated && <>
          <h2>Sign in to continue</h2><p>Use Google to check beta access{devicePage ? " and review this device request" : " or join the waitlist"}. No capture is public just because someone knows its ID.</p>
          <SignIn devicePage={devicePage} code={code} />
        </>}
        {!loading && authenticated && <>
          <div className="hosted-identity"><p>Signed in as <strong>{authenticated.user.email}</strong></p><button disabled={busy} onClick={() => void act(async () => { await api.logout(authenticated.csrfToken); const next = await api.session(); if (next.authenticated) throw new Error("Sign-out could not be confirmed. Reload and try again."); setAccount(next); setRequest(null) })}>Log out</button></div>
          <h2>{authenticated.beta.access ? "You're in the beta" : authenticated.beta.enrolled >= authenticated.beta.capacity ? "The beta is full" : "Beta access is limited"}</h2>
          <p>{authenticated.beta.enrolled} of {authenticated.beta.capacity} beta places are occupied. The initial beta has 20 places.</p>
          {!authenticated.beta.access && <>
            <p>Access is not enabled for this account. {authenticated.waitlist.joined ? "You're on the waitlist. We'll use your account email for access updates." : "Join the waitlist for access updates."}</p>
            <form onSubmit={event => { event.preventDefault(); void act(async () => { await api.joinWaitlist(authenticated.csrfToken, interested); const next = await api.session(); if (!next.authenticated || !next.waitlist.joined || next.waitlist.interestedAtFiveDollars !== interested) throw new Error("Your waitlist preference could not be confirmed. Reload to check its status."); setAccount(next) }) }}>
              <label className="hosted-check"><input type="checkbox" checked={interested} disabled={busy} onChange={event => setInterested(event.target.checked)} /><span>I'd be interested at $5/month <span className="hosted-quiet">(optional)</span></span></label>
              <p className="hosted-quiet">This records interest only. No payment details, subscription, or charge.</p>
              <button className="hosted-primary" disabled={busy} type="submit">{busy ? "Saving…" : authenticated.waitlist.joined ? "Update interest" : "Join waitlist"}</button>
            </form>
          </>}
          {devicePage && authenticated.beta.access && <div className="hosted-permission">
            {!code && <p>No device code was supplied. Open the approval link shown by your app or agent. Do not paste access tokens here.</p>}
            {request && <>
              <h2>{request.scope === "upload" ? "Allow capture uploads?" : "Allow an agent to read captures?"}</h2>
              <p>{request.scope === "upload" ? "This connects an uploader that can add, read, and delete its account captures. It is separate from an agent's read-only credential." : "This connects an agent that can read your hosted captures, including recordings, images, and transcripts. It does not grant upload access."}</p>
              <p className="hosted-quiet">Only approve a request you just started on a trusted app or agent. Uploader and agent read credentials are separate. {request.expiresAt ? `Expires ${new Date(request.expiresAt).toLocaleString()}.` : "Codes expire ten minutes after they are requested."}</p>
              {request.status === "approved" ? <p role="status">Permission approved. Return to your app or agent to finish connecting.</p> : request.status === "rejected" ? <p role="status">This request was rejected. Start a new request in your app or agent.</p> : expired ? <p role="status">This code has expired. Request a new one from your app or agent.</p> : <button className="hosted-primary" disabled={busy} onClick={() => void act(async () => { if (request.expiresAt && Date.parse(request.expiresAt) <= Date.now()) throw new Error("This code has expired. Request a new code."); await api.approveDevice(authenticated.csrfToken, code); const next = await api.device(code); if (next.status !== "approved") throw new Error("Approval could not be confirmed. Reload to check its status."); setRequest(next) })}>{busy ? "Confirming…" : request.scope === "upload" ? "Allow uploads" : "Allow agent read access"}</button>}
            </>}
          </div>}
          {!devicePage && authenticated.beta.access && <><p role="status">Your account is ready to connect from Apparent. Start the connection in your app or agent.</p><a href="/hosted/shares">Manage shared links</a></>}
        </>}
      </section>
      <aside className="hosted-policy" aria-label="Beta limits"><h2>Small beta. Clear limits.</h2><p>20 beta places · 500 MB of hosted storage per account · captures retained for seven days.</p><p>Keep a local copy of anything you need longer. Hosted captures require account authorization; a capture ID is not a public sharing link.</p></aside>
      <noscript><p>Enable JavaScript to sign in and review permissions. No approval happens without an explicit confirmation.</p></noscript>
    </main>
  </div>
}
function message(error: unknown) { return error instanceof Error ? error.message : "The request could not be confirmed. Please reload and retry." }
function SignIn({ devicePage, code }: { devicePage: boolean; code: string }) {
  try { return <a className="hosted-primary" href={api.signInUrl(devicePage, code)}>Continue with Google</a> }
  catch { return <p role="alert">Hosted sign-in is not configured correctly. Please try again later.</p> }
}
