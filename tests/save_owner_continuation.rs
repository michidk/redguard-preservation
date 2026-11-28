use rgpre::import::{
    rgm::{
        self,
        script::ActorScript,
        script_program::{CallKind, Operation, decode_script_program},
    },
    savegame::*,
    soup_def::{SoupDef, load_soup_def},
};
use std::collections::BTreeSet;

type InstalledScripts = [(String, ActorScript)];

fn put(body: &mut [u8], at: usize, value: u32) {
    body[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

#[test]
fn complete_code_and_shell_views_preserve_independent_slot_words() {
    let mut code = SaveCodeOwner {
        owner_id: 7,
        body: [0; 204],
    };
    let mut shell = SaveShell {
        body: [0; 186],
        variables: vec![],
    };
    shell.body[..8].copy_from_slice(b"TEST0001");
    for (body, scripts, results, callbacks, status, timer, variables) in [
        (
            &mut code.body[..],
            0x23,
            0x33,
            0x42,
            0x72,
            0x78,
            [0x5a, 0x84, 0x9c, 0xb4],
        ),
        (
            &mut shell.body[..],
            0x12,
            0x22,
            0x30,
            0x60,
            0x66,
            [0x48, 0x72, 0x8a, 0xa2],
        ),
    ] {
        for (i, value) in [100, 101, 102, 103].into_iter().enumerate() {
            put(body, scripts + i * 4, value);
        }
        body[results] = 8;
        body[results + 1] = 9;
        for slot in 0..6 {
            put(
                body,
                callbacks + slot * 4,
                if slot == 5 { 0 } else { 201 + slot as u32 },
            );
            body[status + slot] = slot as u8 + 1;
            body[timer + slot * 2..timer + slot * 2 + 2]
                .copy_from_slice(&(u16::MAX - slot as u16).to_le_bytes());
            for (array, base) in variables.into_iter().enumerate() {
                put(
                    body,
                    base + slot * 4,
                    0xf0000000 | (array as u32) << 8 | slot as u32,
                );
            }
        }
    }
    let code = parse_save_code_owners(&encode_save_code_owners(&[code]))
        .unwrap()
        .remove(0);
    let shell = parse_save_shells(&encode_save_shells(&[shell]).unwrap())
        .unwrap()
        .remove(0);
    for (continuation, tasks) in [
        (code.script_continuation(), code.task_slots()),
        (shell.script_continuation(), shell.task_slots()),
    ] {
        assert_compact_continuation(continuation, tasks);
    }
}

fn assert_compact_continuation(continuation: SaveScriptContinuation, tasks: [SaveTaskSlot; 6]) {
    assert_eq!(
        (
            continuation.data,
            continuation.entry,
            continuation.cursor,
            continuation.return_to
        ),
        (100, 101, 102, 103)
    );
    assert_eq!(
        (
            continuation.result,
            continuation.saved_result,
            continuation.anchor
        ),
        (8, 9, 0)
    );
    for (slot, task) in tasks.iter().enumerate() {
        assert_eq!(task.callback, if slot == 5 { 0 } else { 201 + slot as u32 });
        assert_eq!(
            (task.status, task.pause, task.timer),
            (slot as u8 + 1, 0, u16::MAX - slot as u16)
        );
        assert_eq!(
            task.variables,
            std::array::from_fn(|array| 0xf0000000 | (array as u32) << 8 | slot as u32)
        );
    }
}

#[test]
fn local_owners_bind_storage_without_accepting_saved_pointers() {
    let region = parse_save_script_locals(
        &[99i32, i32::MIN, -1, i32::MAX]
            .into_iter()
            .flat_map(i32::to_le_bytes)
            .collect::<Vec<_>>(),
    )
    .unwrap();
    let mut code = SaveCodeOwner {
        owner_id: 42,
        body: [0; 204],
    };
    put(&mut code.body, 0x3a, 4);
    let bound = region.bind_code_owner(&code, 3).unwrap().unwrap();
    assert_eq!((bound.owner_id, bound.byte_offset), (42, 4));
    assert_eq!(bound.words, [i32::MIN, -1, i32::MAX]);
    assert!(region.bind_code_owner(&code, 4).is_err());
    put(&mut code.body, 0x3a, 3);
    assert!(region.bind_code_owner(&code, 1).is_err());
    put(&mut code.body, 0x3a, u32::MAX);
    assert!(region.bind_code_owner(&code, 0).unwrap().is_none());
    assert!(region.bind_code_owner(&code, 1).is_err());
    let mut shell = SaveShell {
        body: [0; 186],
        variables: [-7i32, i32::MAX]
            .into_iter()
            .flat_map(i32::to_le_bytes)
            .collect(),
    };
    shell.body[..8].copy_from_slice(b"TEST0001");
    put(&mut shell.body, 0x24, 0xdeadbeef);
    put(&mut shell.body, 0x28, 2);
    let bound = shell.bind_locals(2).unwrap();
    assert_eq!(bound.name, *b"TEST0001");
    assert_eq!(bound.words, [-7, i32::MAX]);
    assert!(shell.bind_locals(3).is_err());
    shell.variables.pop();
    assert!(shell.bind_locals(2).is_err());
}

#[test]
fn actor_anchor_is_retained_independently_of_results() {
    let mut bytes = vec![0; 4 + 9 + 783 + 48];
    let len = (bytes.len() - 4) as u32;
    put(&mut bytes, 0, len);
    put(&mut bytes, 13 + 0x23b, u32::MAX);
    bytes[13 + 0x25b] = 73;
    bytes[13 + 0x270] = 8;
    let actor = parse_save_objects(&bytes).unwrap().remove(0);
    assert_eq!(actor.script_continuation().anchor, 73);
    assert_eq!(actor.script_continuation().result, 8);
}

#[test]
fn installed_owner_callbacks_and_locals_bind_to_initialized_programs() {
    let (Some(root), Some(path)) = (
        std::env::var_os("REDGUARD_DIR"),
        std::env::var_os("REDGUARD_SAVE_SLOT"),
    ) else {
        eprintln!("SKIP installed owner continuations: REDGUARD_DIR/REDGUARD_SAVE_SLOT absent");
        return;
    };
    let root = std::path::PathBuf::from(root);
    let slot = load_save_slot(path).unwrap();
    let (map, expected_owners, expected_active, expected_shells) = match slot.game.metadata.world_id
    {
        1 => ("ISLAND.RGM", 243, 17, 1),
        20 => ("BELLTOWR.RGM", 2, 0, 2),
        other => panic!("unsupported retained fixture world {other}"),
    };
    let definition = load_soup_def(&root).unwrap();
    let map = rgm::parse_rgm_file(&std::fs::read(root.join("maps").join(map)).unwrap()).unwrap();
    let scripts = rgm::disassemble_rgm_scripts(&map, Some(&definition));
    let section = |tag| {
        &slot
            .game
            .sections
            .iter()
            .find(|s| s.name == tag)
            .unwrap()
            .data
    };
    let region = parse_save_script_locals(section(*b"SVRG")).unwrap();
    let owners = parse_save_code_owners(section(*b"SMCO")).unwrap();
    let shells = parse_save_shells(section(*b"SMSH")).unwrap();
    assert_eq!(owners.len(), expected_owners);
    assert_eq!(shells.len(), expected_shells);
    let calls = installed_task_operands(&scripts, &definition);
    let active = assert_code_owners(&owners, &region, &scripts, &calls);
    assert_eq!(active, expected_active);
    assert_shell_owners(&shells, &scripts);
    eprintln!(
        "bound {} code owners including {active} active callback owners, and {} shells",
        owners.len(),
        shells.len()
    );
}

fn installed_task_operands(scripts: &InstalledScripts, definition: &SoupDef) -> BTreeSet<u32> {
    let mut calls = BTreeSet::new();
    for (_, script) in scripts {
        if let Ok(program) = decode_script_program(
            &script.bytecode,
            script.script_pc.try_into().unwrap(),
            definition,
        ) {
            for (at, step) in program {
                let (call, operand) = match step.operation {
                    Operation::Call { call } => (call, at + 1),
                    Operation::ObjectCall { call } => (call, at + 4),
                    _ => continue,
                };
                if call.kind != CallKind::Function {
                    calls.insert(script.script_data_offset as u32 + operand);
                }
            }
        }
    }
    calls
}

fn assert_code_owners(
    owners: &[SaveCodeOwner],
    region: &SaveScriptLocals,
    scripts: &InstalledScripts,
    calls: &BTreeSet<u32>,
) -> usize {
    let mut active = 0;
    for owner in owners {
        let continuation = owner.script_continuation();
        let (_, script) = scripts
            .iter()
            .find(|(_, s)| s.script_data_offset as u32 == continuation.data)
            .expect("saved code source must bind installed program");
        let locals = region
            .bind_code_owner(owner, script.num_variables.try_into().unwrap())
            .unwrap();
        assert_eq!(
            locals.as_ref().map_or(0, |l| l.words.len()),
            script.variables.len()
        );
        let tasks = owner.task_slots();
        assert_eq!(
            tasks.map(|s| s.callback),
            owner.script_references().callbacks
        );
        if tasks.iter().any(|s| s.callback != 0) {
            active += 1;
        }
        for task in tasks.iter().filter(|s| s.callback != 0) {
            assert!(
                calls.contains(&task.callback),
                "owner {} callback {} must name a decoded installed task operand",
                owner.owner_id,
                task.callback
            );
            assert_eq!(task.pause, 0);
            assert_eq!(
                task.elapsed(128),
                ((u32::from(task.timer) * 128) >> 8) as i32
            );
        }
    }
    active
}

fn assert_shell_owners(shells: &[SaveShell], scripts: &InstalledScripts) {
    for shell in shells {
        let name = std::str::from_utf8(shell.name())
            .unwrap()
            .trim_end_matches('\0');
        let (_, script) = scripts
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .expect("saved shell name must bind installed definition");
        assert_eq!(
            shell.script_continuation().data,
            script.script_data_offset as u32
        );
        assert_eq!(
            shell
                .bind_locals(script.num_variables.try_into().unwrap())
                .unwrap()
                .words
                .len(),
            script.variables.len()
        );
        assert_eq!(
            shell.task_slots().map(|s| s.callback),
            shell.script_references().callbacks
        );
    }
}

#[test]
fn code_execution_replacement_roundtrips_and_rejects_unrepresentable_state_atomically() {
    let mut owner = SaveCodeOwner {
        owner_id: 73,
        body: [0xa5; 204],
    };
    let original = owner.clone();
    let continuation = SaveScriptContinuation {
        data: 101,
        entry: 202,
        cursor: 303,
        return_to: 404,
        result: 7,
        saved_result: 251,
        anchor: 0,
    };
    let tasks = std::array::from_fn(|index| SaveTaskSlot {
        callback: if index == 5 { 0 } else { 500 + index as u32 },
        status: index as u8 + 128,
        pause: 0,
        timer: u16::MAX - index as u16,
        variables: [u32::MAX, index as u32, 0x80000000, 0],
    });
    owner.replace_execution(continuation, tasks).unwrap();
    let decoded = parse_save_code_owners(&encode_save_code_owners(&[owner.clone()]))
        .unwrap()
        .remove(0);
    assert_eq!(decoded.script_continuation(), continuation);
    assert_eq!(decoded.task_slots(), tasks);
    assert_eq!(decoded.owner_id, original.owner_id);
    assert_eq!(
        decoded.script_local_offset(),
        original.script_local_offset()
    );
    assert_eq!(
        decoded.script_references().native_hook,
        original.script_references().native_hook
    );
    // Replacing execution must not change independently retained packed fields.
    assert_eq!(&decoded.body[..0x23], &original.body[..0x23]);
    assert_eq!(&decoded.body[0x35..0x42], &original.body[0x35..0x42]);
    let before = owner.clone();
    let error = owner
        .replace_execution(
            SaveScriptContinuation {
                anchor: 1,
                ..continuation
            },
            tasks,
        )
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("SMCO owner 73") && error.contains("Anchor"),
        "{error}"
    );
    assert_eq!(owner, before);
    for index in 0..6 {
        let mut paused = tasks;
        paused[index].pause = 1;
        let error = owner
            .replace_execution(continuation, paused)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains(&format!("owner 73 task {index}")) && error.contains("pause"),
            "{error}"
        );
        assert_eq!(owner, before);
    }
}

