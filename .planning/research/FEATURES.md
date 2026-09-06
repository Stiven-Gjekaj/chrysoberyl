# Feature Research

**Domain:** Visual regression and media comparison tools (image, video, PDF, mesh diff)
**Researched:** 2026-09-06
**Confidence:** MEDIUM (cross-checked against official docs, READMEs, and vendor pricing pages; some vendor claims are marketing copy and are marked LOW where not independently verifiable)

## Field Survey

Named products, what they do, their review loop, their price, and their complaints.

### Hosted visual regression services

| Product | What it does | Review workflow | Price | User complaints |
|---------|--------------|------------------|-------|------------------|
| **Chromatic** | Captures a snapshot per Storybook story per browser per viewport, diffs against baseline, hosts a review UI. Ships TurboSnap, which re-tests only the components whose dependency graph changed (skia-style dependency narrowing). | Diff surfaces in a hosted dashboard. Any team member (not just engineers) comments, approves, or rejects. Approval commits the new baseline through a Git-linked flow. | Metered per snapshot: $0.006/snapshot list price. Free tier 5,000 snapshots/month, Team tier ~100,000/month, Enterprise custom. | TurboSnap's dependency-graph config is reported as fragile: "configuration is more complicated and can lead to difficult to debug scenarios or UI changes being missed" (GitHub issue threads on carbon-design-system and sg-orbit repos). Cost scales with snapshot count, which punishes broad coverage. |
| **Percy** (BrowserStack) | Snapshot capture across browsers/viewports, compares against last *approved* build. Every approval becomes the new baseline automatically, with branch-aware baseline carry-forward. | Dashboard-based diff review, comment/approve/reject per snapshot, automatic status checks. | Per-snapshot billing, roughly 30-50% cheaper than Applitools for equivalent volume; a 5-person team at 1,000 snapshots/day across 4 widths lands near $500-1,000/month. | Same metered-snapshot cost pressure as Chromatic; teams report snapshot count exploding once multiple viewports/browsers are added per scenario. |
| **Applitools Eyes** | "Visual AI" perceptual model (trained on large UI image corpora) claims near-human judgment of what is a real bug vs noise. Ultrafast Grid renders one DOM capture across many browsers/viewports in the cloud in parallel, avoiding N full re-runs. Root Cause Analysis correlates a visual diff back to the DOM/CSS change that caused it. | Reviewer only sees checks flagged as changed; approving a check makes it the new baseline. | Per-seat + per-validation. A 5-person team at 1,000 validations/day: roughly $2,000-3,000/month. Entry "Eyes Starter" plan ~$899/month billed annually. | Highest price point in the category. The "Visual AI" classification step is a closed model: search turned up marketing claims of "eliminates 99.9% of false positives" but no public documentation of the classifier's decision boundary, so a reviewer cannot audit *why* a diff was suppressed. This is a trust gap for a CI gate, not just a UX complaint. |
| **Lost Pixel** | Open-source core (self-hosted or "Lost Pixel Platform" hosted tier) targeting Storybook/Ladle stories and full pages as a direct alternative to Percy/Chromatic/Applitools. | GitHub Action posts automatic status checks; open-source core has no hosted review dashboard by default. | Core engine free/open source; hosted platform tier is paid (usage-based, not independently price-checked here — LOW confidence on current price). | Positioned explicitly as the "pay nothing, self-host" answer to the metered-snapshot complaint above. |
| **Argos CI** | MIT-licensed, open source visual testing platform. SDKs for Playwright, Cypress, WebdriverIO, Puppeteer. Review UI with pixel-diff zoom and diff-grouping. | PR status check plus PR comment summarizing all changed screenshots in one place. | $30/month flat subscription plus a limited free tier; free/discounted for qualifying open-source projects. | Praised specifically as the low-cost, non-metered alternative to Percy — evidence that flat/local pricing is a real market want, not just a nice-to-have. |
| **reg-suit** (reg-viz) | CLI-first, snapshot-testing-inspired image diff tool. Stores snapshots to pluggable cloud storage (S3, GCS, etc.) via a plugin system. Produces a static HTML diff report. | GitHub App posts the comparison result as a PR comment; otherwise the HTML report is the review surface, no hosted dashboard. | Free, open source. | Plugin system and manual storage wiring add setup friction versus a fully hosted product; no built-in approve/reject UI, only a report. |
| **Vizzly** (2026 entrant) | Local-first visual testing: runs a real diff on the developer's machine during TDD, then integrates the same diff engine into CI. Ships "Honeydiff," a custom diff engine claimed faster than odiff/pixelmatch in vendor benchmarks (not independently verified here). Works with existing Playwright/Cypress/Puppeteer screenshot capture, not a new capture layer. | Position-based comments, @mentions, approval rules in a lightweight review UI. | Team-based seat pricing ($12-21/user/month), only unique screenshots billed, unchanged images free. | Too new for a mature complaints record; the "unique-screenshot" billing model is itself a direct response to the volume-billing complaint above. |

