//! Prints an installed script inventory without embedding any game content.
use rgpre::import::{
    rgm::{
        self,
        script_program::{Operation, decode_script_program},
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
                    let calls:Vec<_>=steps.values().filter_map(|step| match &step.operation { Operation::Call{call} | Operation::ObjectCall{call}=>Some(call.function.clone()), _=>None }).collect();
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
