use rgpre::import::soup_def::{SoupFunctionKind, parse_soup_def};

#[test]
fn declarations_and_fresh_game_values_retain_installed_order() {
    let def = parse_soup_def("[functions]\ntask Wait parms 1\nfunction Test parms 0\nmultitask Ambient parms 2\n[refs]\nHealth\n[equates]\nauto\nSlave\nendauto\n[flags]\nNUMBER Counter -7; comment\nBOOL Ready true\n").unwrap();
    assert_eq!(def.functions[0].name, "NullFunction");
    assert_eq!(def.functions[1].kind, SoupFunctionKind::Task);
    assert_eq!(def.functions[1].param_count, 1);
    assert_eq!(def.functions[2].kind, SoupFunctionKind::Function);
    assert_eq!(def.references, ["Health"]);
    assert_eq!(def.attributes, ["Slave"]);
    assert_eq!(def.flags[0].initial_value, -7);
    assert_eq!(def.flags[1].initial_value, 1);
}

#[test]
fn malformed_runtime_metadata_is_not_replaced_by_zero() {
    for (text, message) in [
        ("[functions]\ntask Wait parms invalid", "parameter count"),
        ("[functions]\ntask Wait parms 256", "parameter count"),
        ("[functions]\nunknown Wait parms 0", "unknown declaration"),
        ("[flags]\nNUMBER Counter bad", "initial value"),
        ("[flags]\nNUMBER Counter", "expected type name value"),
    ] {
        let error = parse_soup_def(text).unwrap_err();
        assert!(error.contains(message), "{error}");
        assert!(error.contains("line 2"), "{error}");
    }
}
