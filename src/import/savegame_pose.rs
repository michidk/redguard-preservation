//! Saved actor pose ownership, independent of the animation command cursor.
use super::SaveObject;
use crate::{Result, error::Error};

/// A selected frame in the original world's model allocation table.
/// The handle is not an RAAN index, file frame reference or host resource ID.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SaveModelFrame {
    pub model_handle: i16,
    pub frame: i16,
}

/// Actor-owned selected and previous poses and stored default group.
/// Full object load retains these words independently of the playback header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SaveActorPose {
    pub current: SaveModelFrame,
    pub previous: SaveModelFrame,
    pub default_group: i16,
}

/// An explicitly initialized original model allocation and its installed frames.
/// Consumers must obtain the handle from model registration, not infer it from
/// an actor ID or from a matching frame number. Distinct texture variants have
/// distinct handles even when their geometry source is shared.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveModelAllocation {
    pub handle: i16,
    pub source: String,
    pub frame_count: usize,
    /// Zero for the base resource; otherwise the registered texture variant.
    pub texture: u16,
}

/// A validated frame bound to an entry in the supplied allocation list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoundSaveModelFrame {
    pub allocation: usize,
    pub frame: usize,
}

impl SaveModelFrame {
    /// Bind a selected pose without changing any allocation or playback state.
    /// A minus-one model handle denotes no selected pose, even if its frame
    /// word retains an earlier value. Missing, ambiguous or
    /// out-of-bounds ownership rejects instead of substituting a baseline.
    pub fn bind(self, allocations: &[SaveModelAllocation]) -> Result<Option<BoundSaveModelFrame>> {
        if self.model_handle == -1 {
            return Ok(None);
        }
        let invalid = |reason| {
            Error::Parse(format!(
                "save game SMOB model {}/frame {}: {reason}",
                self.model_handle, self.frame
            ))
        };
        if self.model_handle < 1 || self.frame < 0 {
            return Err(invalid("invalid selected pose"));
        }
        let mut matches = allocations
            .iter()
            .enumerate()
            .filter(|(_, a)| a.handle == self.model_handle);
        let Some((allocation, model)) = matches.next() else {
            return Err(invalid("model allocation is not initialized"));
        };
        if matches.next().is_some() {
            return Err(invalid("model allocation is ambiguous"));
        }
        let frame = self.frame as usize;
        if model.source.is_empty() || frame >= model.frame_count {
            return Err(invalid("installed model source or frame is invalid"));
        }
        Ok(Some(BoundSaveModelFrame { allocation, frame }))
    }
}

impl SaveObject {
    /// Decode both retained model/frame pairs and the stored default group.
    /// This does not derive a pose from the current animation command.
    pub fn pose(&self) -> SaveActorPose {
        let word = |offset| i16::from_le_bytes(self.body[offset..offset + 2].try_into().unwrap());
        SaveActorPose {
            current: SaveModelFrame {
                frame: word(0x24f),
                model_handle: word(0x251),
            },
            previous: SaveModelFrame {
                frame: word(0x253),
                model_handle: word(0x255),
            },
            default_group: word(0x257),
        }
    }
}

