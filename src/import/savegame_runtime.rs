//! Bounded saved object records; opaque fields are retained, never host pointers.
use crate::{Result, error::Error};

/// One SMOB object, including fields whose runtime meaning is not decoded.
/// References remain saved identities/offsets and require initialized-world binding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveObject {
    pub object_id: i32,
    pub rope_id: u32,
    pub record_kind: u8,
    pub body: [u8; 783],
    /// Presence byte and optional nine-byte installed resource name.
    pub named_resource: Option<Vec<u8>>,
    pub attributes: Option<[u8; 256]>,
    pub opaque_41: Option<[u8; 41]>,
    pub opaque_20: Option<[u8; 20]>,
    pub animation_header: Option<[u8; 37]>,
    pub script_state: Option<[u8; 422]>,
    pub attachment: Option<[u8; 8]>,
    /// Redundant saved vector blocks, retained verbatim.
    pub vectors: [u8; 48],
    pub opaque_60: Option<[u8; 60]>,
    pub private_variables: Vec<u8>,
    /// Uninterpreted extension after known tails; never silently discarded.
    pub extension: Vec<u8>,
}
impl SaveObject {
    /// Initialized runtime identity used to bind an ordinary saved actor.
    /// Special player record identities remain distinct from this value.
    pub fn runtime_identifier(&self) -> u16 {
        u16::from_le_bytes(self.body[20..22].try_into().unwrap())
    }

    /// Saved pitch, heading and roll, each wrapping modulo 2048.
    pub fn orientation(&self) -> [i32; 3] {
        std::array::from_fn(|i| {
            i32::from_le_bytes(self.body[288 + i * 4..292 + i * 4].try_into().unwrap())
        })
    }

    /// Supporting actor's SMOB identity; minus one denotes no actor support.
    /// This is an identity to resolve, never a native pointer or a vector offset.
    pub fn support_object_id(&self) -> Option<i32> {
        let identity = i32::from_le_bytes(self.body[468..472].try_into().unwrap());
        (identity != -1).then_some(identity)
    }

