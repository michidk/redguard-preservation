//! Typed SYSTEM.INI settings in their original engine units.
use super::ini::{ini_section, required, sections};
use crate::Result;

/// Engine configuration. Missing or empty settings remain `None`.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct SystemIni {
    pub screen: ScreenSettings,
    pub system: SystemSettings,
    pub debug: DebugSettings,
    pub game: GameSettings,
    pub cyrus: CyrusSettings,
    pub manager3d: Manager3dSettings,
    pub xngine: XngineSettings,
    pub camera: CameraSettings,
    pub dialog: DialogSettings,
    pub glide: GlideSettings,
}

impl SystemIni {
    /// Parse case-insensitive sections and keys, retaining the first duplicate key.
    /// Requires `[system]`; unknown fields are ignored and invalid known values fail.
    pub fn parse(text: &str) -> Result<Self> {
        let sections = sections(text, "SYSTEM.INI")?;
        required(&sections, "SYSTEM.INI", "system")?;
        Ok(Self {
            screen: ScreenSettings::parse(sections.get("screen"))?,
            system: SystemSettings::parse(sections.get("system"))?,
            debug: DebugSettings::parse(sections.get("debug"))?,
            game: GameSettings::parse(sections.get("game"))?,
            cyrus: CyrusSettings::parse(sections.get("cyrus"))?,
            manager3d: Manager3dSettings::parse(sections.get("3dmanager"))?,
            xngine: XngineSettings::parse(sections.get("xngine"))?,
            camera: CameraSettings::parse(sections.get("camera"))?,
            dialog: DialogSettings::parse(sections.get("dialog"))?,
            glide: GlideSettings::parse(sections.get("3dfx"))?,
        })
    }
}

ini_section! {
    /// Settings from `[screen]`. Numeric codes retain their stored values.
    ScreenSettings {
        candle_mode: i32 => "candle_mode",
        colour_bits: i32 => "colour_bits",
        resolution: i32 => "resolution",
        palette_red: i32 => "palette_red",
        palette_green: i32 => "palette_green",
        palette_blue: i32 => "palette_blue",
        smk_interlace: bool => "smk_interlace",
    }
}

ini_section! {
    /// Settings from `[system]`. Numeric codes retain their stored values.
    SystemSettings {
        game_bitmap: String => "game_bitmap",
        pointers: String => "pointers",
        system_font: String => "system_font",
        icon_font: String => "icon_font",
        gui_font: String => "gui_font",
        gui_low_font: String => "gui_low_font",
        animation_drive: String => "animation_drive",
        back_texture: i32 => "back_texture",
        volume: i32 => "volume",
        sound: bool => "sound",
        fast_sound: bool => "fast_sound",
        fidelity: i32 => "fidelity",
        redbook: bool => "redbook",
        redbook_volume: i32 => "redbook_volume",
        sound_distance: i32 => "sound_distance",
        post_collide_height: u32 => "post_collide_height",
        hpost_collide_height: i32 => "hpost_collide_height",
        pre_validate: bool => "pre_validate",
        normal_frame_rate: i32 => "normal_frame_rate",
        use_smooth_fps: bool => "use_smooth_fps",
        use_smooth_divisor: bool => "use_smooth_divisor",
        min_frame_rate: i32 => "min_frame_rate",
        max_frame_rate: i32 => "max_frame_rate",
        jump_time: i32 => "jump_time",
        jump_height: i32 => "jump_height",
        sphere_object_scale: i32 => "sphere_object_scale",
        standing_height: i32 => "standing_height",
        disable_text: bool => "disable_text",
        disable_debug_text: bool => "disable_debug_text",
        normal_ink: i32 => "normal_ink",
        slide_range: i32 => "slide_range",
        statics_load: bool => "statics_load",
        flats_load: bool => "flats_load",
        objects_load: bool => "objects_load",
        lights_load: bool => "lights_load",
        ropes_load: bool => "ropes_load",
        task_system: bool => "task_system",
        animation_system: bool => "animation_system",
        floating_point_physics: bool => "floating_point_physics",
        rtx_filename: String => "rtx_filename",
        world_ini: String => "world_ini",
        item_ini: String => "item_ini",
        start_fade: bool => "start_fade",
        compass_xco: i32 => "compass_xco",
        compass_yco: i32 => "compass_yco",
        candle_xco: i32 => "candle_xco",
        candle_yco: i32 => "candle_yco",
        logbook_xco: i32 => "logbook_xco",
        logbook_yco: i32 => "logbook_yco",
        pickup_xco: i32 => "pickup_xco",
        pickup_yco: i32 => "pickup_yco",
        pickup_text_yco: i32 => "pickup_text_yco",
        game_xco1: i32 => "game_xco1",
        game_yco1: i32 => "game_yco1",
        game_xco2: i32 => "game_xco2",
        game_yco2: i32 => "game_yco2",
        lock_windows: bool => "lock_windows",
        disable_drive_check: bool => "disable_drive_check",
        disable_cpu_check: bool => "disable_cpu_check",
        disable_svga_check: bool => "disable_svga_check",
        report_machine: bool => "report_machine",
        max_active_objects: i32 => "max_active_objects",
        max_effects: i32 => "max_effects",
        max_particles: i32 => "max_particles",
        max_remap_objects: i32 => "max_remap_objects",
        disable_effects: bool => "disable_effects",
    }
}

