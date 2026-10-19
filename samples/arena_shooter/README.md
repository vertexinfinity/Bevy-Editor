# Arena Survivor

A top-down twin-stick shooter sample for Bevylite. Drive a hover tank around a neon arena,
aim with the mouse and hold off ever larger waves of enemies.

## What it shows
- Angled top-down camera (55 degree pitch) that smoothly follows the player.
- Mouse aiming: `Camera::viewport_to_world` plus a ground-plane intersection drives the turret.
- Hold-to-fire bullets (8 per second), a Space dash with cooldown, sphere-distance collision.
- Waves: wave N spawns 4 + 2N enemies at the arena edge. Red "runners" are fast and fragile;
  purple "brutes" are slow, take 3 hits and flash white when hit.
- Contact damage with a short invulnerability flash, 100 HP, occasional health pickups.
- HUD (wave, score, HP, dash), a "Wave N" banner, game-over details and best score,
  all through the shared `sample_kit` flow (menu, Esc pause, Enter restart, M menu).
- Static level (`src/level.rs`) shared with the editor: floor tiles and grid, walls, pillars,
  emissive corner pylons with point lights, a shadow-casting sun, player body and camera.

## Controls
| Input | Action |
|---|---|
| WASD | Move |
| Mouse | Aim |
| Left mouse (hold) | Fire |
| Space | Dash |
| Esc | Pause |
| Enter / M | Restart / menu after game over |

## Run
From `samples/`:

```
cargo run -p arena_shooter
```

Test hooks from `sample_kit`: `SAMPLE_AUTOPLAY=1` starts a round and lets a simple autopilot
circle-strafe and shoot the nearest enemy; `SAMPLE_SCREENSHOT=<png>` with
`SAMPLE_SCREENSHOT_DELAY=<seconds>` saves a screenshot and exits.

## Files
- `src/level.rs`: static arena, player body and camera (Bevy prelude only; included by the editor).
- `src/main.rs`: gameplay (input, autopilot, player, bullets, waves, enemies, pickups, HUD, camera follow).
- `bevylite.project.json`: editor project metadata.
