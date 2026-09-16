import React from "react";
import { useTranslation } from "react-i18next";
import Icon from "../../../shared/ui/Icon";
import { userAPI, type Reauth } from "../../social/api/user";
import { authAPI } from "../../auth/api/auth";
import GoogleSignInButton from "../../auth/components/GoogleSignInButton";
import { useAuthStore } from "../../auth/store/authStore";
import { ApiError } from "../../../shared/lib/apiError";

interface DialogProps {
  title: string;
  subtitle?: string;
  onClose: () => void;
  onSubmit: () => void;
  submitLabel: string;
  submitDisabled?: boolean;
  danger?: boolean;
  busy?: boolean;
  error?: string | null;
  children: React.ReactNode;
}

function AccountDialog({
  title,
  subtitle,
  onClose,
  onSubmit,
  submitLabel,
  submitDisabled,
  danger,
  busy,
  error,
  children,
}: DialogProps) {
  const { t } = useTranslation("common");
  React.useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      e.stopImmediatePropagation();
      e.preventDefault();
      onClose();
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [onClose]);
  return (
    <div className="modal-overlay account-dialog-overlay" onMouseDown={onClose}>
      <form
        className="modal"
        onMouseDown={(e) => e.stopPropagation()}
        onSubmit={(e) => {
          e.preventDefault();
          if (!busy && !submitDisabled) onSubmit();
        }}
      >
        <div className="m-head">
          <h3>{title}</h3>
          <button type="button" className="close" onClick={onClose} title={t("close")}>
            <Icon name="x" size={14} />
          </button>
        </div>
        <div className="m-body">
          {subtitle && <div className="account-dialog-sub">{subtitle}</div>}
          {children}
          {error && <div className="s-error account-dialog-error">{error}</div>}
        </div>
        <div className="m-foot">
          <button type="button" onClick={onClose} disabled={busy}>
            {t("cancel")}
          </button>
          <button type="submit" className={danger ? "danger" : "primary"} disabled={busy || submitDisabled}>
            {submitLabel}
          </button>
        </div>
      </form>
    </div>
  );
}

function useGoogleLinked() {
  return useAuthStore((s) => s.user?.linkedProviders?.includes("google") ?? false);
}

function ReauthField({
  password,
  onPassword,
  onGoogleToken,
  googleToken,
}: {
  password: string;
  onPassword: (v: string) => void;
  onGoogleToken: (token: string) => void;
  googleToken: string | null;
}) {
  const { t } = useTranslation("settings");
  const googleLinked = useGoogleLinked();
  const [mode, setMode] = React.useState<"password" | "google">("password");
  if (!googleLinked) {
    return (
      <div className="m-field">
        <label>{t("account.currentPassword")}</label>
        <input type="password" autoComplete="current-password" value={password} onChange={(e) => onPassword(e.target.value)} />
      </div>
    );
  }
  return (
    <div className="m-field">
      <div className="reauth-switch">
        <button
          type="button"
          className={mode === "password" ? "on" : ""}
          onClick={() => {
            setMode("password");
            onGoogleToken("");
          }}
        >
          {t("account.reauthPassword")}
        </button>
        <button
          type="button"
          className={mode === "google" ? "on" : ""}
          onClick={() => {
            setMode("google");
            onPassword("");
          }}
        >
          {t("account.reauthGoogle")}
        </button>
      </div>
      {mode === "password" ? (
        <>
          <label>{t("account.currentPassword")}</label>
          <input type="password" autoComplete="current-password" value={password} onChange={(e) => onPassword(e.target.value)} />
        </>
      ) : googleToken ? (
        <div className="reauth-ok">
          <Icon name="check" size={13} />
          {t("account.googleConfirmed")}
        </div>
      ) : (
        <>
          <label>{t("account.reauthGoogleHint")}</label>
          <GoogleSignInButton onToken={onGoogleToken} text="continue_with" />
        </>
      )}
    </div>
  );
}

const reauthOf = (password: string, googleToken: string | null): Reauth | null =>
  googleToken ? { googleIdToken: googleToken } : password ? { password } : null;

