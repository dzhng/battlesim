// HTTP refusals carry a public code and status, never arbitrary request text.
export class HttpRefusal extends Error {
  constructor(
    readonly status: number,
    readonly reason: string,
  ) {
    super(reason);
  }
}
