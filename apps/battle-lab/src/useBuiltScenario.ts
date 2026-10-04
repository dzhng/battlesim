import { useEffect, useRef, useState } from "react";
import { useLabLoading } from "./LabLoading";
import { loadWasm, type Wasm } from "@web/battle/sim/module";

/** What the simulation builds from `options` (a scenario JSON, or a route's
 *  own record of one): null while it builds (or while the last build was for
 *  other options, so a view never runs one battle under another's options),
 *  `{ error }` if building failed. */
export function useBuiltScenario<Options, Built = string>(
  options: Options,
  build: (wasm: Wasm, options: Options, signal: AbortSignal) => Built | Promise<Built>,
): Built | { error: string } | null {
  const key = JSON.stringify(options);
  const [built, setBuilt] = useState<{ key: string; value: Built } | { error: string } | null>(
    null,
  );
  const buildRef = useRef(build);
  buildRef.current = build;
  const optionsRef = useRef(options);
  optionsRef.current = options;
  useEffect(() => {
    let live = true;
    const controller = new AbortController();
    setBuilt(null);
    loadWasm()
      .then((wasm) => {
        if (controller.signal.aborted) throw new Error("Scenario preparation cancelled");
        return buildRef.current(wasm, optionsRef.current, controller.signal);
      })
      .then(
        (value) => live && setBuilt({ key, value }),
        (e: Error) => live && setBuilt({ error: e.message }),
      );
    return () => {
      live = false;
      controller.abort();
    };
  }, [key]);
  useLabLoading(
    "preparation",
    !!built && !("error" in built) && built.key === key,
    built && "error" in built ? built.error : null,
  );
  if (built && "error" in built) return built;
  return built?.key === key ? built.value : null;
}

/** Whether a build failed. */
export const buildFailed = (built: unknown): built is { error: string } =>
  typeof built === "object" && built !== null && "error" in built;
