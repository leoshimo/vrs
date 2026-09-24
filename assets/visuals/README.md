# Visual assets

- `logomark.png`: selected N16 / E Ink retained-image mark, cropped from `source/n16.png`.
- `manicule.png`: original generated engraved pointer, cropped and reduced from
  `source/manicule.png`. CSS points it toward the tour label.
- `readme-{light,dark}.{gif,png}`: README banners, rendered from the same sphere
  and lettering effects as the website, with transparent backgrounds. Light mode
  uses dark ink; dark mode uses light ink. GIFs repeat a seamless twelve-second
  cycle; PNGs serve reduced-motion readers.
- `sphere-{light,dark}.png`: the website's no-JavaScript/no-WebGL fallback.
- `../vrs-venn.png` and `../vrs-venn-dark.png`: the influences diagram with only
  the exterior paper removed. Circle fills and intersections remain opaque.
  Both are derived from `source/venn.png`; dark mode inverts the same pixels.
- `source/prompts.json`: generation prompts for the selected artwork. The raster
  sources are preserved; prompts document provenance, not deterministic regeneration.

Generate these with [`tools/visuals/export-visuals`](../../tools/visuals/README.md).
Do not hand-edit generated exports. Keep changes to the renderer and its exports
in the same commit. Only the selected direction is shipped here; exploration
rounds remain outside the product repository.

Charter is already bundled under `tools/docs/assets/fonts/`, with its original
[Bitstream redistribution notice](../../tools/docs/assets/fonts/charter-LICENSE.txt).
No additional font is needed for the logomark, which is raster artwork.
