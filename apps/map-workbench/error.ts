import type { MapDiagnostic } from "../../web/src/maps/source";

/** HTTP status shared by the local store, native executor and request boundary. */
export class WorkbenchError extends Error {
  constructor(
    message: string,
    readonly status = 400,
    readonly diagnostics: MapDiagnostic[] = [],
  ) {
    super(message);
  }
}
