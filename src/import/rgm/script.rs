use super::shared::{read_i32_le, read_script_name_9};
use crate::import::soup_def::SoupDef;
use std::fmt::Write;

const RAHD_ITEM_SIZE: usize = 165;
const COMPARE_OPS: [&str; 6] = ["=", "!=", "<", ">", "<=", ">="];
const FORMULA_OPS: [&str; 12] = [
    "", "+", "-", "*", "/", "<<", ">>", "&", "|", "^", "++", "--",
];

const DIALOGUE_FUNCTIONS: [&str; 9] = [
    "ACTIVATE",
    "RTX",
    "rtxAnim",
    "RTXp",
    "RTXpAnim",
    "AddLog",
    "AmbientRtx",
    "menuAddItem",
    "TorchActivate",
];

const ATTRIBUTE_FUNCTIONS: [&str; 4] = ["SetAttribute", "GetAttribute", "SetMyAttr", "GetMyAttr"];

const ANIM_GROUP_FUNCTIONS: [&str; 6] = [
    "PlayAnimation",
    "PushAnimation",
    "PushControlAnimation",
    "WaitAnimFrame",
    "WaitPlayerAnimFrame",
    "SyncWithGroup",
];

const AXIS_FUNCTIONS: [&str; 8] = [
    "MoveByAxis",
    "MoveAlongAxis",
    "RotateByAxis",
    "RotateToAxis",
    "sMoveByAxis",
    "sMoveAlongAxis",
    "sRotateByAxis",
    "sRotateToAxis",
];

const AXIS_NAMES: [&str; 3] = ["axis_x", "axis_y", "axis_z"];

#[derive(Debug, Clone)]
pub struct ActorScript {
    pub script_length: i32,
    pub script_data_offset: i32,
    pub script_pc: i32,
    pub num_strings: i32,
    pub num_variables: i32,
    pub strings: Vec<String>,
    pub variables: Vec<i32>,
    pub instructions: Vec<ScriptInstruction>,
}

#[derive(Debug, Clone)]
pub struct ScriptInstruction {
    pub addr: usize,
    pub indent: usize,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ValueMode {
    Main,
    Lhs,
    Rhs,
    Parameter,
    Formula,
}

struct Decoder<'a> {
    code: &'a [u8],
    pc: usize,
    strings: &'a [String],
    soup_def: Option<&'a SoupDef>,
    instructions: Vec<ScriptInstruction>,
    indent: usize,
}

impl<'a> Decoder<'a> {
    fn new(
        code: &'a [u8],
        script_pc: i32,
        strings: &'a [String],
        soup_def: Option<&'a SoupDef>,
    ) -> Self {
        let pc = usize::try_from(script_pc.max(0)).unwrap_or(0);
        Self {
            code,
            pc,
            strings,
            soup_def,
            instructions: Vec::new(),
            indent: 0,
        }
    }

    fn read_u8(&mut self) -> Result<u8, String> {
        let value = self
            .code
            .get(self.pc)
            .copied()
            .ok_or_else(|| "unexpected end of script".to_string())?;
        self.pc += 1;
        Ok(value)
    }

    fn read_u16(&mut self) -> Result<u16, String> {
        let chunk: [u8; 2] = self
            .code
            .get(self.pc..self.pc + 2)
            .ok_or_else(|| "unexpected end of script".to_string())?
            .try_into()
            .map_err(|_| "invalid u16 read".to_string())?;
        self.pc += 2;
        Ok(u16::from_le_bytes(chunk))
    }

    fn read_i32(&mut self) -> Result<i32, String> {
        let chunk: [u8; 4] = self
            .code
            .get(self.pc..self.pc + 4)
            .ok_or_else(|| "unexpected end of script".to_string())?
            .try_into()
            .map_err(|_| "invalid i32 read".to_string())?;
        self.pc += 4;
        Ok(i32::from_le_bytes(chunk))
    }

    fn skip(&mut self, count: usize) -> Result<(), String> {
        if self.pc + count > self.code.len() {
            return Err("unexpected end of script".to_string());
        }
        self.pc += count;
        Ok(())
    }

