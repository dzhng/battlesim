import { useEffect, useRef, useState } from "react";
import { loadWasm, type Wasm } from "@web/battle/sim/module";

/** A scenario JSON the simulation builds from `options`: null while it builds
 *  (or while the last build was for other options, so a view never runs one
 *  battle under another's options), `{ error }` if building failed. */
export function useBuiltScenario<Options>(
  options: Options,
  build: (wasm: Wasm, options: Options) => string,
): string | { error: string } | null {
  const key = JSON.stringify(options);
  const [built, setBuilt] = useState<{ key: string; json: string } | { error: string } | null>(
    null,
  );
  const buildRef = useRef(build);
  buildRef.current = build;
  const optionsRef = useRef(options);
  optionsRef.current = options;
  useEffect(() => {
    let live = true;
    setBuilt(null);
    loadWasm()
      .then((wasm) => buildRef.current(wasm, optionsRef.current))
      .then(
        (json) => live && setBuilt({ key, json }),
        (e: Error) => live && setBuilt({ error: e.message }),
      );
    return () => {
      live = false;
    };
  }, [key]);
  if (built && "error" in built) return built;
  return built?.key === key ? built.json : null;
}