    /// World position in signed 24.8 map units.
    pub fn position_q8(&self) -> [i32; 3] {
        std::array::from_fn(|i| {
            i32::from_le_bytes(self.body[203 + i * 4..207 + i * 4].try_into().unwrap())
        })
    }
}
fn invalid(message: impl std::fmt::Display) -> Error {
    Error::Parse(format!("save game SMOB: {message}"))
}
fn take<'a>(bytes: &mut &'a [u8], size: usize) -> Result<&'a [u8]> {
    let result = bytes
        .get(..size)
        .ok_or_else(|| invalid(format!("truncated field needing {size} bytes")))?;
    *bytes = &bytes[size..];
    Ok(result)
}
fn array<const N: usize>(bytes: &mut &[u8]) -> Result<[u8; N]> {
    Ok(take(bytes, N)?.try_into().unwrap())
}
fn word(body: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(body[offset..offset + 4].try_into().unwrap())
}
/// Decode SMOB framing and conditional tails using the saved presence fields.
/// The script-state marker is an identity: zero is present, -1 is absent.
/// Binding must additionally validate this against initialized actor topology.
pub fn parse_save_objects(mut bytes: &[u8]) -> Result<Vec<SaveObject>> {
    let mut objects = Vec::new();
    while !bytes.is_empty() {
        let size = u32::from_le_bytes(array(&mut bytes)?) as usize;
        let mut record = take(&mut bytes, size)?;
        let object_id = i32::from_le_bytes(array(&mut record)?);
        let rope_id = u32::from_le_bytes(array(&mut record)?);
        let record_kind = take(&mut record, 1)?[0];
        let body: [u8; 783] = array(&mut record)?;
        let named_resource = if body[0xc1] == 1 {
            let presence = take(&mut record, 1)?[0];
            let mut name = vec![presence];
            if presence != 0 {
                name.extend_from_slice(take(&mut record, 9)?);
            }
            Some(name)
        } else {
            None
        };
        let attributes = (word(&body, 0x272) != 0)
            .then(|| array(&mut record))
            .transpose()?;
        let opaque_41 = (word(&body, 0x23f) != 0)
            .then(|| array(&mut record))
            .transpose()?;
        let opaque_20 = (word(&body, 0x243) != 0)
            .then(|| array(&mut record))
            .transpose()?;
        let animation_header = (word(&body, 0x247) != 0)
            .then(|| array(&mut record))
            .transpose()?;
        let script_state = (word(&body, 0x23b) != u32::MAX)
            .then(|| array(&mut record))
            .transpose()?;
        let attachment = (rope_id != 0).then(|| array(&mut record)).transpose()?;
        let vectors = array(&mut record)?;
        let opaque_60 = (word(&body, 0x21) == 1)
            .then(|| array(&mut record))
            .transpose()?;
        let private_variables = if word(&body, 0x1c) != 0 {
            take(&mut record, usize::from(body[0x20]) * 4)?.to_vec()
        } else {
            Vec::new()
        };
        objects.push(SaveObject {
            object_id,
            rope_id,
            record_kind,
            body,
            named_resource,
            attributes,
            opaque_41,
            opaque_20,
            animation_header,
            script_state,
            attachment,
            vectors,
            opaque_60,
            private_variables,
            extension: record.to_vec(),
        });
    }
    Ok(objects)
}
/// Encode decoded records losslessly, rejecting edited presence/length mismatches.
pub fn encode_save_objects(objects: &[SaveObject]) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    for object in objects {
        let mut record = Vec::new();
        record.extend_from_slice(&object.object_id.to_le_bytes());
        record.extend_from_slice(&object.rope_id.to_le_bytes());
        record.push(object.record_kind);
        record.extend_from_slice(&object.body);
        if let Some(v) = &object.named_resource {
            record.extend_from_slice(v);
        }
        if let Some(v) = &object.attributes {
            record.extend_from_slice(v);
        }
        if let Some(v) = &object.opaque_41 {
            record.extend_from_slice(v);
        }
        if let Some(v) = &object.opaque_20 {
            record.extend_from_slice(v);
        }
        if let Some(v) = &object.animation_header {
            record.extend_from_slice(v);
        }
        if let Some(v) = &object.script_state {
            record.extend_from_slice(v);
        }
        if let Some(v) = &object.attachment {
            record.extend_from_slice(v);
        }
        record.extend_from_slice(&object.vectors);
        if let Some(v) = &object.opaque_60 {
            record.extend_from_slice(v);
        }
        record.extend_from_slice(&object.private_variables);
        record.extend_from_slice(&object.extension);
        let length = u32::try_from(record.len()).map_err(|_| invalid("record too large"))?;
        let mut framed = length.to_le_bytes().to_vec();
        framed.extend_from_slice(&record);
        if parse_save_objects(&framed)?.as_slice() != std::slice::from_ref(object) {
            return Err(invalid("presence fields disagree with supplied tails"));
        }
        bytes.extend_from_slice(&framed);
    }
    Ok(bytes)
}

