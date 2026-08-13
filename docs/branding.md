# Invokrum visual identity

Invokrum uses a restrained technical-botanical identity: protective layers surround a deterministic core. The mark is intentionally structural rather than decorative and mirrors the product boundary: consumer-defined overlays surround a small generic composition engine.

## Name and visual rationale

**Invokrum** combines the sound of *invoke* with *involucrum*, the botanical structure of bracts surrounding a flower or flower cluster.

The mark expresses three ideas:

- **hexagonal shell** — a bounded, inspectable technical system;
- **green bracts** — ordered protective layers that add constraints around the core;
- **blue core** — the deterministic composition mechanism that remains stable beneath consumer policy.

The visual system should stay technical, compact, and legible. Avoid ornamental botanical illustration, photorealistic leaves, proprietary typefaces, or imagery that implies the engine itself supplies governance policy.

## Assets

| Asset | Intended use |
| --- | --- |
| `assets/invokrum-mark.svg` | Default repository/documentation mark |
| `assets/invokrum-mark-light.svg` | Mark tuned for light host backgrounds |
| `assets/invokrum-mark-dark.svg` | Mark tuned for dark host backgrounds |
| `assets/invokrum-favicon.svg` | Simplified small-size mark for favicons and compact UI |
| `assets/invokrum-social-square.svg` | Square repository/social image on a self-contained dark field |

All assets are SVG and have explicit `viewBox` geometry, accessible `<title>` and `<desc>` elements, and no font dependencies.

## Size guidance

Use `invokrum-favicon.svg` at **16px and 32px**. It intentionally removes the lower bract layers and internal border detail that become noisy at favicon scale.

Use the full mark variants at **128px and 512px**. The default README rendering at 168px is also within the full-detail range.

Use `invokrum-social-square.svg` for square repository/social surfaces. Export raster derivatives only when a platform requires them; keep the SVG as the source of truth.

Do not add fine strokes, text, or extra internal details to the favicon source. Do not stretch any mark non-uniformly.

## Color roles

The palette is deliberately small:

- shell dark: `#17232c` / dark-surface shell `#243440`;
- structural outline: `#435d6e` / dark-surface outline `#88a5b7`;
- deterministic core: `#237eaf` / dark-surface core `#58c8e8`;
- protective layers: `#328a58`, `#26774c`, `#205f40` on light-host variants and `#72cf79`, `#55b96c`, `#439d60`, `#a2e482` on dark-host variants;
- social field: `#0b1218`.

These colors are identity colors, not a general application UI palette.

## Contrast and accessibility

The repository tests calculate representative relative-luminance contrast pairs as a design guardrail:

- light shell `#17232c` against white: approximately **16.0:1**;
- light blue core `#237eaf` against `#17232c`: approximately **3.6:1**;
- light green layer `#328a58` against `#17232c`: approximately **3.7:1**;
- dark-surface outline `#88a5b7` against `#0b1218`: approximately **7.3:1**;
- dark blue core `#58c8e8` against `#243440`: approximately **6.6:1**;
- dark green layer `#72cf79` against `#243440`: approximately **6.7:1**.

The dark shell itself is intentionally subtle against the social background; silhouette separation is supplied by the brighter structural outline. These ratios are visual-identity checks rather than a claim that a logo substitutes for accessible UI text or controls.

When embedding the mark in HTML, retain useful alt text. Recommended concise alt text:

> Invokrum: protective green layers surrounding a blue deterministic core.

If the mark is purely decorative and adjacent text already identifies Invokrum, use an empty HTML `alt` attribute instead of duplicating the label for screen-reader users.

## Ownership and licensing

The SVG assets in this repository use project-authored vector geometry and do not embed third-party fonts, stock images, or external image resources. They are distributed with Invokrum under the repository's Apache License 2.0, to the extent applicable rights exist. Contributions or refinements to these assets follow the same repository contribution and licensing terms unless a file explicitly states otherwise.

Generated raster exports are derivatives of these SVG sources and should preserve the same licensing notice when redistributed as project assets.

## Change control

Brand changes should preserve:

- the protective-layer/deterministic-core metaphor;
- a legible small-size variant;
- accessible SVG metadata;
- no proprietary font dependency;
- strong separation of the core and protective layers;
- clear ownership/licensing provenance.

The automated brand-asset tests are intentionally narrow. Human review remains necessary for visual balance and recognition after geometry or palette changes.
