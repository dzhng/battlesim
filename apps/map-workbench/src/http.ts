import type { WorkbenchAPI } from "./protocol";

export function workbenchAPI(): WorkbenchAPI {
  async function request<T>(operation: string, body?: unknown, signal?: AbortSignal): Promise<T> {
    const response = await fetch(`/__map-workbench/${operation}`, {
      method: body === undefined ? "GET" : "POST",
      headers: body === undefined ? undefined : { "Content-Type": "application/json" },
      body: body === undefined ? undefined : JSON.stringify(body),
      signal,
    });
    const result = await response.json();
    if (!response.ok) {
      const detail = result.diagnostics
        ?.map((row: { location: string; message: string }) => `${row.location}: ${row.message}`)
        .join("; ");
      throw new Error(
        detail || result.message || result.error || `Request failed (${response.status})`,
      );
    }
    return result as T;
  }
  return {
    snapshot: (signal) => request("snapshot", undefined, signal),
    generate: (input, signal) => request("generate", input, signal),
    inspect: (artifactId, crop, signal) => request("inspect", { artifactId, crop }, signal),
    sight: (artifactId, signal) => request("sight", { artifactId }, signal),
    preview: (draft, signal) => request("preview", draft, signal),
    save: (review, signal) => request("save", review, signal),
    export: (artifactId, signal) => request("export", { artifactId }, signal),
  };
}