    fn function_name(&self, func_id: u16) -> String {
        self.soup_def
            .and_then(|def| def.functions.get(usize::from(func_id)))
            .map_or_else(|| format!("func_{func_id}"), |f| f.name.clone())
    }

    fn flag_name(&self, flag_id: u16) -> String {
        self.soup_def
            .and_then(|def| def.flags.get(usize::from(flag_id)))
            .map_or_else(|| format!("Flag{flag_id}"), |f| f.name.clone())
    }

    fn ref_name(&self, ref_id: u16) -> String {
        let idx = usize::from(ref_id & 0x00FF);
        self.soup_def
            .and_then(|def| def.references.get(idx))
            .cloned()
            .unwrap_or_else(|| format!("ref_{idx}"))
    }

    fn label_name(offset: i32) -> String {
        format!("{:04X}", u32::from_le_bytes(offset.to_le_bytes()))
    }

    fn decode_ref(&mut self) -> Result<String, String> {
        let ref_id = self.read_u16()?;
        Ok(self.ref_name(ref_id))
    }

    fn decode_object(&mut self) -> Result<String, String> {
        let obj_type = self.read_u8()?;
        match obj_type {
            0 => {
                self.skip(1)?;
                Ok("Me".to_string())
            }
            1 => {
                self.skip(1)?;
                Ok("Player".to_string())
            }
            2 => {
                self.skip(1)?;
                Ok("Camera".to_string())
            }
            4 => {
                let str_idx = usize::from(self.read_u8()?);
                Ok(self
                    .strings
                    .get(str_idx)
                    .cloned()
                    .unwrap_or_else(|| format!("strings[{str_idx}]")))
            }
            10 => {
                let var_idx = self.read_u8()?;
                Ok(format!("var{var_idx}"))
            }
            _ => Ok(format!("ObjectType{obj_type}")),
        }
    }

    fn decode_formula(&mut self) -> Result<String, String> {
        let mut out = String::from(" = ");
        loop {
            out.push_str(&self.decode_value(ValueMode::Formula, None)?);
            let op = self.read_u8()?;
            if op == 10 {
                out.push_str(FORMULA_OPS[10]);
                self.skip(1)?;
                break;
            }
            if op == 11 {
                out.push_str(FORMULA_OPS[11]);
                self.skip(1)?;
                break;
            }
            if (1..=9).contains(&op) {
                out.push(' ');
                out.push_str(FORMULA_OPS[usize::from(op)]);
                out.push(' ');
                continue;
            }
            break;
        }
        Ok(out)
    }

    fn decode_if(&mut self) -> Result<String, String> {
        let mut out = String::from("if ");
        loop {
            let lhs = self.decode_value(ValueMode::Lhs, None)?;
            let cmp = usize::from(self.read_u8()?);
            let rhs = self.decode_value(ValueMode::Rhs, None)?;
            let conj = self.read_u8()?;
            let cmp_text = COMPARE_OPS.get(cmp).copied().unwrap_or("?");
            out.push_str(&lhs);
            out.push(' ');
            out.push_str(cmp_text);
            out.push(' ');
            out.push_str(&rhs);
            match conj {
                1 => out.push_str(" and "),
                2 => out.push_str(" or "),
                _ => break,
            }
        }

        let end_offset = self.read_i32()?;
        let block_end = usize::try_from(end_offset.max(0)).unwrap_or(self.code.len());
        let target_end = block_end.min(self.code.len());
        if target_end > self.pc {
            let mut nested = Vec::new();
            while self.pc < target_end {
                let nested_start = self.pc;
                match self.decode_value(ValueMode::Main, None) {
                    Ok(text) => nested.push(format!("{nested_start}:{text}")),
                    Err(err) => {
                        nested.push(format!("{nested_start}:<error: {err}>"));
                        self.pc = target_end;
                        break;
                    }
                }
            }
            if !nested.is_empty() {
                out.push_str(" { ");
                out.push_str(&nested.join("; "));
                out.push_str(" }");
            }
        }
        Ok(out)
    }