### Local and open source diff engines

| Product | What it does | Review workflow | Price | User complaints |
|---------|--------------|------------------|-------|------------------|
| **odiff** | Native (Zig, SIMD) image comparison. Uses the YIQ perceptual color-difference model instead of raw RGB delta. Antialiased pixels are detected and excluded from the diff count when antialiasing mode is on. Default threshold 0.1 on a 0-1 scale. | None built in; it is an engine, not a workflow. Consumers wire it into their own CI/report layer. | Free, open source (Apache-2.0-family license). | Praised in vendor comparisons as faster than pixelmatch at scale; being a pure engine, it inherits whatever review UX wraps it, which is thin unless paired with a report generator. |
| **pixelmatch** (Mapbox) | The reference pixel-diff library most other JS tools embed or copied from. Perceptual color-difference metric plus automatic anti-aliased pixel detection (`includeAA: false` by default). `threshold` option 0-1, default 0.1. | None; it is a library, not a product. | Free, open source (ISC). | None specific found; its simplicity is the selling point, but it has no ignore-region or masking primitive of its own — callers must pre-mask images before diffing. |
| **ImageMagick `compare`** | General-purpose CLI compare with many metrics (AE, RMSE, PSNR, etc.). `-fuzz` treats color distance as a Euclidean/Pythagorean sphere so near-identical colors count as equal within the fuzz radius; only the AE metric is fuzz-aware. | None; CLI output plus an optional diff image. | Free, open source. | The fuzz/metric interaction is confusing enough that ImageMagick's own discussion forum has multi-page threads asking "how is fuzz actually calculated" — this is a documented usability sharp edge, not a rare complaint. |
| **dssim** (Kornel Lesiński) | Multi-scale SSIM variant in Rust, operating in linear-light RGB and L\*a\*b\* color space, closer to human perceptual difference than raw SSIM. Outputs `1/SSIM-1` (0 = identical, unbounded above = more different). Available as CLI, Rust crate, C library, and WASM. | None; a similarity-score engine, not a workflow. | Dual-licensed AGPL or commercial. | AGPL default license is a blocker for some commercial adopters (hence the paid commercial license option) — an actual adoption complaint, not a UX one. |
| **Resemble.js** | Browser/Node image analysis with `ignore: "antialiasing"`, `ignoreColors`, bounding-box regions, and `ignoreAreasColoredWith` (mask by baseline color). Skips pixels above a size threshold by default for performance (`largeImageThreshold`). | None built in; used as a library inside BackstopJS and others. | Free, open source (MIT). | Open GitHub issues report antialiasing-ignore not working correctly on some inputs, and a transparent-grid artifact appears on large images with AA ignored — both are open, unresolved correctness complaints against the antialiasing feature specifically. |
| **BackstopJS** | Scenario-based screenshot diff tool (wraps Resemble.js/pixelmatch-family engines) built around named page/component "scenarios." `hideSelectors` (display:none) and `removeSelectors` (DOM removal) exclude dynamic chrome from the shot; `selectors` scopes the shot to specific elements. | Generates a static HTML report with reference/test/diff triptych per scenario; no hosted dashboard, approve = re-run reference generation. | Free, open source. | Selector-based ignore config is brittle against markup churn (a CSS selector is not a stable contract the way a named region would be). |
| **jest-image-snapshot** | Jest matcher wrapping pixelmatch. Two independent thresholds: `customDiffConfig.threshold` (per-pixel color sensitivity) and `failureThreshold`/`failureThresholdType` (whole-image pass/fail, in pixel count or percent). | Snapshot file written to disk on first run; subsequent runs diff against it; failing test blocks CI. No dedicated review UI — a rejected snapshot is just a failing test with a diff PNG artifact. | Free, open source. | The two-threshold system is a frequent source of confusion (per-pixel vs whole-image), visible in the volume of blog posts written just to explain it. |
| **cargo-insta / insta** (Rust) | Snapshot testing library with a terminal-based interactive reviewer (`cargo insta review`). New snapshots land as `.snap.new` sitting next to the accepted `.snap` until reviewed. Supports both file-based and inline (in-source) snapshots. | `enter`/`a` accepts, `escape`/`r` rejects, `space`/`s` skips — a keyboard-driven terminal loop, no browser needed. `cargo insta test --review` chains test-run and review into one command. | Free, open source. | None specific found; frequently cited as the terminal UX gold standard for snapshot review precisely because it needs no server, browser, or account. This is the closest existing precedent to what Chrysoberyl's native-window reviewer should feel like, minus the window. |

### Adjacent fields

