/** The longest a response may run. Responses never reach the simulation, so they have no
 *  `ShoutRules.talk_s`; barks are held to theirs (fixtures/game.json `shouts.talk_s`). */
export const RESPONSE_TALK_S = 4;
/** Word error rate above which a take is flagged. Short lines make one wrong word worth 0.2–0.33. */
export const MAX_WORD_ERROR = 0.4;
/** Words whose loss changes the order; the set's callsign is always one. */
const KEYWORDS = ["sabot", "gunner"];

const UNITS = ["zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine"];
const TEENS = [
  "ten",
  "eleven",
  "twelve",
  "thirteen",
  "fourteen",
  "fifteen",
  "sixteen",
  "seventeen",
  "eighteen",
  "nineteen",
];
const TENS = ["", "", "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety"];
const digits = /^\d+$/;

/**
 * The words a listener hears: no audio tags or punctuation, and numbers as
 * digits however they are spoken ("two-one", "twenty one" and "21" alike).
 */
export function words(text: string): string[] {
  const tokens = text
    .toLowerCase()
    .replace(/\[[^\]]*\]/g, " ")
    .replace(/['’]/g, "")
    .split(/[^a-z0-9]+/)
    .filter(Boolean)
    .map((word) => {
      if (UNITS.includes(word)) return String(UNITS.indexOf(word));
      if (TEENS.includes(word)) return String(10 + TEENS.indexOf(word));
      if (TENS.includes(word)) return String(10 * TENS.indexOf(word));
      return word;
    });
  const out: string[] = [];
  for (const token of tokens) {
    const last = out.at(-1);
    if (last && digits.test(last) && digits.test(token)) {
      // "fifty five" is 55; "five five" and "two one" are read digit by digit.
      const tens = Number(last) % 10 === 0 && Number(last) >= 20 && token.length === 1;
      out[out.length - 1] = tens ? String(Number(last) + Number(token)) : last + token;
    } else out.push(token);
  }
  return out;
}

function distance(a: string[], b: string[]): number {
  let row = Array.from({ length: b.length + 1 }, (_, j) => j);
  for (let i = 1; i <= a.length; i++) {
    const next = [i];
    for (let j = 1; j <= b.length; j++)
      next[j] = Math.min(row[j] + 1, next[j - 1] + 1, row[j - 1] + (a[i - 1] === b[j - 1] ? 0 : 1));
    row = next;
  }
  return row[b.length];
}

export interface Match {
  /** 1 − word error rate, floored at 0. */
  score: number;
  /** Keywords the script has and the transcript lost. */
  missing: string[];
  overlong: boolean;
  flagged: boolean;
}

/** Compares what a take says with its script, and its length with its trigger's talk time. */
export function matches(
  script: string,
  transcript: string,
  {
    callsign,
    durationS = 0,
    talkS = RESPONSE_TALK_S,
  }: { callsign: string; durationS?: number; talkS?: number },
): Match {
  const said = words(script);
  const heard = words(transcript);
  const wordError = said.length ? distance(said, heard) / said.length : heard.length ? 1 : 0;
  const keywords = new Set([...words(callsign), ...KEYWORDS]);
  const missing = [...new Set(said.filter((word) => keywords.has(word) && !heard.includes(word)))];
  const overlong = durationS > talkS;
  const score = Math.max(0, 1 - wordError);
  return {
    score: Math.round(score * 100) / 100,
    missing,
    overlong,
    flagged: wordError > MAX_WORD_ERROR || missing.length > 0 || overlong,
  };
}
