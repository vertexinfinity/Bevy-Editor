# Editor Showcase

Open **Editor Showcase** under **Your Projects** in the Bevylite hub. Reopening this
bundled procedural scene restores the original fixture. No downloads are needed.
The editor and standalone sample use the same scene generator, with editor authoring
components added by `src/scene/showcase.rs` in the editor repository.

For direct editor testing from the repository root, run
`cargo run --example editor_showcase`. This opens a separate editor instance directly
on the fixture and preserves work in any already-open editor window.

Create an independent copy from the editor repository:

```powershell
cargo run -- new MyShowcase --template editor_showcase --output D:\Projects
```

Run the standalone app from this directory with `cargo run`. **Space** opens/closes
the gate and the beacon rotates. In the editor, Play enables the orbit camera and
beacon animation. Lua assets demonstrate inspector data; the editor does not execute
Lua. The fixture assigns script properties and entity links in Rust.

## Fixture map

| Branch / entity | Coverage |
| --- | --- |
| Environment | Nested entities, local/global transforms, meshes and shadows |
| GateDoor | Float, integer, boolean, string and vector properties in grouped sections |
| Switch_01 | Outgoing gate link and incoming reference from TriggerZone |
| TriggerZone | Disabled script, gate target and switch activator references |
| PBR Material Gallery | 25 independent metallic/roughness combinations |
| Lighting | One shadow-casting sun, warm point light and cool spotlight |
| Beacon | Emissive material and Play-only spin animation |
| Hierarchy Stress | 64 initially hidden props for scrolling, search and visibility |
| Showcase Orbit camera | Scene camera activation and Play/Pause/Stop |

## Manual verification checklist

Start from a fresh sample. Record platform, resolution, revision and failed steps.

- [ ] Open sample; GateDoor is selected and its grouped properties are visible.
- [ ] Expand/collapse groups; search `Scroll Test Prop 42` with ancestors collapsed.
- [ ] Expand Hierarchy Stress; scroll to its last child; toggle its visibility and restore it.
- [ ] Resize hierarchy, inspector and asset dock; test narrow/wide layouts and scrolling.
- [ ] GateDoor Relations shows its parent, switch, camera and two incoming references.
- [ ] Click its switch link; Switch_01 is selected; use its gate link to return.
- [ ] Select TriggerZone; script starts disabled and both links have valid targets.
- [ ] Toggle script enabled in Active Scripts; check property actions and selection.
- [ ] Edit one gallery sphere's material; verify another sphere is unchanged.
- [ ] Change roughness, metallic, emissive and unlit; inspect viewport and labels; restore.
- [ ] Select sun/point light; change bake mode/shadows; inspect spotlight in Debug View.
- [ ] Edit a child's transforms; test Local/Global coordinates and focus with F.
- [ ] Play: orbit camera and rotating beacon; Pause freezes; Stop restores Edit mode.
- [ ] Browse script/material/scene folders; create assets in a disposable sample copy.
- [ ] Save As, reload, and compare entity counts, links, materials and transforms.
- [ ] Try CPU/GPU baking at low resolution; inspect completion/errors.
- [ ] Run standalone sample; Space toggles gate motion and the beacon rotates.

Run the fixture regression check from the editor repository:

```powershell
cargo test --lib showcase_has_real_hierarchy_valid_links_and_material_variation
```

## Test boundaries

The sample exercises existing functionality. It does not supply physics trigger
execution, Lua execution, arbitrary custom inspector renderers, a relationship graph,
or tabbed docking. Save/reload and Play/Stop are deliberate regression checks: the
current scene sidecar does not preserve every script/hierarchy field. Reopening the
sample is the reset path; a successful save alone does not prove a complete roundtrip.
