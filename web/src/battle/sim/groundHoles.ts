/** One occupancy bit per tile, not per-cell ground values. At 18 km this
 * metadata costs 158,204 bytes and first-page arrival is constant work. */
export class GroundHoles {
  private readonly words: Uint32Array;
  constructor(count: number) {
    this.words = new Uint32Array(Math.ceil(count / 32)).fill(0xffffffff);
    const tail = count % 32;
    if (tail) this.words[this.words.length - 1] = 0xffffffff >>> (32 - tail);
  }
  remove(index: number): void {
    this.words[index >>> 5] &= ~(1 << (index & 31));
  }
  /** Scans bounded occupancy words; no retained-page key copy or sort. */
  forEach(emit: (index: number) => void): void {
    for (let at = 0; at < this.words.length; at++) {
      let word = this.words[at];
      while (word) {
        const bit = 31 - Math.clz32(word & -word);
        emit(at * 32 + bit);
        word &= word - 1;
      }
    }
  }
}