ini_section! {
    /// Settings from `[debug]`. Numeric codes retain their stored values.
    DebugSettings {
        final_version: bool => "final_version",
        enable_logs: bool => "enable_logs",
        map_log: bool => "map_log",
        console_error: bool => "console_error",
        software_interrupt: bool => "software_interrupt",
        software_break: bool => "software_break",
        attempt_recover: bool => "attempt_recover",
        object_log: bool => "object_log",
        video_log: bool => "video_log",
        node_marker: bool => "node_marker",
        task_debug: bool => "task_debug",
        memory_manager: i32 => "memory_manager",
        monitor_object: String => "monitor_object",
        display_masters: bool => "display_masters",
        display_slaves: bool => "display_slaves",
        display_edges: bool => "display_edges",
        ignore_errors: bool => "ignore_errors",
        disable_family: bool => "disable_family",
        disable_master_slaves: bool => "disable_master_slaves",
        disable_slaves: bool => "disable_slaves",
        family_log: bool => "family_log",
        script_log: bool => "script_log",
        show_manager: bool => "show_manager",
        display_node_map: bool => "display_node_map",
        display_nodes: bool => "display_nodes",
        display_markers: bool => "display_markers",
        network_marker_file: String => "network_marker_file",
        object_system_log: bool => "object_system_log",
        convert_static_angles: bool => "convert_static_angles",
        def_checksum: bool => "def_checksum",
        render_log: bool => "render_log",
        idebug: i32 => "idebug",
        idebug_refresh: bool => "idebug_refresh",
        memory_monitor: bool => "memory_monitor",
    }
}

ini_section! {
    /// Settings from `[game]`. Numeric codes retain their stored values.
    GameSettings {
        fall_bounce_height: i32 => "fall_bounce_height",
        fall_death_height: i32 => "fall_death_height",
        fall_hurt_height: i32 => "fall_hurt_height",
        fall_hurt_zap: i32 => "fall_hurt_zap",
        slide_hurt_height: i32 => "slide_hurt_height",
        slide_hurt_zap: i32 => "slide_hurt_zap",
        rope_jump_add: i32 => "rope_jump_add",
        rope_attach_angle: i32 => "rope_attach_angle",
        slide_speed: i32 => "slide_speed",
        rtx_pickup_override_time: i32 => "rtx_pickup_override_time",
        swim_depth: i32 => "swim_depth",
        player_dead_time: i32 => "player_dead_time",
        player_fall_dead_time: i32 => "player_fall_dead_time",
        old_combat: bool => "old_combat",
        lineup_distance: i32 => "lineup_distance",
        dialog_radius: i32 => "dialog_radius",
        combat_sphere_scale: i32 => "combat_sphere_scale",
    }
}

