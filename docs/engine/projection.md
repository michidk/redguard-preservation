# Glide view projection

The Glide renderer uses a fixed horizontal half-field tangent of `0.788` for
world and book-menu rendering. The viewport is initialized to the full display.

For viewport width `w`, height `h`, and top-left corner `(left, top)`, the
normalized horizontal and vertical projection scales are:

```text
sx = 1 / 0.788
sy = (w / h) / 0.788
```

The viewport center is `(left + w/2, top + h/2)`. For camera-space coordinates
`(x, y, z)`, with positive Y down and positive Z forward, screen coordinates are:

```text
screen_x = left + w/2 + (w/2) * x / (0.788 * z)
screen_y = top  + h/2 + (w/2) * y / (0.788 * z)
```

The equivalent vertical field of view is `2 * atan(0.788 * h / w)`, about
61.166 degrees at 640 by 480. The horizontal field of view stays fixed when
the viewport aspect ratio changes.

The renderer subtracts its view position from world coordinates before applying
the view matrix. Horizontal and vertical projection scales multiply the first
and second rows of that matrix; the depth row remains unscaled.

See [fog and clipping](fog.md) for depth ranges and [book menu](menu.md) for its
camera paths.