const tokenOrNull = (token: string): string | null => (token ? token : null);

const messageOf = (err: unknown, fallback: string) =>
  err instanceof ApiError ? err.message || fallback : fallback;

export function ChangePasswordDialog({ onClose }: { onClose: () => void }) {
  const { t } = useTranslation("settings");
  const [current, setCurrent] = React.useState("");
  const [googleToken, setGoogleToken] = React.useState<string | null>(null);
  const [next, setNext] = React.useState("");
  const [repeat, setRepeat] = React.useState("");
  const [busy, setBusy] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);

  const submit = async () => {
    const reauth = reauthOf(current, googleToken);
    if (!reauth) return;
    if (next.length < 4 || next.length > 30) {
      setError(t("account.passwordLength"));
      return;
    }
    if (next !== repeat) {
      setError(t("account.passwordMismatch"));
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const tokens = await userAPI.changePassword(reauth, next);
      useAuthStore.getState().setTokens(tokens);
      const user = useAuthStore.getState().user;
      if (user) {
        useAuthStore.getState().setUser({ ...user, passwordChangedAt: new Date().toISOString() });
      }
      onClose();
    } catch (err) {
      const code = err instanceof ApiError ? err.code : undefined;
      setError(
        code === "INVALID_PASSWORD"
          ? t("account.wrongPassword")
          : code === "SAME_PASSWORD"
            ? t("account.samePassword")
            : messageOf(err, t("account.genericError")),
      );
    } finally {
      setBusy(false);
    }
  };

  return (
    <AccountDialog
      title={t("account.changePasswordTitle")}
      subtitle={t("account.changePasswordSub")}
      onClose={onClose}
      onSubmit={() => void submit()}
      submitLabel={t("actions.change")}
      submitDisabled={!reauthOf(current, googleToken) || !next || !repeat}
      busy={busy}
      error={error}
    >
      <ReauthField password={current} onPassword={setCurrent} googleToken={googleToken} onGoogleToken={(t) => setGoogleToken(tokenOrNull(t))} />
      <div className="m-field">
        <label>{t("account.newPassword")}</label>
        <input type="password" autoComplete="new-password" value={next} onChange={(e) => setNext(e.target.value)} />
      </div>
      <div className="m-field">
        <label>{t("account.repeatPassword")}</label>
        <input type="password" autoComplete="new-password" value={repeat} onChange={(e) => setRepeat(e.target.value)} />
      </div>
    </AccountDialog>
  );
}

