import { MECHANICS_API, type MechanicsDraft } from "./protocol";

/** A refused request; `conflict` when the sources changed under the draft. */
export class RequestError extends Error {
  constructor(
    message: string,
    readonly conflict: boolean,
  ) {
    super(message);
  }
}

/** Ask the local development API, posting `draft` when given. */
export async function request<T>(suffix = "", draft?: MechanicsDraft): Promise<T> {
  const response = await fetch(
    `${MECHANICS_API}${suffix}`,
    draft
      ? {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(draft),
        }
      : undefined,
  );
  const body = (await response.json()) as T & { error?: string };
  if (!response.ok)
    throw new RequestError(
      body.error ?? `Request failed (${response.status}).`,
      response.status === 409,
    );
  return body;
}
