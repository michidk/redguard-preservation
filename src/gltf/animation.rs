use super::*;
use crate::gltf::{UnrolledPrimitive, primitives::build_unrolled_primitives};
use crate::{Result, error::Error, import::palette::Palette, model3d::Model3DFile};
use json::animation::{
    Animation, Channel, Interpolation, Property, Sampler, Target as AnimationTarget,
};
use json::mesh::MorphTarget;

impl GltfBuilder<'_> {
    pub(in crate::gltf) fn has_model_animation(&self) -> bool {
        !self.animations.is_empty()
    }

    pub(in crate::gltf) fn append_animated_mesh(
        &mut self,
        base: Vec<UnrolledPrimitive>,
        model: &Model3DFile,
        palette: Option<&Palette>,
        fps: f32,
    ) -> Result<usize> {
        let frame_count = model.header.num_frames as usize;
        if model.frame_data.len() != frame_count
            || model.frame_vertex_data.len() != frame_count
            || model.frame_normal_data.len() != frame_count
        {
            return Err(Error::Conversion(
                "Animation has an incomplete frame table".into(),
            ));
        }
        let target_count = frame_count - 1;
        let times: Vec<f32> = (0..=frame_count).map(|frame| frame as f32 / fps).collect();
        if times.iter().any(|time| !time.is_finite())
            || times.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return Err(Error::Conversion(
                "Frame rate cannot represent distinct finite frame times".into(),
            ));
        }
        let mesh_index = self.append_mesh(base.clone());
        self.append_morph_targets(mesh_index, &base, model, palette)?;
        self.meshes[mesh_index].weights = Some(vec![0.0; target_count]);
        let input = self.push_animation_scalars(&times, true);
        let weight_count = times
            .len()
            .checked_mul(target_count)
            .ok_or_else(|| Error::Conversion("Animation weight count overflow".into()))?;
        let mut weights = vec![0.0; weight_count];
        for frame in 1..frame_count {
            weights[frame * target_count + frame - 1] = 1.0;
        }
        // Hold the final pose for one full frame interval.
        weights[frame_count * target_count + target_count - 1] = 1.0;
        let output = self.push_animation_scalars(&weights, false);
        self.append_pose_channel(input, output);
        Ok(mesh_index)
    }

    fn append_morph_targets(
        &mut self,
        mesh_index: usize,
        base: &[UnrolledPrimitive],
        model: &Model3DFile,
        palette: Option<&Palette>,
    ) -> Result<()> {
        for frame in 1..model.header.num_frames as usize {
            let posed = model.at_frame(frame)?;
            let primitives = build_unrolled_primitives(&posed, palette, self.has_texture_cache());
            if primitives.len() != base.len() {
                return Err(Error::Conversion("Animation changes mesh topology".into()));
            }
            for (index, (posed, base)) in primitives.iter().zip(base).enumerate() {
                if posed.material_key != base.material_key
                    || posed.positions.len() != base.positions.len()
                    || posed.indices != base.indices
                {
                    return Err(Error::Conversion("Animation changes mesh topology".into()));
                }
                let positions = deltas(&posed.positions, &base.positions)?;
                let normals = deltas(&posed.normals, &base.normals)?;
                let (min, max) = bounds(&positions);
                let positions = self.push_vec3_accessor(&positions, Some(min), Some(max));
                let normals = self.push_vec3_accessor(&normals, None, None);
                self.meshes[mesh_index].primitives[index]
                    .targets
                    .get_or_insert_default()
                    .push(MorphTarget {
                        positions: Some(Index::new(Self::index_u32(positions))),
                        normals: Some(Index::new(Self::index_u32(normals))),
                        tangents: None,
                    });
            }
        }
        Ok(())
    }

    fn append_pose_channel(&mut self, input: usize, output: usize) {
        if self.animations.is_empty() {
            self.animations.push(Animation {
                name: Some("Model poses".into()),
                channels: Vec::new(),
                samplers: Vec::new(),
                extensions: None,
                extras: Default::default(),
            });
        }
        let animation = &mut self.animations[0];
        animation.channels.push(Channel {
            sampler: Index::new(Self::index_u32(animation.samplers.len())),
            target: AnimationTarget {
                node: Index::new(Self::index_u32(self.nodes.len())),
                path: Checked::Valid(Property::MorphTargetWeights),
                extensions: None,
                extras: Default::default(),
            },
            extensions: None,
            extras: Default::default(),
        });
        animation.samplers.push(Sampler {
            input: Index::new(Self::index_u32(input)),
            output: Index::new(Self::index_u32(output)),
            interpolation: Checked::Valid(Interpolation::Step),
            extensions: None,
            extras: Default::default(),
        });
    }

    fn push_animation_scalars(&mut self, values: &[f32], time_bounds: bool) -> usize {
        let bytes = values
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect();
        let index = self.push_accessor_raw(
            bytes,
            values.len(),
            ComponentType::F32,
            Type::Scalar,
            Target::ArrayBuffer,
            time_bounds.then(|| serde_json::json!([values[0]])),
            time_bounds.then(|| serde_json::json!([values[values.len() - 1]])),
        );
        let view = self.accessors[index].buffer_view.unwrap().value();
        self.buffer_views[view].target = None;
        index
    }
}

fn deltas(posed: &[[f32; 3]], base: &[[f32; 3]]) -> Result<Vec<[f32; 3]>> {
    posed
        .iter()
        .zip(base)
        .map(|(posed, base)| {
            let delta = std::array::from_fn(|axis| posed[axis] - base[axis]);
            if delta.iter().all(|value| value.is_finite()) {
                Ok(delta)
            } else {
                Err(Error::Conversion(
                    "Animation contains non-finite geometry".into(),
                ))
            }
        })
        .collect()
}

fn bounds(values: &[[f32; 3]]) -> ([f32; 3], [f32; 3]) {
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for value in values {
        for axis in 0..3 {
            min[axis] = min[axis].min(value[axis]);
            max[axis] = max[axis].max(value[axis]);
        }
    }
    (min, max)
}
