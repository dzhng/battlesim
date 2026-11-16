/** Draw one bounded index without changing how many random values are consumed. */
export const drawIndex = (random: () => number, length: number) =>
  Math.min(length - 1, Math.max(0, Math.floor(random() * length)));
