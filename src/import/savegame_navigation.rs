//! Original-save actor navigation identities, distinct from native task words.
use super::SaveObject;
use crate::Result;
use crate::error::Error;

/// Actor-owned navigation retained in the object body.
///
/// Loading binds the saved map identity to an initialized map and copies all
/// four indices unchanged. It clears global node reservations separately; held
/// indices do not imply that a node is reserved. Ordinary navigation validation
/// runs when the resumed command updates, not while decoding the save.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SaveActorNavigation {
    /// Initialized walk-map identity; zero is valid, all-ones denotes no map.
    pub map_id: Option<u32>,
    /// Held current-node index; minus one denotes no held node.
    pub current_node: i32,
    /// Held next-node index; minus one denotes no held node.
    pub next_node: i32,
    /// Previous next-node history, independent of current reservations.
    pub previous_next_node: i32,
    /// Final destination index; minus one denotes no destination.
    pub destination_node: i32,
}

impl SaveActorNavigation {
    /// Resolve the saved identity against explicitly initialized map identities.
    ///
    /// The returned index belongs to the supplied list, not the original linked
    /// list order. Missing or ambiguous identities reject before any mutation.
    /// Node indices remain untouched, including stale values awaiting ordinary
    /// navigation validation. This operation neither claims nor clears nodes.
    pub fn bind_map(self, initialized_ids: &[u32]) -> Result<Option<usize>> {
        let Some(id) = self.map_id else {
            return Ok(None);
        };
        let mut matches = initialized_ids
            .iter()
            .enumerate()
            .filter(|(_, value)| **value == id);
        let Some((index, _)) = matches.next() else {
            return Err(Error::Parse(format!(
                "save game SMOB navigation: walk-map identity {id} is not initialized"
            )));
        };
        if matches.next().is_some() {
            return Err(Error::Parse(format!(
                "save game SMOB navigation: walk-map identity {id} is ambiguous"
            )));
        }
        Ok(Some(index))
    }
}

impl SaveObject {
    /// Decode saved walk-map identity and node history without reconstructing a
    /// route or accepting the saved map field as a host pointer.
    pub fn navigation(&self) -> SaveActorNavigation {
        let word = |offset| u32::from_le_bytes(self.body[offset..offset + 4].try_into().unwrap());
        let map = word(0x1e9);
        SaveActorNavigation {
            map_id: (map != u32::MAX).then_some(map),
            current_node: word(0x1ed) as i32,
            next_node: word(0x1f1) as i32,
            previous_next_node: word(0x1f5) as i32,
            destination_node: word(0x1f9) as i32,
        }
    }
}
