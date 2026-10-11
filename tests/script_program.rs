use rgpre::import::{
    rgm::script_program::{CallKind, Object, Operation, Place, Value, decode_script_step},
    soup_def::parse_soup_def,
};

fn definition() -> rgpre::import::soup_def::SoupDef {
    parse_soup_def(
        "[functions]\ntask Wait parms 2\nfunction Query parms 0\n[flags]\nNUMBER Counter 3",
    )
    .unwrap()
}

#[test]
fn parameter_modes_and_nested_functions_preserve_values() {
    let step = decode_script_step(
        &[0, 1, 0, 2, 6, 0, 0, 0, 0, 10, 3, 0, 0, 0],
        0,
        &definition(),
    )
    .unwrap();
    assert_eq!(step.next, 14);
    let Operation::Call { call } = step.operation else {
        panic!("call expected")
    };
    assert_eq!(call.kind, CallKind::Task);
    assert_eq!(call.target, Object::Me);
    assert_eq!(
        call.arguments,
        [
            Value::Read {
                place: Place::Flag { index: 0 }
            },
            Value::Read {
                place: Place::Local { index: 3 }
            }
        ]
    );
    let step = decode_script_step(&[6, 0, 0, 2, 2, 0, 0, 0], 0, &definition()).unwrap();
    let Operation::Assign { formula, .. } = step.operation else {
        panic!("assignment expected")
    };
    assert!(matches!(formula.first, Value::Call { .. }));
    assert_eq!(step.next, 8);
}

#[test]
fn target_calls_preserve_dispatch_and_low_reference_byte() {
    let step = decode_script_step(&[25, 4, 3, 2, 2, 0, 0, 99], 0, &definition()).unwrap();
    assert_eq!(step.next, 7);
    let Operation::ObjectCall { call } = step.operation else {
        panic!("call expected")
    };
    assert_eq!(call.target, Object::Named { string: 3 });
    assert_eq!(call.kind, CallKind::Function);
    let step = decode_script_step(&[15, 0, 0, 9, 255], 0, &definition()).unwrap();
    assert_eq!(
        step.operation,
        Operation::Increment {
            destination: Place::Property {
                object: Object::Me,
                reference: 9
            },
            delta: 1
        }
    );
}

#[test]
fn branch_decoding_does_not_consume_its_body() {
    let step = decode_script_step(
        &[3, 7, 1, 0, 0, 0, 0, 7, 2, 0, 0, 0, 0, 18, 0, 0, 0, 18],
        0,
        &definition(),
    )
    .unwrap();
    assert_eq!(step.next, 17);
    let Operation::If { conditions, target } = step.operation else {
        panic!("if expected")
    };
    assert_eq!(target, 18);
    assert_eq!(conditions.len(), 1);
}

#[test]
fn truncation_wrong_counts_and_unknown_encodings_fail_in_context() {
    for bytes in [&[0, 1, 0, 1][..], &[5, 1][..], &[255][..], &[30, 0][..]] {
        let error = decode_script_step(bytes, 0, &definition()).unwrap_err();
        assert!(error.contains("instruction 0"), "{error}");
    }
}

#[test]
fn program_traversal_rejects_branches_into_operands_and_keeps_unknowns() {
    use rgpre::import::rgm::script_program::decode_script_program;
    let malformed = decode_script_program(&[4, 1, 0, 0, 0], 0, &definition()).unwrap_err();
    assert!(malformed.contains("inside instruction"));
    let steps = decode_script_program(&[255], 0, &definition()).unwrap();
    assert!(matches!(steps[&0].operation, Operation::Unsupported { .. }));
}

#[test]
fn actor_headers_retain_authored_order_and_separate_instance_locals() {
    let mut headers = vec![0; 8 + 165];
    headers[..4].copy_from_slice(&1u32.to_le_bytes());
    let row = &mut headers[8..];
    row[4..8].copy_from_slice(b"TEST");
    row[13..15].copy_from_slice(&2u16.to_le_bytes());
    row[0x4D..0x51].copy_from_slice(&5i32.to_le_bytes());
    row[0x75..0x79].copy_from_slice(&1i32.to_le_bytes());
    let variables: Vec<_> = [4i32, 9i32]
        .into_iter()
        .flat_map(i32::to_le_bytes)
        .collect();
    let actors = rgpre::import::rgm::script::disassemble_actor_scripts(
        &headers,
        &[5, 0, 0, 0, 0],
        &[],
        &[],
        &variables,
        Some(&definition()),
    );
    assert_eq!(actors.len(), 1);
    assert_eq!(actors[0].1.header_index, 0);
    assert_eq!(actors[0].1.instances, 2);
    assert_eq!(actors[0].1.instance_variables, [vec![4], vec![9]]);
}

#[test]
fn actor_controls_and_script_return_branches_keep_exact_continuations() {
    for (opcode, operation) in [
        (27, Operation::TaskStop { label: 7 }),
        (28, Operation::TaskPause { label: 7 }),
        (29, Operation::TaskResume { label: 7 }),
    ] {
        let step = decode_script_step(&[opcode, 7, 0, 0, 0], 0, &definition()).unwrap();
        assert_eq!(step.operation, operation);
        assert_eq!(step.next, 5);
    }
    assert_eq!(
        decode_script_step(&[24], 0, &definition())
            .unwrap()
            .operation,
        Operation::Yield
    );
    let step = decode_script_step(&[30, 3, 17, 9, 0, 0, 0], 0, &definition()).unwrap();
    assert_eq!(step.next, 2);
    assert_eq!(
        step.operation,
        Operation::ScriptRv {
            expected: 3,
            target: 7,
            save_return: true
        }
    );
    let step = decode_script_step(&[30, 3, 18, 0, 0, 0, 24], 0, &definition()).unwrap();
    assert_eq!(step.next, 6);
}

#[test]
fn condition_updates_and_parameter_truncation_are_lossless() {
    let step = decode_script_step(
        &[3, 10, 0, 10, 0, 20, 0, 0, 1, 0, 11, 0, 16, 0, 0, 0],
        0,
        &definition(),
    )
    .unwrap();
    assert_eq!(step.next, 16);
    let Operation::If { conditions, .. } = step.operation else {
        panic!("if expected")
    };
    assert_eq!(
        conditions[0].lhs,
        Value::Update {
            place: Place::Local { index: 0 },
            delta: 1
        }
    );
    assert_eq!(
        conditions[0].rhs,
        Value::Update {
            place: Place::Property {
                object: Object::Me,
                reference: 1
            },
            delta: -1
        }
    );
    let step = decode_script_step(
        &[0, 1, 0, 2, 22, 255, 255, 255, 255, 21, 4, 3, 2, 1],
        0,
        &definition(),
    )
    .unwrap();
    let Operation::Call { call } = step.operation else {
        panic!("call expected")
    };
    assert_eq!(
        call.arguments,
        [Value::Integer { value: 65535 }, Value::String { index: 4 }]
    );
}

#[test]
fn nonbinary_formula_operators_follow_the_invalid_encoding_path() {
    for operator in [10, 11, 12] {
        let error =
            decode_script_step(&[10, 0, 7, 1, 0, 0, 0, operator], 0, &definition()).unwrap_err();
        assert!(error.contains("invalid formula operator"), "{error}");
    }
}
