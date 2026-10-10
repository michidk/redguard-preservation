//! Public SOUP decoding contracts with synthetic, non-game bytecode.
use rgpre::import::rgm::script_step::{LiteralScriptOperation, decode_literal_script_step};

#[test]
fn literal_task_preserves_function_index_and_signed_arguments() {
    let step = decode_literal_script_step(&[0, 9, 1, 2, 7, 255, 255, 255, 255, 22, 3, 0, 0, 0], 0)
        .unwrap();
    assert_eq!(step.next, 14);
    assert_eq!(
        step.operation,
        LiteralScriptOperation::Task {
            function_id: 265,
            arguments: vec![-1, 3],
        }
    );
}

#[test]
fn end_retains_its_continuation_instead_of_following_it() {
    let step = decode_literal_script_step(&[99, 5, 1, 0, 0, 0], 1).unwrap();
    assert_eq!(step.next, 6);
    assert_eq!(step.operation, LiteralScriptOperation::End { target: 1 });
}

#[test]
fn null_task_has_no_parameter_count_byte() {
    let step = decode_literal_script_step(&[0, 0, 0, 99], 0).unwrap();
    assert_eq!(step.next, 3);
    assert_eq!(
        step.operation,
        LiteralScriptOperation::Task {
            function_id: 0,
            arguments: vec![],
        }
    );
}

#[test]
fn truncated_and_unsupported_instructions_are_not_skipped() {
    for length in 0..14 {
        let bytes = [0, 9, 0, 2, 7, 1, 0, 0, 0, 7, 2, 0, 0, 0];
        assert!(decode_literal_script_step(&bytes[..length], 0).is_err());
    }
    assert!(
        decode_literal_script_step(&[0, 9, 0, 1, 10, 0, 0, 0, 0], 0)
            .unwrap_err()
            .contains("nonliteral")
    );
    assert!(
        decode_literal_script_step(&[1], 0)
            .unwrap_err()
            .contains("opcode 1 unsupported")
    );
    assert!(
        decode_literal_script_step(&[5], usize::MAX)
            .unwrap_err()
            .contains("overflow")
    );
}

#[test]
fn installed_flag_sequence_retains_finite_tasks_and_restart_target() {
    use rgpre::import::{rgm, soup_def};
    let Some(root) = std::env::var_os("REDGUARD_DIR") else {
        eprintln!("SKIP installed FLAG2 script: REDGUARD_DIR is absent");
        return;
    };
    let root = std::path::PathBuf::from(root);
    let bytes = std::fs::read(root.join("maps/ISLAND.RGM")).expect("read installed ISLAND.RGM");
    let definition = soup_def::try_load_soup_def(&root).expect("installed SOUP386.DEF");
    let map = rgm::parse_rgm_file(&bytes).expect("parse installed ISLAND.RGM");
    let scripts = rgm::disassemble_rgm_scripts(&map, Some(&definition));
    let (_, script) = scripts
        .iter()
        .find(|(name, _)| name == "FLAG2")
        .expect("FLAG2 script");
    assert_eq!(script.bytecode.len(), 61);
    assert_eq!(script.script_pc, 0);
    let mut cursor = 0;
    for expected in [[1, 25], [10, 1], [1, 3], [10, 2]] {
        let step = decode_literal_script_step(&script.bytecode, cursor).unwrap();
        let LiteralScriptOperation::Task {
            function_id,
            arguments,
        } = step.operation
        else {
            panic!("expected blocking animation task at {cursor}");
        };
        assert_eq!(
            definition.functions[usize::from(function_id)].name,
            "PushAnimation"
        );
        assert_eq!(arguments, expected);
        cursor = step.next;
    }
    let end = decode_literal_script_step(&script.bytecode, cursor).unwrap();
    assert_eq!(end.operation, LiteralScriptOperation::End { target: 0 });
    assert_eq!(end.next, 61);
}
