/** HTTP status shared by the local store, native executor and request boundary. */
export class WorkbenchError extends Error {
  constructor(
    message: string,
    readonly status = 400,
  ) {
    super(message);
  }
}
