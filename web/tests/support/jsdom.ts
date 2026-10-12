// What every browser the game runs in has and jsdom lacks. jsdom has no
// layout, so an observer here never sees a size change: pages that observe
// one (the main menu's plate) mount and keep their unmeasured fallback.
if (typeof window !== "undefined" && !("ResizeObserver" in globalThis)) {
  globalThis.ResizeObserver = class {
    observe() {}
    unobserve() {}
    disconnect() {}
  };
}