| Product | What it does | Notes for Chrysoberyl |
|---------|--------------|------------------------|
| **diff-pdf** | CLI/GUI tool that renders PDF pages to compare visually; exit code 0/1 for automation, `--output-diff` produces a highlighted PDF, interactive GUI supports page-shift alignment (Ctrl-arrow) for pages that are near- but not exactly-aligned. | Direct precedent for Chrysoberyl's PDF-page comparison feature and its exit-code CI contract. The Ctrl-arrow shift-to-align feature is a manual escape hatch for near-identical-but-offset pages — worth studying as a bounded, human-triggered alternative to automatic image registration. |
| **VMAF / SSIM / PSNR tooling (FFmpeg `libvmaf`)** | One FFmpeg filter graph emits VMAF, PSNR, and SSIM together per frame via `log_fmt=json` (a `frames` array, one entry per frame per metric) instead of three separate tool invocations. VMAF is ML-trained on human perceptual ratings; Netflix's VMAF NEG variant exists because default VMAF can be gamed by sharpening/saturation boosts before encoding. | The "one pass emits several perceptual scores per frame" pattern, and the "the naive metric can be gamed, ship a hardened variant" lesson, both apply directly to a structural-diff engine that will also be tuned against real inputs after ship. |
| **Skia GM (Golden Master) tests** | Google's rendering-correctness harness: draws reference images specifically to catch (a) unexpected rendering regressions and (b) cross-platform/cross-config rendering divergence. Chrome's "fauntlet" tool specifically pixel-compares FreeType vs Skrifa font rasterization across large font corpora to bound acceptable AA/hinting drift. | This is the closest large-scale precedent for "deterministic CPU rasterization, committed goldens, cross-platform verdict" — Chrysoberyl's core value is Skia GM's methodology generalized past one rendering engine. |
| **Unicode text-rendering-tests / fonttest** | Conformance suites that exercise FreeType/HarfBuzz/FriBidi/Raqm against known-correct shaping and rendering outcomes across many scripts. | Evidence that font/text rendering variance is a solved-enough problem to have its own conformance corpus — Chrysoberyl does not need to reinvent this, only avoid depending on GPU text rendering, which these suites do not cover (they test CPU shaping/rasterization stacks). |
| **CAD/mesh diff (CADfix, LEDAS Geometry Comparison, 3DViewStation, Autodesk ReCap Photo, MeshDev)** | Enterprise CAD-focused: face-level or surface-level geometric differencing between two versions of an assembly, or between a CAD source and a manufactured/scanned mesh. Priced and scoped for engineering teams, not CI gates. | No existing tool treats mesh diff as a CI-gated, cheap, render-and-compare problem the way Chrysoberyl's fixed eight-view rig does — this is genuinely underserved territory, not a crowded one. |

## Review Workflow Patterns (Accept/Reject Loop)

What makes a reviewer trust the loop, across the products above:

1. **The diff must be visible before the decision, not inferred.** Every trusted workflow (Chromatic, Percy, Argos, cargo-insta) shows the actual before/after/diff, never just a pass/fail verdict. Applitools' Visual AI is the one product whose classification step is *not* independently auditable by the reviewer — this is the trust gap that a structural, declarative engine can close by construction.
2. **Approval commits the new baseline atomically and is Git-linked.** Percy and Chromatic both wire "approve" directly into the next baseline, tied to a specific commit/branch. cargo-insta does the same with `.snap.new` → `.snap`. This is why Chrysoberyl's committed-goldens-in-the-repo model is not a downgrade from a hosted dashboard — it is the same mechanism, without the hosted middle-man.
3. **Terminal-based review (cargo-insta) is trusted precisely because it has no server dependency.** For a CI-first, no-account tool, this is the stronger precedent than any browser dashboard.
4. **PR-comment summaries reduce reviewer load by pre-triaging.** Argos and reg-suit both post a single PR comment listing all changed screenshots, rather than requiring a dashboard visit for zero-change PRs.

## Flaky-Baseline Mechanisms (Not Marketing Words)

Root causes and the actual, checkable mechanisms products use against them:

