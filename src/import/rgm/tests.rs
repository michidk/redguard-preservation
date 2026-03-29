use super::*;

fn approx_eq(a: f32, b: f32) {
    let diff = (a - b).abs();
    assert!(diff < 1.0e-6, "expected {b}, got {a}, diff {diff}");
}

#[test]
fn decode_position_applies_fixed_point_scaling_on_all_axes() {
    let pos = positioning::decode_position(1, -2, 3);

    approx_eq(pos[0], -256.0 / 5120.0);
    approx_eq(pos[1], 512.0 / 5120.0);
    approx_eq(pos[2], -((0x00FF_FFFF_i64 - (3_i64 * 256)) as f32) / 5120.0);
}

#[test]
fn decode_position_does_not_collapse_small_values_to_origin() {
    let pos = positioning::decode_position(1, 1, 1);

    assert!(pos[0].abs() > 0.01, "x too close to origin: {}", pos[0]);
    assert!(pos[1].abs() > 0.01, "y too close to origin: {}", pos[1]);
}

fn pack_cmd_type0(opcode: u8, handle: i16, vertex: i16) -> [u8; 3] {
    let handle_u = (handle as u16) & 0x3FF;
    let vertex_u = (vertex as u16) & 0x3FF;
    let packed: u32 = (opcode as u32 & 0xF) | ((handle_u as u32) << 4) | ((vertex_u as u32) << 14);
    [packed as u8, (packed >> 8) as u8, (packed >> 16) as u8]
}

#[test]
fn ragr_command_decodes_vertex_and_handle_for_opcode_0() {
    let [b0, b1, b2] = pack_cmd_type0(0, 3, 47);
    let cmd = metadata::decode_ragr_command(b0, b1, b2);

    assert_eq!(cmd.opcode, 0);
    assert!(cmd.sets_attachment());
    assert_eq!(cmd.handle_index(), 3);
    assert_eq!(cmd.vertex_index(), 47);
}

#[test]
fn ragr_command_decodes_negative_vertex_index() {
    let [b0, b1, b2] = pack_cmd_type0(0, -1, -5);
    let cmd = metadata::decode_ragr_command(b0, b1, b2);

    assert_eq!(cmd.opcode, 0);
    assert_eq!(cmd.handle_index(), -1);
    assert_eq!(cmd.vertex_index(), -5);
}

#[test]
fn ragr_command_opcode_4_is_sound_trigger_not_attachment() {
    let [b0, b1, b2] = pack_cmd_type0(4, 0, 100);
    let cmd = metadata::decode_ragr_command(b0, b1, b2);

    assert_eq!(cmd.opcode, 4);
    assert!(!cmd.sets_attachment());
}

#[test]
fn ragr_command_opcode_6_does_not_set_attachment() {
    let cmd = metadata::decode_ragr_command(0x06, 0x00, 0x00);
    assert_eq!(cmd.opcode, 6);
    assert!(!cmd.sets_attachment());
}
