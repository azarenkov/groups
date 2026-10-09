import React from "react";
import { useQuery } from "@tanstack/react-query";
import { queryClient, queryKeys } from "../../../shared/query/queryClient";
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
  const previewQuery = useQuery(
    {
      queryKey: queryKeys.invitePreview(token ?? ""),
      queryFn: () => communityAPI.previewInvite(token as string),
      enabled: !!token,
    },
    queryClient,
  );
  const preview: InvitePreview | null = token ? (previewQuery.data ?? null) : null;
  const failed = !!token && previewQuery.isError && !previewQuery.data;

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
        <LanguageSwitcher />
      </div>

      <div className="auth-form-wrap">
        <div className="auth-card join-card">
          <div className="auth-mark">
            <Mark size={52} />
          </div>
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