/// SMCO code owner identity and packed runtime state. Script references remain
/// offsets; live owner links and runtime-only fields must be rebound on restore.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveCodeOwner {
    pub owner_id: u32,
    pub body: [u8; 204],
}
/// Decode complete fixed-width SMCO records without interpreting host pointers.
pub fn parse_save_code_owners(bytes: &[u8]) -> Result<Vec<SaveCodeOwner>> {
    if !bytes.len().is_multiple_of(208) {
        return Err(Error::Parse(
            "save game SMCO: incomplete 208-byte owner record".into(),
        ));
    }
    Ok(bytes
        .as_chunks::<208>()
        .0
        .iter()
        .map(|record| SaveCodeOwner {
            owner_id: u32::from_le_bytes(record[..4].try_into().unwrap()),
            body: record[4..].try_into().unwrap(),
        })
        .collect())
}
/// Encode code-owner records, preserving all undecoded fields.
pub fn encode_save_code_owners(owners: &[SaveCodeOwner]) -> Vec<u8> {
    owners
        .iter()
        .flat_map(|owner| owner.owner_id.to_le_bytes().into_iter().chain(owner.body))
        .collect()
}
/// SMSH shell identified by its eight-byte installed name. Runtime pointers in
/// the body are opaque; the variable array is restored into initialized storage.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveShell {
    pub body: [u8; 186],
    pub variables: Vec<u8>,
}
impl SaveShell {
    /// Installed shell name bytes, without interpreting encoding or padding.
    pub fn name(&self) -> &[u8] {
        &self.body[..8]
    }
}
/// Decode counted SMSH records and their private variable arrays.
/// Consumers must validate the world shell count, then reconstruct each shell
/// from its installed named definition before applying saved state.
pub fn parse_save_shells(mut bytes: &[u8]) -> Result<Vec<SaveShell>> {
    let invalid = |message: &str| Error::Parse(format!("save game SMSH: {message}"));
    if bytes.len() < 4 {
        return Err(invalid("missing shell count"));
    }
    let count = u32::from_le_bytes(bytes[..4].try_into().unwrap()) as usize;
    bytes = &bytes[4..];
    if count > bytes.len() / 186 {
        return Err(invalid("shell count exceeds payload"));
    }
    let mut shells = Vec::new();
    for _ in 0..count {
        let body: [u8; 186] = bytes[..186].try_into().unwrap();
        bytes = &bytes[186..];
        let size = (word(&body, 0x28) as usize)
            .checked_mul(4)
            .ok_or_else(|| invalid("variable length overflow"))?;
        let variables = bytes
            .get(..size)
            .ok_or_else(|| invalid("truncated variable array"))?
            .to_vec();
        bytes = &bytes[size..];
        shells.push(SaveShell { body, variables });
        if shells.len() < count && bytes.len() < 186 {
            return Err(invalid("truncated shell body"));
        }
    }
    if !bytes.is_empty() {
        return Err(invalid("trailing bytes"));
    }
    Ok(shells)
}
/// Encode shell records; reject inconsistent variable counts.
pub fn encode_save_shells(shells: &[SaveShell]) -> Result<Vec<u8>> {
    let invalid = |message: &str| Error::Parse(format!("save game SMSH: {message}"));
    let count = u32::try_from(shells.len()).map_err(|_| invalid("too many shells"))?;
    let mut bytes = count.to_le_bytes().to_vec();
    for shell in shells {
        if u64::from(word(&shell.body, 0x28)) * 4 != shell.variables.len() as u64 {
            return Err(invalid("variable count differs from body"));
        }
        bytes.extend_from_slice(&shell.body);
        bytes.extend_from_slice(&shell.variables);
    }
    Ok(bytes)
}

/// Saved relative references. Zero denotes no reference; nonzero values are
/// relative to the installed world's native-hook or script base respectively.
/// They are not Cyrus instruction numbers or host addresses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveScriptReferences {
    pub native_hook: u32,
    pub scripts: [u32; 4],
    pub callbacks: [u32; 6],
}
impl SaveScriptReferences {
    fn read(body: &[u8], native: usize, scripts: usize, callbacks: usize) -> Self {
        Self {
            native_hook: word(body, native),
            scripts: std::array::from_fn(|i| word(body, scripts + i * 4)),
            callbacks: std::array::from_fn(|i| word(body, callbacks + i * 4)),
        }
    }
}
impl SaveObject {
    /// Saved script offsets before initialized-world relocation.
    pub fn script_references(&self) -> SaveScriptReferences {
        SaveScriptReferences::read(&self.body, 0x25c, 0x260, 0x27f)
    }
    /// Next combat allocation identity in the optional combat record.
    /// Prefer [`SaveObject::combat`] for typed combat ownership.
    pub fn script_owner_id(&self) -> Option<u32> {
        self.script_state.as_ref().map(|state| word(state, 0))
    }
    /// Combat owner and target identities in the optional combat record. Retains saved
    /// sentinel identities rather than resolving them against a live world.
    pub fn script_object_ids(&self) -> Option<[i32; 2]> {
        self.script_state.as_ref().map(|state| {
            std::array::from_fn(|i| {
                i32::from_le_bytes(state[4 + i * 4..8 + i * 4].try_into().unwrap())
            })
        })
    }
}
impl SaveCodeOwner {
    /// Saved script offsets before initialized-world relocation.
    pub fn script_references(&self) -> SaveScriptReferences {
        SaveScriptReferences::read(&self.body, 0x1f, 0x23, 0x42)
    }
}
impl SaveShell {
    /// Saved script offsets before installed-name binding and relocation.
    pub fn script_references(&self) -> SaveScriptReferences {
        SaveScriptReferences::read(&self.body, 0x0e, 0x12, 0x30)
    }
}