    fn decode_if_block(&mut self, addr: usize) -> Result<(), String> {
        let mut condition = String::from("if ");
        loop {
            let lhs = self.decode_value(ValueMode::Lhs, None)?;
            let cmp = usize::from(self.read_u8()?);
            let rhs = self.decode_value(ValueMode::Rhs, None)?;
            let conj = self.read_u8()?;
            let cmp_text = COMPARE_OPS.get(cmp).copied().unwrap_or("?");
            condition.push_str(&lhs);
            condition.push(' ');
            condition.push_str(cmp_text);
            condition.push(' ');
            condition.push_str(&rhs);
            match conj {
                1 => condition.push_str(" and "),
                2 => condition.push_str(" or "),
                _ => break,
            }
        }

        let end_offset = self.read_i32()?;
        let block_end = usize::try_from(end_offset.max(0))
            .unwrap_or(self.code.len())
            .min(self.code.len());

        self.instructions.push(ScriptInstruction {
            addr,
            indent: self.indent,
            text: condition,
        });

        self.indent += 1;
        while self.pc < block_end {
            if let Err(err) = self.decode_statement() {
                self.instructions.push(ScriptInstruction {
                    addr: self.pc,
                    indent: self.indent,
                    text: format!("<error: {err}>"),
                });
                self.pc = block_end;
                break;
            }
        }
        self.indent -= 1;

        Ok(())
    }

    fn decode_statement(&mut self) -> Result<(), String> {
        let addr = self.pc;
        let opcode = *self
            .code
            .get(self.pc)
            .ok_or_else(|| "unexpected end of script".to_string())?;

        if opcode == 0x03 {
            self.pc += 1;
            self.decode_if_block(addr)?;
        } else if opcode == 0x1E {
            self.pc += 1;
            let rv = self.read_u8()?;
            let condition = format!("if <ScriptRv> = {rv}");
            self.instructions.push(ScriptInstruction {
                addr,
                indent: self.indent,
                text: condition,
            });

            let block_end = self.pc.saturating_add(4).min(self.code.len());
            self.indent += 1;
            while self.pc < block_end {
                if let Err(err) = self.decode_statement() {
                    self.instructions.push(ScriptInstruction {
                        addr: self.pc,
                        indent: self.indent,
                        text: format!("<error: {err}>"),
                    });
                    self.pc = block_end;
                    break;
                }
            }
            self.indent -= 1;
        } else {
            let text = self.decode_value(ValueMode::Main, None)?;
            self.instructions.push(ScriptInstruction {
                addr,
                indent: self.indent,
                text,
            });
        }

        Ok(())
    }

    fn decode_call(&mut self, call_type: u8) -> Result<String, String> {
        let func_id = self.read_u16()?;
        let name = self.function_name(func_id);
        let param_count = if func_id == 0 {
            0
        } else {
            usize::from(self.read_u8()?)
        };
        let mut params = Vec::with_capacity(param_count);
        for param_idx in 0..param_count {
            params.push(self.decode_value(ValueMode::Parameter, Some((&name, param_idx)))?);
        }
        let prefix = if call_type == 1 { "@" } else { "" };
        Ok(format!("{prefix}{name}({})", params.join(", ")))
    }

