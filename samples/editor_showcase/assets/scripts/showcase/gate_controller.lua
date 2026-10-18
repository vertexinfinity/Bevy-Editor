-- Inspector demonstration data. Lua execution is not implemented by the editor.
-- The standalone Rust sample implements Space-to-open and beacon animation.
return {
    open_angle = 90.0, move_speed = 2.0, jump_force = 10.0, is_active = true,
    interaction_count = 0, display_name = "Courtyard Gate",
    approach_offset = { 0.0, 0.0, 2.0 },
    -- Editor fixture assigns switch -> Switch_01 and camera -> Showcase Orbit.
}
