# Coin Courtyard

A third-person 3D collect-a-thon built with Bevy and the shared `sample_kit`.

Run around a walled sandstone courtyard and grab all 12 spinning, bobbing gold coins before the
90 second clock runs out. Some coins sit on raised platforms, reached by jumping up stairs and
terraces. Collecting every coin wins. If the timer reaches zero, you lose.

The sample shows:

- A capsule character (with a visor, so you can see which way it faces) that moves relative to
  the camera, with acceleration easing, sprinting, jumping and gravity.
- Simple box collision: landing on platform tops, being pushed out of platform, pillar and
  tree sides, and walls that keep you inside the courtyard.
- An orbit follow camera: hold the right mouse button to orbit, use the wheel to zoom. The
  camera follows the player smoothly. On the title screen it slowly circles above the courtyard.
- The shared game flow from `sample_kit`: title menu, HUD, Esc to pause, game-over screen, restart.
- A static level in its own file (`src/level.rs`) that the Bevylite editor can load.

## Controls

| Input | Action |
| --- | --- |
| WASD | Move (relative to the camera) |
| Shift | Sprint |
| Space | Jump |
| Right mouse (hold) + drag | Orbit camera |
| Mouse wheel | Zoom |
| Esc | Pause / resume |
| Enter | Start / play again |
| M | Back to the menu (on the result screen) |

## Running

From the `samples/` folder:

```
cargo run -p third_person
```

Test hooks (see `sample_kit`): `SAMPLE_AUTOPLAY=1` starts a round on its own and turns on an
autopilot that runs to the nearest coin it can reach. `SAMPLE_SCREENSHOT=<path>` with
`SAMPLE_SCREENSHOT_DELAY=<seconds>` saves a screenshot and exits.

## Scoring

The HUD shows `Coins n / 12`, the seconds left and a short hint. Banners announce the start,
the last coin, the final 10 seconds and "All coins!". `BestScore` holds your fastest clear time
in whole seconds (lower is better) for this session. It is shown on the result screen and on
the title menu.

## Files

| File | Contents |
| --- | --- |
| `src/level.rs` | Static level: ground, walls, platforms, pillars, trees, lights, player body, camera. Shared with the editor, so it only uses the Bevy prelude. |
| `src/main.rs` | App setup, coins, round timer, win/lose, HUD and best time. |
| `src/player.rs` | Player movement, jumping, box collision and the test autopilot. |
| `src/camera.rs` | Orbit follow camera and title-screen flyover. |
| `bevylite.project.json` | Editor project manifest. |