#[test]
fn installed_code_execution_changed_state_roundtrips_without_replacing_other_owners() {
    let Some(path) = std::env::var_os("REDGUARD_SAVE_SLOT") else {
        eprintln!("SKIP installed code execution export: REDGUARD_SAVE_SLOT absent");
        return;
    };
    let slot = load_save_slot(path).unwrap();
    let payload = &slot
        .game
        .sections
        .iter()
        .find(|section| section.name == *b"SMCO")
        .expect("installed SMCO absent")
        .data;
    let mut owners = parse_save_code_owners(payload).unwrap();
    let original = owners.clone();
    let owner = owners
        .first_mut()
        .expect("installed SMCO has no code owners");
    let mut continuation = owner.script_continuation();
    continuation.result = continuation.result.wrapping_add(1);
    continuation.saved_result = continuation.saved_result.wrapping_add(7);
    let mut tasks = owner.task_slots();
    tasks[5].timer = tasks[5].timer.wrapping_add(17);
    tasks[5].variables[3] ^= u32::MAX;
    owner.replace_execution(continuation, tasks).unwrap();
    let encoded = encode_save_code_owners(&owners);
    assert_ne!(encoded, *payload);
    let decoded = parse_save_code_owners(&encoded).unwrap();
    assert_eq!(decoded[0].script_continuation(), continuation);
    assert_eq!(decoded[0].task_slots(), tasks);
    assert_eq!(decoded[0].owner_id, original[0].owner_id);
    assert_eq!(
        decoded[0].script_local_offset(),
        original[0].script_local_offset()
    );
    assert_eq!(decoded[1..], original[1..]);
}
