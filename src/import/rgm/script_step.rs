//! Typed literal task and End decoding from the SOUP bytecode specification.

/// A decoded operation, with offsets relative to its actor's RASC script slice.
#[derive(Debug, PartialEq, Eq)]
pub struct LiteralScriptStep {
    /// Byte offset of the following encoded instruction, independent of an End target.
    pub next: usize,
    /// The operation; nonliteral operands and other opcodes are explicitly unsupported.
    pub operation: LiteralScriptOperation,
}

/// The subset required by literal blocking animation tasks and invocation endings.
#[derive(Debug, PartialEq, Eq)]
pub enum LiteralScriptOperation {
    /// Function ID indexes the installed SOUP386.DEF, including its null entry.
    Task {
        function_id: u16,
        arguments: Vec<i32>,
    },
    /// Store this script-relative continuation and yield the current invocation.
    End { target: u32 },
}

/// Decodes one operation without executing it or following its continuation.
/// Errors include the instruction offset; unsupported input is never skipped.
pub fn decode_literal_script_step(code: &[u8], offset: usize) -> Result<LiteralScriptStep, String> {
    let mut cursor = offset;
    let mut read = |count: usize| {
        let end = cursor
            .checked_add(count)
            .ok_or_else(|| format!("script instruction {offset}: operand offset overflow"))?;
        let bytes = code
            .get(cursor..end)
            .ok_or_else(|| format!("script instruction {offset}: truncated operand at {cursor}"))?;
        cursor = end;
        Ok::<_, String>(bytes)
    };
    let opcode = read(1)?[0];
    let operation = match opcode {
        0 => {
            let bytes = read(2)?;
            let function_id = u16::from_le_bytes([bytes[0], bytes[1]]);
            let count = if function_id == 0 { 0 } else { read(1)?[0] };
            let mut arguments = Vec::with_capacity(usize::from(count));
            for _ in 0..count {
                let operand = read(1)?[0];
                if !matches!(operand, 7 | 22) {
                    return Err(format!(
                        "script instruction {offset}: nonliteral task operand {operand} unsupported"
                    ));
                }
                let bytes = read(4)?;
                arguments.push(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]));
            }
            LiteralScriptOperation::Task {
                function_id,
                arguments,
            }
        }
        5 => {
            let bytes = read(4)?;
            LiteralScriptOperation::End {
                target: u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            }
        }
        _ => {
            return Err(format!(
                "script instruction {offset}: opcode {opcode} unsupported by literal task decoder"
            ));
        }
    };
    Ok(LiteralScriptStep {
        next: cursor,
        operation,
    })
}
