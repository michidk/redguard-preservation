//! Prints an installed script inventory without embedding any game content.
use rgpre::import::{
    rgm::{
        self,
        script_program::{Call, Operation, Value, decode_script_program},
    },
    soup_def::load_soup_def,
};
use std::{collections::BTreeMap, path::PathBuf};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(std::env::var("REDGUARD_DIR")?);
    let definition = load_soup_def(&root)?;
    let mut maps = BTreeMap::new();
    for file in std::fs::read_dir(root.join("maps"))? {
        let path = file?.path();
        if !path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("rgm"))
        {
            continue;
        }
        let map = rgm::parse_rgm_file(&std::fs::read(&path)?)?;
        let mut actors = BTreeMap::new();
        for (name, source) in rgm::disassemble_rgm_scripts(&map, Some(&definition)) {
            let entry = u32::try_from(source.script_pc)?;
            let result = decode_script_program(&source.bytecode, entry, &definition);
            actors.insert(name,match result {
                Ok(steps)=> {
                    let mut calls = Vec::new();
                    for step in steps.values() { operation_calls(&step.operation, &mut calls); }
                    let unsupported:Vec<_>=steps.values().filter_map(|step| match &step.operation { Operation::Unsupported{reason}=>Some(reason.clone()), _=>None }).collect();
                    serde_json::json!({"entry":entry,"instructions":steps.len(),"calls":calls,"unsupported":unsupported})
                }
                Err(error)=>serde_json::json!({"malformed":error}),
            });
        }
        maps.insert(
            path.file_name().unwrap().to_string_lossy().into_owned(),
            actors,
        );
    }
    println!("{}", serde_json::to_string_pretty(&maps)?);
    Ok(())
}

fn value_calls(value: &Value, names: &mut Vec<String>) {
    if let Value::Call { call } = value {
        call_names(call, names);
    }
}
fn call_names(call: &Call, names: &mut Vec<String>) {
    names.push(call.function.clone());
    for value in &call.arguments {
        value_calls(value, names);
    }
}
fn operation_calls(operation: &Operation, names: &mut Vec<String>) {
    match operation {
        Operation::Call { call } | Operation::ObjectCall { call } => call_names(call, names),
        Operation::Assign { formula, .. } => {
            value_calls(&formula.first, names);
            for (_, value) in &formula.rest {
                value_calls(value, names);
            }
        }
        Operation::If { conditions, .. } => {
            for condition in conditions {
                value_calls(&condition.lhs, names);
                value_calls(&condition.rhs, names);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rgpre::import::rgm::script_program::{CallKind, Condition, Formula, Object, Place};
    fn query(name: &str) -> Value {
        Value::Call {
            call: Box::new(Call {
                function_id: 0,
                kind: CallKind::Function,
                target: Object::Me,
                function: name.into(),
                arguments: vec![],
            }),
        }
    }
    #[test]
    fn nested_queries_in_calls_assignments_and_conditions_are_inventoried() {
        let Value::Call { mut call } = query("Outer") else {
            unreachable!()
        };
        call.arguments.push(query("Nested"));
        let operations = [
            Operation::ObjectCall { call: *call },
            Operation::Assign {
                destination: Place::Local { index: 0 },
                formula: Formula {
                    first: query("First"),
                    rest: vec![(1, query("Second"))],
                },
            },
            Operation::If {
                target: 0,
                conditions: vec![Condition {
                    lhs: query("Left"),
                    comparison: 0,
                    rhs: query("Right"),
                    conjunction: 0,
                }],
            },
        ];
        let mut names = Vec::new();
        for operation in &operations {
            operation_calls(operation, &mut names);
        }
        assert_eq!(
            names,
            ["Outer", "Nested", "First", "Second", "Left", "Right"]
        );
    }
}
