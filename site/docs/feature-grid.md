# Homepage feature grid

The homepage places four animated explanations immediately after `VideoDemo` and before the belief section. Titles are black, bold, and left aligned. The layout is two columns on desktop and one on mobile.

`src/features/FeatureGrid.tsx` renders the section. The four isolated animation documents live in `public/feature-demos/`; `package-apparent.mjs` explicitly includes them in the public artifact. Frames load near the viewport, pause offscreen, and share a visible pause/play control. Reduced motion starts paused. The documents accept play/pause messages only from their same-origin parent.

These are illustrative examples, not live recording or agent connections. Handoff uses sample words and placeholder paths; the visuals highlight image references, timestamped text, and interactions before an illustrative Send. Nothing is sent to an agent.

The whiteboard movie is a silent cropped excerpt (17.8–27.4 seconds) of the already approved `public/Apparent-ghost-demo-v4-lilac.mp4`, excluding the webcam and terminal. The ghost graphics match that published demonstration. No private capture bundle or private audio is included.

Validate with the public-site build (`APPARENT_PUBLIC_SITE=1 npm run build`), `node scripts/package-apparent.mjs`, and the public homepage/video tests. This section changes no desktop behavior and includes no deployment step.
