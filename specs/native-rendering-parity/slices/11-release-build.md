# 11 — Release build

**Contract:** native is measured as it would ship, as a release export of the Godot project, not the editor binary's DEBUG run. The browser is measured as its production build.

**Seam:**
- Godot 4.7.2 export templates. They are not installed today; fetch them into the user's Godot data folder.
- An export preset for macOS in `native/godot/`.
- One command that exports to `throwaway/` and runs the timing mode from the exported app.
- The browser side uses the scene runner's production build.

**Runnable artifact:** debug and release timings side by side for both clients.

**Focused gates:**
- The exported build renders the same plates within the noise floor.
- The report states its build kind.

**Out of scope:** signing, notarisation and distribution.

**Delegated:** preset details.
