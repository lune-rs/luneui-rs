---
id: core.image
title: Image drawing
area: core
status: in-progress
consumers: lunegit, wane
source: crates/lune-ui-core/src/image.rs
owner: orch/toolkit-v1
summary: Image primitive: app-decoded RGBA drawn into a rect with a fit mode, alpha and optional corner radius; explicit resource lifetime
---

## What it is

The painter's `image` primitive: the app decodes a picture to RGBA pixels, registers it once under an
id, and draws it into a rectangle; the GPU adapter uploads and samples it. The toolkit decodes no
file format and fetches nothing (D11, D13). Lunegit shows images in three places: a file viewer for
image files, standalone images in rendered Markdown and a logo on the empty repository and gate
views; all of them scale the picture uniformly to fit. This confirms D11's condition for adding the
primitive.

## Needs

- Image drawing (R-core-9).
- A new primitive recorded as plain data in the scene, drawn by the wgpu adapter, with no wgpu type in core signatures (C2, R-render-1).
- Rendered Markdown images (R-text-3) and the viewer.

## Existing code

Built in M5 (D105, D106): `crates/lune-ui-core/src/image.rs` (`ImageStore`, `ImageData`, `ImageId`, `Fit`, `image_rect`), `Painter::image`, `image_rotated`, `text_rotated`, the wgpu image pipeline in `lune-ui-wgpu`; the fake is `testing::FakeImages`. Before it, the painter had no image, gradient, shadow, arc or rotation primitive (`crates/lune-ui-core/src/paint.rs`, fill :215, fill_radii :219, ring :231, acrylic :243, custom :254, text :274-306, stroke :329).
Lunegit: `src/Lunegit.Desktop/Views/FileViewerView.axaml:352-356`, `Views/MarkdownPreviewView.axaml:166-174` and `.axaml.cs:404-483`, `Views/EmptyRepositoryView.axaml:12-22`, `ViewModels/Explorer/FileViewerViewModel.cs:80-90,234-253,519-545,616-624`.

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: Lunegit src/Lunegit.Desktop/Views/FileViewerView.axaml:333-360, Views/MarkdownPreviewView.axaml:166-174, Views/MarkdownPreviewView.axaml.cs:404-483, Views/EmptyRepositoryView.axaml:10-22,
Views/GitRequiredGateView.axaml:36, Views/HelpPageView.axaml:64, App.axaml:213, ViewModels/Explorer/FileViewerViewModel.cs:80-90,234-253,519-545,616-624; this repo crates/lune-ui-core/src/paint.rs:160-175,215-260; Wane docs/ui/components.md (no image component: a search for `image` in the component sections finds none).
A grep over `src/Lunegit.Desktop/**/*.axaml` (instrument: `grep -rhn 'Stretch="'` piped to `sort | uniq -c`, 2026-11-04) gives only `Uniform` (16 hits, six of them on `<Image>` elements); `grep -rn 'RenderOptions\|BitmapInterpolationMode'` finds none, so the sampling filter is Avalonia's default and is not stated by Lunegit.
No conflicts among sources.

### Anatomy