| Problem | Root cause | Real mechanism used |
|---------|-----------|----------------------|
| Antialiasing noise | GPU/OS/font-hinting produces different edge pixels for the same logical image | pixelmatch/odiff: detect antialiased pixels structurally (compare a pixel's neighborhood, not just its color) and exclude them from the diff count, controlled by a boolean (`includeAA` / `antialiasing`). Not a blur or fuzzy match — a classification of "this pixel is an AA edge" before the diff runs. |
| Font rendering differences across OS | Text rendering depends on the host OS's font stack and hinting, confirmed as the #1 cause of cross-platform visual test failures even with identical browser/version | The only real fixes found are environmental (run all captures inside one Docker image so every capture uses the same font stack) or engine-level (Chrysoberyl's CPU-only rasterization removes the GPU variable but does not remove font-stack variance — this remains an open risk unless Chrysoberyl also owns text shaping/rasterization end to end). |
| Animation / timing | A pixel diff captured at different points in a CSS transition or loader animation is a real pixel difference that is not a real regression | Products either freeze/disable animation before capture (CSS override), or accept the flake as a review-load cost. No tool found solves this at the diff-engine level; it is solved upstream, at capture time. |
| Timestamps / dynamic content | Any timestamp, counter, or personalized value renders differently every run | Universal mechanism: mask/hide/remove the region before comparison (Playwright `mask`, BackstopJS `hideSelectors`/`removeSelectors`, Resemble.js `ignoreAreasColoredWith`). None of these tools infer dynamic regions automatically — a human always draws the box or names the selector, once. |
| Machine-to-machine rendering variance (GPU vendor/driver) | GPU rasterization is not bit-identical across vendors/drivers, so any tool that rasterizes on the GPU inherits this as an unavoidable baseline drift | The entire hosted-visual-regression industry answers this by controlling the rendering *environment* instead: Applitools' Ultrafast Grid and Percy's render farm both centralize rendering onto one fixed cloud fleet so every customer's baseline was produced by the same machines. This is environment control, not elimination — it still assumes a fixed vendor fleet never changes. Chrysoberyl's CPU-only rasterization is a structurally different answer: it makes the baseline portable to *any* machine, not just one controlled fleet. |
| Threshold miscalibration | A single global threshold is either too strict (constant false positives) or too loose (misses real regressions) | ImageMagick's `-fuzz` (a color-distance sphere radius), pixelmatch/odiff's `threshold` (0-1 perceptual color delta), and jest-image-snapshot's dual threshold (per-pixel + whole-image) are the three real parameterizations found. All are global scalars; none of the surveyed tools scope threshold by named region or change-kind out of the box — this is a genuine, verified gap in the field that lines up with Chrysoberyl's declarative TOML rule file (tolerance scoped by kind of change and by named region). |

## Real Config Examples

**Playwright** (`playwright.config.ts`, dynamic-content masking plus global tolerance):
```typescript
export default defineConfig({
  expect: {
    toHaveScreenshot: {
      maxDiffPixelRatio: 0.02, // allow up to 2% of pixels to differ
    },
  },
});

// in a test:
await expect(page).toHaveScreenshot({
  mask: [page.locator('.timestamp'), page.locator('.user-avatar')],
});
```

**pixelmatch / odiff** (library call, not a file):
```js
const numDiffPixels = pixelmatch(img1, img2, diff, 800, 600, {
  threshold: 0.1,   // 0-1 perceptual color delta, lower = stricter
  includeAA: false, // false = ignore antialiased-edge pixels (the default)
});
```

**Resemble.js** (region + color-key ignore):
```js
resemble(file1)
  .compareTo(file2)
  .ignoreAntialiasing()
  .ignoreAreasColoredWith(255, 0, 255) // any magenta-keyed pixel is excluded
  .onComplete(function (data) { /* ... */ });
```

**BackstopJS scenario** (selector-based exclude):
```json
{
  "label": "homepage",
  "url": "http://localhost:3000",
  "hideSelectors": [".cookie-banner"],
  "removeSelectors": [".live-chat-widget"],
  "selectors": [".hero", ".pricing-table"]
}
```

**jest-image-snapshot** (dual threshold):
```js
const customConfig = { threshold: 0 }; // per-pixel sensitivity
expect(image).toMatchImageSnapshot({
  customDiffConfig: customConfig,
  failureThreshold: 0.01,           // whole-image tolerance
  failureThresholdType: 'percent',
});
```

None of these config shapes let a reviewer scope a tolerance by *kind of change* (a color shift vs a layout shift vs a missing element) — every example above is either a flat global number or a spatial region. This confirms the gap Chrysoberyl's TOML rule file is designed to close.

## Integration Points Teams Actually Use

- **CLI exit code** — universal contract (diff-pdf: 0/1; every CI-oriented tool surveyed uses this as the actual gate, dashboards are supplementary).
- **GitHub Actions** — either an official action (Chromatic, Argos, Lost Pixel all ship one) or `actions/upload-artifact` for report/diff-image artifacts plus a custom PR-comment step (the reg-actions and Playwright community patterns both do this without any vendor SDK).
- **PR status checks + PR comment** — Argos and reg-suit both post one comment summarizing all changed screenshots; this is the pattern to copy, not a dashboard link buried in a status check.
- **Storybook** — Chromatic and Lost Pixel both treat "one story = one snapshot" as the unit of capture; this is a capture-layer integration, not something the diff engine itself needs to know about.
- **Playwright / Cypress** — used purely as capture layers that hand pre-rendered screenshots to a diff engine (Vizzly, Argos, and the DIY pixelmatch pattern all work this way); Chrysoberyl's role is downstream of capture, same as odiff/pixelmatch.

## Feature Landscape

### Table Stakes (Users Expect These)

