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
/** One spelling for a word written two ways: the scripts are British, speech-to-text is American. */
const SPELLING: Record<string, string> = {
  armour: "armor",
  colour: "color",
  defence: "defense",
  centre: "center",
  metre: "meter",
  metres: "meters",
  kilometre: "kilometer",
  kilometres: "kilometers",
  manoeuvre: "maneuver",
  grey: "gray",
  tyre: "tire",
  goddammit: "goddamnit",
};
/** The most words one side may write as a single word ("God damn it" is "goddamnit"). */
const MAX_COMPOUND = 3;

/**
 * The words a listener hears: no audio tags or punctuation, one spelling per
 * word, and numbers as digits however they are spoken ("two-one", "twenty one"
 * and "21" alike; "four hundred" is 400).
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
      return SPELLING[word] ?? word;
    });
  const out: string[] = [];
  for (const token of tokens) {
    const last = out.at(-1);
    if (last && digits.test(last) && token === "hundred") out[out.length - 1] = `${last}00`;
    else if (last && digits.test(last) && digits.test(token)) {
      // "fifty five" is 55, "four hundred five" 405; "five five" and "two one" are read digit by digit.
      const value = Number(last);
      const sums =
        (value >= 100 && value % 100 === 0 && Number(token) < 100) ||
        (value >= 20 && value % 10 === 0 && token.length === 1);
      out[out.length - 1] = sums ? String(value + Number(token)) : last + token;
    } else out.push(token);
  }
  return out;
}

/**
 * `side` with each run of up to `MAX_COMPOUND` words joined where `other`
 * writes them as one word: "tree line" against "treeline", "co pilot" against
 * "copilot". Words the other side doesn't join are left alone, so a misread
 * ("rock it" for "rocket") still differs.
 */
function joined(side: string[], other: string[]): string[] {
  const whole = new Set(other);
  const out: string[] = [];
  for (let i = 0; i < side.length; ) {
    let width = 1;
    for (let n = Math.min(MAX_COMPOUND, side.length - i); n > 1; n--)
      if (whole.has(side.slice(i, i + n).join(""))) {
        width = n;
        break;
      }
    out.push(side.slice(i, i + width).join(""));
    i += width;
  }
  return out;
}

function editDistance(a: string[], b: string[]): number {
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

/** Compares what a take says with its script and, given `length`, its length with its
 *  trigger's talk time. */
export function matches(
  script: string,
  transcript: string,
  { callsign, length }: { callsign: string; length?: { durationS: number; talkS: number } },
): Match {
  const written = words(script);
  const spoken = words(transcript);
  // A compound either side writes as one word counts as one word on both.
  const said = joined(written, spoken);
  const heard = joined(spoken, said);
  const wordError = said.length ? editDistance(said, heard) / said.length : heard.length ? 1 : 0;
  const keywords = new Set([...words(callsign), ...KEYWORDS]);
  // Keywords are judged on the words as written: joining never hides a lost one.
  const missing = [
    ...new Set(written.filter((word) => keywords.has(word) && !spoken.includes(word))),
  ];
  const overlong = length !== undefined && length.durationS > length.talkS;
  const score = Math.max(0, 1 - wordError);
  return {
    score: Math.round(score * 100) / 100,
    missing,
    overlong,
    flagged: wordError > MAX_WORD_ERROR || missing.length > 0 || overlong,
  };
}
