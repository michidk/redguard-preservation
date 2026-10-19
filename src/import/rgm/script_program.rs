//! Typed SOUP instructions and expressions from the documented bytecode contract.
use crate::import::soup_def::SoupDef;
use serde::{Deserialize, Serialize};

/// A single decoded statement, with offsets relative to its actor's script slice.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Step {
    pub next: u32,
    pub operation: Operation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Operation {
    Call {
        call: Call,
    },
    ObjectCall {
        call: Call,
    },
    Assign {
        destination: Place,
        formula: Formula,
    },
    Increment {
        destination: Place,
        delta: i32,
    },
    If {
        conditions: Vec<Condition>,
        target: u32,
    },
    Goto {
        target: u32,
    },
    End {
        target: u32,
    },
    Gosub {
        target: u32,
    },
    Return,
    Endint,
    Anchor {
        value: u8,
    },
    Yield,
    TaskStop {
        label: u32,
    },
    TaskPause {
        label: u32,
    },
    TaskResume {
        label: u32,
    },
    ScriptRv {
        expected: u8,
        target: u32,
        save_return: bool,
    },
    Unsupported {
        reason: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Call {
    /// Original index in the installed SOUP386.DEF, including its synthetic null entry.
    pub function_id: u16,
    pub kind: CallKind,
    pub target: Object,
    pub function: String,
    pub arguments: Vec<Value>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CallKind {
    Task,
    Multitask,
    Function,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Object {
    Me,
    Player,
    Camera,
    Named { string: u32 },
    Local { index: u8 },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Place {
    Flag { index: u16 },
    Local { index: u8 },
    Property { object: Object, reference: u16 },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Value {
    Integer {
        value: i32,
    },
    String {
        index: u32,
    },
    Read {
        place: Place,
    },
    Update {
        place: Place,
        delta: i32,
    },
    Call {
        call: Box<Call>,
    },
    /// Object-targeted expression encoding, including an explicit Me target.
    ObjectCall {
        call: Box<Call>,
    },
}

/// Formula operators retain their byte values; evaluation is owned by the consumer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Formula {
    pub first: Value,
    pub rest: Vec<(u8, Value)>,
}

/// Conjunction joins this comparison to the next: zero ends, one AND, two OR.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Condition {
    pub lhs: Value,
    pub comparison: u8,
    pub rhs: Value,
    pub conjunction: u8,
}

#[derive(Clone, Copy)]
enum Mode {
    Lhs,
    Rhs,
    Parameter,
    Formula,
}

struct Decoder<'a> {
    bytes: &'a [u8],
    cursor: usize,
    definition: &'a SoupDef,
    depth: usize,
}

impl Decoder<'_> {
    fn read(&mut self, count: usize) -> Result<&[u8], String> {
        let end = self
            .cursor
            .checked_add(count)
            .ok_or("operand offset overflow")?;
        let bytes = self
            .bytes
            .get(self.cursor..end)
            .ok_or_else(|| format!("truncated operand at {}", self.cursor))?;
        self.cursor = end;
        Ok(bytes)
    }
    fn byte(&mut self) -> Result<u8, String> {
        Ok(self.read(1)?[0])
    }
    fn word(&mut self) -> Result<u16, String> {
        Ok(u16::from_le_bytes(self.read(2)?.try_into().unwrap()))
    }
    fn dword(&mut self) -> Result<u32, String> {
        Ok(u32::from_le_bytes(self.read(4)?.try_into().unwrap()))
    }
    fn object(&mut self) -> Result<Object, String> {
        let kind = self.byte()?;
        let operand = self.byte()?;
        Ok(match kind {
            0 => Object::Me,
            1 => Object::Player,
            2 => Object::Camera,
            4 => Object::Named {
                string: u32::from(operand),
            },
            10 => Object::Local { index: operand },
            _ => return Err(format!("unsupported object kind {kind}")),
        })
    }
    fn place(&mut self, opcode: u8) -> Result<Place, String> {
        Ok(match opcode {
            6 => Place::Flag {
                index: self.word()?,
            },
            10 => Place::Local {
                index: self.byte()?,
            },
            20 => Place::Property {
                object: self.object()?,
                reference: self.word()?,
            },
            _ => return Err(format!("unsupported assignment opcode {opcode}")),
        })
    }
    fn call(&mut self, target: Object, kind: CallKind) -> Result<Call, String> {
        let id = self.word()?;
        let count = if id == 0 { 0 } else { self.byte()? };
        self.call_arguments(id, count, target, kind)
    }
    fn call_arguments(
        &mut self,
        id: u16,
        count: u8,
        target: Object,
        kind: CallKind,
    ) -> Result<Call, String> {
        let function = self
            .definition
            .functions
            .get(usize::from(id))
            .ok_or_else(|| format!("SOUP386.DEF function {id} absent"))?;
        if count != function.param_count {
            return Err(format!(
                "function {} requires {} arguments, found {count}",
                function.name, function.param_count
            ));
        }
        let name = function.name.clone();
        let mut arguments = Vec::with_capacity(usize::from(count));
        for _ in 0..count {
            arguments.push(self.value(Mode::Parameter)?);
        }
        Ok(Call {
            function_id: id,
            target,
            kind,
            function: name,
            arguments,
        })
    }
    fn kind(&mut self) -> Result<CallKind, String> {
        match self.byte()? {
            0 => Ok(CallKind::Task),
            1 => Ok(CallKind::Multitask),
            2 => Ok(CallKind::Function),
            byte => Err(format!("unsupported call kind {byte}")),
        }
    }
    fn value(&mut self, mode: Mode) -> Result<Value, String> {
        // Depth is a malformed-input limit, never a simulated yield.
        if self.depth == 128 {
            return Err("expression nesting limit exceeded".into());
        }
        self.depth += 1;
        let result = self.value_inner(mode);
        self.depth -= 1;
        result
    }
    fn value_inner(&mut self, mode: Mode) -> Result<Value, String> {
        let opcode = self.byte()?;
        if matches!(mode, Mode::Parameter) && matches!(opcode, 2 | 25 | 26) {
            return Err("unsupported native call in parameter mode".into());
        }
        Ok(match opcode {
            7 => Value::Integer {
                value: self.dword()? as i32,
            },
            22 => {
                let value = self.dword()?;
                Value::Integer {
                    value: if matches!(mode, Mode::Parameter) {
                        i32::from(value as u16)
                    } else {
                        value as i32
                    },
                }
            }
            21 => Value::String {
                index: {
                    let value = self.dword()?;
                    if matches!(mode, Mode::Parameter) {
                        u32::from(value as u8)
                    } else {
                        value
                    }
                },
            },
            6 | 10 | 20 => {
                let place = self.place(opcode)?;
                let extra = match (opcode, mode) {
                    (6 | 10 | 20, Mode::Lhs | Mode::Rhs) => 1,
                    (6, Mode::Parameter) => 2,
                    (10, Mode::Parameter) => 3,
                    _ => 0,
                };
                let trailing = self.read(extra)?;
                if matches!(mode, Mode::Lhs | Mode::Rhs) {
                    match trailing[0] {
                        0 => Value::Read { place },
                        10 => Value::Update { place, delta: 1 },
                        11 => Value::Update { place, delta: -1 },
                        value => return Err(format!("unsupported conditional update {value}")),
                    }
                } else {
                    Value::Read { place }
                }
            }
            2 => Value::Call {
                call: Box::new(self.call(Object::Me, CallKind::Function)?),
            },
            25 | 26 => {
                let target = self.object()?;
                Value::ObjectCall {
                    call: Box::new(self.call(target, CallKind::Function)?),
                }
            }
            _ => return Err(format!("unsupported value opcode {opcode}")),
        })
    }
    fn formula(&mut self) -> Result<Formula, String> {
        let first = self.value(Mode::Formula)?;
        let mut rest = Vec::new();
        loop {
            let operator = self.byte()?;
            match operator {
                0 => break,
                1..=9 => rest.push((operator, self.value(Mode::Formula)?)),
                10 | 11 => return Err(format!("invalid formula operator {operator}")),
                _ => return Err(format!("invalid formula operator {operator}")),
            }
        }
        Ok(Formula { first, rest })
    }
    fn statement(&mut self) -> Result<Operation, String> {
        let opcode = self.byte()?;
        Ok(match opcode {
            0..=2 => {
                let kind =
                    [CallKind::Task, CallKind::Multitask, CallKind::Function][usize::from(opcode)];
                Operation::Call {
                    call: self.call(Object::Me, kind)?,
                }
            }
            3 => {
                let mut conditions = Vec::new();
                loop {
                    let lhs = self.value(Mode::Lhs)?;
                    let comparison = self.byte()?;
                    if comparison > 5 {
                        return Err(format!("invalid comparison {comparison}"));
                    }
                    let rhs = self.value(Mode::Rhs)?;
                    let conjunction = self.byte()?;
                    if conjunction > 2 {
                        return Err(format!("invalid conjunction {conjunction}"));
                    }
                    conditions.push(Condition {
                        lhs,
                        comparison,
                        rhs,
                        conjunction,
                    });
                    if conjunction == 0 {
                        break;
                    }
                }
                Operation::If {
                    conditions,
                    target: self.dword()?,
                }
            }
            4 => Operation::Goto {
                target: self.dword()?,
            },
            5 => Operation::End {
                target: self.dword()?,
            },
            6 | 10 | 20 => Operation::Assign {
                destination: self.place(opcode)?,
                formula: self.formula()?,
            },
            15 | 16 => Operation::Increment {
                destination: Place::Property {
                    object: self.object()?,
                    reference: self.word()?,
                },
                delta: if opcode == 15 { 1 } else { -1 },
            },
            17 => Operation::Gosub {
                target: self.dword()?,
            },
            18 => Operation::Return,
            19 => Operation::Endint,
            23 => Operation::Anchor {
                value: self.byte()?,
            },
            24 => Operation::Yield,
            25 | 26 => {
                let target = self.object()?;
                let kind = self.kind()?;
                Operation::ObjectCall {
                    call: self.call(target, kind)?,
                }
            }
            27 => Operation::TaskStop {
                label: self.dword()?,
            },
            28 => Operation::TaskPause {
                label: self.dword()?,
            },
            29 => Operation::TaskResume {
                label: self.dword()?,
            },
            30 => {
                let expected = self.byte()?;
                let target = u32::try_from(self.cursor + 5)
                    .map_err(|_| "ScriptRV continuation exceeds u32")?;
                let opcode = *self
                    .bytes
                    .get(self.cursor)
                    .ok_or("truncated ScriptRV body")?;
                if opcode == 18 {
                    self.read(4)?;
                }
                Operation::ScriptRv {
                    expected,
                    target,
                    save_return: opcode == 17,
                }
            }
            _ => return Err(format!("unsupported opcode {opcode}")),
        })
    }
}

/// Decode one actor's RAHK native hook, independently of its RASC continuations.
/// A zero RAHD hook offset means no call; nonzero offsets are relative to RAHK.
/// The stored record uses a function index, count byte and five-byte parameters.
///
/// # Errors
/// Returns a contextual error for absent/truncated sections, invalid offsets,
/// definition mismatches or unsupported parameter encodings.
pub fn decode_native_hook(
    rgm: &super::RgmFile,
    header_index: usize,
    definition: &SoupDef,
) -> Result<Option<Call>, String> {
    let decode = || -> Result<Option<Call>, String> {
        let headers = rgm
            .sections
            .iter()
            .find_map(|section| match section {
                super::RgmSection::Rahd(_, data) => Some(data.as_slice()),
                _ => None,
            })
            .ok_or("RAHD section absent")?;
        let count = u32::from_le_bytes(
            headers
                .get(..4)
                .ok_or("RAHD count truncated")?
                .try_into()
                .unwrap(),
        );
        if header_index >= count as usize {
            return Err("RAHD index outside record count".into());
        }
        let start = header_index
            .checked_mul(165)
            .and_then(|offset| offset.checked_add(8))
            .ok_or("RAHD record offset overflow")?;
        let end = start.checked_add(165).ok_or("RAHD record end overflow")?;
        let header = headers.get(start..end).ok_or("RAHD record truncated")?;
        let offset = super::shared::read_i32_le(header, 0x5d).ok_or("RAHD hook offset absent")?;
        if offset == 0 {
            return Ok(None);
        }
        let offset = usize::try_from(offset).map_err(|_| "negative RAHK offset")?;
        let bytes = rgm
            .sections
            .iter()
            .find_map(|section| match section {
                super::RgmSection::Rahk(_, data) => Some(data.as_slice()),
                _ => None,
            })
            .ok_or("RAHK section absent")?;
        let mut decoder = Decoder {
            bytes,
            cursor: offset,
            definition,
            depth: 0,
        };
        let function = decoder.word()?;
        let count = decoder.byte()?;
        if count > 8 {
            return Err(format!("RAHK argument count {count} exceeds eight"));
        }
        let end = decoder
            .cursor
            .checked_add(usize::from(count) * 5)
            .ok_or("RAHK record end overflow")?;
        decoder.bytes = bytes.get(..end).ok_or("RAHK arguments truncated")?;
        let call = decoder.call_arguments(function, count, Object::Me, CallKind::Function)?;
        if decoder.cursor != end {
            return Err("RAHK arguments do not occupy five bytes each".into());
        }
        Ok(Some(call))
    };
    decode().map_err(|error| format!("actor header {header_index} native hook: {error}"))
}

/// Decodes one statement without traversing branches or executing native handlers.
/// Unsupported encodings and malformed inputs retain the actor-relative instruction.
pub fn decode_script_step(
    bytes: &[u8],
    offset: usize,
    definition: &SoupDef,
) -> Result<Step, String> {
    let mut decoder = Decoder {
        bytes,
        cursor: offset,
        definition,
        depth: 0,
    };
    let operation = decoder
        .statement()
        .map_err(|error| format!("script instruction {offset}: {error}"))?;
    let next = u32::try_from(decoder.cursor)
        .map_err(|_| format!("script instruction {offset}: continuation exceeds u32"))?;
    Ok(Step { next, operation })
}

/// Follows all explicit continuations from the main and optional interrupt entries.
/// Unknown instructions remain terminal unsupported records. Malformed known encodings
/// and branches into operands fail instead of inventing instruction boundaries.
pub fn decode_script_program(
    bytes: &[u8],
    entry: u32,
    definition: &SoupDef,
) -> Result<std::collections::BTreeMap<u32, Step>, String> {
    use std::collections::{BTreeMap, BTreeSet};
    let mut pending = BTreeSet::from([0, entry]);
    let mut steps: BTreeMap<u32, Step> = BTreeMap::new();
    while let Some(offset) = pending.pop_first() {
        if steps.contains_key(&offset) {
            continue;
        }
        if offset as usize >= bytes.len() {
            return Err(format!(
                "script instruction {offset}: target outside script"
            ));
        }
        if let Some((&start, step)) = steps.range(..offset).next_back()
            && offset < step.next
        {
            return Err(format!(
                "script instruction {offset}: target inside instruction {start}"
            ));
        }
        let step = match decode_script_step(bytes, offset as usize, definition) {
            Ok(step) => step,
            Err(reason) if reason.contains(": unsupported ") => Step {
                next: offset,
                operation: Operation::Unsupported { reason },
            },
            Err(reason) => return Err(reason),
        };
        if let Some((&start, _)) = steps.range(offset..step.next).next() {
            return Err(format!(
                "script instruction {offset}: overlaps instruction {start}"
            ));
        }
        match &step.operation {
            Operation::Goto { target } | Operation::End { target } => {
                pending.insert(*target);
            }
            Operation::If { target, .. }
            | Operation::Gosub { target }
            | Operation::ScriptRv { target, .. } => {
                pending.insert(*target);
                pending.insert(step.next);
            }
            Operation::Return | Operation::Endint | Operation::Unsupported { .. } => {}
            _ => {
                pending.insert(step.next);
            }
        }
        steps.insert(offset, step);
    }
    Ok(steps)
}
