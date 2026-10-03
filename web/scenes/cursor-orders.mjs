import { partialBuildingCursor, narrowRefusal, readoutEdge } from "./_cursorOrders.mjs";

export async function run(ctx) {
  await partialBuildingCursor(ctx);
  await narrowRefusal(ctx);
  await readoutEdge(ctx);
}
