import { act, renderHook } from "@testing-library/react";
import { expect, test } from "vitest";
import { useReplayCamera } from "@apps/battle-lab/src/useReplayCamera";
import type { BattleSession } from "@apps/battle-lab/src/useBattleSession";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";

const initial: Camera3DParams = {
  target: [10, 20, 3],
  distance: 100,
  pitch: 0.7,
  yaw: 0,
  fovY: 0.8,
  near: 0.5,
  aspect: 1.6,
};

test("only playing humans record; restart clears their track and watching preserves the original", () => {
  let time = 0;
  const client = { presentation: { clock: () => time } };
  const session = {
    viewer: { id: "blue-1", side: "blue", controller: "human" },
    sim: { client },
  } as unknown as BattleSession;
  const hook = renderHook(
    ({ playing, recorded, inCommand = true }) =>
      useReplayCamera(session, initial, recorded, playing, inCommand),
    {
      initialProps: {
        playing: true,
        recorded: undefined as ReturnType<typeof useReplayCamera>["track"],
        inCommand: true,
      },
    },
  );
  hook.result.current.record(0, initial);
  const saved = JSON.parse(JSON.stringify(hook.result.current.track));
  expect(saved.samples[0].camera.target).toEqual(initial.target);
  expect(saved.player).toBe("blue-1");
  session.sim.client = { ...client } as unknown as typeof session.sim.client;
  hook.rerender({ playing: true, recorded: undefined, inCommand: true });
  expect(hook.result.current.track?.samples).toEqual([]);
  hook.rerender({ playing: true, recorded: undefined, inCommand: false });
  hook.result.current.record(1, initial);
  expect(hook.result.current.track?.samples).toEqual([]);
  hook.rerender({ playing: false, recorded: saved, inCommand: true });
  hook.result.current.record(10, { ...initial, distance: 500 });
  expect(hook.result.current.track).toEqual(saved);
  time = 10;
  expect(hook.result.current.pilot?.pose(99)).toMatchObject({
    target: initial.target,
    distance: 100,
  });
  act(() => hook.result.current.setFollowing(false));
  expect(hook.result.current.pilot?.pose(99)).toBeNull();
  session.viewer = null;
  hook.rerender({ playing: true, recorded: undefined, inCommand: true });
  hook.result.current.record(10, initial);
  expect(hook.result.current.track).toBeUndefined();
  hook.unmount();
});
