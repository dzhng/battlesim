/** A battle's player address and private control address may share a listener. */
export interface Machine {
  address: string;
  control: string;
  stop(): void;
}
export interface BattleMachines {
  capacity: number;
  start(key: string): Promise<Machine>;
}