/// One retained SOUP task slot. Callback values are saved script references,
/// not native pointers or actor-local instruction indices.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SaveTaskSlot {
    /// Zero denotes an empty slot; nonzero values name the byte after its opcode.
    pub callback: u32,
    /// Blocking-task byte, retained without boolean normalization.
    pub status: u8,
    /// Nonzero pauses the slot without advancing its timer.
    pub pause: u8,
    /// Wrapping invocation counter; elapsed input also needs the current frame scale.
    pub timer: u16,
    /// Four retained native invocation words in task-variable field order.
    /// Preserve all bits, including values retained by an empty or paused slot.
    pub variables: [u32; 4],
}

impl SaveObject {
    /// Decode all six task slots, preserving inactive-slot words as well.
    /// Code references require binding against the initialized world's script base.
    /// Restoring animation alone does not restore these task continuations.
    pub fn task_slots(&self) -> [SaveTaskSlot; 6] {
        std::array::from_fn(|index| SaveTaskSlot {
            callback: word(&self.body, 0x27f + index * 4),
            status: self.body[0x2af + index],
            pause: self.body[0x2b5 + index],
            timer: u16::from_le_bytes(
                self.body[0x2bb + index * 2..0x2bd + index * 2]
                    .try_into()
                    .unwrap(),
            ),
            variables: [0x297, 0x2c7, 0x2df, 0x2f7].map(|base| word(&self.body, base + index * 4)),
        })
    }

    /// Signed words from the optional object-private allocation. These are
    /// separate from authored script locals in SVRG.
    pub fn private_variable_words(&self) -> Vec<i32> {
        self.private_variables
            .as_chunks::<4>()
            .0
            .iter()
            .map(|bytes| i32::from_le_bytes(*bytes))
            .collect()
    }

    /// Optional authored-local byte offset into the world's SVRG array.
    /// Zero is a valid offset; only the all-ones sentinel means absent.
    pub fn script_local_offset(&self) -> Option<u32> {
        let offset = word(&self.body, 0x276);
        (offset != u32::MAX).then_some(offset)
    }

    /// Optional object-private words, not authored script locals.
    #[deprecated(
        note = "use private_variable_words; bind authored locals through SaveScriptLocals"
    )]
    pub fn script_variables(&self) -> Vec<i32> {
        self.private_variable_words()
    }
}

impl SaveTaskSlot {
    /// Elapsed input for the next existing-task invocation at the supplied Q8
    /// frame scale. Arithmetic wraps before shifting; this does not advance the timer.
    pub fn elapsed(&self, frame_scale: u32) -> i32 {
        (u32::from(self.timer).wrapping_mul(frame_scale) >> 8) as i32
    }
}

/// Saved actor script references and result bytes, before world binding.
/// References stay in the saved code domain; zero references remain zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SaveScriptContinuation {
    pub data: u32,
    pub entry: u32,
    pub cursor: u32,
    pub return_to: u32,
    pub result: u8,
    pub saved_result: u8,
    /// Actor Anchor byte. Code and shell owners do not retain Anchor between
    /// their temporary actor executions, so their execution view starts at zero.
    pub anchor: u8,
}

impl SaveObject {
    /// Read the actor's script continuation without relocating code or executing it.
    /// Restore with matching task slots, invocation words and local variables.
    pub fn script_continuation(&self) -> SaveScriptContinuation {
        let [data, entry, cursor, return_to] = self.script_references().scripts;
        SaveScriptContinuation {
            data,
            entry,
            cursor,
            return_to,
            result: self.body[0x270],
            saved_result: self.body[0x271],
            anchor: self.body[0x25b],
        }
    }
}

/// Saved world's authored script-local storage from SVRG, in signed word order.
/// This is distinct from each object's optional private allocation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveScriptLocals {
    words: Vec<i32>,
}

/// Authored-local range bound to one saved actor identity.
/// The initialized world must establish the actor's matching script definition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SaveActorLocals<'a> {
    pub object_id: i32,
    pub runtime_identifier: u16,
    pub byte_offset: u32,
    pub words: &'a [i32],
}

