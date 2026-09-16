import { authRedirectTarget } from "../../../shared/lib/inviteLinks";
import React, { useEffect, useState } from "react";
import { useLocation, useNavigate } from "react-router-dom";
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
  defaultMode?: "signin" | "signup";
}

const AuthPage: React.FC<AuthPageProps> = ({ defaultMode = "signin" }) => {
  const navigate = useNavigate();
  const { search } = useLocation();
  const tokens = useAuthStore((s) => s.tokens);
  const storedUser = useAuthStore((s) => s.user);
  const hasUnverifiedSession =
    !!tokens?.accessToken && !isAccessTokenVerified(tokens.accessToken);

  const [mode, setMode] = useState<Mode>(
    hasUnverifiedSession ? "verify-email" : defaultMode,
  );
  const [email, setEmail] = useState(storedUser?.email ?? "");

  useEffect(() => {
    if (hasUnverifiedSession) {
      setMode("verify-email");
      if (storedUser?.email) setEmail(storedUser.email);
    }
  }, [hasUnverifiedSession, storedUser?.email]);

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
              onSubmit={() => setMode("reset-sent")}
              onBack={() => setMode("signin")}
            />
          )}
          {mode === "reset-sent" && (
            <ResetSent
              email={email}
              onResend={() => {}}
              onOpenLink={() => setMode("reset-password")}
              onBack={() => setMode("signin")}
            />
          )}
          {mode === "reset-password" && (
            <ResetPassword
              onSubmit={() => setMode("reset-success")}
              onBack={() => setMode("signin")}
            />
          )}
          {mode === "reset-success" && (
            <ResetSuccess onContinue={() => setMode("signin")} />
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
