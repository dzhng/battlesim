/** A bounded buddy allocator for sampled words, with no GPU resource of its
 * own. Pages return their blocks before changed pages acquire replacements. */
export class ScarPool {
  private readonly free: Set<number>[];
  readonly capacity: number;
  readonly maxLevel: number;
  constructor(capacity: number) {
    this.capacity = capacity;
    this.maxLevel = Math.floor(Math.log2(capacity));
    this.free = Array.from({ length: this.maxLevel + 1 }, () => new Set<number>());
    let offset = 0;
    while (offset < capacity) {
      const level = Math.floor(Math.log2(capacity - offset));
      this.free[level].add(offset);
      offset += 2 ** level;
    }
  }
  allocate(words: number): number {
    const level = Math.ceil(Math.log2(Math.max(16, words)));
    let found = level;
    while (found <= this.maxLevel && !this.free[found].size) found++;
    if (found > this.maxLevel) return -1;
    const offset = this.free[found].values().next().value!;
    this.free[found].delete(offset);
    while (found > level) {
      found--;
      this.free[found].add(offset + 2 ** found);
    }
    return offset;
  }
  release(offset: number, words: number): void {
    let level = Math.ceil(Math.log2(Math.max(16, words)));
    while (level < this.maxLevel) {
      const buddy = offset ^ (2 ** level);
      if (!this.free[level].has(buddy)) break;
      this.free[level].delete(buddy);
      offset = Math.min(offset, buddy);
      level++;
    }
    this.free[level].add(offset);
  }
}
