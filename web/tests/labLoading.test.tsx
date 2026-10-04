import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import { LabRouter } from "@apps/battle-lab/src/router";
import { lazy } from "react";
import { LabLoading, useLabLoading } from "@apps/battle-lab/src/LabLoading";

vi.mock("@apps/battle-lab/src/routes/contacts", () => ({
  default: () => <div>Contact battle preparation</div>,
}));

afterEach(cleanup);

test("lab loading covers preparation and the first frame, then reveals the battle", async () => {
  let resolve!: (value: { default: typeof View }) => void;
  function View({ ready = false }: { ready?: boolean }) {
    useLabLoading("renderer", ready);
    return <div>Battle</div>;
  }
  const Route = lazy(
    () =>
      new Promise<{ default: typeof View }>((done) => {
        resolve = done;
      }),
  );
  const view = render(
    <LabLoading subject="CONTACTS">
      <Route />
    </LabLoading>,
  );
  expect(screen.getByTestId("loading-subject").textContent).toBe("CONTACTS");
  expect(screen.getByTestId("loading-cancel").getAttribute("href")).toBe("/labs");
  resolve({ default: View });
  await screen.findByText("Battle");
  await waitFor(() =>
    expect(screen.getByTestId("loading-stage").textContent).toBe("Starting the battle"),
  );
  view.rerender(
    <LabLoading subject="CONTACTS">
      <Route ready />
    </LabLoading>,
  );
  expect(screen.queryByTestId("loading")).toBeNull();
});

test("a drawn lab stays covered until its first observation arrives", () => {
  function View({ observed }: { observed: boolean }) {
    useLabLoading("renderer", true);
    useLabLoading("world", observed);
    return <div>Battle</div>;
  }
  const view = render(
    <LabLoading subject="CONTACTS">
      <View observed={false} />
    </LabLoading>,
  );
  expect(screen.getByTestId("loading-stage").textContent).toBe("Building the battlefield");
  view.rerender(
    <LabLoading subject="CONTACTS">
      <View observed />
    </LabLoading>,
  );
  expect(screen.queryByTestId("loading")).toBeNull();
});

test("startup failures replace loading with the shared refusal and diagnostics", () => {
  function View() {
    useLabLoading("preparation", false, "Saved map missing");
    return <div data-testid="error">Old refusal</div>;
  }
  render(
    <LabLoading subject="CONTACTS">
      <View />
    </LabLoading>,
  );
  expect(screen.getAllByTestId("error")).toHaveLength(1);
  expect(screen.getByRole("heading").textContent).toBe("Aborted");
  expect(screen.getByTestId("loading").getAttribute("aria-busy")).toBe("false");
  fireEvent.click(screen.getByRole("button", { name: "Details" }));
  expect(screen.getByTestId("error-details").textContent).toBe("Saved map missing");
});

test("the router covers a lab while its route prepares", async () => {
  render(<LabRouter path="/lab/contacts" />);
  expect(screen.getByTestId("loading-subject").textContent).toBe("CONTACTS");
  await screen.findByText("Contact battle preparation");
  expect(screen.getByTestId("loading")).toBeDefined();
});

test.each(["/lab/cursor", "/lab/panels", "/benchmark"])(
  "non-battle view %s becomes usable",
  async (path) => {
    render(<LabRouter path={path} />);
    await waitFor(() => expect(screen.queryByTestId("loading")).toBeNull());
  },
);
