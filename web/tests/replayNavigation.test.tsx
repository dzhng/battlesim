import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { BrowserRouter, Link, Route, Routes } from "react-router";
import { afterEach, expect, test, vi } from "vitest";
import { ReplayImport, parseReplayFile, type ReplayFile } from "@apps/battle-lab/src/replayFile";

/** A saved battle as the viewer reads it: its prepared battle and commands. */
const SAVED = JSON.stringify({
  battle: { scenario: "{}", report: { request: {} } },
  replay: "{}",
});

test("a human's camera survives loading a replay of a seat that began under AI", () => {
  const file = JSON.parse(SAVED);
  file.battle.report.request.players = [{ id: "red-2", side: "red", controller: "active_ai" }];
  file.camera = {
    player: "red-2",
    side: "red",
    samples: [
      {
        time: 10,
        camera: { target: [30, 40, 0], distance: 100, pitch: 0.7, yaw: 1, fovY: 0.8, near: 1 },
      },
    ],
  };
  expect(parseReplayFile(JSON.stringify(file)).camera).toEqual(file.camera);
});
import { PageVisit } from "@apps/battle-lab/src/navigation";

// Complete browser storage at its transaction boundary without real user state.
function storage(holdWrites = false) {
  let finishWrite: (() => void) | undefined;
  vi.stubGlobal("indexedDB", {
    open() {
      const opened = {} as IDBOpenDBRequest;
      queueMicrotask(() => {
        Object.defineProperty(opened, "result", {
          value: {
            close() {},
            transaction(_store: string, mode: IDBTransactionMode) {
              const transaction = {
                objectStore: () => ({
                  put() {
                    return {};
                  },
                  get() {
                    return { result: undefined };
                  },
                }),
              } as unknown as IDBTransaction;
              const finish = () => transaction.oncomplete?.(new Event("complete"));
              if (holdWrites && mode === "readwrite") finishWrite = finish;
              else queueMicrotask(finish);
              return transaction;
            },
          },
        });
        opened.onsuccess?.(new Event("success"));
      });
      return opened;
    },
  });
  return () => finishWrite!();
}

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  window.history.replaceState(null, "", "/");
});

test("departing while a file read is pending discards its viewer continuation", async () => {
  storage();
  const loaded: ReplayFile[] = [];
  let finish!: (text: string) => void;
  const file = {
    text: () =>
      new Promise<string>((resolve) => {
        finish = resolve;
      }),
  };
  render(
    <BrowserRouter unstable_useTransitions={false}>
      <PageVisit>
        <Routes>
          <Route
            path="/"
            element={
              <>
                <ReplayImport onLoad={(file) => loaded.push(file)} />
                <Link to="/elsewhere">Leave</Link>
              </>
            }
          />
          <Route path="/elsewhere" element={<p>Another screen</p>} />
        </Routes>
      </PageVisit>
    </BrowserRouter>,
  );
  fireEvent.change(screen.getByTestId("replay-file"), { target: { files: [file] } });
  fireEvent.click(screen.getByRole("link", { name: "Leave" }));
  await act(async () => {
    finish(SAVED);
  });
  expect(screen.getByText("Another screen")).toBeDefined();
  expect(window.location.pathname).toBe("/elsewhere");
  expect(loaded).toEqual([]);
});

test("a completed storage write cannot hand its battle over after departure", async () => {
  const finishWrite = storage(true);
  const loaded: ReplayFile[] = [];
  const file = { text: async () => SAVED };
  render(
    <BrowserRouter unstable_useTransitions={false}>
      <PageVisit>
        <Routes>
          <Route
            path="/"
            element={
              <>
                <ReplayImport onLoad={(file) => loaded.push(file)} />
                <Link to="/elsewhere">Leave</Link>
              </>
            }
          />
          <Route path="/elsewhere" element={<p>Another screen</p>} />
        </Routes>
      </PageVisit>
    </BrowserRouter>,
  );
  await act(async () => {
    fireEvent.change(screen.getByTestId("replay-file"), { target: { files: [file] } });
  });
  fireEvent.click(screen.getByRole("link", { name: "Leave" }));
  await act(async () => {
    finishWrite();
  });
  expect(screen.getByText("Another screen")).toBeDefined();
  expect(window.location.pathname).toBe("/elsewhere");
  expect(loaded).toEqual([]);
});
