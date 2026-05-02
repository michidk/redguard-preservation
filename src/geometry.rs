use crate::model3d::Model3DFile;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TriangleWinding {
    Forward,
    Reversed,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct GeometryConvention {
    pub(crate) x_sign: f32,
    pub(crate) y_sign: f32,
    pub(crate) z_sign: f32,
    pub(crate) winding: TriangleWinding,
}

pub(crate) const GLB_CONVENTION: GeometryConvention = GeometryConvention {
    x_sign: -1.0,
    y_sign: -1.0,
    z_sign: 1.0,
    winding: TriangleWinding::Forward,
};

pub(crate) const FFI_CONVENTION: GeometryConvention = GeometryConvention {
    x_sign: 1.0,
    y_sign: -1.0,
    z_sign: 1.0,
    winding: TriangleWinding::Reversed,
};

#[must_use]
pub(crate) const fn sanitize_f32(value: f32) -> f32 {
    if value.is_nan() { 0.0 } else { value }
}

#[must_use]
pub(crate) fn transform_position(
    x: f32,
    y: f32,
    z: f32,
    scale: f32,
    convention: GeometryConvention,
) -> [f32; 3] {
    [
        convention.x_sign * sanitize_f32(x) / scale,
        convention.y_sign * sanitize_f32(y) / scale,
        convention.z_sign * sanitize_f32(z) / scale,
    ]
}

#[must_use]
pub(crate) fn transform_normal(x: f32, y: f32, z: f32, convention: GeometryConvention) -> [f32; 3] {
    [
        convention.x_sign * sanitize_f32(x),
        convention.y_sign * sanitize_f32(y),
        convention.z_sign * sanitize_f32(z),
    ]
}

#[must_use]
pub(crate) fn resolve_vertex_normal(
    model: &Model3DFile,
    vertex_index: usize,
    cumulative_fv_index: usize,
    face_normal: [f32; 3],
    convention: GeometryConvention,
) -> [f32; 3] {
    let vn_index = if !model.normal_indices.is_empty() {
        model
            .normal_indices
            .get(cumulative_fv_index)
            .and_then(|&index| usize::try_from(index).ok())
    } else if !model.vertex_normals.is_empty() {
        Some(vertex_index)
    } else {
        None
    };

    if let Some(idx) = vn_index
        && let Some(vn) = model.vertex_normals.get(idx)
        && !vn.x.is_nan()
        && !vn.y.is_nan()
        && !vn.z.is_nan()
    {
        return transform_normal(vn.x, vn.y, vn.z, convention);
    }

    face_normal
}

#[must_use]
pub(crate) const fn triangle_vertex_offsets(i: usize, winding: TriangleWinding) -> [usize; 3] {
    match winding {
        TriangleWinding::Forward => [0, i, i + 1],
        TriangleWinding::Reversed => [0, i + 1, i],
    }
}