export function ChangeEmailDialog({ onClose }: { onClose: () => void }) {
  const { t } = useTranslation("settings");
  const [step, setStep] = React.useState<"form" | "code">("form");
  const [password, setPassword] = React.useState("");
  const [googleToken, setGoogleToken] = React.useState<string | null>(null);
  const [email, setEmail] = React.useState("");
  const [code, setCode] = React.useState("");
  const [busy, setBusy] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);

  const request = async () => {
    const reauth = reauthOf(password, googleToken);
    if (!reauth) return;
    setBusy(true);
    setError(null);
    try {
      await userAPI.requestEmailChange(reauth, email.trim());
      setStep("code");
    } catch (err) {
      const apiErr = err instanceof ApiError ? err : null;
      setError(
        apiErr?.code === "INVALID_PASSWORD"
          ? t("account.wrongPassword")
          : apiErr?.code === "EMAIL_TAKEN"
            ? t("account.emailTaken")
            : apiErr?.code === "SAME_EMAIL"
              ? t("account.sameEmail")
              : messageOf(err, t("account.genericError")),
      );
    } finally {
      setBusy(false);
    }
  };

  const confirm = async () => {
    setBusy(true);
    setError(null);
    try {
      const saved = await userAPI.confirmEmailChange(code.trim());
      try {
        const fresh = await authAPI.refresh();
        useAuthStore.getState().setTokens(fresh);
      } catch (refreshErr) {
        console.warn("[account] token refresh after email change failed:", refreshErr);
      }
      const user = useAuthStore.getState().user;
      if (user) useAuthStore.getState().setUser({ ...user, email: saved, emailVerified: true });
      onClose();
    } catch (err) {
      const apiErr = err instanceof ApiError ? err : null;
      setError(
        apiErr?.code === "BAD_CODE"
          ? t("account.badCode")
          : apiErr?.code === "CODE_EXPIRED"
            ? t("account.codeExpired")
            : apiErr?.code === "TOO_MANY_ATTEMPTS"
              ? t("account.tooManyAttempts")
              : messageOf(err, t("account.genericError")),
      );
    } finally {
      setBusy(false);
    }
  };

  if (step === "code") {
    return (
      <AccountDialog
        title={t("account.confirmEmailTitle")}
        subtitle={t("account.confirmEmailSub", { email: email.trim() })}
        onClose={onClose}
        onSubmit={() => void confirm()}
        submitLabel={t("account.confirm")}
        submitDisabled={code.trim().length < 4}
        busy={busy}
        error={error}
      >
        <div className="m-field">
          <label>{t("account.verificationCode")}</label>
          <input
            autoFocus
            inputMode="numeric"
            autoComplete="one-time-code"
            maxLength={6}
            value={code}
            onChange={(e) => setCode(e.target.value.replace(/\D/g, ""))}
          />
        </div>
        <button type="button" className="account-dialog-link" disabled={busy} onClick={() => void request()}>
          {t("account.resendCode")}
        </button>
      </AccountDialog>
    );
  }

  return (
    <AccountDialog
      title={t("account.changeEmailTitle")}
      subtitle={t("account.changeEmailSub")}
      onClose={onClose}
      onSubmit={() => void request()}
      submitLabel={t("account.sendCode")}
      submitDisabled={!reauthOf(password, googleToken) || !email.includes("@")}
      busy={busy}
      error={error}
    >
      <div className="m-field">
        <label>{t("account.newEmail")}</label>
        <input type="email" autoFocus autoComplete="email" value={email} onChange={(e) => setEmail(e.target.value)} />
      </div>
      <ReauthField password={password} onPassword={setPassword} googleToken={googleToken} onGoogleToken={(t) => setGoogleToken(tokenOrNull(t))} />
    </AccountDialog>
  );
}

export function DangerPasswordDialog({
  mode,
  onClose,
  onDone,
}: {
  mode: "disable" | "delete";
  onClose: () => void;
  onDone: () => void;
}) {
  const { t } = useTranslation("settings");
  const [password, setPassword] = React.useState("");
  const [googleToken, setGoogleToken] = React.useState<string | null>(null);
  const [busy, setBusy] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);

  const submit = async () => {
    const reauth = reauthOf(password, googleToken);
    if (!reauth) return;
    setBusy(true);
    setError(null);
    try {
      if (mode === "disable") await userAPI.disableAccount(reauth);
      else await userAPI.deleteAccount(reauth);
      onDone();
    } catch (err) {
      const apiErr = err instanceof ApiError ? err : null;
      setError(
        apiErr?.code === "INVALID_PASSWORD"
          ? t("account.wrongPassword")
          : apiErr?.code === "OAUTH_NOT_LINKED"
            ? t("account.googleNotLinked")
            : messageOf(err, t("account.genericError")),
      );
    } finally {
      setBusy(false);
    }
  };

  return (
    <AccountDialog
      title={mode === "disable" ? t("account.disableTitle") : t("account.deleteTitle")}
      subtitle={mode === "disable" ? t("account.disableSub") : t("account.deleteSub")}
      onClose={onClose}
      onSubmit={() => void submit()}
      submitLabel={mode === "disable" ? t("actions.disable") : t("actions.delete")}
      submitDisabled={!reauthOf(password, googleToken)}
      danger
      busy={busy}
      error={error}
    >
      <ReauthField password={password} onPassword={setPassword} googleToken={googleToken} onGoogleToken={(t) => setGoogleToken(tokenOrNull(t))} />
    </AccountDialog>
  );
}