- Input: decoded RGBA8, straight alpha, width and height in pixels, owned by the app until registered; the toolkit never decodes. Lunegit's viewer accepts png, jpg, jpeg, gif, webp, bmp and ico, all decoded by Avalonia's `Bitmap` [L src/Lunegit.Desktop/ViewModels/Explorer/FileViewerViewModel.cs:89-90,532-533]; the app's decoder is the app's choice.
- Draw call: `image(dst_rect, id, fit, align, alpha)` recorded as plain data in the layer (`ImageItem { id, dst, fit, align, alpha, clip }`) beside the shapes, drawn by the adapter in layer order [R crates/lune-ui-core/src/paint.rs:215-260], [C2].
- Fit modes found in Lunegit's use: `Uniform` only: the picture scales, up or down, keeping its aspect ratio, to the largest size that fits the destination rect [L src/Lunegit.Desktop/Views/FileViewerView.axaml:352-356], [L src/Lunegit.Desktop/Views/MarkdownPreviewView.axaml:168], [L src/Lunegit.Desktop/Views/EmptyRepositoryView.axaml:17-20]. The primitive names it `Contain`. No fill, cover, stretch or none mode is used, so none is specified.
- Alignment inside the rect: centred in the viewer (`HorizontalAlignment` and `VerticalAlignment` centre), left in Markdown blocks, centred for the logo [L src/Lunegit.Desktop/Views/FileViewerView.axaml:354-355], [L src/Lunegit.Desktop/Views/MarkdownPreviewView.axaml:168].
- Destination size is the caller's: Markdown caps the width at 640 px, the logo is 100 by 100 (`EmptyStateLogoSize`), the viewer fills its region [L src/Lunegit.Desktop/Views/MarkdownPreviewView.axaml:168], [L src/Lunegit.Desktop/App.axaml:213], [L src/Lunegit.Desktop/Views/EmptyRepositoryView.axaml:17-20].
- Alpha: a constant multiplier for disabled or faded images (the painter already has an alpha in `Xform`) [R crates/lune-ui-core/src/paint.rs:160-175]. No tint, no rounded clip and no border in Lunegit's uses; a rounded clip (`clip-rounded`) is not required by this entry.
- Resource lifetime: the app registers pixels under an id and releases it explicitly; the adapter holds the GPU texture until then. Lunegit disposes the previous bitmap whenever it is replaced, because an undisposed native backing store leaks on every reload [L src/Lunegit.Desktop/ViewModels/Explorer/FileViewerViewModel.cs:616-624], [L src/Lunegit.Desktop/Views/MarkdownPreviewView.axaml.cs:463-483]. An unknown or released id draws nothing and is not an error.
- Size limits: Lunegit caps an image file at 20,000,000 bytes [L src/Lunegit.Desktop/ViewModels/Explorer/FileViewerViewModel.cs:83-84], but a small file can decode to a huge bitmap. The adapter refuses an image whose width or height exceeds the device's maximum texture size (it reports the device limit) and the app downsizes; the primitive never panics on a refusal [derived: the GPU limit; Lunegit has no pixel cap, grep found none].
- Sampling: bilinear when the destination differs from the source size [derived: Avalonia's default, not stated by Lunegit; Open questions]. At an integer scale of 1 the image is drawn pixel for pixel.
- No SVG: Lunegit renders Mermaid and maths to SVG through its host; the toolkit takes RGBA, so such a host rasterizes first [L src/Lunegit.Desktop/Markdown/IMarkdownPreviewHost.cs:37-49], [D13].
- Not drawn here: loading placeholders (text in `text.markdown-preview`), the "binary file" and "not found" states of the viewer (the app's) [L src/Lunegit.Desktop/Views/FileViewerView.axaml:358-390].

### States

| state | look | cite |
|---|---|---|
| drawn | the picture, `Contain` fitted in the rect | [L src/Lunegit.Desktop/Views/FileViewerView.axaml:352-356] |
| faded | alpha below 1 | [R crates/lune-ui-core/src/paint.rs:160-175] |
| unknown or released id | nothing drawn | [derived: a missing resource must not panic] |
| refused (too large) | nothing drawn; the adapter reports the refusal | [derived: GPU limit] |
| hover, pressed, focus-visible, disabled | not applicable: an image is not interactive | [derived: no Lunegit image is interactive] |

### Transitions

n/a — Lunegit swaps a loaded bitmap in at once (`IsVisible` flips) [L src/Lunegit.Desktop/Views/MarkdownPreviewView.axaml.cs:446-447]; an image appearing is high-frequency-free content with no rule to animate it. The alpha is a constant per draw.

### Input and focus

n/a — no pointer, keys, focus or layer of its own; the layer is the layer it is recorded on.

### Retained state

- Transient: none. Persistent: the registered pixels and the GPU texture, until the app releases the id. App-owned: the id, the pixels, the destination rect.
- Exit retention: no.

### Accessibility

- Role `image` on the node of the widget that draws it, name = the alt text the app passes (Lunegit's Markdown shows the alt text or the target as the placeholder) [infra.accessibility], [L src/Lunegit.Desktop/Views/MarkdownPreviewView.axaml.cs:422,438,451]. A purely decorative image (the logo) emits no node.

### Strings

None: no text of its own.

### Gallery scenes

- `image-contain-wide`: a 200 by 100 test pattern into a 100 by 100 rect, centred, settled.
- `image-contain-tall`: a 100 by 200 pattern into a 100 by 100 rect, settled.
- `image-align-left`: the wide pattern left-aligned in a 300 by 100 rect, settled.
- `image-alpha`: the pattern at alpha 0.5 over a filled rect, settled.
- `image-scale-1`: the pattern at scale 1, pixel for pixel, settled.
- `image-missing-id`: an unregistered id, nothing drawn, settled.
- `image-too-large`: an image beyond the device limit refused, nothing drawn, settled.

### Painter primitives

- Existing: fill, clip-rect, xform (alpha).
- New: image (the viewer, Markdown and logo draw bitmaps: [L src/Lunegit.Desktop/Views/FileViewerView.axaml:352], [L src/Lunegit.Desktop/Views/MarkdownPreviewView.axaml.cs:445], [L src/Lunegit.Desktop/Views/EmptyRepositoryView.axaml:17]).

### Acceptance

- Geometry (pure function, known answers): a 200 by 100 image `Contain` in (0, 0, 100, 100) is drawn at (0, 25, 100, 50); a 100 by 200 image in the same rect at (25, 0, 50, 100); left-aligned in (0, 0, 300, 100) a 200 by 100 image is drawn at (0, 0, 200, 100) (scale 1); a zero-size image or rect draws nothing and does not panic.
- Pixel probes at scale 2: the wide pattern in a 100 by 100 rect covers 200 by 100 device pixels at (0, 50); a 2 by 2 pattern of four colours shows four exact quadrants (nearest sampling at an integer ratio) [derived: sampling is open].
- Recording: the painter records one `ImageItem` per call with the right layer and clip; `FakeImages` (a recording adapter) sees the same list; no wgpu type appears in a core signature (boundary script).
- Lifetime: releasing an id frees the texture; drawing it afterwards draws nothing; registering a second image under a used id replaces it and frees the old one [L src/Lunegit.Desktop/ViewModels/Explorer/FileViewerViewModel.cs:621-624].
- Limits: an image over the device limit is refused without a panic, and an RGBA buffer whose length is not `4 * width * height` is refused.
- Conformance and semantic node: no transitions; the host widget's node carries role `image` and the alt text.

### Build

- Milestone: M5. Built in M5 (steps 5 and 6; the page is `image`, 7 scenes). Open for `done` (D47): M11 re-checks the whole bar on every entry; the Windows and Linux run time is unverified (the Linux and Windows lint is `scripts/check-targets.sh`).
- Depends on: core.painter, core.scene, render.gpu, render.frame, render.surface

## Open questions

- Answered (D106): sampling filter: Lunegit leaves Avalonia's default; the toolkit picks bilinear for scaled draws and nearest for integer ratios, or states one filter; M5 decides and records the pixel probe.
- Answered (D106, straight alpha in, premultiplied on upload): whether alpha-premultiplied input is accepted: the app passes straight-alpha RGBA; the painter's colours are straight alpha with premultiply helpers (R-core-3).
- Answered (D105, `clip-rounded` applies to images): a rounded clip for avatars or thumbnails: none in Lunegit's uses; out of this entry until a consumer names it.
- Still open (D106): texture memory budget and eviction: the app releases explicitly; a cache with a cap is not specified (D41 sets no memory target in v1).

## Notes for the planner

New in M3 (PD9, R-core-9). The primitive is the only new painter item the viewer, Markdown and logo need; the arcs, rotation and dash needs of other entries are separate `New:` lines in their own specs and are aggregated into `core.painter` by step 15.
