// Content outside its panel: the geometry of a panel sized too small for
// what it holds (a card collapsed to its padding, a row too long for its
// box). A panel can still have a box on screen and every word drawn, so
// neither proves the panel holds them.

/**
 * In the page: every visible element inside a `selector` panel that lies
 * past the panel's edges by more than half a pixel, as "panel › element"
 * strings. Content a clipping or scrolling element between them hides on
 * purpose (overflow other than visible) is not a spill, and a panel nested
 * in another is judged on its own content, not as its parent's.
 */
export function spills(page, selector) {
  return page.evaluate((selector) => {
    const name = (e) =>
      `${e.tagName.toLowerCase()}.${[...e.classList].join(".") || "?"}`.replace(/\.$/, "");
    const out = [];
    for (const panel of document.querySelectorAll(selector)) {
      const p = panel.getBoundingClientRect();
      if (p.width === 0 && p.height === 0) continue;
      for (const child of panel.querySelectorAll("*")) {
        const c = child.getBoundingClientRect();
        if (c.width === 0 || c.height === 0) continue;
        if (getComputedStyle(child).visibility === "hidden") continue;
        // A panel within the panel (a popover) is judged as a panel itself.
        if (child.matches(selector)) continue;
        let skip = false;
        for (let a = child.parentElement; a && a !== panel; a = a.parentElement) {
          const s = getComputedStyle(a);
          if (s.overflowX !== "visible" || s.overflowY !== "visible") skip = true;
          if (a.matches(selector)) skip = true;
        }
        if (skip) continue;
        if (
          c.left < p.left - 0.5 ||
          c.right > p.right + 0.5 ||
          c.top < p.top - 0.5 ||
          c.bottom > p.bottom + 0.5
        )
          out.push(`${name(panel)} › ${name(child)}`);
      }
    }
    return out;
  }, selector);
}