/// Decode SVRG's flat little-endian signed words without executing scripts.
pub fn parse_save_script_locals(bytes: &[u8]) -> Result<SaveScriptLocals> {
    if !bytes.len().is_multiple_of(4) {
        return Err(Error::Parse(format!(
            "save game SVRG: unaligned payload length {}",
            bytes.len()
        )));
    }
    Ok(SaveScriptLocals {
        words: bytes
            .as_chunks::<4>()
            .0
            .iter()
            .map(|word| i32::from_le_bytes(*word))
            .collect(),
    })
}

impl SaveScriptLocals {
    /// Resolve a retained instance byte offset against this current SVRG pool.
    /// Counts come from the matching installed script definition. The source
    /// need not remain a member of the loaded world's actor census.
    ///
    /// # Errors
    /// Rejects unaligned offsets, overflowing counts and ranges outside the pool.
    pub fn bind_offset(&self, byte_offset: u32, variable_count: usize) -> Result<&[i32]> {
        if !byte_offset.is_multiple_of(4) {
            return Err(Error::Parse(format!(
                "save game SVRG: retained local offset {byte_offset} is unaligned"
            )));
        }
        let start = usize::try_from(byte_offset / 4)
            .map_err(|_| Error::Parse("save game SVRG: retained local offset overflow".into()))?;
        let end = start
            .checked_add(variable_count)
            .ok_or_else(|| Error::Parse("save game SVRG: retained local count overflow".into()))?;
        self.words.get(start..end).ok_or_else(|| Error::Parse(format!(
            "save game SVRG: retained local offset {byte_offset} count {variable_count} exceeds current pool"
        )))
    }
    /// Bind an actor's retained byte offset using its initialized script's word
    /// count. The count must come from the matching installed definition, never
    /// from a neighboring actor offset or the private-variable tail.
    /// Missing storage is accepted only for a definition with no variables.
    pub fn bind_actor(
        &self,
        actor: &SaveObject,
        variable_count: usize,
    ) -> Result<Option<SaveActorLocals<'_>>> {
        let fail = |reason: &str| {
            Error::Parse(format!(
                "save game SVRG: actor {} (runtime {}) local count {variable_count}: {reason}",
                actor.object_id,
                actor.runtime_identifier()
            ))
        };
        let Some(byte_offset) = actor.script_local_offset() else {
            return if variable_count == 0 {
                Ok(None)
            } else {
                Err(fail("missing authored-local storage"))
            };
        };
        if !byte_offset.is_multiple_of(4) {
            return Err(fail("unaligned authored-local offset"));
        }
        let start = usize::try_from(byte_offset / 4).map_err(|_| fail("local offset overflow"))?;
        let end = start
            .checked_add(variable_count)
            .ok_or_else(|| fail("local count overflow"))?;
        let words = self
            .words
            .get(start..end)
            .ok_or_else(|| fail("authored-local range exceeds payload"))?;
        Ok(Some(SaveActorLocals {
            object_id: actor.object_id,
            runtime_identifier: actor.runtime_identifier(),
            byte_offset,
            words,
        }))
    }
}

/// Authored-local range bound to a saved code-owner identity in SVRG.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SaveCodeLocals<'a> {
    pub owner_id: u32,
    pub byte_offset: u32,
    pub words: &'a [i32],
}

/// Shell-owned local words. The installed definition supplies the expected
/// count; saved allocation pointers are never accepted as ownership.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveShellLocals {
    pub name: [u8; 8],
    pub words: Vec<i32>,
}

impl SaveCodeOwner {
    /// Complete code-owner interpreter view before relocation. Anchor is cleared
    /// in the temporary actor and is not copied back to the code record.
    pub fn script_continuation(&self) -> SaveScriptContinuation {
        let [data, entry, cursor, return_to] = self.script_references().scripts;
        SaveScriptContinuation {
            data,
            entry,
            cursor,
            return_to,
            result: self.body[0x33],
            saved_result: self.body[0x34],
            anchor: 0,
        }
    }

