import React from "react";
import { useTranslation } from "react-i18next";

import Icon from "../../../shared/ui/Icon";
import LoadingDots from "../../../shared/ui/LoadingDots";
import { CommunityLogo, UserAvatar } from "../../../shared/ui/Avatar";
import { colorForId } from "../../../shared/lib/avatar";
import { formatListTime } from "../../dm/lib/directOrder";
import { useProfileModalStore } from "../../social/store/profileModalStore";
import PendingFriendsList from "../../social/components/PendingFriendsList";
import { usePendingFriendships } from "../../social/hooks/usePendingFriendships";
import { inboxAPI, type InboxItem } from "../api/inbox";
import { useInboxItems, useInboxUnread } from "../hooks/useInbox";
import { markReadLocally, removeLocally } from "../lib/inbox";

interface InboxMenuProps {
  onOpenCommunity: (communityId: string) => Promise<void> | void;
}

const MESSAGE_KEY: Record<InboxItem["kind"], string> = {
  community_invitation: "inboxInvitation",
  removed_from_community: "inboxRemoved",
  friend_request_received: "inboxFriendRequest",
  friend_request_accepted: "inboxFriendAccepted",
};

const InboxMenu: React.FC<InboxMenuProps> = ({ onOpenCommunity }) => {
  const { t, i18n } = useTranslation("sidebar");
  const [open, setOpen] = React.useState(false);
  const [fresh, setFresh] = React.useState<ReadonlySet<string>>(new Set());
  const [busyId, setBusyId] = React.useState<string | null>(null);
  const [failedId, setFailedId] = React.useState<string | null>(null);
  const menuRef = React.useRef<HTMLDivElement>(null);
  const attempted = React.useRef<Set<string>>(new Set());
  const unread = useInboxUnread();
  const inbox = useInboxItems(open);
  const pending = usePendingFriendships(open);
  const now = new Date();

  React.useEffect(() => {
    if (!open) return;
    const onDoc = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    document.addEventListener("mousedown", onDoc);
    window.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDoc);
      window.removeEventListener("keydown", onKey);
    };
  }, [open]);

  const markAllRead = React.useCallback(() => {
    if (!inbox.userId) return;
    const rollback = markReadLocally(inbox.userId);
    inboxAPI.markRead().catch(() => rollback());
  }, [inbox.userId]);

  React.useEffect(() => {
    if (!open || inbox.loading || !inbox.userId) return;
    const shown = inbox.items.filter((item) => !item.read && !attempted.current.has(item.id)).map((item) => item.id);
    if (shown.length === 0) return;
    shown.forEach((id) => attempted.current.add(id));
    setFresh((prev) => new Set([...Array.from(prev), ...shown]));
    const rollback = markReadLocally(inbox.userId, new Set(shown));
    inboxAPI.markRead(shown).catch(() => rollback());
  }, [open, inbox.loading, inbox.items, inbox.userId]);

  React.useEffect(() => {
    if (open) return;
    setFresh(new Set());
    attempted.current = new Set();
  }, [open]);

  const toggle = () => setOpen((value) => !value);

  const run = async (item: InboxItem, action: () => Promise<void>) => {
    if (!inbox.userId) return;
    setBusyId(item.id);
    setFailedId(null);
    const rollback = removeLocally(inbox.userId, item.id);
    try {
      await action();
    } catch {
      rollback();
      setFailedId(item.id);
    } finally {
      setBusyId(null);
    }
  };

  const actionsFor = (item: InboxItem) => {
    if (item.kind === "community_invitation" && item.invitationId) {
      const invitationId = item.invitationId;
      return [
        {
          label: t("inboxAccept"),
          primary: true,
          run: () =>
            run(item, async () => {
              const { communityId } = await inboxAPI.acceptInvitation(invitationId);
              setOpen(false);
              await onOpenCommunity(communityId);
            }),
        },
        { label: t("inboxDecline"), primary: false, run: () => run(item, () => inboxAPI.declineInvitation(invitationId)) },
      ];
    }
    return [];
  };

  const badge = unread > 99 ? "99+" : String(unread);

  return (
    <div className="inbox-menu" ref={menuRef}>
      <button
        type="button"
        className={"inbox-trigger" + (open ? " on" : "")}
        title={t("inboxTitle")}
        aria-label={unread > 0 ? t("inboxUnreadLabel", { count: unread }) : t("inboxTitle")}
        aria-haspopup="dialog"
        aria-expanded={open}
        onClick={toggle}
      >
        <Icon name="mail" size={18} />
        {unread > 0 && <span className="inbox-badge">{badge}</span>}
      </button>
      {open && (
        <div className="inbox-dropdown" role="dialog" aria-label={t("inboxTitle")}>
          <div className="inbox-head">
            <span className="inbox-title">{t("inboxTitle")}</span>
            {unread > 0 && (
              <button type="button" className="inbox-read-all" onClick={markAllRead}>
                {t("inboxReadAll")}
              </button>
            )}
          </div>
          <div className="inbox-list">
            <PendingFriendsList pending={pending} onOpenProfile={() => setOpen(false)} />
            {inbox.loading ? (
              <div className="inbox-note">
                <LoadingDots />
              </div>
            ) : inbox.failed && inbox.items.length === 0 ? (
              <button type="button" className="inbox-note inbox-retry" onClick={inbox.retry}>
                {t("inboxFailed")}
              </button>
            ) : inbox.items.length === 0 ? (
              <div className="inbox-note">{t("inboxEmpty")}</div>
            ) : (
              inbox.items.map((item) => {
                const actions = actionsFor(item);
                return (
                  <div key={item.id} className={"inbox-item" + (fresh.has(item.id) || !item.read ? " unread" : "")}>
                    <button
                      type="button"
                      className="inbox-item-main"
                      onClick={() => {
                        setOpen(false);
                        useProfileModalStore.getState().open(item.actor.id);
                      }}
                    >
                      {item.community ? (
                        <CommunityLogo
                          communityId={item.community.id}
                          name={item.community.name}
                          className="inbox-item-ava"
                          style={{ background: colorForId(item.community.id) }}
                        />
                      ) : (
                        <UserAvatar
                          userId={item.actor.id}
                          name={item.actor.name}
                          className="inbox-item-ava round"
                          style={{ background: colorForId(item.actor.id) }}
                        />
                      )}
                      <span className="inbox-item-text">
                        <span className="inbox-item-message">
                          {t(MESSAGE_KEY[item.kind], { actor: item.actor.name, community: item.community?.name ?? "" })}
                        </span>
                        <span className="inbox-item-time">
                          {failedId === item.id ? t("inboxActionFailed") : formatListTime(item.createdAt, now, i18n.language)}
                        </span>
                      </span>
                    </button>
                    {actions.length > 0 && (
                      <div className="inbox-item-actions">
                        {actions.map((action) => (
                          <button
                            key={action.label}
                            type="button"
                            className={"inbox-action" + (action.primary ? " primary" : "")}
                            disabled={busyId === item.id}
                            onClick={() => void action.run()}
                          >
                            {action.label}
                          </button>
                        ))}
                      </div>
                    )}
                  </div>
                );
              })
            )}
            {inbox.hasMore && (
              <button type="button" className="inbox-more" disabled={inbox.loadingMore} onClick={inbox.loadMore}>
                {inbox.loadingMore ? <LoadingDots /> : t("inboxMore")}
              </button>
            )}
          </div>
        </div>
      )}
    </div>
  );
};

export default InboxMenu;
