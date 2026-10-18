# Initial verification

Passed in the development workspace:

- `cargo test --lib showcase_has_real_hierarchy_valid_links_and_material_variation`
- `cargo check --manifest-path samples/editor_showcase/Cargo.toml --offline --target-dir target`
- `cargo run --example editor_showcase` opens the fixture with 113 entities and GateDoor selected.
- GateDoor's Relations card lists parent, switch, camera and two incoming references.
- GateDoor -> Switch_01 -> GateDoor navigation updates hierarchy selection and inspector.
- Keyboard search `42` finds Scroll Test Prop 42 through the collapsed hidden stress branch;
  selecting the result displays its transform, material and parent relationship.
- Long hierarchy names remain on one clipped line instead of overlapping adjacent rows.
- `git diff --check` passes (Git reports line-ending conversion warnings).

The standalone project has a harmless unused-field warning for editor-only handles in
the shared scene generator. Its executable was type-checked, not launched during this pass.
Two simultaneous editor instances share the default RPC port, so the second logs a
port-in-use warning; UI checks work, but remote requests must target the owning instance.

The rest of the manual checklist in README.md remains to be exercised on this fixture,
especially scene roundtrip, Play/Stop restoration, baking and standalone runtime behavior.