    /// All retained native words/status/timers, including empty slots. Pause is
    /// zero at each temporary actor invocation and is not retained by this owner.
    pub fn task_slots(&self) -> [SaveTaskSlot; 6] {
        compact_task_slots(&self.body, 0x42, 0x72, 0x78, [0x5a, 0x84, 0x9c, 0xb4])
    }

    /// Byte offset in the world's SVRG allocation, with the all-ones absent
    /// sentinel. The initialized code owner's script determines its word count.
    pub fn script_local_offset(&self) -> Option<u32> {
        let offset = word(&self.body, 0x3a);
        (offset != u32::MAX).then_some(offset)
    }
}

impl SaveShell {
    /// Complete shell interpreter view. Anchor is cleared for each temporary
    /// actor execution rather than persisted in the shell record.
    pub fn script_continuation(&self) -> SaveScriptContinuation {
        let [data, entry, cursor, return_to] = self.script_references().scripts;
        SaveScriptContinuation {
            data,
            entry,
            cursor,
            return_to,
            result: self.body[0x22],
            saved_result: self.body[0x23],
            anchor: 0,
        }
    }

    /// All retained task words/status/timers. Pause is reset for each shell run;
    /// there is no saved pause array in the shell body.
    pub fn task_slots(&self) -> [SaveTaskSlot; 6] {
        compact_task_slots(&self.body, 0x30, 0x60, 0x66, [0x48, 0x72, 0x8a, 0xa2])
    }

    /// Validate the shell-local allocation against its matching initialized
    /// named definition. Shell words are separate from world SVRG storage.
    /// This never relocates or uses the saved allocation pointer.
    pub fn bind_locals(&self, variable_count: usize) -> Result<SaveShellLocals> {
        let fail = || {
            Error::Parse(format!(
                "save game SMSH shell {:?}: local allocation differs from installed count {variable_count}",
                String::from_utf8_lossy(self.name())
            ))
        };
        if usize::try_from(word(&self.body, 0x28)).ok() != Some(variable_count)
            || variable_count.checked_mul(4) != Some(self.variables.len())
        {
            return Err(fail());
        }
        Ok(SaveShellLocals {
            name: self.body[..8].try_into().unwrap(),
            words: self
                .variables
                .as_chunks::<4>()
                .0
                .iter()
                .map(|bytes| i32::from_le_bytes(*bytes))
                .collect(),
        })
    }
}

fn compact_task_slots(
    body: &[u8],
    callbacks: usize,
    status: usize,
    timers: usize,
    variables: [usize; 4],
) -> [SaveTaskSlot; 6] {
    std::array::from_fn(|index| SaveTaskSlot {
        callback: word(body, callbacks + index * 4),
        status: body[status + index],
        pause: 0,
        timer: u16::from_le_bytes(
            body[timers + index * 2..timers + index * 2 + 2]
                .try_into()
                .unwrap(),
        ),
        variables: variables.map(|base| word(body, base + index * 4)),
    })
}

impl SaveScriptLocals {
    /// Bind SVRG storage to the initialized code owner's installed script count.
    /// Zero is a valid offset. No range is inferred from adjacent owners.
    pub fn bind_code_owner(
        &self,
        owner: &SaveCodeOwner,
        variable_count: usize,
    ) -> Result<Option<SaveCodeLocals<'_>>> {
        let fail = |reason: &str| {
            Error::Parse(format!(
                "save game SVRG code owner {} local count {variable_count}: {reason}",
                owner.owner_id
            ))
        };
        let Some(byte_offset) = owner.script_local_offset() else {
            return if variable_count == 0 {
                Ok(None)
            } else {
                Err(fail("missing authored-local storage"))
            };
        };
        if !byte_offset.is_multiple_of(4) {
            return Err(fail("unaligned authored-local offset"));
        }
        let start = usize::try_from(byte_offset / 4).map_err(|_| fail("local offset overflow"))?;
        let end = start
            .checked_add(variable_count)
            .ok_or_else(|| fail("local count overflow"))?;
        let words = self
            .words
            .get(start..end)
            .ok_or_else(|| fail("authored-local range exceeds payload"))?;
        Ok(Some(SaveCodeLocals {
            owner_id: owner.owner_id,
            byte_offset,
            words,
        }))
    }
}
