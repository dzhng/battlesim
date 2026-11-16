/** A battle exposes one address; control requests authenticate in headers. */
export interface Machine {
  address: string;
  stop(): void;
}
export interface BattleMachines {
  capacity: number;
  start(key: string): Promise<Machine>;
}

/** Allocation failed before any battlefield could be tried. */
export class MachineUnavailable extends Error {
  constructor() {
    super("battle machine unavailable");
  }
}
