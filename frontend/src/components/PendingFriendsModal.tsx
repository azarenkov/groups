import React, { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import Icon from "./rift/Icon";
import { useFriendshipStore } from "../store/friendshipStore";
import { friendshipAPI } from "../api/friendship";
import type { Friendship, FriendshipStatus } from "../types/friendship";
import { UserAvatar } from "./rift/Avatar";
import { useProfileModalStore } from "../store/profileModalStore";

interface PendingFriendsModalProps {
  isOpen: boolean;
  onClose: () => void;
  onAddFriend?: () => void;
}

type PendingStatus = "pending_incoming" | "pending_outgoing" | "declined";

interface Section {
  rows: Friendship[];
  cursor: string | null;
  loading: boolean;
}

const PENDING_STATUSES: PendingStatus[] = ["pending_incoming", "pending_outgoing", "declined"];
const PAGE_SIZE = 50;

const emptySections = (): Record<PendingStatus, Section> => ({
  pending_incoming: { rows: [], cursor: null, loading: false },
  pending_outgoing: { rows: [], cursor: null, loading: false },
  declined: { rows: [], cursor: null, loading: false },
});

const PendingFriendsModal: React.FC<PendingFriendsModalProps> = ({
  isOpen,
  onClose,
  onAddFriend,
}) => {
  const { t } = useTranslation(["modals", "common"]);
  const statusById = useFriendshipStore((s) => s.statusById);
  const accept = useFriendshipStore((s) => s.accept);
  const remove = useFriendshipStore((s) => s.remove);
  const [sections, setSections] = useState<Record<PendingStatus, Section>>(emptySections);
  const [busy, setBusy] = useState<Set<string>>(() => new Set());
  const [error, setError] = useState<string | null>(null);
  const loadSeq = useRef(0);

  const loadFirstPages = useCallback(async () => {
    const seq = ++loadSeq.current;
    setSections((prev) => {
      const next = { ...prev };
      for (const status of PENDING_STATUSES) next[status] = { ...prev[status], loading: true };
      return next;
    });
    await Promise.all(
      PENDING_STATUSES.map(async (status) => {
        try {
          const page = await friendshipAPI.list({ status, limit: PAGE_SIZE });
          if (seq !== loadSeq.current) return;
          setSections((prev) => ({
            ...prev,
            [status]: { rows: page.friends, cursor: page.nextCursor, loading: false },
          }));
        } catch {
          if (seq !== loadSeq.current) return;
          setSections((prev) => ({ ...prev, [status]: { ...prev[status], loading: false } }));
        }
      }),
    );
  }, []);

  const loadMore = useCallback(
    async (status: PendingStatus) => {
      const section = sections[status];
      if (!section.cursor || section.loading) return;
      const seq = loadSeq.current;
      setSections((prev) => ({ ...prev, [status]: { ...prev[status], loading: true } }));
      try {
        const page = await friendshipAPI.list({ status, cursor: section.cursor, limit: PAGE_SIZE });
        if (seq !== loadSeq.current) return;
        setSections((prev) => {
          const known = new Set(prev[status].rows.map((f) => f.friendId));
          return {
            ...prev,
            [status]: {
              rows: [...prev[status].rows, ...page.friends.filter((f) => !known.has(f.friendId))],
              cursor: page.nextCursor,
              loading: false,
            },
          };
        });
      } catch {
        setSections((prev) => ({ ...prev, [status]: { ...prev[status], loading: false } }));
      }
    },
    [sections],
  );

  useEffect(() => {
    if (!isOpen) return;
    void loadFirstPages();
  }, [isOpen, statusById, loadFirstPages]);

  useEffect(() => {
    if (!isOpen) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  const dropRow = (friendId: string) =>
    setSections((prev) => {
      const next = { ...prev };
      for (const status of PENDING_STATUSES) {
        if (prev[status].rows.some((f) => f.friendId === friendId)) {
          next[status] = { ...prev[status], rows: prev[status].rows.filter((f) => f.friendId !== friendId) };
        }
      }
      return next;
    });

  const runAction = async (friendId: string, action: () => Promise<void>) => {
    setError(null);
    setBusy((prev) => new Set(prev).add(friendId));
    try {
      await action();
      dropRow(friendId);
    } catch {
      setError(t("pendingFriends.actionFailed"));
    } finally {
      setBusy((prev) => {
        const next = new Set(prev);
        next.delete(friendId);
        return next;
      });
    }
  };

  const countOf = (status: FriendshipStatus) =>
    Object.values(statusById).filter((s) => s === status).length;
  const totals = {
    pending_incoming: countOf("pending_incoming"),
    pending_outgoing: countOf("pending_outgoing"),
    declined: countOf("declined"),
  };
  const visible = (status: PendingStatus) =>
    totals[status] > 0 || sections[status].rows.length > 0;
  const countFor = (status: PendingStatus) =>
    Math.max(totals[status], sections[status].rows.length);
  const anyVisible = PENDING_STATUSES.some(visible);
  const anyLoading = PENDING_STATUSES.some((status) => sections[status].loading);
  const nothingPending = !anyVisible && !anyLoading;

  const renderLoadMore = (status: PendingStatus) =>
    sections[status].cursor ? (
      <button
        type="button"
        className="sb-load-more"
        disabled={sections[status].loading}
        onClick={() => void loadMore(status)}
      >
        {t("pendingFriends.loadMore")}
      </button>
    ) : null;

  const renderPeer = (f: Friendship) => (
    <button
      type="button"
      className="pf-peer"
      title={t("profile:viewProfile")}
      onClick={() => useProfileModalStore.getState().open(f.friendId)}
    >
      <UserAvatar userId={f.friendId} name={f.friendName} className="pf-ava" />
      <div className="pf-name">{f.friendName}</div>
    </button>
  );

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="m-head">
          <h3>{t("pendingFriends.title")}</h3>
          <button
            type="button"
            className="close"
            onClick={onClose}
            title={t("common:close")}
          >
            <Icon name="x" size={14} />
          </button>
        </div>
        <div className="m-body">
          {error && <div className="m-error">{error}</div>}
          {!anyVisible && anyLoading && (
            <div className="m-empty">
              <span className="loading-spinner" />
            </div>
          )}
          {nothingPending && (
            <div className="m-empty">
              <span className="m-empty-ico">
                <Icon name="mail" size={22} />
              </span>
              <div className="m-empty-title">{t("pendingFriends.emptyTitle")}</div>
              <div className="m-empty-sub">{t("pendingFriends.emptyHint")}</div>
              {onAddFriend && (
                <button
                  type="button"
                  className="m-empty-cta"
                  onClick={() => {
                    onClose();
                    onAddFriend();
                  }}
                >
                  <Icon name="user-plus" size={14} />
                  {t("pendingFriends.emptyCta")}
                </button>
              )}
            </div>
          )}

          {visible("pending_incoming") && (
            <div className="pf-section">
              <div className="pf-section-title">
                {t("pendingFriends.incoming")} · {countFor("pending_incoming")}
              </div>
              {sections.pending_incoming.rows.map((f) => (
                <div key={f.friendId} className="pf-row">
                  {renderPeer(f)}
                  <button
                    type="button"
                    className="pf-btn accept"
                    title={t("pendingFriends.accept")}
                    disabled={busy.has(f.friendId)}
                    onClick={() => void runAction(f.friendId, () => accept(f.friendId))}
                  >
                    <Icon name="check" size={14} />
                  </button>
                  <button
                    type="button"
                    className="pf-btn decline"
                    title={t("pendingFriends.decline")}
                    disabled={busy.has(f.friendId)}
                    onClick={() => void runAction(f.friendId, () => remove(f.friendId))}
                  >
                    <Icon name="x" size={14} />
                  </button>
                </div>
              ))}
              {renderLoadMore("pending_incoming")}
            </div>
          )}

          {visible("pending_outgoing") && (
            <div className="pf-section">
              <div className="pf-section-title">
                {t("pendingFriends.outgoing")} · {countFor("pending_outgoing")}
              </div>
              {sections.pending_outgoing.rows.map((f) => (
                <div key={f.friendId} className="pf-row">
                  {renderPeer(f)}
                  <span className="pf-tag">sent</span>
                  <button
                    type="button"
                    className="pf-btn decline"
                    title={t("pendingFriends.cancel")}
                    disabled={busy.has(f.friendId)}
                    onClick={() => void runAction(f.friendId, () => remove(f.friendId))}
                  >
                    <Icon name="x" size={14} />
                  </button>
                </div>
              ))}
              {renderLoadMore("pending_outgoing")}
            </div>
          )}

          {visible("declined") && (
            <div className="pf-section">
              <div className="pf-section-title">
                Declined · {countFor("declined")}
              </div>
              {sections.declined.rows.map((f) => (
                <div key={f.friendId} className="pf-row">
                  {renderPeer(f)}
                  <span className="pf-tag pf-tag-danger">declined</span>
                  <button
                    type="button"
                    className="pf-btn decline"
                    title="Dismiss"
                    disabled={busy.has(f.friendId)}
                    onClick={() => void runAction(f.friendId, () => remove(f.friendId))}
                  >
                    <Icon name="x" size={14} />
                  </button>
                </div>
              ))}
              {renderLoadMore("declined")}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

export default PendingFriendsModal;