ini_section! {
    /// Settings from `[cyrus]`. Numeric codes retain their stored values.
    CyrusSettings {
        sheath_sword_delay: i32 => "sheath_sword_delay",
        auto_defend: bool => "auto_defend",
        walk_mode: i32 => "walk_mode",
        turn_speed: u32 => "turn_speed",
        turn_max_speed: u32 => "turn_max_speed",
        mouse_turn: bool => "mouse_turn",
        joy_tolerance: i32 => "joy_tolerance",
        camera_distance: i32 => "camera_distance",
        tap_time: i32 => "tap_time",
        poly_push_units: u32 => "poly_push_units",
        auto_grab: bool => "auto_grab",
        smooth_post_min: i32 => "smooth_post_min",
        smooth_post_max: i32 => "smooth_post_max",
        smooth_post_divisor: i32 => "smooth_post_divisor",
    }
}

ini_section! {
    /// Settings from `[3dmanager]`. Numeric codes retain their stored values.
    Manager3dSettings {
        buffer_kbytes: i32 => "buffer_kbytes",
        heap_kbytes: i32 => "heap_kbytes",
        max_objects: i32 => "max_objects",
        compress: bool => "compress",
        save_compressed: bool => "save_compressed",
        cache_objects: bool => "cache_objects",
        cache_lifetime: i32 => "cache_lifetime",
        dummy_manager: bool => "dummy_manager",
        in_view_enabled: bool => "in_view_enabled",
        shutdown_mode: bool => "shutdown_mode",
    }
}

ini_section! {
    /// Settings from `[xngine]`. Numeric codes retain their stored values.
    XngineSettings {
        texture_kbytes: i32 => "texture_kbytes",
        gfx_kbytes: i32 => "gfx_kbytes",
        front_plane: i32 => "front_plane",
        back_plane: i32 => "back_plane",
        perspective_low_x: i32 => "perspective_low_x",
        perspective_low_y: i32 => "perspective_low_y",
        perspective_med_x: i32 => "perspective_med_x",
        perspective_med_y: i32 => "perspective_med_y",
        perspective_high_x: i32 => "perspective_high_x",
        perspective_high_y: i32 => "perspective_high_y",
        perspective_ultra_x: i32 => "perspective_ultra_x",
        perspective_ultra_y: i32 => "perspective_ultra_y",
        detail: i32 => "detail",
        ambient_light: i32 => "ambient_light",
        screen_scale: i32 => "screen_scale",
        haze_depth: i32 => "haze_depth",
        sky_disable: bool => "sky_disable",
        sky_move: bool => "sky_move",
        sky_xrotate: i32 => "sky_xrotate",
        sky_yrotate: i32 => "sky_yrotate",
        game_detail: i32 => "game_detail",
        exclusion: i32 => "exclusion",
    }
}

