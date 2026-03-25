/*
 * rgpre.h — C header for the Redguard Preservation native plugin.
 *
 * Generated from src/ffi/ (types.rs, scene.rs, mod.rs).
 * Keep in sync with the Rust source when adding or changing FFI functions.
 */

#ifndef RGPRE_H
#define RGPRE_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ── Memory ─────────────────────────────────────────────────────────── */

typedef struct ByteBuffer {
    uint8_t *ptr;
    int32_t  length;
    int32_t  capacity;
} ByteBuffer;

void        rg_free_buffer(ByteBuffer *buffer);
ByteBuffer *rg_last_error(void);

/* ── Texture structs ────────────────────────────────────────────────── */

typedef struct TextureHeader {      /* 16 bytes */
    int32_t width;
    int32_t height;
    int32_t frame_count;
    int32_t rgba_size;
    /* followed by rgba_size bytes of RGBA pixel data */
} TextureHeader;

typedef struct AllFramesHeader {    /* 12 bytes */
    int32_t width;
    int32_t height;
    int32_t frame_count;
    /* followed by frame_count frames, each prefixed by int32_t rgba_size */
} AllFramesHeader;

/* ── RGMD (mesh data) ──────────────────────────────────────────────── */

typedef struct RgmdHeader {         /* 28 bytes */
    uint8_t magic[4];               /* "RGMD" */
    uint8_t version[4];             /* format version (1.0.0.0) */
    int32_t submesh_count;
    int32_t frame_count;            /* 1 for models, 0 for terrain */
    int32_t total_vertex_count;
    int32_t total_index_count;
    float   radius;                 /* bounding sphere, scaled coords */
} RgmdHeader;

typedef struct RgmdSubmeshHeader {  /* 16 bytes */
    uint8_t  textured;              /* 0 = solid color, 1 = textured */
    uint8_t  color_r;               /* resolved RGB (solid) or 0 */
    uint8_t  color_g;
    uint8_t  color_b;
    uint16_t texture_id;            /* TEXBSI id (textured) or 0 */
    uint8_t  image_id;              /* TEXBSI image (textured) or 0 */
    uint8_t  _pad;
    int32_t  vertex_count;
    int32_t  index_count;
} RgmdSubmeshHeader;

typedef struct RgmdVertex {         /* 32 bytes */
    float position[3];
    float normal[3];
    float uv[2];
} RgmdVertex;
/* Followed by index_count × uint32_t indices. */

/* ── RGPL (placements + lights) ────────────────────────────────────── */

typedef struct RgplHeader {         /* 12 bytes */
    uint8_t magic[4];               /* "RGPL" */
    int32_t placement_count;
    int32_t light_count;
} RgplHeader;

typedef struct RgplPlacement {      /* 132 bytes */
    uint8_t  model_name[32];        /* null-terminated ASCII */
    uint8_t  source_id[32];         /* null-terminated ASCII */
    float    transform[16];         /* 4×4 column-major matrix */
    uint16_t texture_id;            /* TEXBSI texture for flat sprites */
    uint8_t  image_id;              /* TEXBSI image for flat sprites */
    uint8_t  object_type;           /* 0=mesh, 1=flat sprite, 2=rope link */
} RgplPlacement;

typedef struct RgplLight {          /* 64 bytes */
    uint8_t name[32];               /* null-terminated ASCII */
    float   color[3];               /* linear RGB, 0.0–1.0 */
    float   position[3];
    float   range;
    uint8_t light_type;             /* 0 = point */
    uint8_t _pad[3];
} RgplLight;

/* ── ROB (segment archive) ─────────────────────────────────────────── */

typedef struct RobHeader {          /* 4 bytes */
    int32_t segment_count;
} RobHeader;

typedef struct RobSegmentHeader {   /* 16 bytes */
    uint8_t segment_name[8];        /* null-terminated ASCII */
    uint8_t has_model;              /* 0 or 1 */
    uint8_t _pad[3];
    int32_t model_data_size;        /* 0 if no model */
    /* if has_model == 1, followed by model_data_size bytes of RGMD data */
} RobSegmentHeader;

/* ── GLB export ────────────────────────────────────────────────────── */

ByteBuffer *rg_convert_model_from_path(const char *file_path, const char *assets_dir);
ByteBuffer *rg_convert_rgm_from_path(const char *file_path, const char *assets_dir);
ByteBuffer *rg_convert_wld_from_path(const char *file_path, const char *assets_dir);

/* ── RGM section access ───────────────────────────────────────────── */

int32_t     rg_rgm_section_count(const char *file_path, const char *section_tag);
ByteBuffer *rg_get_rgm_section(const char *file_path, const char *section_tag, int32_t section_index);

/* ── Scene data ────────────────────────────────────────────────────── */

ByteBuffer *rg_parse_model_data(const char *file_path, const char *assets_dir);
ByteBuffer *rg_parse_rob_data(const char *file_path, const char *assets_dir);
ByteBuffer *rg_parse_wld_terrain_data(const char *file_path);
ByteBuffer *rg_parse_rgm_placements(const char *file_path);

/* ── Textures ──────────────────────────────────────────────────────── */

ByteBuffer *rg_decode_texture(const char *assets_dir, uint16_t texture_id, uint8_t image_id);
ByteBuffer *rg_decode_texture_all_frames(const char *assets_dir, uint16_t texture_id, uint8_t image_id);
int32_t     rg_texbsi_image_count(const char *assets_dir, uint16_t texture_id);
int32_t     rg_gxa_frame_count(const char *file_path);
ByteBuffer *rg_decode_gxa(const char *file_path, int32_t frame);

/* ── Audio ─────────────────────────────────────────────────────────── */

ByteBuffer *rg_convert_sfx_to_wav(const char *file_path, int32_t effect_index);
int32_t     rg_sfx_effect_count(const char *file_path);
ByteBuffer *rg_convert_rtx_entry_to_wav(const char *file_path, int32_t entry_index);
int32_t     rg_rtx_entry_count(const char *file_path);
ByteBuffer *rg_get_rtx_subtitle(const char *file_path, int32_t entry_index);

/* ── Other ─────────────────────────────────────────────────────────── */

ByteBuffer *rg_convert_fnt_to_ttf(const char *file_path);

#ifdef __cplusplus
}
#endif

#endif /* RGPRE_H */
