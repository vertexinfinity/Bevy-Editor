# Neon Runner

A three-lane endless runner. The runner stays put while the track, arches, barriers and
coins scroll toward the camera, and the speed keeps climbing until you crash.

## Controls

| Input | Action |
| --- | --- |
| A / D or Left / Right | Switch lane |
| Space / W / Up | Jump (clears yellow barriers, not red walls) |
| Esc | Pause |
| Enter | Start / play again |
| M | Back to the title screen after a run |

Score = metres run + 25 per coin. The best score is kept for the session.

## Run it

From the `samples/` folder:

```powershell
cargo run -p endless_runner
```

Or open **Neon Runner** from the **Samples** section of the Bevylite hub to see the level
in the editor, then press **Run App (Cargo)**.

## Files

| File | Purpose |
| --- | --- |
| `src/level.rs` | Static track, arches, lights, runner and camera. The editor loads this same file. |
| `src/main.rs` | Lanes, jumping, scrolling, hazard and coin spawning, collisions, HUD, autopilot test hook. |
| `../sample_kit` | Shared title menu, HUD panel, pause, game-over and restart flow. |

`SAMPLE_AUTOPLAY=1` makes the runner dodge on its own (used for automated screenshots).
