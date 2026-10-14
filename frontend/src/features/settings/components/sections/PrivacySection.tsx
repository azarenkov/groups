import React from "react";
import { useTranslation } from "react-i18next";
import { UserAvatar } from "../../../../shared/ui/Avatar";
import { homeAPI, type PrivacySettings } from "../../../home/api/home";
import { queryClient, queryKeys } from "../../../../shared/query/queryClient";
import { useQuery } from "@tanstack/react-query";
import { useAuthStore } from "../../../auth/store/authStore";
import { useBlockStore } from "../../../social/store/blockStore";

import { SettingsRow, Switch } from "../controls";
import LoadingDots from "../../../../shared/ui/LoadingDots";

export function PrivacySection() {
  const { t } = useTranslation("settings");
  const privacyQuery = useQuery(
    { queryKey: queryKeys.privacySettings(), queryFn: () => homeAPI.privacy() },
    queryClient,
  );
  const settings: PrivacySettings | null = privacyQuery.data ?? null;
  const failed = privacyQuery.isError && !settings;
  const [busy, setBusy] = React.useState(false);
  const apply = (patch: Partial<PrivacySettings>) => {
    if (busy) return;
    setBusy(true);
    const session = useAuthStore.getState().user?.id;
    homeAPI
      .updatePrivacy(patch)
      .then(async (saved) => {
        if (!session || useAuthStore.getState().user?.id !== session) return;
        await queryClient.cancelQueries({ queryKey: queryKeys.privacySettings() });
        if (useAuthStore.getState().user?.id !== session) return;
        queryClient.setQueryData(queryKeys.privacySettings(), saved);
      })
      .catch((err) => console.warn("[privacy] update failed:", err))
      .finally(() => setBusy(false));
  };
  return (
    <>
      <h1>{t("privacy.title")}</h1>
      <p className="lead">{t("privacy.lead")}</p>

      <div className="s-section">
        <h2>{t("privacy.directMessages")}</h2>
        {failed ? (
          <div className="s-error">{t("privacy.loadFailed")}</div>
        ) : !settings ? (
          <div className="s-row"><LoadingDots /></div>
        ) : (
          <>
            <SettingsRow label={t("privacy.whoCanDm")} hint={t("privacy.whoCanDmHint")}>
              <div className="s-seg">
                <button disabled={busy} className={settings.dmPolicy === "all" ? "on" : ""} onClick={() => apply({ dmPolicy: "all" })}>{t("privacy.everyone")}</button>
                <button disabled={busy} className={settings.dmPolicy === "friends" ? "on" : ""} onClick={() => apply({ dmPolicy: "friends" })}>{t("privacy.friends")}</button>
                <button disabled={busy} className={settings.dmPolicy === "no_one" ? "on" : ""} onClick={() => apply({ dmPolicy: "no_one" })}>{t("privacy.noOne")}</button>
              </div>
            </SettingsRow>
            <SettingsRow label={t("privacy.allowRequests")} hint={t("privacy.allowRequestsHint")}>
              <Switch on={settings.acceptFriendRequests} disabled={busy} onClick={() => apply({ acceptFriendRequests: !settings.acceptFriendRequests })} />
            </SettingsRow>
          </>
        )}
      </div>
      {!failed && settings && (
        <div className="s-section">
          <h2>{t("privacy.calls")}</h2>
          <SettingsRow label={t("privacy.whoCanCall")} hint={t("privacy.whoCanCallHint")}>
            <div className="s-seg">
              <button disabled={busy} className={settings.callPolicy === "all" ? "on" : ""} onClick={() => apply({ callPolicy: "all" })}>{t("privacy.everyone")}</button>
              <button disabled={busy} className={settings.callPolicy === "friends" ? "on" : ""} onClick={() => apply({ callPolicy: "friends" })}>{t("privacy.friends")}</button>
              <button disabled={busy} className={settings.callPolicy === "no_one" ? "on" : ""} onClick={() => apply({ callPolicy: "no_one" })}>{t("privacy.noOne")}</button>
            </div>
          </SettingsRow>
        </div>
      )}
      {!failed && settings && (
        <div className="s-section">
          <h2>{t("privacy.profile")}</h2>
          <SettingsRow label={t("privacy.showLocalTime")} hint={t("privacy.showLocalTimeHint")}>
            <Switch on={settings.showLocalTime} disabled={busy} onClick={() => apply({ showLocalTime: !settings.showLocalTime })} />
          </SettingsRow>
        </div>
      )}
      <BlockedUsersSection />
    </>
  );
}

function BlockedUsersSection() {
  const { t } = useTranslation(["settings", "common"]);
  const items = useBlockStore((s) => s.page);
  const loaded = useBlockStore((s) => s.pageLoaded);
  const pageError = useBlockStore((s) => s.pageError);
  const hasMore = useBlockStore((s) => !!s.pageCursor);
  const loadingMore = useBlockStore((s) => s.pageLoading);
  const [busy, setBusy] = React.useState<string | null>(null);
  React.useEffect(() => {
    void useBlockStore.getState().fetchPage();
  }, []);
  const unblock = (userId: string) => {
    setBusy(userId);
    useBlockStore
      .getState()
      .unblock(userId)
      .catch((err) => console.warn("[privacy] unblock failed:", err))
      .finally(() => setBusy(null));
  };
  return (
    <div className="s-section">
      <h2>{t("privacy.blockedUsers")}</h2>
      <p className="s-hint">{t("privacy.blockedUsersHint")}</p>
      {!loaded && items.length === 0 ? (
        pageError ? (
          <div className="s-row">
            <span className="s-muted">{t("privacy.blockedLoadFailed")}</span>
            <button
              type="button"
              className="s-btn secondary"
              onClick={() => void useBlockStore.getState().fetchPage()}
            >
              {t("common:retry")}
            </button>
          </div>
        ) : (
          <div className="s-row"><LoadingDots /></div>
        )
      ) : items.length === 0 ? (
        <div className="s-row s-muted">{t("privacy.noBlockedUsers")}</div>
      ) : (
        items.map((b) => (
          <div className="s-row s-blocked-row" key={b.userId}>
            <UserAvatar userId={b.userId} name={b.name || "?"} className="s-blocked-ava" />
            <span className="s-blocked-name">{b.name || b.userId}</span>
            <button
              type="button"
              className="s-btn secondary"
              disabled={busy === b.userId}
              onClick={() => unblock(b.userId)}
            >
              {t("privacy.unblock")}
            </button>
          </div>
        ))
      )}
      {hasMore && (
        <div className="s-row">
          <button
            type="button"
            className="s-btn secondary"
            disabled={loadingMore}
            onClick={() => void useBlockStore.getState().loadMorePage()}
          >
            {t("privacy.loadMore")}
          </button>
        </div>
      )}
    </div>
  );
}
