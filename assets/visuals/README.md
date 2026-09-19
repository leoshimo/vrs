# Visual assets

- `logomark.png`: selected N16 / E Ink retained-image mark, cropped from `source/n16.png`.
- `manicule.png`: original generated engraved pointer, cropped and reduced from
  `source/manicule.png`. CSS points it toward the tour label.
- `readme-{light,dark}.{gif,png}`: README banners, rendered from the same sphere
  and lettering effects as the website. PNGs serve reduced-motion readers.
- `sphere-{light,dark}.png`: the website's no-JavaScript/no-WebGL fallback.
- `source/prompts.json`: generation prompts for the selected artwork. The raster
  sources are preserved; prompts document provenance, not deterministic regeneration.

Generate these with [`tools/visuals/export-visuals`](../../tools/visuals/README.md).
Do not hand-edit generated exports. Keep changes to the renderer and its exports
in the same commit. Only the selected direction is shipped here; exploration
rounds remain outside the product repository.

Charter is already bundled under `docs/fonts/`, with its original
[Bitstream redistribution notice](../../docs/fonts/charter-LICENSE.txt).
No additional font is needed for the logomark, which is raster artwork.
