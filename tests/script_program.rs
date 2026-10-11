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
    let Operation::Call { call } = step.operation else {
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
