-- actor_694.lua
-- Bevylite Actor Script

local actor = {
    move_speed = 5.0,
    jump_force = 10.0,
    is_active = true,
}

function actor:on_start()
    print("[actor_694] Initialized")
end

function actor:on_update(dt)
    -- Insert per-frame logic here
end

return actor
