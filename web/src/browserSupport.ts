/** Which browsers the game opens in: Chrome, or another browser on Chrome's
 *  engine (Edge, Brave, Opera, Arc), which draws WebGPU the same way. Safari,
 *  Firefox and every iOS browser (all WebKit underneath, whatever their name)
 *  get a notice instead of the game. */

/** Whether the browser reporting `userAgent` runs on Chromium: every Chromium
 *  user agent names Chrome, and no other engine's does (iOS Chrome calls
 *  itself CriOS). */
export const isChromium = (userAgent: string): boolean => /Chrome\/\d/.test(userAgent);
