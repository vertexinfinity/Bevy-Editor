# Target Range (first person)

A first-person shooting gallery for Bevylite. You have 60 seconds on an open-air range
with three lanes. Red and white disc targets pop up at spawn points (some slide left and
right); shoot as many as you can with a 12-round pistol.

What the sample shows:

- An FPS controller: mouse look with clamped pitch (`AccumulatedMouseMotion`), WASD
  movement, Shift sprint, Space jump with gravity, arena bounds and collision against
  crates (you can also jump onto the lower ones).
- Hitscan shooting without a physics engine: ray vs sphere for targets, ray vs box for
  walls, crates and dividers, so cover blocks shots.
- Magazine and reload (R, 1.2 s, automatic when empty), recoil kick, a view-model pistol
  and a muzzle light flash.
- Pop-up animation, hit sparks, dust puffs on misses, a `+100` / `+150` banner and a
  crosshair that flashes red on a hit.
- HUD (`Score`, `Time`, `Ammo` or `Reloading...`, `Accuracy`), a results screen with
  hits and accuracy, and the session best score.
- Menu, pause (Esc), restart and cursor locking come from `sample_kit`.

## Controls

| Input | Action |
| --- | --- |
| Mouse | Look |
| W A S D | Move |
| Shift | Sprint |
| Space | Jump |
| Left click | Fire |
| R | Reload |
| Esc | Pause / resume |
| Enter | Start / play again |
| M | Back to the menu (after a round) |

Hits are worth 100 points, moving targets 150.

## Running

From the `samples/` folder:

```
cargo run -p first_person
```

Test hooks (from `sample_kit`): `SAMPLE_AUTOPLAY=1` starts a round automatically and
turns on an autopilot that aims at the nearest target and fires every 0.4 s;
`SAMPLE_SCREENSHOT=<path>` with `SAMPLE_SCREENSHOT_DELAY=<seconds>` saves a screenshot and
exits.

## Files

| File | Contents |
| --- | --- |
| `src/level.rs` | Static range (floor, walls, berm, lane dividers, crates, lamps, lights, camera). Shared with the editor via `#[path]`, so it only uses the Bevy prelude. Returns `LevelEntities` with target spots, colliders and bounds. |
| `src/main.rs` | Gameplay: FPS controller, shooting, targets, particles, HUD, round flow and the autopilot. |
| `bevylite.project.json` | Editor project manifest (`scene_source` points at `src/level.rs`). |