ini_section! {
    /// Settings from `[camera]`. Numeric codes retain their stored values.
    CameraSettings {
        static_rope_threshold: i32 => "static_rope_threshold",
        obstacle_size: i32 => "obstacle_size",
        camera_size: i32 => "camera_size",
        camera_scape_size: i32 => "camera_scape_size",
        target_offset_x: i32 => "target_offset_x",
        target_offset_y: i32 => "target_offset_y",
        target_offset_z: i32 => "target_offset_z",
        offset_pos_x: i32 => "offset_pos_x",
        offset_pos_y: i32 => "offset_pos_y",
        offset_pos_z: i32 => "offset_pos_z",
        offset_angle_x: i32 => "offset_angle_x",
        offset_angle_y: i32 => "offset_angle_y",
        offset_angle_z: i32 => "offset_angle_z",
        camera_distance: i32 => "camera_distance",
        camera_min_distance: i32 => "camera_min_distance",
        camera_right_pos: i32 => "camera_right_pos",
        camera_left_pos: i32 => "camera_left_pos",
        camera_combat_angle_offset_x: i32 => "camera_combat_angle_offset_x",
        camera_combat_angle_offset_y: i32 => "camera_combat_angle_offset_y",
        camera_combat_angle_offset_z: i32 => "camera_combat_angle_offset_z",
        camera_combat_distance: i32 => "camera_combat_distance",
        camera_hang_angle_offset_x: i32 => "camera_hang_angle_offset_x",
        camera_hang_angle_offset_y: i32 => "camera_hang_angle_offset_y",
        camera_hang_angle_offset_z: i32 => "camera_hang_angle_offset_z",
        camera_hang_distance: i32 => "camera_hang_distance",
        camera_rope_max_vel: i32 => "camera_rope_max_vel",
        camera_rope_angle_offset_x: i32 => "camera_rope_angle_offset_x",
        camera_rope_angle_offset_y: i32 => "camera_rope_angle_offset_y",
        camera_rope_angle_offset_z: i32 => "camera_rope_angle_offset_z",
        camera_rope_distance: i32 => "camera_rope_distance",
        camera_rope_above_angle_offset_x: i32 => "camera_rope_above_angle_offset_x",
        camera_rope_above_angle_offset_y: i32 => "camera_rope_above_angle_offset_y",
        camera_rope_above_angle_offset_z: i32 => "camera_rope_above_angle_offset_z",
        camera_rope_above_distance: i32 => "camera_rope_above_distance",
        camera_rope_above_aim_x: i32 => "camera_rope_above_aim_x",
        camera_rope_above_aim_y: i32 => "camera_rope_above_aim_y",
        camera_rope_above_aim_z: i32 => "camera_rope_above_aim_z",
        camera_rope_below_angle_offset_x: i32 => "camera_rope_below_angle_offset_x",
        camera_rope_below_angle_offset_y: i32 => "camera_rope_below_angle_offset_y",
        camera_rope_below_angle_offset_z: i32 => "camera_rope_below_angle_offset_z",
        camera_rope_below_distance: i32 => "camera_rope_below_distance",
        camera_rope_below_aim_x: i32 => "camera_rope_below_aim_x",
        camera_rope_below_aim_y: i32 => "camera_rope_below_aim_y",
        camera_rope_below_aim_z: i32 => "camera_rope_below_aim_z",
        camera_debug_angle_offset_x: i32 => "camera_debug_angle_offset_x",
        camera_debug_angle_offset_y: i32 => "camera_debug_angle_offset_y",
        camera_debug_angle_offset_z: i32 => "camera_debug_angle_offset_z",
        camera_debug_distance: i32 => "camera_debug_distance",
        camera_combat_max_vel: i32 => "camera_combat_max_vel",
        camera_hang_max_vel: i32 => "camera_hang_max_vel",
        camera_rope_above_max_vel: i32 => "camera_rope_above_max_vel",
        camera_rope_below_max_vel: i32 => "camera_rope_below_max_vel",
        camera_debug_max_vel: i32 => "camera_debug_max_vel",
        max_x_angle: i32 => "max_x_angle",
        min_x_angle: i32 => "min_x_angle",
        max_vel: i32 => "max_vel",
        max_y_vel: i32 => "max_y_vel",
        max_acc: i32 => "max_acc",
        player_control_x_inc: i32 => "player_control_x_inc",
        player_control_y_inc: i32 => "player_control_y_inc",
        glide_x: f32 => "glide_x",
        glide_y: f32 => "glide_y",
        glide_z: f32 => "glide_z",
        glide_angle_x: f32 => "glide_angle_x",
        glide_angle_y: f32 => "glide_angle_y",
        glide_angle_z: f32 => "glide_angle_z",
        cam_prox_up: f32 => "cam_prox_up",
    }
}

ini_section! {
    /// Settings from `[dialog]`. Numeric codes retain their stored values.
    DialogSettings {
        menu_start_x: i32 => "menu_start_x",
        menu_start_y: i32 => "menu_start_y",
        dialog_max_menu_items: i32 => "dialog_max_menu_items",
        dialog_max_dialog_lines: i32 => "dialog_max_dialog_lines",
        dialog_max_text_width: i32 => "dialog_max_text_width",
        dialog_print_text: bool => "dialog_print_text",
        dialog_use_speech: bool => "dialog_use_speech",
        dialog_max_distance: i32 => "dialog_max_distance",
        menu_traverse_delay: i32 => "menu_traverse_delay",
    }
}

ini_section! {
    /// Settings from `[3dfx]`. Numeric codes retain their stored values.
    GlideSettings {
        resolution: i32 => "resolution",
        text_scale: [i32; 2] => "text_scale",
        anim_text_scale: [i32; 2] => "anim_text_scale",
        font_sel: [i32; 2] => "font_sel",
        font_norm: [i32; 2] => "font_norm",
        font_used: [i32; 2] => "font_used",
    }
}
