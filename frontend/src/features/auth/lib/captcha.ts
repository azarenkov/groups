export interface AltchaChallenge {
  algorithm: string;
  challenge: string;
  maxnumber: number;
  salt: string;
  signature: string;
}

export const CAPTCHA_ERROR_CODES = ["CAPTCHA_REQUIRED", "CAPTCHA_INVALID", "CAPTCHA_EXPIRED"];

export class CaptchaSolveError extends Error {
  constructor(readonly reason: "unsupported" | "unsolved" | "aborted") {
    super(reason);
  }
}

const BATCH = 256;

const toHex = (buffer: ArrayBuffer) => {
  const bytes = new Uint8Array(buffer);
  let out = "";
  for (let i = 0; i < bytes.length; i += 1) out += bytes[i].toString(16).padStart(2, "0");
  return out;
};

export const sha256Hex = async (text: string): Promise<string> => {
  const subtle = globalThis.crypto?.subtle;
  if (!subtle) throw new CaptchaSolveError("unsupported");
  return toHex(await subtle.digest("SHA-256", new TextEncoder().encode(text)));
};

export const solveChallenge = async (challenge: AltchaChallenge, signal?: AbortSignal): Promise<number> => {
  if (challenge.algorithm !== "SHA-256") throw new CaptchaSolveError("unsupported");
  const target = challenge.challenge.toLowerCase();
  for (let start = 0; start <= challenge.maxnumber; start += BATCH) {
    if (signal?.aborted) throw new CaptchaSolveError("aborted");
    const end = Math.min(challenge.maxnumber, start + BATCH - 1);
    const numbers = Array.from({ length: end - start + 1 }, (_, i) => start + i);
    const hashes = await Promise.all(numbers.map((n) => sha256Hex(`${challenge.salt}${n}`)));
    const hit = hashes.indexOf(target);
    if (hit >= 0) return numbers[hit];
  }
  throw new CaptchaSolveError("unsolved");
};

const toBase64 = (text: string) => {
  const bytes = new TextEncoder().encode(text);
  let binary = "";
  bytes.forEach((b) => {
    binary += String.fromCharCode(b);
  });
  return btoa(binary);
};

export const encodeSolution = (challenge: AltchaChallenge, number: number) =>
  toBase64(
    JSON.stringify({
      algorithm: challenge.algorithm,
      challenge: challenge.challenge,
      number,
      salt: challenge.salt,
      signature: challenge.signature,
    }),
  );
