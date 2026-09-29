import { webcrypto } from "crypto";
import { TextEncoder } from "util";

import { CaptchaSolveError, encodeSolution, sha256Hex, solveChallenge } from "../captcha";

beforeAll(() => {
  Object.defineProperty(globalThis, "crypto", { value: webcrypto, configurable: true });
  Object.defineProperty(globalThis, "TextEncoder", { value: TextEncoder, configurable: true });
});

const challengeFor = async (salt: string, number: number, maxnumber = 2000) => ({
  algorithm: "SHA-256",
  challenge: await sha256Hex(`${salt}${number}`),
  maxnumber,
  salt,
  signature: "sig",
});

describe("solveChallenge", () => {
  it("finds the secret number", async () => {
    const challenge = await challengeFor("abc?expires=1", 777);
    await expect(solveChallenge(challenge)).resolves.toBe(777);
  });

  it("finds numbers at the batch edges", async () => {
    await expect(solveChallenge(await challengeFor("s?expires=1", 0))).resolves.toBe(0);
    await expect(solveChallenge(await challengeFor("s?expires=1", 2000))).resolves.toBe(2000);
  });

  it("gives up past maxnumber", async () => {
    const challenge = await challengeFor("s?expires=1", 2500, 300);
    await expect(solveChallenge(challenge)).rejects.toEqual(new CaptchaSolveError("unsolved"));
  });

  it("stops when aborted", async () => {
    const controller = new AbortController();
    controller.abort();
    const challenge = await challengeFor("s?expires=1", 5);
    await expect(solveChallenge(challenge, controller.signal)).rejects.toEqual(
      new CaptchaSolveError("aborted"),
    );
  });
});

describe("encodeSolution", () => {
  it("encodes the ALTCHA payload as base64 JSON", async () => {
    const challenge = await challengeFor("s?expires=1", 5);
    const decoded = JSON.parse(Buffer.from(encodeSolution(challenge, 5), "base64").toString("utf8"));
    expect(decoded).toEqual({
      algorithm: "SHA-256",
      challenge: challenge.challenge,
      number: 5,
      salt: "s?expires=1",
      signature: "sig",
    });
  });
});
