// @vitest-environment node
import { expect, test } from "vitest";
import { Presentation } from "@wasm/renderer_browser";
import {
  ScenePresentation,
  loadPresentation,
  type PresentedFrame,
} from "@web/battle/sim/presentation";

function words(frame: PresentedFrame) {
  const { header, ...sections } = frame;
  return {
    header: Array.from(header),
    sections: Object.fromEntries(
      Object.entries(sections).map(([name, { count, u32 }]) => [
        name,
        { count, words: Array.from(u32) },
      ]),
    ),
  };
}

test("the shared browser module reacquires every UI stream after its linear memory grows", async () => {
  const { memory } = await loadPresentation();
  const scene = new ScenePresentation();
  try {
    scene.loadWorld({ sun_azimuth: 0.4, sun_elevation: 0.8 });
    const before = scene.frame(
      2.75,
      {
        eye: [12, -20, 30],
        target: [0, 0, 0],
        up: [0, 0, 1],
        fovY: 1,
        near: 0.1,
        width: 320,
        height: 180,
      },
      { selected: [], xray: false, ghosts: [] },
      0,
    );
    const expected = words(before);
    scene.noteAudio({ note: (feed) => expect(feed).toBeInstanceOf(Presentation) });

    memory.grow(1);
    expect(before.header.byteLength).toBe(0);
    const after = scene.lastFrame();

    expect(after.camera.f32.buffer).toBe(memory.buffer);
    expect(words(after)).toEqual(expected);
  } finally {
    scene.free();
  }
});