Features users assume exist. Missing these = product feels incomplete.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Perceptual pixel-diff core (not raw RGB delta) | Every surveyed engine (odiff, pixelmatch, dssim, Resemble.js) uses a perceptual or structural metric; raw RGB diff is known to over-flag | LOW | Already implied by Chrysoberyl's "structural change, not pixel count" requirement; this is the substrate the structural layer sits on. |
| Antialiasing-aware comparison | The single most common cause of false positives across every surveyed tool and every complaint thread found | MEDIUM | Must classify AA edge pixels structurally (neighborhood check), matching odiff/pixelmatch's approach, not a blur/fuzz radius (ImageMagick's `-fuzz` approach is reported as confusing). |
| Configurable global tolerance/threshold | Every engine surveyed exposes at least one scalar threshold | LOW | Already covered by the TOML rule file requirement; keep the parameterization explicit (0-1 perceptual delta), matching the field's convention so users transfer intuition from odiff/pixelmatch. |
| Named-region / masked-region ignore | Universal need (timestamps, avatars, dynamic content) across Playwright, BackstopJS, Resemble.js | MEDIUM | Chrysoberyl's declarative TOML region scoping already targets this; the field's precedent is CSS-selector or bounding-box based — Chrysoberyl's producer-named hint channel is a stronger contract than a selector (see Differentiators). |
| CLI with non-zero exit on failure | Universal CI-gate contract, confirmed across diff-pdf, jest-image-snapshot, and every hosted tool's CLI wrapper | LOW | Already an Active requirement. |
| Diff report artifact (HTML or image) | reg-suit, BackstopJS, and jest-image-snapshot all produce a static artifact as the actual review surface when no dashboard exists | MEDIUM | Needed for the CI-gate-first, no-hosted-service model — this replaces what a hosted dashboard would otherwise provide. |
| Baseline approve/reject workflow with atomic commit | Percy, Chromatic, and cargo-insta all tie approval directly to the next baseline | MEDIUM | Chrysoberyl's committed-goldens-in-repo model plus native-window review is the direct equivalent of cargo-insta's terminal loop, generalized to media. |
| Side-by-side / overlay diff view | Every review UI surveyed (dashboard or terminal) shows before/after/diff together, never a verdict alone | MEDIUM | Native window with pan/zoom/scrub already covers this; must render three views (baseline, candidate, diff overlay), not just the diff. |
| GitHub Actions integration + PR comment | Universal pattern across every hosted and open-source tool surveyed | MEDIUM | Depends on: diff report artifact (need something to summarize into the comment). |

### Differentiators (Competitive Advantage)

Features that set the product apart. Not required, but valuable.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| CPU-only deterministic rasterization | The entire hosted-VRT industry solves cross-machine variance by centralizing rendering on one controlled cloud fleet (Applitools Ultrafast Grid, Percy render farm). Chrysoberyl solves the same problem by removing the GPU from the raster path entirely, so the baseline is portable to *any* machine, not just one vendor's fleet. This is the project's stated Core Value and is verifiably different from every competitor's approach. | HIGH | This is the load-bearing differentiator; every other feature is secondary to proving this claim holds (see PROJECT.md Open Question on video decode). |
| Structural change classification (not a pixel/percent score) | No surveyed tool reports *what kind* of change occurred (moved, recolored, resized, appeared/disappeared) — every tool reports a score or a highlighted region. A reviewer still has to look and infer the kind of change themselves. | HIGH | Builds on the perceptual-diff table-stakes layer; this is the "structural diff" layer PROJECT.md's Key Decisions table calls out as pending. |
| Tolerance scoped by kind-of-change AND named region, in one declarative file | Confirmed gap: every config example found (Playwright, Resemble.js, BackstopJS, jest-image-snapshot) scopes tolerance either globally or spatially, never by change-kind. | MEDIUM | Depends on: structural change classification (you cannot scope tolerance by "kind of change" until the engine can name the kind). |
| Producer-named hint channel (regions named by the thing that produced the image, not guessed by the reviewer afterward) | Every surveyed ignore-region mechanism (CSS selector, bounding box, color key) is applied *after the fact* by whoever configures the test. A hint channel lets the producer of the image assert "this region is expected to vary" at the source. | MEDIUM | Enhances the named-region table-stakes feature; optional at runtime (works without hints, better with them). |
| One engine across six media kinds (raster, animation, video, PDF, SVG, mesh) | Every competitor surveyed is siloed: image-only (odiff, pixelmatch, Percy, Chromatic), PDF-only (diff-pdf), or CAD-mesh-only (CADfix, LEDAS). No tool found spans this range under one meaning of "changed." | HIGH | This is the "no per-thing special case" architecture bet from AGENTS.md; each additional format is a staged feature behind the same core engine, not a new engine. |
| Fixed eight-view mesh comparison rig, CI-gated and cheap | Existing mesh-diff tools (CADfix, LEDAS, 3DViewStation) are enterprise CAD tools, not CI gates, and are priced/scoped accordingly. No tool found treats mesh diff as a cheap, repo-native CI check. | HIGH | Genuinely underserved niche per the field survey; complexity is in the rig and blind-spot reporting, not the per-view image diff (which reuses the core raster engine). |
| No account, no hosted service, git-native committed goldens | Every hosted competitor's top complaint category is billing (metered snapshots/validations). Argos ($30/mo flat) and Vizzly (unique-screenshot billing) both exist specifically because teams want out of per-snapshot metering. Chrysoberyl removes the billing question entirely by having no hosted service. | LOW (architecturally; this is a non-feature, an absence) | This is a direct, verified response to the field's most common cost complaint. |

