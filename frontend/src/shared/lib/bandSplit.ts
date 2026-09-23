export const DECIMATION = 3;
export const LOWPASS_TAPS = 49;

export function designLowpass(
  taps: number,
  cutoffHz: number,
  sampleRate: number,
): Float32Array {
  const coefficients = new Float32Array(taps);
  const middle = (taps - 1) / 2;
  const omega = (2 * Math.PI * cutoffHz) / sampleRate;
  let sum = 0;
  for (let i = 0; i < taps; i++) {
    const n = i - middle;
    const sinc = n === 0 ? omega / Math.PI : Math.sin(omega * n) / (Math.PI * n);
    const window =
      0.54 - 0.46 * Math.cos((2 * Math.PI * i) / (taps - 1));
    const value = sinc * window;
    coefficients[i] = value;
    sum += value;
  }
  for (let i = 0; i < taps; i++) coefficients[i] /= sum;
  return coefficients;
}

export function responseAt(
  coefficients: Float32Array,
  hz: number,
  sampleRate: number,
): number {
  const omega = (2 * Math.PI * hz) / sampleRate;
  let real = 0;
  let imaginary = 0;
  for (let i = 0; i < coefficients.length; i++) {
    real += coefficients[i] * Math.cos(-omega * i);
    imaginary += coefficients[i] * Math.sin(-omega * i);
  }
  return Math.sqrt(real * real + imaginary * imaginary);
}

export function firFilter(
  input: Float32Array,
  coefficients: Float32Array,
): Float32Array {
  const output = new Float32Array(input.length);
  for (let i = 0; i < input.length; i++) {
    let acc = 0;
    for (let k = 0; k < coefficients.length; k++) {
      const index = i - k;
      if (index < 0) break;
      acc += coefficients[k] * input[index];
    }
    output[i] = acc;
  }
  return output;
}

export const MODEL_LOOKBACK_FRAMES = 3;

export function bandReconstruction(
  input: Float32Array,
  coefficients: Float32Array,
  decimation: number,
  modelLatency = 0,
): Float32Array {
  const low = firFilter(input, coefficients);
  const up = new Float32Array(input.length);
  for (let i = 0; i * decimation < input.length; i++) {
    up[i * decimation] = low[i * decimation] * decimation;
  }
  const reference = firFilter(up, coefficients);
  const delay = coefficients.length - 1;
  const output = new Float32Array(input.length);
  for (let i = 0; i < input.length; i++) {
    const source = i - modelLatency;
    if (source < 0) continue;
    const delayed = source - delay >= 0 ? input[source - delay] : 0;
    output[i] = reference[source] + (delayed - reference[source]);
  }
  return output;
}
