# Glide terrain lighting

Terrain receives an RGB lighting value at each vertex, after its normals are
computed and before rasterization.

## World inputs

**Verified:** world loading sets the Glide ambient channels from
`world_ambientrgb[N]`. If that entry is absent, the channels default to
`128,128,128`.

The directional-light channel strengths are the first three components of
`world_sunrgb[N]`, defaulting to `128,128,100` when the entry is absent.
The fourth component does not multiply those channels in the terrain lighting
calculation. World loading constructs the direction using `world_sunangle[N]`
and `world_sunskew[N]` and the engine's trigonometric tables. The direction
setter normalizes a nonzero vector. A zero vector disables directional lighting.

These are the Glide terrain inputs. The `world_sun[N]` vector and intensity,
`world_ambient[N]`, and `world_ambientfx[N]` are not operands of this terrain
vertex calculation. The visible sun image has separate tint inputs described
in [sky.md](sky.md).

The installed ISLAND day entry uses ambient RGB `64,64,64` and directional RGB
`255,255,255`. Its `world_sunrgb` fourth component is 2, which does not double
those directional RGB values.

## Vertex calculation

**Verified:** let `A` be the ambient RGB vector, `S` the directional RGB
strength, `D` the normalized light direction, and `N` the supplied terrain
vertex normal, expressed in the same renderer coordinate system.

The directional weight is `max(0, -dot(D, N))`. With directional lighting
disabled, the weight is zero. Each output channel is
`clamp(A[channel] + weight * S[channel], 0, 255)`.

The routine consumes the supplied normal without normalizing it again.
It retains floating-point RGB values at this stage. It does not apply
material roughness, specular lighting, inverse-square attenuation, or the
model renderer's local-light list to this terrain calculation.

For ambient `64,64,64`, directional strength `255,220,200`, and direction
`0,1,0`, a normal of `0,-1,0` gives `255,255,255` after clamping. A normal of
`0,1,0` gives `64,64,64`. A normal of `-0.6,-0.8,0` gives approximately
`255,240,224`.

## Terrain updates

**Verified:** rebuilding the camera's terrain grid recomputes face normals,
then vertex normals, then vertex lighting. The wave update repeats these
stages for dirty terrain entries after changing heights. Lighting therefore
uses the displaced water normals, as described in [water.md](water.md).
