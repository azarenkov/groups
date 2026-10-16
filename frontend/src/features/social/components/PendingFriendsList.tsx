import React from "react";
import { useTranslation } from "react-i18next";

import Icon from "../../../shared/ui/Icon";
import { UserAvatar } from "../../../shared/ui/Avatar";
import type { Friendship } from "../../../shared/types/friendship";
import { useProfileModalStore } from "../store/profileModalStore";
import { PENDING_STATUSES, type PendingStatus, type usePendingFriendships } from "../hooks/usePendingFriendships";

interface PendingFriendsListProps {
  pending: ReturnType<typeof usePendingFriendships>;
  onOpenProfile?: () => void;
}

const TITLE_KEY: Record<PendingStatus, string> = {
  pending_incoming: "pendingFriends.incoming",
  pending_outgoing: "pendingFriends.outgoing",
  declined: "pendingFriends.declinedTitle",
};

const PendingFriendsList: React.FC<PendingFriendsListProps> = ({ pending, onOpenProfile }) => {
  const { t } = useTranslation(["modals", "common", "profile"]);
  const { sections, busy, actionFailed, loadMore, retry, countFor, visible, accept, remove } = pending;

  const renderPeer = (f: Friendship) => (
    <button
      type="button"
      className="pf-peer"
      title={t("profile:viewProfile")}
      onClick={() => {
        onOpenProfile?.();
        useProfileModalStore.getState().open(f.friendId);
      }}
    >
      <UserAvatar userId={f.friendId} name={f.friendName} className="pf-ava" />
      <div className="pf-name">{f.friendName}</div>
    </button>
  );

  const actionButton = (f: Friendship, kind: "accept" | "decline", title: string, run: (id: string) => Promise<void>) => (
    <button
      type="button"
      className={"pf-btn " + kind}
      title={title}
      aria-label={title}
      disabled={busy.has(f.friendId)}
      onClick={() => void run(f.friendId)}
    >
      <Icon name={kind === "accept" ? "check" : "x"} size={14} />
    </button>
  );

  const rowExtras = (status: PendingStatus, f: Friendship) => {
    if (status === "pending_incoming") {
      return (
        <>
          {actionButton(f, "accept", t("pendingFriends.accept"), accept)}
          {actionButton(f, "decline", t("pendingFriends.decline"), remove)}
        </>
      );
    }
    if (status === "pending_outgoing") {
      return (
        <>
          <span className="pf-tag">{t("pendingFriends.sent")}</span>
          {actionButton(f, "decline", t("pendingFriends.cancel"), remove)}
        </>
      );
    }
    return (
      <>
        <span className="pf-tag pf-tag-danger">{t("pendingFriends.declined")}</span>
        {actionButton(f, "decline", t("pendingFriends.dismiss"), remove)}
      </>
    );
  };

  const shown = PENDING_STATUSES.filter(visible);
  if (shown.length === 0) return null;

  return (
    <div className="pf-list">
      {actionFailed && <div className="m-error">{t("pendingFriends.actionFailed")}</div>}
      {shown.map((status) => {
        const section = sections[status];
        return (
          <div key={status} className="pf-section">
            <div className="pf-section-title">
              {t(TITLE_KEY[status])} · {countFor(status)}
            </div>
            {section.rows.map((f) => (
              <div key={f.friendId} className="pf-row">
                {renderPeer(f)}
                {rowExtras(status, f)}
              </div>
            ))}
            {section.error && (
              <div className="m-error">
                {t("pendingFriends.loadFailed")}
                <button type="button" className="sb-load-more" disabled={section.loading} onClick={() => void retry(status)}>
                  {t("common:retry")}
                </button>
              </div>
            )}
            {section.cursor && !section.error && (
              <button type="button" className="sb-load-more" disabled={section.loading} onClick={() => void loadMore(status)}>
                {t("pendingFriends.loadMore")}
              </button>
            )}
          </div>
        );
      })}
    </div>
  );
};

export default PendingFriendsList;
