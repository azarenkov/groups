declare module "fft.js" {
  export default class FFT {
    constructor(size: number);
    size: number;
    createComplexArray(): number[];
    fromComplexArray(input: ArrayLike<number>, out?: number[]): number[];
    toComplexArray(input: ArrayLike<number>, out?: number[]): number[];
    completeSpectrum(spectrum: number[]): void;
    transform(out: number[], input: ArrayLike<number>): void;
    realTransform(out: number[], input: ArrayLike<number>): void;
    inverseTransform(out: number[], input: ArrayLike<number>): void;
  }
}
