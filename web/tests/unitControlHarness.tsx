// The unit control at its input seam, for the tests that drive it: a
// catalog whose rifle and recon share a role, a client that records what it
// is sent, and a selected rifle squad (armed) and supply truck (unarmed).
import { act, renderHook } from "@testing-library/react";
import type { ReactNode } from "react";
import { UnitCatalog } from "@packages/scene-assets/src/units";
import { SessionCatalogProvider } from "@web/battle/catalog/context";
import { TEST_CATALOG } from "./catalog";
import { useUnitControl } from "../src/battle/input/useUnitControl";
import type { PointerPick } from "../src/battle/input/pointerIntent";
import type { SimClient } from "../src/battle/sim/client";
import type { ContactView, ObservationView, OwnUnitView } from "../src/battle/sim/observationView";
import type { Order } from "../src/battle/sim/protocol";

// Distinct types sharing a role make type selection distinguishable from role selection.
export const catalog = {
  ...TEST_CATALOG,
  units: new UnitCatalog({
    ...TEST_CATALOG.units.view,
    units: TEST_CATALOG.units.view.units.map((type) =>
      type.id === "test_recon"
        ? { ...type, roles: TEST_CATALOG.units.type("test_rifle").roles }
        : type,
    ),
  }),
};
export const wrapper = ({ children }: { children: ReactNode }) => (
  <SessionCatalogProvider catalog={catalog}>{children}</SessionCatalogProvider>
);

/** A client that records what it is sent and accepts all of it. */
export function recordingClient() {
  const sent: { order: Order; queued: boolean }[] = [];
  const client = {
    command: async (order: Order, queued = false) => {
      sent.push({ order, queued });
      return { seq: sent.length, applied_tick: 0, error: null, refused: [] };
    },
  } as unknown as SimClient;
  return { client, sent };
}

export const own = (id: number, kind: string) =>
  ({ id, kind, position: [0, 0, 0], yaw: 0 }) as unknown as OwnUnitView;

/** A rifle squad (armed) and a supply truck (unarmed), both selected. */
export async function control(contacts: ContactView[] = []) {
  const observation = {
    own: [own(1, "test_rifle"), own(2, "test_supply")],
    contacts,
  } as unknown as ObservationView;
  const { client, sent } = recordingClient();
  const hook = renderHook(() => useUnitControl(client, observation), { wrapper });
  act(() => hook.result.current.setSelected([1, 2]));
  // Each click a second after the last unless it says when: never a double.
  let clock = 0;
  const click = async (pick: Partial<PointerPick>) => {
    await act(async () =>
      hook.result.current.onPointer({
        unit: null,
        button: "right",
        shift: false,
        ctrl: false,
        x: 0,
        y: 0,
        time: (clock += 1000),
        ground: [50, 50],
        ...pick,
      }),
    );
  };
  return { hook, sent, click };
}
