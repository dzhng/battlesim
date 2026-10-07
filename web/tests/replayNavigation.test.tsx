import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { BrowserRouter, Link, Route, Routes } from "react-router";
import { afterEach, expect, test, vi } from "vitest";
import { ReplayImport, type ReplayFile } from "@apps/battle-lab/src/replayFile";

/** A saved battle as the viewer reads it: its prepared battle and commands. */
const SAVED = JSON.stringify({
  battle: { scenario: "{}", report: { request: {} } },
  replay: "{}",
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

test("an imported replay opens its viewer through the live router", async () => {
  storage();
  render(
    <BrowserRouter unstable_useTransitions={false}>
      <PageVisit>
        <Routes>
          <Route
            path="/"
            element={
              <ReplayImport />
            }
          />
          <Route path="/battle" element={<p>Imported battle viewer</p>} />
        </Routes>
      </PageVisit>
    </BrowserRouter>,
  );
  const file = { text: async () => SAVED };
  await act(async () => {
    fireEvent.change(screen.getByTestId("replay-file"), { target: { files: [file] } });
  });
  expect(window.location.pathname + window.location.search).toBe("/battle?replay=saved");
  expect(screen.getByText("Imported battle viewer")).toBeDefined();
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

test("a completed storage write cannot navigate after departure", async () => {
  const finishWrite = storage(true);
  const file = { text: async () => SAVED };
  render(
    <BrowserRouter unstable_useTransitions={false}>
      <PageVisit>
        <Routes>
          <Route
            path="/"
            element={
              <>
                <ReplayImport />
                <Link to="/elsewhere">Leave</Link>
              </>
            }
          />
          <Route path="/elsewhere" element={<p>Another screen</p>} />
          <Route path="/battle" element={<p>Imported battle viewer</p>} />
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
});