    fn decode_numeric_parameter(&self, value: i32, context: Option<(&str, usize)>) -> String {
        let Some((function_name, param_index)) = context else {
            return value.to_string();
        };
        if param_index != 0 {
            return value.to_string();
        }
        if DIALOGUE_FUNCTIONS
            .iter()
            .any(|name| name.eq_ignore_ascii_case(function_name))
        {
            let chars = value.to_le_bytes();
            if chars.iter().all(|b| b.is_ascii_graphic() || *b == b' ') {
                return format!(
                    "\"{}\"",
                    chars
                        .iter()
                        .map(|b| char::from(*b))
                        .collect::<String>()
                        .trim_end_matches('\0')
                );
            }
        }
        if ATTRIBUTE_FUNCTIONS
            .iter()
            .any(|name| name.eq_ignore_ascii_case(function_name))
            && let Some(attr_name) = self
                .soup_def
                .and_then(|def| def.attributes.get(usize::try_from(value).ok()?))
        {
            return attr_name.clone();
        }
        if ANIM_GROUP_FUNCTIONS
            .iter()
            .any(|name| name.eq_ignore_ascii_case(function_name))
            && let Some(anim_name) = self
                .soup_def
                .and_then(|def| def.anim_groups.get(usize::try_from(value).ok()?))
        {
            return anim_name.clone();
        }
        if AXIS_FUNCTIONS
            .iter()
            .any(|name| name.eq_ignore_ascii_case(function_name))
            && let Some(axis_name) =
                AXIS_NAMES.get(usize::try_from(value).ok().unwrap_or(usize::MAX))
        {
            return (*axis_name).to_string();
        }
        value.to_string()
    }

    fn decode_value(
        &mut self,
        mode: ValueMode,
        param_context: Option<(&str, usize)>,
    ) -> Result<String, String> {
        let opcode = self.read_u8()?;
        match opcode {
            0x00 => self.decode_call(0),
            0x01 => self.decode_call(1),
            0x02 => self.decode_call(2),
            0x03 => self.decode_if(),
            0x04 => Ok(format!("Goto {}", Self::label_name(self.read_i32()?))),
            0x05 => {
                let target = self.read_i32()?;
                if target == 0 {
                    Ok("End".to_string())
                } else {
                    Ok(format!("End {}", Self::label_name(target)))
                }
            }
            0x06 => {
                let flag_id = self.read_u16()?;
                let mut out = self.flag_name(flag_id);
                match mode {
                    ValueMode::Main => out.push_str(&self.decode_formula()?),
                    ValueMode::Lhs | ValueMode::Rhs => {
                        let _ = self.read_u8()?;
                    }
                    ValueMode::Parameter => {
                        self.skip(2)?;
                    }
                    ValueMode::Formula => {}
                }
                Ok(out)
            }
            0x07 | 0x16 => {
                let value = self.read_i32()?;
                if mode == ValueMode::Parameter {
                    Ok(self.decode_numeric_parameter(value, param_context))
                } else {
                    Ok(value.to_string())
                }
            }
            0x0A => {
                let idx = self.read_u8()?;
                let mut out = format!("var{idx}");
                match mode {
                    ValueMode::Main => out.push_str(&self.decode_formula()?),
                    ValueMode::Lhs | ValueMode::Rhs => {
                        let _ = self.read_u8()?;
                    }
                    ValueMode::Parameter => {
                        self.skip(3)?;
                    }
                    ValueMode::Formula => {}
                }
                Ok(out)
            }
            0x0F => Ok(format!(
                "{}.{}++",
                self.decode_object()?,
                self.decode_ref()?
            )),
            0x10 => Ok(format!(
                "{}.{}--",
                self.decode_object()?,
                self.decode_ref()?
            )),
            0x11 => Ok(format!("Gosub {}", Self::label_name(self.read_i32()?))),
            0x12 => Ok("Return".to_string()),
            0x13 => Ok("Endint".to_string()),
            0x14 => {
                let mut out = format!("{}.{}", self.decode_object()?, self.decode_ref()?);
                match mode {
                    ValueMode::Main => out.push_str(&self.decode_formula()?),
                    ValueMode::Lhs => {
                        let _ = self.read_u8()?;
                    }
                    ValueMode::Rhs | ValueMode::Parameter | ValueMode::Formula => {}
                }
                Ok(out)
            }
            0x15 => {
                let idx = usize::try_from(self.read_i32()?.max(0)).unwrap_or(0);
                let value = self
                    .strings
                    .get(idx)
                    .map_or_else(|| format!("strings[{idx}]"), |s| format!("\"{s}\""));
                Ok(value)
            }
            0x17 => Ok(format!("<Anchor>={}", self.read_u8()?)),
            0x19 | 0x1A => {
                let object = self.decode_object()?;
                if mode == ValueMode::Main {
                    let inner = self.decode_value(ValueMode::Formula, None)?;
                    Ok(format!("{object}.{inner}"))
                } else {
                    Ok(format!("{object}.{}", self.decode_call(2)?))
                }
            }
            0x1B => Ok(format!(
                "<TaskPause({})>",
                Self::label_name(self.read_i32()?)
            )),
            0x1E => {
                let rv = self.read_u8()?;
                let b0 = self.read_u8()?;
                let b1 = self.read_u8()?;
                let b2 = self.read_u8()?;
                let b3 = self.read_u8()?;
                Ok(format!(
                    "if <ScriptRv> = {rv} [block {:02X} {:02X} {:02X} {:02X}]",
                    b0, b1, b2, b3
                ))
            }
            _ => Err(format!("unknown opcode 0x{opcode:02X}")),
        }
    }
}