/// Ordered model registration requests made while initializing a world.
/// Registration deduplicates a base source and then its texture variants. A
/// variant consumes a separate handle after its base has been registered.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SaveModelAllocations {
    entries: Vec<SaveModelAllocation>,
}
impl SaveModelAllocations {
    /// Register an installed model in original request order. Source is the
    /// resolved installed filename; path resolution belongs to the install.
    /// Repeated requests must agree on the installed frame count.
    pub fn register(&mut self, source: &str, frame_count: usize, texture: u16) -> Result<i16> {
        if source.is_empty() || frame_count == 0 || texture > 511 {
            return Err(Error::Parse(
                "save game model registration: invalid installed source, frame count or texture"
                    .into(),
            ));
        }
        let source = source.replace('\\', "/").to_ascii_uppercase();
        // Validate before adding either the base or its variant.
        if self
            .entries
            .iter()
            .any(|a| a.source == source && a.frame_count != frame_count)
        {
            return Err(Error::Parse(format!(
                "save game model registration {source}: inconsistent installed frame count"
            )));
        }
        let missing = [0, texture]
            .into_iter()
            .filter(|t| {
                !self
                    .entries
                    .iter()
                    .any(|a| a.source == source && a.texture == *t)
            })
            .collect::<std::collections::BTreeSet<_>>();
        if self.entries.len() + missing.len() > i16::MAX as usize {
            return Err(Error::Parse(
                "save game model registration: handle capacity exceeded".into(),
            ));
        }
        for texture in [0, texture] {
            if !self
                .entries
                .iter()
                .any(|a| a.source == source && a.texture == texture)
            {
                self.entries.push(SaveModelAllocation {
                    handle: (self.entries.len() + 1) as i16,
                    source: source.clone(),
                    frame_count,
                    texture,
                });
            }
        }
        Ok(self
            .entries
            .iter()
            .find(|a| a.source == source && a.texture == texture)
            .unwrap()
            .handle)
    }
    /// Allocation order is stable; frame bindings index this slice.
    pub fn entries(&self) -> &[SaveModelAllocation] {
        &self.entries
    }
}

impl SaveModelAllocations {
    /// Reconstruct registration from installed placements, followed by explicitly
    /// ordered persistent actor definitions. The resolver selects the installed
    /// model file and returns its actual frame count. Static ROB models do not
    /// consume animated allocation handles. No save bytes influence this table.
    pub fn from_world(
        world: &crate::import::rgm::RgmFile,
        persistent_actors: &[&str],
        mut resolve: impl FnMut(&str) -> Result<(String, usize)>,
    ) -> Result<Self> {
        let metadata = crate::import::rgm::export_rgm_metadata_json(world, None);
        let actors = metadata["actors"]
            .as_array()
            .ok_or_else(|| Error::Parse("installed world: missing actor metadata".into()))?;
        let mut table = Self::default();
        let mut register_actor = |name: &str, fallback: Option<(&str, u8)>| -> Result<()> {
            let actor = actors
                .iter()
                .find(|a| {
                    a["script_name"]
                        .as_str()
                        .is_some_and(|n| n.eq_ignore_ascii_case(name))
                })
                .ok_or_else(|| {
                    Error::Parse(format!("installed world: missing model owner {name}"))
                })?;
            let entries = actor["raan_entries"].as_array();
            let animated = actor["model_load_path"] == "animation_table";
            if animated && let Some(entries) = entries {
                let texture = actor["raex"]["texture_id"].as_u64().unwrap_or(0);
                let texture = u16::try_from(texture).map_err(|_| {
                    Error::Parse(format!("installed owner {name}: invalid texture"))
                })?;
                for entry in entries {
                    let path = entry["file_path"].as_str().ok_or_else(|| {
                        Error::Parse(format!("installed owner {name}: missing RAAN model"))
                    })?;
                    let (source, frames) = resolve(path)?;
                    table.register(&source, frames, texture)?;
                }
            } else if entries.is_none()
                && let Some((path, count)) = fallback
                && count != 1
                && !path.is_empty()
            {
                let (source, frames) = resolve(path)?;
                table.register(&source, frames, 0)?;
            }
            Ok(())
        };
        let placements = metadata["mpob_objects"]
            .as_array()
            .ok_or_else(|| Error::Parse("installed world: missing placements".into()))?;
        for placement in placements {
            if placement["object_type"] == 1 {
                let name = placement["script_name"].as_str().ok_or_else(|| {
                    Error::Parse("installed placement: missing actor name".into())
                })?;
                let model = placement["model_name"].as_str().unwrap_or("");
                let count = placement["is_static"]
                    .as_u64()
                    .and_then(|n| u8::try_from(n).ok())
                    .ok_or_else(|| {
                        Error::Parse(format!(
                            "installed placement {name}: invalid model frame count"
                        ))
                    })?;
                register_actor(name, Some((model, count)))?;
            }
        }
        for name in persistent_actors {
            register_actor(name, None)?;
        }
        Ok(table)
    }
}
