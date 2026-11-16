import { StreamBroken } from "./stream/index";

/** A normal HTTP refusal is terminal; reconnecting would repeat the request. */
export class RequestRefused extends StreamBroken {
  constructor(readonly reason: string) {
    super("Request refused");
  }
}
