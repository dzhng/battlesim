import type { MapChoice } from "../../../web/src/maps/source";
import type { Draft, JsonObject, Report, WorkbenchAPI } from "./protocol";

export interface SeedSample {
  input: Draft;
  choices: MapChoice[];
  rows: (Report & { source?: number })[];
  sources: JsonObject[];
  receiptErrors: string[];
  complete: boolean;
  cancelled: boolean;
}

/** Each artifact is exported before the next replaces it. Geometry is not
 * retained by a sample; each row names its exact source bundle by index. */
export async function collectSample(
  api: WorkbenchAPI,
  input: Draft,
  choices: MapChoice[],
  signal: AbortSignal,
  publish: (progress: SeedSample) => void,
): Promise<SeedSample> {
  const rows: SeedSample["rows"] = [];
  const sources: JsonObject[] = [];
  const receiptErrors: string[] = [];
  const snapshot = (): SeedSample => ({
    input,
    choices,
    rows: [...rows],
    sources: [...sources],
    receiptErrors: [...receiptErrors],
    complete: rows.length === choices.length,
    cancelled: signal.aborted,
  });
  publish(snapshot());
  for (const choice of choices) {
    if (signal.aborted) break;
    try {
      const {
        svg: _svg,
        features: _features,
        ...report
      } = await api.generate(
        { purpose: "sample", retainedArtifactIds: [], draft: input, choice },
        signal,
      );
      if (signal.aborted) break;
      const row: SeedSample["rows"][number] = report;
      rows.push(row);
      if (report.artifactId) {
        try {
          const { inputs, receipts } = await api.export(report.artifactId, signal);
          const existing = sources.findIndex(
            (source) => JSON.stringify(source.receipts) === JSON.stringify(receipts),
          );
          row.source = existing >= 0 ? existing : sources.push({ inputs, receipts }) - 1;
        } catch (error) {
          receiptErrors.push(`${choice.type} ${choice.size} ${choice.seed}: ${String(error)}`);
        }
      }
    } catch (error) {
      if (signal.aborted) break;
      rows.push({
        fingerprint: input.revision,
        choice,
        status: "refused",
        stage: "execution",
        diagnostics: [
          { code: "execution_failed", feature: null, location: "$", message: String(error) },
        ],
      });
    }
    publish(snapshot());
  }
  return snapshot();
}
