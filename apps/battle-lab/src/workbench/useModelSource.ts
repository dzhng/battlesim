/** Source selection owns the complete async read/validation lifetime. */
import { useCallback, useEffect, useRef, useState } from "react";
import type { AppearanceUnit } from "@packages/scene-assets/src/schema";
import type { LooseOptions } from "@packages/scene-assets/src/loose";
import type { LoadedModel } from "./sources";

export interface SourceOptions {
  unit: AppearanceUnit | "auto";
  type: string | "auto";
  scenery: string;
  yaw: "auto" | number;
}

export function validationOptions(opts: SourceOptions): LooseOptions {
  return {
    unit: opts.unit === "auto" ? undefined : opts.unit,
    type: opts.type === "auto" ? undefined : opts.type,
    scenery: opts.unit === "scenery" ? opts.scenery : undefined,
    yaw: opts.yaw === "auto" ? undefined : opts.yaw,
  };
}

export interface DroppedSource {
  name: string;
  read(): Uint8Array | Promise<Uint8Array>;
}

export function useModelSource(install: (model: LoadedModel) => void) {
  const version = useRef(0);
  const dropped = useRef<DroppedSource | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  useEffect(
    () => () => {
      version.current++;
    },
    [],
  );

  const load = useCallback(
    async (read: () => Promise<LoadedModel>, file: DroppedSource | null = null) => {
      const ticket = ++version.current;
      dropped.current = file;
      setBusy(true);
      setError(null);
      try {
        const next = await read();
        if (version.current !== ticket) return null;
        install(next);
        return { ticket };
      } catch (error) {
        if (version.current !== ticket) return null;
        setError(error instanceof Error ? error.message : String(error));
        return { error };
      } finally {
        if (version.current === ticket) setBusy(false);
      }
    },
    [install],
  );
  return { version, dropped, error, setError, busy, load };
}
