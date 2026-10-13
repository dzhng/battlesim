import { renderHook } from "@testing-library/react";
import { expect, test } from "vitest";
import { useLive } from "@apps/battle-lab/src/feed";

// React's development build logs a changed prop's contents into a
// performance.measure detail. A holder that reads a large live object (a
// battle's presentation) must therefore stay one object when its source
// changes (a Restart's new sim client), or each change copies the old
// object's tables into the detail.
test("a live reading stays one object while its source changes, and reads the latest", () => {
  const first = { tables: new Float32Array(4) };
  const second = { tables: new Float32Array(4) };
  const view = renderHook(({ client }) => useLive(() => client?.tables ?? null), {
    initialProps: { client: first as { tables: Float32Array } | null },
  });
  const holder = view.result.current;
  expect(holder.current).toBe(first.tables);
  view.rerender({ client: second });
  expect(view.result.current).toBe(holder);
  expect(holder.current).toBe(second.tables);
  view.rerender({ client: null });
  expect(view.result.current).toBe(holder);
  expect(holder.current).toBeNull();
});
