# Glide shading

The world renderer calculates lighting at polygon corners and combines the
interpolated lighting colors with textures, fog and framebuffer color precision.

## Flat and smooth shading

Lighting produces an RGB color at each source corner before clipping and
rasterization. The rasterizer interpolates these colors across each submitted
triangle, a process called Gouraud shading. It interpolates lighting colors
rather than normals; it does not evaluate the lighting formula anew per pixel.

Normal selection is a separate step. A face normal shared by all corners gives
the same directional-light contribution at each corner. Different corner normals
can give different contributions and a smoother appearance across neighboring
faces. Local lights also depend on corner positions, so a shared face normal
does not require identical corner colors or a uniformly lit face.

Terrain constructs a shading normal at each grid vertex from the normalized
normals of its incident triangles, then normalizes the sum. These operations use
the engine's quantized reciprocal square root. Models instead consume supplied
normals, retaining their magnitude. The model format describes face normals,
vertex normals, corner indirection and face-normal fallback.

## Color pipeline

1. Prepare corner lighting from ambient RGB, directional light and, for models,
   face brightness and admitted local lights. Retained instance prelighting can
   replace part of the model calculation; actor eligibility depends on the
   runtime model kind and attributes, not simply whether an actor is stationary.
2. Clip polygons while carrying their corner RGB and texture coordinates, then
   project and submit the surviving polygons as triangle fans.
3. Interpolate lighting RGB linearly in screen space. Texture coordinates use
   perspective correction. Sampling uses the selected palette, allocation and
   address rules, and the filtering mode selected by the drawing path.
4. Modulate sampled texture RGB by the interpolated lighting. Solid-color model
   faces incorporate their resolved palette color into corner lighting instead.
   Texture filtering and shading are independent: a smoothly lit surface can
   still use point-sampled textures.
5. Apply fog on encoded RGB channels and the framebuffer's RGB565 quantization
   and dithering rules. These are separate from the lighting calculation. The
   world lighting path has no scene exposure or filmic tone-mapping stage.

This describes the ordinary model and terrain color paths. Placed flats have
their own constant-alpha blending and ordering rules. Sky and sun use separate
drawing and blending paths. Water changes terrain geometry and shading normals;
its displacement and normal-update rules are specified separately.

## Detailed rules

| Topic | Reference |
|---|---|
| Model normals and hard edges | [Model shading selection](../formats/models/3d.md#shading-model-selection) |
| Model brightness, directional light and local lights | [Model lighting](model-lighting.md#ambient-and-directional-contribution) |
| Retained map-light colors | [World prelighting](model-lighting.md#world-prelighting) and [actor prelighting](actor-prelighting.md) |
| Terrain lighting and averaged normals | [Terrain vertex calculation](terrain-lighting.md#vertex-calculation) and [shading normals](terrain-lighting.md#shading-normals) |
| Clipping, projection and color gradients | [Model clipping and projection](model-lighting.md#polygon-clipping), [terrain preparation](terrain-lighting.md#polygon-preparation), [screen-space color setup](terrain-lighting.md#screen-space-color-interpolation) and [view projection](projection.md) |
| Texture coordinates, filtering and modulation | [Model texture addressing](model-lighting.md#texture-address-selection), [model draw flags](model-lighting.md#draw-flag-ownership), [terrain texture setup](terrain-lighting.md#texture-coordinates-and-setup) and [texture combination](terrain-lighting.md#texture-combination) |
| World palette conversion | [COL palette precision](../formats/COL.md#glide-world-palette-conversion) and [model palette ownership](model-lighting.md#palette-ownership) |
| Fog, framebuffer precision and dithering | [Fog lookup](fog.md#fog-lookup) and [integer precision](fog.md#integer-precision-and-framebuffer-dithering) |
| Camera-facing flats and transparency | [Placed flat drawing](model-lighting.md#placed-flat-drawing) and [framebuffer blending](model-lighting.md#flat-framebuffer-blending) |
| Sky and sun | [Sky plane](sky.md#glide-sky-plane) and [sun framebuffer blending](sky.md#sun-framebuffer-blending) |
| Animated terrain and textures | [Water waves](water.md) and [texture animation](texture-animation.md) |