### Anti-Features (Commonly Requested, Often Problematic)

Features that seem good but create problems.

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|------------------|-------------|
| Cloud/GPU rendering grid for cross-browser parity (Applitools Ultrafast Grid / Percy render farm pattern) | Seems like the fastest way to get "every browser tested" | Every pixel produced by a GPU is vendor/driver-dependent; this is the exact thing that makes a baseline machine-dependent, which directly contradicts Chrysoberyl's Core Value. Confirmed as an industry-standard pattern, not a hypothetical risk. | CPU-only rasterization (already the project's committed approach). |
| Black-box "Visual AI" auto-classification of diffs as bug/no-bug | Applitools markets this as eliminating "99.9% of false positives," which sounds like it would kill review fatigue directly | No public documentation of the decision boundary was found; a reviewer cannot audit why a diff was suppressed, which is fatal for a tool whose entire pitch is a checkable claim ("same pair, same verdict, and you can prove it"). A silent classifier is the opposite of a provable verdict. | Deterministic structural diff plus a reviewable, declarative TOML rule file — the verdict is always traceable to a rule a human wrote. |
| Metered/volume-based billing model | Seems like a natural way to monetize and matches industry norms (Chromatic, Percy, Applitools all do it) | It is the single most common vendor complaint surfaced in this research (cost scaling with coverage, teams disabling checks to control spend) and is structurally incompatible with a local, no-account tool anyway. | No billing; local tool, no hosted service, no account (already Out of Scope in PROJECT.md). |
| Automatic/self-healing baseline updates (accept drift without a human sign-off) | Reduces review burden by "trusting" small changes automatically | Silently redefines what "correct" means over time; the Core Value depends on a baseline meaning the same thing until a human deliberately changes it. Percy/Chromatic's "approve becomes new baseline" is fine because a human still clicks approve — full auto-update without that step is the anti-feature. | Explicit approve-and-commit workflow (already implied by the committed-goldens-in-repo model). |
| Scriptable/computed tolerance or ignore-region config | Seems more flexible than a flat rule file — "just let me write a function" | Already identified in PROJECT.md: "a rule file that can compute is a rule file that can lie," and it also breaks the "reviewable in a pull request" property that makes a declarative rule auditable. | Declarative TOML scoped by kind of change and named region (already the committed design). |
| General image registration / auto-alignment for arbitrary, non-aligned pairs | Would let the tool "just work" on more inputs, including screenshots at slightly different crops/offsets | Turns a bounded engineering problem (block matching / phase correlation on near-identical pairs) into open image-registration research, and removes the assumption that makes the guarantee provable. diff-pdf's manual Ctrl-arrow shift is the honest bounded version of this: a human-triggered nudge, not automatic registration. | Assume near-identical pairs (already Out of Scope); offer a manual, bounded alignment nudge if ever needed, never automatic registration. |
| Configurable/scriptable mesh camera rig | Seems more flexible for teams with unusual mesh shapes | A rig that lives in an editable config file is a baseline that moves whenever someone edits that file — the same "rule file that can lie" problem, applied to camera placement instead of tolerance. | Fixed eight-view rig, with the blind spot named explicitly in the output (already the committed design). |
| Growing, unbounded snapshot/history storage | Seems necessary to keep a full audit trail of every run | Vizzly, Chromatic, and Percy all report storage/cost growth as a real operational problem serious enough to build billing tiers and dedup logic around. | Content-addressed hash manifest with dedup of identical baselines (already implied by "baseline hash manifest... with a pluggable store"). |

## Feature Dependencies

```
Perceptual pixel-diff core
    └──requires──> (nothing; substrate layer)

Antialiasing-aware comparison
    └──requires──> Perceptual pixel-diff core

Structural change classification
    └──requires──> Perceptual pixel-diff core
    └──requires──> Antialiasing-aware comparison

Tolerance scoped by kind-of-change + named region (TOML rule file)
    └──requires──> Structural change classification
    └──requires──> Named-region / masked-region ignore

Producer-named hint channel ──enhances──> Named-region / masked-region ignore

Diff report artifact
    └──requires──> Perceptual pixel-diff core (needs a diff image/score to report)

Baseline approve/reject workflow
    └──requires──> Diff report artifact (a reviewer needs to see the diff to approve it)
    └──requires──> Side-by-side / overlay diff view

GitHub Actions integration + PR comment
    └──requires──> Diff report artifact
    └──requires──> CLI with non-zero exit on failure

Video / PDF / mesh comparison (each)
    └──requires──> Perceptual pixel-diff core (reused, not reimplemented per format)
    └──requires──> Structural change classification (same meaning of "changed" across formats)

CPU-only deterministic rasterization ──enables──> every other feature's portability claim
    (if this breaks, no downstream feature's "same verdict on any machine" claim holds)

Cloud/GPU rendering grid (anti-feature) ──conflicts──> CPU-only deterministic rasterization
Black-box Visual AI classification (anti-feature) ──conflicts──> Tolerance scoped by kind-of-change (TOML rule file)
Scriptable tolerance config (anti-feature) ──conflicts──> Declarative TOML rule file (auditability)
```