fn read_null_terminated_string(bytes: &[u8], offset: usize) -> Option<String> {
    let slice = bytes.get(offset..)?;
    let end = slice.iter().position(|b| *b == 0).unwrap_or(slice.len());
    Some(String::from_utf8_lossy(&slice[..end]).to_string())
}

fn read_actor_strings(
    num_strings: i32,
    string_offsets_index: i32,
    rast: &[u8],
    rasb: &[u8],
) -> Vec<String> {
    let mut out = Vec::new();
    if num_strings <= 0 || string_offsets_index < 0 {
        return out;
    }

    let base = usize::try_from(string_offsets_index).unwrap_or(0);
    let count = usize::try_from(num_strings).unwrap_or(0);
    for i in 0..count {
        let offset = base + i * 4;
        let Some(str_off) = read_i32_le(rasb, offset) else {
            break;
        };
        let Ok(str_off) = usize::try_from(str_off.max(0)) else {
            out.push(String::new());
            continue;
        };
        out.push(read_null_terminated_string(rast, str_off).unwrap_or_default());
    }
    out
}

fn read_actor_variables(num_vars: i32, variable_offset: i32, rava: &[u8]) -> Vec<i32> {
    let mut out = Vec::new();
    if num_vars <= 0 || variable_offset < 0 {
        return out;
    }

    let Ok(base_index) = usize::try_from(variable_offset / 4) else {
        return out;
    };
    let count = usize::try_from(num_vars).unwrap_or(0);
    for i in 0..count {
        let byte_offset = (base_index + i) * 4;
        let Some(value) = read_i32_le(rava, byte_offset) else {
            break;
        };
        out.push(value);
    }
    out
}

fn disassemble_script(
    code: &[u8],
    script_pc: i32,
    strings: &[String],
    soup_def: Option<&SoupDef>,
) -> Vec<ScriptInstruction> {
    let mut decoder = Decoder::new(code, script_pc, strings, soup_def);

    if decoder.pc > code.len() {
        decoder.instructions.push(ScriptInstruction {
            addr: code.len(),
            indent: 0,
            text: format!("<error: script_pc {} out of bounds>", script_pc),
        });
        return decoder.instructions;
    }

    while decoder.pc < code.len() {
        if let Err(err) = decoder.decode_statement() {
            decoder.instructions.push(ScriptInstruction {
                addr: decoder.pc,
                indent: decoder.indent,
                text: format!("<error: {err}>"),
            });
            break;
        }
    }

    decoder.instructions
}

