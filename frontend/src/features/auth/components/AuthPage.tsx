import { authRedirectTarget, resetPasswordDeepLink } from "../../../shared/lib/inviteLinks";
import React, { useEffect, useState } from "react";
import { useLocation, useNavigate } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { useAuthStore } from "../store/authStore";
import { isAccessTokenVerified } from "../../../shared/lib/jwt";
import { Mark } from "../../../shared/ui/Icon";
import LanguageSwitcher from "../../../shared/ui/LanguageSwitcher";
import "../../../styles/rift.css";

import { AuthLegal } from "./steps/AuthLegal";
import { AuthTabs, SignInUp } from "./steps/SignInUp";
import { Forgot } from "./steps/Forgot";
import { ResetSent } from "./steps/ResetSent";
import { ResetPassword } from "./steps/ResetPassword";
import { ResetSuccess } from "./steps/ResetSuccess";
import { VerifyEmail } from "./steps/VerifyEmail";
import { TwoFa } from "./steps/TwoFa";
type Mode =
  | "signin"
  | "signup"
  | "forgot"
  | "reset-sent"
  | "reset-password"
  | "reset-success"
  | "verify-email"
  | "2fa";

interface AuthPageProps {
  defaultMode?: "signin" | "signup" | "reset-password";
}

const AuthPage: React.FC<AuthPageProps> = ({ defaultMode = "signin" }) => {
  const navigate = useNavigate();
  const { t } = useTranslation("auth");
  const { search } = useLocation();
  const tokens = useAuthStore((s) => s.tokens);
  const storedUser = useAuthStore((s) => s.user);
  const hasUnverifiedSession =
    !!tokens?.accessToken && !isAccessTokenVerified(tokens.accessToken);

  const [mode, setMode] = useState<Mode>(
    hasUnverifiedSession && defaultMode !== "reset-password" ? "verify-email" : defaultMode,
  );
  const [email, setEmail] = useState(storedUser?.email ?? "");
  const [resetToken, setResetToken] = useState(() =>
    defaultMode === "reset-password" ? new URLSearchParams(search).get("token") : null,
  );
  const resetFlow = mode === "reset-password" || mode === "reset-success";
  const isDesktop = typeof window !== "undefined" && !!window.electron;

  useEffect(() => {
    if (defaultMode !== "reset-password") return;
    const token = new URLSearchParams(search).get("token");
    if (token === null) return;
    setResetToken(token);
    setMode("reset-password");
  }, [defaultMode, search]);

  useEffect(() => {
    if (hasUnverifiedSession && !resetFlow) {
      setMode("verify-email");
      if (storedUser?.email) setEmail(storedUser.email);
    }
  }, [hasUnverifiedSession, storedUser?.email, resetFlow]);

  const onSignedIn = () => navigate(authRedirectTarget(search));
  const [pendingAvatar, setPendingAvatar] = useState<File | null>(null);

  const switchTab = (m: "signin" | "signup") => {
    setMode(m);
    navigate(`${m === "signin" ? "/login" : "/register"}${search}`, { replace: true });
  };

  const showTabs = mode === "signin" || mode === "signup";

  return (
    <div className="auth-page">
      <div className="auth-topbar">
        <span className="auth-brand">
          <Mark size={22} />
          <span>Rift</span>
        </span>
        <LanguageSwitcher />
      </div>

      <div className="auth-form-wrap">
        <div className="auth-card">
          {showTabs && <AuthTabs mode={mode} onSwitch={switchTab} />}

          {(mode === "signin" || mode === "signup") && (
            <SignInUp
              mode={mode}
              email={email}
              onEmailChange={setEmail}
              onSwitchMode={switchTab}
              onSignedIn={onSignedIn}
              go2fa={() => setMode("2fa")}
              goVerify={() => setMode("verify-email")}
              goForgot={() => setMode("forgot")}
              onAvatarPicked={setPendingAvatar}
            />
          )}
          {mode === "forgot" && (
            <Forgot
              email={email}
              onEmailChange={setEmail}
              onSent={() => setMode("reset-sent")}
              onBack={() => switchTab("signin")}
            />
          )}
          {mode === "reset-sent" && (
            <ResetSent email={email} onBack={() => switchTab("signin")} />
          )}
          {mode === "reset-password" && (
            <ResetPassword
              key={resetToken ?? ""}
              token={resetToken}
              onDone={() => {
                if (useAuthStore.getState().tokens) useAuthStore.getState().logout();
                navigate("/reset-password", { replace: true });
                setMode("reset-success");
              }}
              onRequestNew={() => setMode("forgot")}
              onBack={() => switchTab("signin")}
            />
          )}
          {mode === "reset-password" && resetToken && !isDesktop && (
            <div className="auth-foot">
              <a className="auth-link" href={resetPasswordDeepLink(resetToken)}>
                {t("openResetInApp")}
              </a>
            </div>
          )}
          {mode === "reset-success" && (
            <ResetSuccess onContinue={() => switchTab("signin")} />
          )}
          {mode === "verify-email" && (
            <VerifyEmail
              pendingAvatar={pendingAvatar}
              email={email}
              onVerified={onSignedIn}
              onBack={() => setMode("signup")}
              onEmailChanged={setEmail}
            />
          )}
          {mode === "2fa" && (
            <TwoFa onVerified={onSignedIn} onBack={() => setMode("signin")} />
          )}
        </div>
      </div>

      <AuthLegal />
    </div>
  );
};

export default AuthPage;