### Dependency Notes

- **Structural change classification requires the perceptual-diff core and antialiasing handling first**: you cannot reliably name "what kind of change occurred" on top of a noisy substrate; AA noise must be filtered before classification, or every AA edge becomes a false "shape changed" verdict.
- **The TOML rule file (tolerance by kind-of-change and region) requires structural classification**: scoping a tolerance by "kind of change" is meaningless until the engine can name a kind of change. This is why the rule-file feature is downstream of the diff engine, not parallel to it — sequence phases accordingly.
- **Baseline approve/reject requires a visible diff view first**: this mirrors every trusted product surveyed (Percy, Chromatic, cargo-insta) — no product lets a reviewer approve blind.
- **Each additional media format (video, PDF, mesh) reuses the core diff engine rather than reimplementing comparison logic**: this is the architecture rule from AGENTS.md ("no per-thing special case") applied directly to sequencing — the core engine and its structural classification must be stable *before* staging a new format on top of it, matching the Key Decision "Freeze the architecture, stage the formats."
- **Black-box Visual-AI-style classification conflicts with the declarative rule file**: these are mutually exclusive design directions found in the field (Applitools vs. everyone else), and Chrysoberyl has already chosen the declarative side; do not let a future "smart" heuristic reintroduce this conflict.
- **CPU-only rasterization is a hard dependency for every portability claim in the roadmap**: if any phase introduces GPU-produced pixels into a comparison, every feature built on top inherits the same-machine-only caveat, which quietly invalidates the project's one checkable claim.

## MVP Definition

### Launch With (v1)

Minimum viable product — matches Chrysoberyl's own Active requirements, sequenced by the dependencies above.

- [ ] Perceptual, antialiasing-aware raster diff core — the substrate every other feature depends on
- [ ] CPU-only deterministic rasterization for vector/document input — the core value proposition, must ship first and be provable
- [ ] Structural change classification on top of the diff core — the actual differentiator vs. odiff/pixelmatch
- [ ] Declarative TOML rule file, tolerance scoped by kind-of-change and named region — closes the confirmed field-wide gap
- [ ] CLI with non-zero exit code and a report artifact — table-stakes CI contract
- [ ] Committed-goldens baseline store with hash manifest, pluggable backend — the anti-billing, anti-hosted differentiator
- [ ] Native window review (pan/zoom/scrub, side-by-side + diff overlay) — table-stakes review surface, no server required
- [ ] Producer-named hint channel — differentiator, optional at runtime
- [ ] Animation / numbered frame sequence comparison — reuses the core engine, first staged format

### Add After Validation (v1.x)

Features to add once core is working — matches Chrysoberyl's own feature-gated (off-by-default) Active requirements.

- [ ] Video comparison (Cargo feature, off by default) — trigger: core raster engine and structural classification proven stable; also blocked on resolving the Open Question about bit-exact decode across hardware/software paths
- [ ] PDF comparison (Cargo feature, off by default) — trigger: core engine stable; diff-pdf is a usable reference implementation to study for the exit-code/report contract
- [ ] GitHub Actions reusable workflow + standard PR-comment format — trigger: report artifact format is stable enough to template into a comment

### Future Consideration (v2+)

Features to defer until the core claim and format breadth are both validated.

- [ ] 3D mesh comparison via fixed eight-view rig — already an Active requirement but the highest-complexity item (HIGH); defer sequencing until the format-staging pattern is proven on video/PDF first, per the "freeze the architecture, stage the formats" decision
- [ ] Additional baseline store backends beyond committed goldens (e.g., a networked artifact store) — only after the pluggable-store interface is proven against the default backend
- [ ] Any AI-assisted "likely a false positive" hint layer — only ever as a suggestion surfaced next to the deterministic verdict, never replacing it, to avoid recreating the Applitools trust gap

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| CPU-only deterministic rasterization | HIGH | HIGH | P1 |
| Perceptual + antialiasing-aware diff core | HIGH | MEDIUM | P1 |
| Structural change classification | HIGH | HIGH | P1 |
| TOML rule file (tolerance by kind + region) | HIGH | MEDIUM | P1 |
| CLI exit code + report artifact | HIGH | LOW | P1 |
| Committed-goldens baseline store + hash manifest | HIGH | MEDIUM | P1 |
| Native window review (pan/zoom/scrub) | MEDIUM | MEDIUM | P1 |
| Producer-named hint channel | MEDIUM | MEDIUM | P2 |
| Animation / frame sequence comparison | MEDIUM | MEDIUM | P1 |
| Video comparison (feature-gated) | MEDIUM | HIGH | P2 |
| PDF comparison (feature-gated) | MEDIUM | MEDIUM | P2 |
| GitHub Actions workflow + PR comment | MEDIUM | LOW | P2 |
| 3D mesh eight-view rig | MEDIUM | HIGH | P3 |
| Pluggable baseline store backends beyond default | LOW | MEDIUM | P3 |

