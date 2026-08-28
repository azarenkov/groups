import React from "react";
import { useTranslation } from "react-i18next";
import Icon from "../rift/Icon";
import { userAPI } from "../../api/user";
import { useAuthStore } from "../../store/authStore";
import { ApiError } from "../../utils/apiError";

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
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
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

const messageOf = (err: unknown, fallback: string) =>
  err instanceof ApiError ? err.message || fallback : fallback;

export function ChangePasswordDialog({ onClose }: { onClose: () => void }) {
  const { t } = useTranslation("settings");
  const [current, setCurrent] = React.useState("");
  const [next, setNext] = React.useState("");
  const [repeat, setRepeat] = React.useState("");
  const [busy, setBusy] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);

  const submit = async () => {
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
      await userAPI.changePassword(current, next, useAuthStore.getState().tokens?.refreshToken);
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
      submitDisabled={!current || !next || !repeat}
      busy={busy}
      error={error}
    >
      <div className="m-field">
        <label>{t("account.currentPassword")}</label>
        <input type="password" autoFocus autoComplete="current-password" value={current} onChange={(e) => setCurrent(e.target.value)} />
      </div>
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
  const [email, setEmail] = React.useState("");
  const [code, setCode] = React.useState("");
  const [busy, setBusy] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);

  const request = async () => {
    setBusy(true);
    setError(null);
    try {
      await userAPI.requestEmailChange(password, email.trim());
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
      submitDisabled={!password || !email.includes("@")}
      busy={busy}
      error={error}
    >
      <div className="m-field">
        <label>{t("account.newEmail")}</label>
        <input type="email" autoFocus autoComplete="email" value={email} onChange={(e) => setEmail(e.target.value)} />
      </div>
      <div className="m-field">
        <label>{t("account.currentPassword")}</label>
        <input type="password" autoComplete="current-password" value={password} onChange={(e) => setPassword(e.target.value)} />
      </div>
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
  const [busy, setBusy] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);

  const submit = async () => {
    setBusy(true);
    setError(null);
    try {
      if (mode === "disable") await userAPI.disableAccount(password);
      else await userAPI.deleteAccount(password);
      onDone();
    } catch (err) {
      const apiErr = err instanceof ApiError ? err : null;
      setError(apiErr?.code === "INVALID_PASSWORD" ? t("account.wrongPassword") : messageOf(err, t("account.genericError")));
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
      submitDisabled={!password}
      danger
      busy={busy}
      error={error}
    >
      <div className="m-field">
        <label>{t("account.currentPassword")}</label>
        <input type="password" autoFocus autoComplete="current-password" value={password} onChange={(e) => setPassword(e.target.value)} />
      </div>
    </AccountDialog>
  );
}
