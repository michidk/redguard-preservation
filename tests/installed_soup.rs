use rgpre::import::{
    rgm::{self, script_program::decode_script_step},
    soup_def::load_soup_def,
};
#[test]
fn installed_definition_retains_all_authored_defaults_and_flag_animation() {
    let Some(root) = std::env::var_os("REDGUARD_DIR") else {
        eprintln!("SKIP installed SOUP: REDGUARD_DIR absent");
        return;
    };
    let root = std::path::PathBuf::from(root);
    let def = load_soup_def(&root).unwrap();
    let text = std::fs::read_to_string(root.join("soup386/SOUP386.DEF")).unwrap();
    let declarations = text
        .lines()
        .filter(|line| {
            let first = line.split_whitespace().next().unwrap_or("");
            matches!(
                first.to_ascii_lowercase().as_str(),
                "task" | "multitask" | "function"
            )
        })
        .count();
    assert_eq!(def.functions.len(), declarations + 1);
    assert_eq!(def.flags.len(), 369);
    assert_eq!(
        def.flags
            .iter()
            .filter(|flag| flag.initial_value != 0)
            .count(),
        6
    );
    let bytes = std::fs::read(root.join("maps/ISLAND.RGM")).unwrap();
    let map = rgm::parse_rgm_file(&bytes).unwrap();
    let scripts = rgm::disassemble_rgm_scripts(&map, Some(&def));
    let (_, script) = scripts.iter().find(|(name, _)| name == "FLAG2").unwrap();
    let mut cursor = 0;
    while cursor < script.bytecode.len() {
        let step = decode_script_step(&script.bytecode, cursor, &def).unwrap();
        cursor = step.next as usize;
    }
    assert_eq!(cursor, 61);
}