**Priority key:**
- P1: Must have for launch
- P2: Should have, add when possible
- P3: Nice to have, future consideration

## Competitor Feature Analysis

| Feature | Applitools / Chromatic / Percy (hosted) | odiff / pixelmatch / Resemble.js (local libraries) | Chrysoberyl's Approach |
|---------|------------------------------------------|------------------------------------------------------|--------------------------|
| Cross-machine consistency | Centralize rendering on one controlled cloud fleet (GPU-based) | Not addressed; assumes caller already has consistent captures | Eliminate the GPU from the raster path entirely; portable to any machine by construction |
| What changed | A score or highlighted region; Applitools' AI additionally guesses bug-vs-noise, opaquely | A pixel count or perceptual score, no semantic labeling | Structural classification: name the kind of change, not just where it is |
| Tolerance config | Global per-project settings in a hosted UI | A flat numeric threshold parameter, sometimes plus a spatial ignore region | Declarative TOML scoped by kind of change AND named region, reviewable in a PR |
| Review surface | Hosted dashboard (account required) | None; caller builds their own, or uses a wrapping tool's report/dashboard | Native window (no server, no account) plus a CI report artifact |
| Billing | Per-snapshot or per-validation metering | Free, but no support/hosting | No billing; local tool, no hosted component |
| Format breadth | Web/app screenshots only | Raster images only | Raster, animation, video, PDF, SVG, mesh — one engine, one meaning of "changed" |

## Sources

- Chromatic pricing and TurboSnap: chromatic.com/pricing, chromatic.com/docs/turbosnap, docs.chromatic.com/docs/turbosnap
- Percy: browserstack.com/percy, browserstack.com/percy/features, percy.io/pricing
- Applitools: applitools.com/platform/eyes, applitools.com/platform/ultrafast-grid
- Lost Pixel: github.com/lost-pixel/lost-pixel
- Argos CI: argos-ci.com/docs/learn/billing-and-subscription/open-source, github.com/argos-ci/argos
- reg-suit: github.com/reg-viz/reg-suit
- Vizzly: vizzly.dev/pricing, vizzly.dev/features, vizzly.dev/blog/honeydiff-vs-odiff-pixelmatch-benchmarks
- odiff: github.com/dmtrKovalenko/odiff
- pixelmatch: npmjs.com/package/pixelmatch, github.com/mapbox/pixelmatch
- ImageMagick compare/fuzz: imagemagick.org/compare, imagemagick.org discussion forum threads on `-fuzz`
- dssim: github.com/kornelski/dssim, kornel.ski/dssim
- Resemble.js: github.com/rsmbl/Resemble.js, and its open issues #52 and #151 on antialiasing-ignore correctness
- BackstopJS: github.com/garris/BackstopJS
- jest-image-snapshot: github.com/americanexpress/jest-image-snapshot, npmjs.com/package/jest-image-snapshot
- cargo-insta / insta: insta.rs/docs/cli, github.com/mitsuhiko/insta
- Playwright screenshot assertions: playwright.dev/docs/api/class-snapshotassertions
- diff-pdf: github.com/vslavik/diff-pdf, vslavik.github.io/diff-pdf
- VMAF/SSIM/PSNR: forasoft.com/learn/video-encoding/articles/quality-metrics-psnr-ssim-vmaf, arxiv.org/pdf/2107.10220
- Skia GM tests and font conformance: skia.org/docs/dev/testing/fonts, github.com/unicode-org/text-rendering-tests, developer.chrome.com/blog/memory-safety-fonts (fauntlet)
- CAD/mesh diff tools: cadinterop.com (CADfix), ledas.com/en/lgc, meshdev.sourceforge.net
- Git binary-file/LFS problem: git-tower.com/learn/git/faq/handling-large-files-with-lfs
- Review-fatigue and false-positive causes: shakacode.com/blog/flaky-visual-regression-tests-and-what-to-do-about-them, dev.to/delta-qa/reduce-false-positives-in-visual-testing-the-problem-nobody-really-solves

---
*Feature research for: visual regression and media comparison tools*
*Researched: 2026-09-06*
