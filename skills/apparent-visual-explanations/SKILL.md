---
name: apparent-visual-explanations
description: Use when creating Apparent explanatory visuals.
version: 1.0.0
author: Hermes Agent
license: MIT
metadata:
  hermes:
    tags: [apparent, visuals, slides, diagrams, brand]
    related_skills: [make-it-apparent]
---

# Apparent explanatory visuals

## When to use

Create slides, diagrams, illustrations, callouts or title/end cards only when real footage cannot explain a necessary idea clearly, or when explicitly requested. This is the optional companion to [make-it-apparent](../make-it-apparent/SKILL.md), which owns recording, inspecting, narrating and delivering real screen evidence. Do not turn every walkthrough into a slide deck. A recording that already shows the mechanism and result needs no extra visual.

Use Apparent's brand for every agent-created explanatory visual in an Apparent walkthrough, including temporary HTML slides and diagrams. Do not recolor the recorded product or third-party UI to match the brand. Preserve the established readable rounded dark subtitle style; a supplementary diagram is not permission to redesign the captions.

## Decide what needs a visual

First write: “The viewer needs to understand how ___ causes ___, and the footage cannot show it because ___.” Use one visual to resolve that gap. Prefer a before/after comparison, a small input-to-result diagram or a single relationship over a component inventory. If this sentence has no useful answer, use the real recording alone.

Inspect the actual source, diff, manifest, relevant test results and observed app state first. Distinguish a recorded failure, documented historical failure, proposed change and verified new behavior. Never invent an old UI, an unobserved failure, a benchmark, a success or a release to complete a story. Mark explanatory diagrams as conceptual where they could be mistaken for app screenshots. A generated slide is not proof of a working feature. No slide deck may substitute for requested live footage without explaining the blocker and obtaining agreement. Recording the slide window does not turn its claims into app evidence.

## Story structure

Use **background → problem → change → now → human action**. Restore the immediate project context lost during context switching without teaching familiar basics. Begin one step before the local problem: a viewer can know the domain but need a reminder of this part's purpose or an earlier design decision. Define an unfamiliar term by its useful class and distinguishing role here, not a dictionary detour. Give the broad point first, then only the detail needed to follow it.

- **Background:** What is this product or part for, where does this part fit, and what must the viewer remember?
- **Problem:** What happened, who was affected and why did it matter? Use real before-footage if safely available; otherwise explain the documented problem and disclose the missing before-proof. Do not infer production impact from a bug report.
- **Change:** Explain the cause and fix's mechanism in plain language. A causal diagram can help here; a file list usually cannot. Show the necessary parts and their actions, not a component inventory.
- **Now:** Return to real footage of the same task in the tested new build. Explain what visibly changed and its actual availability: candidate, open PR, merged or released. Separate observed results, test assertions and expected user effects. State material risks, untested cases and open questions before the final action. A diagram cannot establish this result, and a merge does not establish a release.
- **Human action:** Finish with who must do exactly what and why, or “No human action is needed.” Distinguish approval, review, decision and hands-on action. Never invent a review task or hand the human work the agent still owns. Do not imply merge or release authorization.

These are story beats, not five required slides or a fixed prose template. A short spoken sentence over the real app often suffices. Keep background brief, give the mechanism only the detail it needs, and hold evidence long enough to read. For blocked work, show the real remaining failure rather than manufacture a successful new state. Do not require image generation, emoji cues or six labeled paragraphs to tell this story.

## Apparent brand

Resolve the current brand from this checkout before producing assets:

- [Approved brand assets](../../assets/brand/): use the real wordmark or icon rather than redrawing it.
- [Site styles](../../site/src/styles.css): the Paper/Lilac identity section defines Host Grotesk and site treatments.
- [Pond styles](../../site/src/pond-hero.css): the same Paper/Carbon typography and accents.
- [Public fonts and assets](../../site/public/): use the supplied `HostGrotesk-Variable.ttf` when available.

A later explicit approved brand direction takes priority. Prefer a clean no-logo visual to a fabricated logo. No private checkout or external framework file is needed to follow this skill.

| Role | Value | Use |
| --- | --- | --- |
| Paper | `#F5F0E8` | Default flat canvas |
| Carbon | `#020102` | Main text and diagram lines |
| Lilac | `#B7A5D8` | Restrained grouping or one highlighted region |
| Signal | `#7A58C7` | The causal path or changed part |
| Stock | `#E7E0EE` | Secondary grouping |
| Secondary ink | `#342D40` | Supporting text |

Use **Host Grotesk** when the supplied font is available; otherwise use a readable system sans and disclose the substitution when brand fidelity matters. Use monospace for literal commands, code and identifiers. Keep labels short, large and next to what they describe. Prefer flat fills, plain lines, square-edged diagram geometry and generous empty space. Avoid gradients, textures, decorative shadows, stock illustrations and unrelated blue/green palettes. Keep brand effects subordinate to the explanation. Do not use Lilac for small low-contrast text on Paper. Make meaning survive grayscale; labels and arrow direction must work without color.

Use the real approved wordmark only when useful, with its proportions and clear space intact. Do not add a giant logo to every frame. Do not invent light/dark logo variants or stretch the app icon into a wordmark. For dark supplementary visuals, use Carbon as the canvas with Paper text, verified contrast and the same restrained accents; check any actual logo asset against that background. Square-edged diagram geometry does not change the recording skill's rounded dark captions.

## Build and verify

Choose an available authoring route suited to the requested artifact: deterministic HTML/SVG for precise diagrams or slides; an explicitly requested image tool for illustration. Do not install a new renderer, add a product dependency, or use a paid image service without checking the task's authority. Use existing browser/HTML preview tools when available and follow the active browser rules. Keep a conceptual HTML slide separate from the application under test.

Render the actual final asset and inspect its pixels, not just its source. Verify literal labels, code punctuation, arrow direction, claim truth, current brand assets, font loading, contrast, clipping and legibility at the intended video/player size. Check captions do not cover results or diagram labels. Keep editable sources and use a new output path for revisions so clients do not show stale cached media. Keep private paths, customer data, footage and auth material out of public assets and git.

When inserting a visual into a narrated walkthrough, preserve the raw capture. Treat an inserted slide as an explicit edit with its own duration, not evidence that the source app displayed it. Recompute the shared video/audio/subtitle timing map when duration changes. Preserve the recording skill's verified policy: original time by default, opt-in gap compression only with source audio dropped, unchanged speech tempo, protected real-time spans and the accelerated-only indicator. An insertion is not gap compression; do not reuse a stale source/output map after adding a slide. See [the export contract](../../docs/proof-video.md) for supported options and reports.

Synchronize each claim with the visible state and inspect the whole final export, including entry/exit, caption boundaries, generated voice and final action. Do not promise a slide-compositing CLI flag that installed help does not expose; use an available approved editor or report the missing capability. Return to [make-it-apparent](../make-it-apparent/SKILL.md) for source inspection, privacy, capture and playable delivery requirements.

## Common pitfalls and final check

- Extra slides that repeat the narration slow the explanation. Remove any visual that adds no understanding.
- Brand tokens are not evidence. Keep observed results and conceptual explanation distinct.
- A new state must include honest build/platform/release limits. Never hide them outside the video.
- Verify the story is understandable without the chat, every explanatory asset matches the brand, and the last spoken/captioned instruction names the real next action.
- Verify optional visuals have not replaced real proof, restyled recorded UI, changed caption style or invalidated the verified timing map.
