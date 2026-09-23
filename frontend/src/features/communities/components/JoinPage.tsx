import React, { useEffect, useState } from "react";
import { Navigate, useNavigate, useParams } from "react-router-dom";
import { useTranslation } from "react-i18next";
import Icon, { Mark } from "../../../shared/ui/Icon";
import LanguageSwitcher from "../../../shared/ui/LanguageSwitcher";
import InvitePreviewCard, { inviteStatusMessage } from "./InvitePreviewCard";
import { communityAPI } from "../api/community";
import { useAuthStore } from "../../auth/store/authStore";
import type { InvitePreview } from "../../../shared/types/community";
import { appDeepLink, extractInviteToken } from "../../../shared/lib/inviteLinks";
import { isAccessTokenVerified } from "../../../shared/lib/jwt";

const JoinPage: React.FC = () => {
  const { token: rawToken = "" } = useParams();
  const navigate = useNavigate();
  const { t } = useTranslation(["modals", "common"]);
  const { isAuthenticated, tokens } = useAuthStore();
  const hasUnverifiedSession =
    !!tokens?.accessToken && !isAccessTokenVerified(tokens.accessToken);
  const token = extractInviteToken(rawToken);
  const [preview, setPreview] = useState<InvitePreview | null>(null);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    setPreview(null);
    setFailed(false);
    if (!token) return;
    let cancelled = false;
    communityAPI
      .previewInvite(token)
      .then((res) => {
        if (!cancelled) setPreview(res);
      })
      .catch(() => {
        if (!cancelled) setFailed(true);
      });
    return () => {
      cancelled = true;
    };
  }, [token]);

  if (token && isAuthenticated && !hasUnverifiedSession) {
    return <Navigate to={`/app?join=${encodeURIComponent(token)}`} replace />;
  }

  const next = token ? `?next=${encodeURIComponent(`/join/${token}`)}` : "";
  const isDesktop = typeof window !== "undefined" && !!window.electron;
  const unavailable = !token || failed || (preview !== null && preview.status !== "active");
  const message = !token || failed
    ? t("communityModal.inviteNotFound")
    : preview && preview.status !== "active"
      ? inviteStatusMessage(preview.status, t)
      : null;

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
        <div className="auth-card join-card">
          <h1>{t("communityModal.joinPageTitle")}</h1>
          <p className="sub">{t("communityModal.joinPageSub")}</p>
          {preview?.community && <InvitePreviewCard community={preview.community} />}
          {!preview && !unavailable && (
            <div className="join-loading">
              <span className="loading-spinner" />
            </div>
          )}
          {message && (
            <div className="srvm-hint join-status">
              <Icon name="info" size={15} />
              <span>{message}</span>
            </div>
          )}
          <div className="join-actions">
            {!unavailable && (
              <>
                <button
                  type="button"
                  className="srvm-btn primary"
                  onClick={() => navigate(`/login${next}`)}
                >
                  {t("communityModal.signInToJoin")}
                </button>
                <button
                  type="button"
                  className="srvm-btn"
                  onClick={() => navigate(`/register${next}`)}
                >
                  {t("communityModal.createAccount")}
                </button>
                {!isDesktop && token && (
                  <a className="srvm-btn join-open-app" href={appDeepLink(token)}>
                    {t("communityModal.openInApp")}
                  </a>
                )}
              </>
            )}
            {unavailable && (
              <button type="button" className="srvm-btn" onClick={() => navigate("/login")}>
                {t("communityModal.back")}
              </button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};

export default JoinPage;
