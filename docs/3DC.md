# 3DC File Format Documentation

The `3DC` files contain 3D model data, including vertices, faces, normals, and texture mapping information. The format is a binary, little-endian file.

## File Structure

A `3DC` file is organized as follows:
1.  A 64-byte header.
2.  Several data blocks for frames, faces, vertices, etc. The header contains offsets to these blocks.

The file format has multiple versions (e.g., v2.6, v2.7, v4.0, v5.0), which can affect the structure of some data blocks, particularly face data.

### Header

The 64-byte header contains metadata about the model and offsets to the various data sections.

| Offset | Size | Data Type | Description |
|--------|------|------------------|-------------|
| 0x00   | 4    | `[u8; 4]`        | Version string (e.g., "v2.7") |
| 0x04   | 4    | `u32`            | Number of vertices |
| 0x08   | 4    | `u32`            | Number of faces |
| 0x0C   | 4    | `u32`            | Radius of the model's bounding sphere |
| 0x10   | 4    | `u32`            | Number of animation frames |
| 0x14   | 4    | `u32`            | Offset to frame data |
| 0x18   | 4    | `u32`            | Number of UV offsets (for some versions) |
| 0x1C   | 4    | `u32`            | Offset to Section 4 (unknown purpose) |
| 0x20   | 4    | `u32`            | Count for Section 4 |
| 0x24   | 4    | `u32`            | Unknown (always 0) |
| 0x28   | 4    | `u32`            | Offset to UV offsets table |
| 0x2C   | 4    | `u32`            | Offset to UV data |
| 0x30   | 4    | `u32`            | Offset to vertex coordinates |
| 0x34   | 4    | `u32`            | Offset to face normals |
| 0x38   | 4    | `u32`            | Number of UV offsets (duplicate?) |
| 0x3C   | 4    | `u32`            | Offset to face data |

### Data Sections

#### 1. Frame Data
- Located at `offset_frame_data`.
- Contains animation keyframe data. The exact structure is unknown.

#### 2. Face Data
- Located at `offset_face_data`.
- A list of `num_faces` structures.
- Each face is composed of a header and a list of vertices.

**Face Structure**
| Size | Data Type | Description |
|------|------------------|-------------|
| 1    | `u8`             | Number of vertices in this face (`vertex_count`) |
| 1    | `u8`             | Unknown |
| variable | `TextureData`    | Texture information. See below. |
| 4    | `u32`            | Unknown |
| variable | `Vec<FaceVertex>`| List of vertices. |

**TextureData**
- If the face is solid-colored, this is a `u8` color index.
- If textured, this is a structure containing a `texture_id` (`u16`) and an `image_id` (`u8`).

**FaceVertex Structure**
| Size | Data Type | Description |
|------|------------------|-------------|
| 4    | `u32`            | Index into the main vertex coordinate list. |
| 2    | `i16`            | U texture coordinate. |
| 2    | `i16`            | V texture coordinate. |

#### 3. Vertex Coordinates
- Located at `offset_vertex_coords`.
- A list of `num_vertices` 3D points.

**VertexCoord Structure**
| Size | Data Type | Description |
|------|------------------|-------------|
| 4    | `f32`            | X coordinate |
| 4    | `f32`            | Y coordinate |
| 4    | `f32`            | Z coordinate |

#### 4. Face Normals
- Located at `offset_face_normals`.
- A list of `num_faces` 3D vectors.

**FaceNormal Structure**
| Size | Data Type | Description |
|------|------------------|-------------|
| 4    | `f32`            | X component |
| 4    | `f32`            | Y component |
| 4    | `f32`            | Z component |

#### 5. UV Offsets and Coordinates
- The `offset_uv_offsets` points to a table of `u32` offsets.
- The `offset_uv_data` points to the UV coordinate data. The structure is not fully clear but seems to be a list of 3D vectors similar to `VertexCoord`.