#[must_use]
pub fn disassemble_actor_scripts(
    rahd_data: &[u8],
    rasc_data: &[u8],
    rast_data: &[u8],
    rasb_data: &[u8],
    rava_data: &[u8],
    soup_def: Option<&SoupDef>,
) -> Vec<(String, ActorScript)> {
    let mut out = Vec::new();
    if rahd_data.len() < 8 {
        return out;
    }

    let count =
        u32::from_le_bytes([rahd_data[0], rahd_data[1], rahd_data[2], rahd_data[3]]) as usize;

    for i in 0..count {
        let rec_off = 8 + i * RAHD_ITEM_SIZE;
        if rec_off + RAHD_ITEM_SIZE > rahd_data.len() {
            break;
        }
        let item = &rahd_data[rec_off..rec_off + RAHD_ITEM_SIZE];

        let script_name = read_script_name_9(item, 4).unwrap_or_default();
        if script_name.is_empty() {
            continue;
        }

        let num_strings = read_i32_le(item, 0x41).unwrap_or(0);
        let string_offsets_index = read_i32_le(item, 0x49).unwrap_or(0);
        let script_length = read_i32_le(item, 0x4D).unwrap_or(0);
        let script_data_offset = read_i32_le(item, 0x51).unwrap_or(0);
        let script_pc = read_i32_le(item, 0x55).unwrap_or(0);
        let num_variables = read_i32_le(item, 0x75).unwrap_or(0);
        let variable_offset = read_i32_le(item, 0x7D).unwrap_or(0);

        if script_length <= 0 {
            continue;
        }

        let strings = read_actor_strings(num_strings, string_offsets_index, rast_data, rasb_data);
        let variables = read_actor_variables(num_variables, variable_offset, rava_data);

        let mut instructions = Vec::new();
        let start = usize::try_from(script_data_offset.max(0)).unwrap_or(0);
        let Ok(script_len) = usize::try_from(script_length.max(0)) else {
            continue;
        };
        let end = start.saturating_add(script_len);
        if start >= rasc_data.len() || end > rasc_data.len() {
            instructions.push(ScriptInstruction {
                addr: 0,
                indent: 0,
                text: format!(
                    "<error: script slice out of bounds offset={} length={}>",
                    script_data_offset, script_length
                ),
            });
        } else {
            let code = &rasc_data[start..end];
            instructions = disassemble_script(code, script_pc, &strings, soup_def);
        }

        out.push((
            script_name,
            ActorScript {
                script_length,
                script_data_offset,
                script_pc,
                num_strings,
                num_variables,
                strings,
                variables,
                instructions,
            },
        ));
    }

    out
}

#[must_use]
pub fn format_soup_text(
    name: &str,
    source_file: &str,
    script: &ActorScript,
    rtx_labels: Option<&std::collections::HashMap<String, String>>,
) -> String {
    let mut out = String::new();

    let _ = writeln!(out, "; SOUP script: {name}");
    let _ = writeln!(out, "; Source: {source_file}");
    out.push('\n');

    for inst in &script.instructions {
        let indent_str = "  ".repeat(inst.indent);
        let _ = write!(out, "{:04X}:\t{indent_str}{}", inst.addr, inst.text);
        if let Some(labels) = rtx_labels {
            for rtx_key in extract_rtx_keys(&inst.text) {
                if let Some(label) = labels.get(&rtx_key) {
                    let truncated = if label.len() > 80 {
                        format!("{}...", &label[..77])
                    } else {
                        label.clone()
                    };
                    let _ = write!(out, "\t; {truncated}");
                    break;
                }
            }
        }
        out.push('\n');
    }

    out
}

fn extract_rtx_keys(text: &str) -> Vec<String> {
    let mut keys = Vec::new();
    let mut chars = text.char_indices().peekable();
    while let Some((i, ch)) = chars.next() {
        if ch == '"' {
            let start = i + 1;
            let mut end = start;
            for (j, c) in chars.by_ref() {
                if c == '"' {
                    end = j;
                    break;
                }
            }
            if end > start && end - start <= 4 {
                keys.push(text[start..end].to_string());
            }
        }
    }
    keys
}

#[must_use]
pub fn script_metadata_json(script: &ActorScript) -> serde_json::Value {
    serde_json::json!({
        "script_length": script.script_length,
        "script_data_offset": script.script_data_offset,
        "script_pc": script.script_pc,
        "num_variables": script.num_variables,
        "variables": script.variables,
        "num_strings": script.num_strings,
        "strings": script.strings,
    })
}
