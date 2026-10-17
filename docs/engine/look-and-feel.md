# Look and feel

The rendered scene combines authored palettes and textures with separate rules
for terrain, models, sky, and on-screen presentation. Matching those rules keeps
lighting, distance, motion, and framing consistent with the original engine.

## Camera framing and distance

[View projection](projection.md) fixes the horizontal field of view. Changing
the display aspect ratio changes the vertical field of view, so a wider viewport
does not simply reveal more of the world on either side.

[Fog and clipping](fog.md) determine how geometry disappears into the distance.
Scene fog depends on camera-space depth, using a lookup table and the world's
fog color. The sky has its own fog table and clipping distances. Its distant
texture blends toward the same color used to fill uncovered background pixels.
The RGB565 framebuffer and dithering also affect the final color, especially
in gradual fog transitions.

## Terrain color and shading

[WLD terrain](../formats/WLD.md) supplies heights, texture choices, and texture
rotation. [Terrain lighting](terrain-lighting.md) combines ambient RGB with a
directional contribution at each vertex. The resulting color depends on the
terrain normals, then passes through polygon clipping, screen-space color
interpolation, and texture combination.

The terrain and model lighting paths have different inputs. Terrain does not
use the model renderer's local-light list. World settings for ambient color,
sun direction, and directional RGB are described in
[WORLD.INI](../config/world-ini.md).

## Models, palettes, and local lights

[Model lighting](model-lighting.md) combines face brightness, ambient and
directional light, local lights, and retained instance colors. World-entry
prelighting stores map-light contributions for static placements and eligible
[static-model actors](actor-prelighting.md). These stored colors continue to
matter when the world is drawn.

[COL palettes](../formats/COL.md) supply the colors of indexed artwork.
Palette ownership and transparency rules determine how texture edges and
camera-facing flats appear. Model textures, terrain textures, and placed flats
have distinct drawing rules; the model reference covers chroma-key rejection,
texture filtering, flat blending, and depth writes.

## Water shape and changing light

[Water waves](water.md) displace selected terrain vertices. Their phase uses
squared distance from the whole terrain midpoint and the BIOS clock. World
wave settings control amplitude, speed, and spatial frequency. Moving the
camera changes the terrain window, but does not move the ripple origin.

Displacement also changes normals and lighting. The engine updates marked
entries and retains the other entries' earlier values until a later update or
grid rebuild. Those rules affect water-edge shading as well as the wave shape.
[Terrain lighting](terrain-lighting.md#terrain-updates) describes how the changed
normals feed back into vertex color.

## Sky, sun, and texture motion

The [sky](sky.md) is a finite, tilted plane with scrolling texture coordinates.
It is visible from either side and can be crossed vertically. Its background
fill, fog, scrolling direction, and sun-disc blending each contribute to the
visible horizon.

[Texture animation](texture-animation.md) supplies motion within textured
surfaces. Playback state belongs to a loaded image, so objects sharing that
image also share its current frame. Authored delays and explicit frame
selection determine animation timing independently of water displacement.

## Movement, interface, and sound

[Player movement](player.md) and [actor animation](animation.md) determine the
pace of movement, turning, and animation. Their coordinate and timing rules
connect the camera view to what the player does.

The [HUD](hud.md), [book menu](menu.md), and [subtitles](subtitles.md) define
artwork placement, fonts, draw order, and text timing. The book menu also uses
3D models and animated camera paths. [Positional sound](sound.md) supplies
distance attenuation and stereo pan; [music](music.md) supplies world-specific
CD tracks and their switching rules.

## Renderer-specific resources

Choose the renderer's matching model, texture, and palette resources before
applying these rules. The [asset directory and edition reference](../#asset-directories)
describes the available sets and their differences. Software
[texture banks](../formats/TEXTURE.md) and
[shade and haze tables](../formats/SHADE-HAZE.md) have their own formats.
The Glide rendering pages describe that renderer's behavior.
