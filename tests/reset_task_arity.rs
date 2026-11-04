use rgpre::import::{
    rgm::script_program::{CallKind, Operation, Value, decode_script_step},
    soup_def::parse_soup_def,
};

#[test]
fn literal_reset_task_preserves_extra_operands_and_continuation() {
    let definition = parse_soup_def("[functions]\nfunction ResetAllAction parms 0\n").unwrap();
    let mut bytes = vec![0, 1, 0, 2, 7];
    bytes.extend(23_i32.to_le_bytes());
    bytes.push(7);
    bytes.extend((-4_i32).to_le_bytes());
    let step = decode_script_step(&bytes, 0, &definition).unwrap();
    assert_eq!(step.next, 14);
    let Operation::Call { call } = step.operation else {
        panic!("self call absent")
    };
    assert_eq!(call.kind, CallKind::Task);
    assert_eq!(
        call.arguments,
        [Value::Integer { value: 23 }, Value::Integer { value: -4 }]
    );
    for opcode in [1, 2] {
        bytes[0] = opcode;
        assert!(decode_script_step(&bytes, 0, &definition).is_err());
    }
    bytes[0] = 0;
    let other = parse_soup_def("[functions]\nfunction Other parms 0\n").unwrap();
    assert!(decode_script_step(&bytes, 0, &other).is_err());
    bytes[3] = 1;
    assert!(decode_script_step(&bytes, 0, &definition).is_err());
    bytes[3] = 2;
    bytes[4] = 6;
    assert!(
        decode_script_step(&bytes, 0, &definition)
            .unwrap_err()
            .contains("evidenced numeric literals")
    );
    bytes[4] = 7;
    bytes.pop();
    assert!(decode_script_step(&bytes, 0, &definition).is_err());
}

#[test]
fn reset_extra_operands_do_not_relax_object_or_expression_calls() {
    let definition = parse_soup_def("[functions]\nfunction ResetAllAction parms 0\n").unwrap();
    let operands = [7, 1, 0, 0, 0, 7, 2, 0, 0, 0];
    for target in [0, 1, 2] {
        let mut bytes = vec![25, target, 0, 0, 1, 0, 2];
        bytes.extend(operands);
        assert!(
            decode_script_step(&bytes, 0, &definition)
                .unwrap_err()
                .contains("requires 0 arguments, found 2")
        );
    }
    let mut expression = vec![6, 0, 0, 2, 1, 0, 2];
    expression.extend(operands);
    expression.push(0);
    assert!(decode_script_step(&expression, 0, &definition).is_err());
    let mut object_expression = vec![6, 0, 0, 25, 0, 0, 1, 0, 2];
    object_expression.extend(operands);
    object_expression.push(0);
    assert!(decode_script_step(&object_expression, 0, &definition).is_err());
}

#[test]
fn installed_gremlin_reset_task_preserves_both_authored_operands() {
    use rgpre::import::{rgm, soup_def::load_soup_def};
    let Some(root) = std::env::var_os("REDGUARD_DIR") else {
        eprintln!("SKIP installed reset task: REDGUARD_DIR absent");
        return;
    };
    let root = std::path::PathBuf::from(root);
    let definition = load_soup_def(&root).unwrap();
    let bytes = std::fs::read(root.join("maps/HIDEOUT.RGM")).unwrap();
    let map = rgm::parse_rgm_file(&bytes).unwrap();
    let scripts = rgm::disassemble_rgm_scripts(&map, Some(&definition));
    let (_, script) = scripts.iter().find(|(name, _)| name == "GREMLIN").unwrap();
    let step = decode_script_step(&script.bytecode, 38, &definition).unwrap();
    assert_eq!(step.next, 52);
    let Operation::Call { call } = step.operation else {
        panic!("GREMLIN reset self task absent")
    };
    assert_eq!(call.function, "ResetAllAction");
    assert_eq!(call.kind, CallKind::Task);
    assert_eq!(call.target, rgm::script_program::Object::Me);
    assert_eq!(
        definition.functions[usize::from(call.function_id)].param_count,
        0
    );
    assert_eq!(
        call.arguments,
        [Value::Integer { value: 10 }, Value::Integer { value: 1 }]
    );
}
