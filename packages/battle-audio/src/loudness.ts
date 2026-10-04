/** Library reference, before recipe, weapon, distance and bus gains. */
export const SOURCE_LOUDNESS_DB = -23;

type Biquad = [number, number, number, number, number];

// BS.1770 K-weighting; coefficient conversion follows libebur128's sample-rate
// parameterization: https://github.com/jiixyj/libebur128/blob/master/ebur128/ebur128.c
function weighting(sampleRate: number): [Biquad, Biquad] {
  let k = Math.tan((Math.PI * 1681.974450955533) / sampleRate);
  let q = 0.7071752369554196;
  const high = 10 ** (3.999843853973347 / 20);
  const middle = high ** 0.4996667741545416;
  let denominator = 1 + k / q + k * k;
  const shelf: Biquad = [
    (high + (middle * k) / q + k * k) / denominator,
    (2 * (k * k - high)) / denominator,
    (high - (middle * k) / q + k * k) / denominator,
    (2 * (k * k - 1)) / denominator,
    (1 - k / q + k * k) / denominator,
  ];
  k = Math.tan((Math.PI * 38.13547087602444) / sampleRate);
  q = 0.5003270373238773;
  denominator = 1 + k / q + k * k;
  return [shelf, [1, -2, 1, (2 * (k * k - 1)) / denominator, (1 - k / q + k * k) / denominator]];
}

/** Maximum K-weighted 100 ms window, 25 ms hops. Short reports are zero-padded;
 *  silence or a long quiet tail cannot dilute their calibration. This is a
 *  short-effect reference, not an integrated programme LUFS measurement. */
export function maxMomentaryLoudness(
  channels: readonly Float32Array[],
  sampleRate: number,
): number {
  const window = Math.max(1, Math.round(sampleRate * 0.1));
  const hop = Math.max(1, Math.round(window / 4));
  const frames = channels[0].length + window;
  const energy = new Float64Array(frames);
  const [shelf, highpass] = weighting(sampleRate);
  for (const samples of channels) {
    let s1 = 0,
      s2 = 0,
      h1 = 0,
      h2 = 0;
    for (let frame = 0; frame < frames; frame++) {
      const input = samples[frame] ?? 0;
      const shelved = shelf[0] * input + s1;
      s1 = shelf[1] * input - shelf[3] * shelved + s2;
      s2 = shelf[2] * input - shelf[4] * shelved;
      const weighted = highpass[0] * shelved + h1;
      h1 = highpass[1] * shelved - highpass[3] * weighted + h2;
      h2 = highpass[2] * shelved - highpass[4] * weighted;
      energy[frame] += weighted * weighted;
    }
  }
  let sum = 0,
    maximum = 0;
  for (let frame = 0; frame < frames; frame++) {
    sum += energy[frame];
    if (frame >= window) sum -= energy[frame - window];
    if (frame >= window - 1 && (frame - window + 1) % hop === 0)
      maximum = Math.max(maximum, sum / window);
  }
  return maximum > 0 ? -0.691 + 10 * Math.log10(maximum) : -Infinity;
}

export function sourceNormalization(buffer: AudioBuffer, authoredGain = 1): number {
  const loudness = maxMomentaryLoudness(
    Array.from({ length: buffer.numberOfChannels }, (_, channel) => buffer.getChannelData(channel)),
    buffer.sampleRate,
  );
  // Silent/muted material stays silent; normalization never undoes an authored gain.
  return loudness < -90 || authoredGain === 0
    ? 1
    : 10 ** ((SOURCE_LOUDNESS_DB - loudness) / 20) * authoredGain;
}
