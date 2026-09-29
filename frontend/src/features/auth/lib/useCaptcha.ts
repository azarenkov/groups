import React from "react";

import { authAPI } from "../api/auth";
import { ApiError } from "../../../shared/lib/apiError";
import { CAPTCHA_ERROR_CODES, encodeSolution, solveChallenge } from "./captcha";

const MAX_AGE_MS = 4 * 60 * 1000;

interface Prepared {
  promise: Promise<string | null>;
  createdAt: number;
  controller: AbortController;
}

export const isCaptchaError = (err: unknown) =>
  err instanceof ApiError && !!err.code && CAPTCHA_ERROR_CODES.includes(err.code);

export function useCaptcha() {
  const preparedRef = React.useRef<Prepared | null>(null);
  const [solving, setSolving] = React.useState(false);

  const prepare = React.useCallback((): Promise<string | null> => {
    const current = preparedRef.current;
    if (current && Date.now() - current.createdAt < MAX_AGE_MS) return current.promise;
    current?.controller.abort();
    const controller = new AbortController();
    const promise = authAPI.captchaChallenge().then(async (challenge) => {
      if (!challenge) return null;
      return encodeSolution(challenge, await solveChallenge(challenge, controller.signal));
    });
    promise.catch(() => {
      if (preparedRef.current?.promise === promise) preparedRef.current = null;
    });
    preparedRef.current = { promise, createdAt: Date.now(), controller };
    return promise;
  }, []);

  const warmUp = React.useCallback(() => {
    prepare().catch(() => {});
  }, [prepare]);

  const take = React.useCallback(async (): Promise<string | null> => {
    const promise = prepare();
    preparedRef.current = null;
    setSolving(true);
    try {
      return await promise;
    } finally {
      setSolving(false);
    }
  }, [prepare]);

  React.useEffect(() => () => preparedRef.current?.controller.abort(), []);

  return { warmUp, take, solving };
}
