import React from "react";
import { useTranslation } from "react-i18next";

import { authAPI } from "../api/auth";
import { ApiError } from "../../../shared/lib/apiError";
import { CaptchaSolveError } from "./captcha";
import { isCaptchaError, useCaptcha } from "./useCaptcha";

export function useForgotPassword() {
  const { t } = useTranslation("auth");
  const captcha = useCaptcha();
  const { warmUp } = captcha;
  const [sending, setSending] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);

  React.useEffect(() => warmUp(), [warmUp]);

  const send = React.useCallback(
    async (email: string): Promise<boolean> => {
      setSending(true);
      setError(null);
      try {
        await authAPI.forgotPassword(email.trim(), await captcha.take());
        return true;
      } catch (err) {
        warmUp();
        if (err instanceof CaptchaSolveError || isCaptchaError(err)) setError(t("captchaFailed"));
        else if (err instanceof ApiError && err.code === "VALIDATION_FAILED") setError(t("resetEmailInvalid"));
        else setError(t("resetSendFailed"));
        return false;
      } finally {
        setSending(false);
      }
    },
    [captcha, warmUp, t],
  );

  return { send, sending, solving: captcha.solving, error };
}
