# Independent Desktop host spike

**Acceptance spike, not production integration.** Recommended next host: an Electron
sidecar for ordinary Desktop windows. Keep the existing GPUI/native overlay,
picker, capsule, global shortcuts, capture/permission machinery and retention as
the default. This package imports none of them and cannot control the live app.

## Run the harmless control boundary

Node >=22.12.0 (matching Electron and its downloader); no installation needed for browser mode:

```sh
cd capture/desktop/host-spike
npm run build
npm test
npm run dev
# Open http://127.0.0.1:18841 in a browser.
```

Send diagnostic pulse → host validates a versioned command → returns a
`probe.echoed` event and authoritative snapshot → the page renders the new host
revision. Reconnect reads host state. Try stale command demonstrates refusal.
There is no simulated recorder, pretend permission success or Connect account.
The browser adapter only binds IPv4 loopback, checks Host/Origin, bounds input,
serves three allowlisted assets and denies media permissions. It is a throwaway
transport, **not** a proposed production HTTP recording endpoint.

For an independent Electron window on an authorized disposable desktop:

```sh
npm ci
npm run readiness # real Electron binary in Node mode; NO native window proof
HOST_SPIKE_PROFILE=/absolute/disposable/profile npm run desktop
```

The sidecar uses a sandboxed, context-isolated renderer, a two-method preload,
main-frame/sender validation, local asset allowlisting, denied permissions,
blocked navigation/popups and no global shortcuts. The renderer and diagnostic
host are the same files in browser and Electron modes. Closing the window exits
only the sidecar. User profile is mandatory and separate from Apparent.

## Boundaries and pending evidence

All changes are new files in this directory. Root dependencies, lock, sources,
CI, site, live config, native overlays and other workers' freezes are unchanged.
The parent owns native GUI/VM/macOS proof. No native GUI, screen capture or live
application interaction is authorized or claimed by this spike. See
[ACCEPTANCE.md](ACCEPTANCE.md) for the repository-based comparison, pinned primary
API evidence and exact open Phase C line-100 gates.

## Verdict: diagnostic PASS; native acceptance OPEN

The runnable diagnostic protocol, four Node/HTTP tests, real browser controls and
installed Electron binary readiness passed on Linux. Browser mode exercised
command → event → rendered state, stale refusal and snapshot reconnect at desktop
and narrow sizes. Electron was executed in Node mode only: its native window,
preload IPC and GUI teardown are unrun. No startup latency was measured, browser
or native. No Desktop host has passed native acceptance. See
[VERIFICATION.md](VERIFICATION.md) for exact tested scope and limits.
