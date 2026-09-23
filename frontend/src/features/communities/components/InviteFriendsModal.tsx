import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import Icon from "../../../shared/ui/Icon";
import { dmAPI } from "../../dm/api/dm";
import { communityAPI } from "../api/community";
import { ApiError } from "../../../shared/lib/apiError";
import { UserAvatar } from "../../../shared/ui/Avatar";
import { useFriendshipStore } from "../../social/store/friendshipStore";
import type { Friendship } from "../../../shared/types/friendship";
import type { Community } from "../../../shared/types/community";

interface InviteFriendsModalProps {
  community: Community | null;
  friends: Friendship[];
  onClose: () => void;
}

const InviteFriendsModal: React.FC<InviteFriendsModalProps> = ({
  community,
  friends,
  onClose,
}) => {
  const { t } = useTranslation(["modals", "common"]);
  const [query, setQuery] = useState("");
  const [invited, setInvited] = useState<Set<string>>(new Set());
  const [members, setMembers] = useState<Set<string>>(new Set());
  const [pending, setPending] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!community) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [community, onClose]);

  useEffect(() => {
    if (community) void useFriendshipStore.getState().loadAll();
  }, [community]);

  useEffect(() => {
    setQuery("");
    setInvited(new Set());
    setMembers(new Set());
    setPending(null);
    setError(null);
  }, [community?.id]);

  useEffect(() => {
    if (!community) return;
    let cancelled = false;
    (async () => {
      try {
        const [allMembers, invitesRes] = await Promise.all([
          communityAPI.listAllMembers(community.id),
          communityAPI.listSentInvites(community.id),
        ]);
        if (cancelled) return;
        setMembers((prev) => {
          const next = new Set(prev);
          for (const m of allMembers) next.add(m.id);
          return next;
        });
        setInvited((prev) => {
          const next = new Set(prev);
          for (const id of invitesRes.recipientIds) next.add(id);
          return next;
        });
      } catch {
        return;
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [community]);

  if (!community) return null;

  const q = query.trim().toLowerCase();
  const visible = q
    ? friends.filter((f) => f.friendName.toLowerCase().includes(q))
    : friends;

  const handleInvite = async (friendId: string) => {
    setError(null);
    setPending(friendId);
    try {
      await dmAPI.sendCommunityInvite(friendId, community.id);
      setInvited((prev) => {
        const next = new Set(prev);
        next.add(friendId);
        return next;
      });
    } catch (err) {
      const apiErr = err instanceof ApiError ? err : null;
      const code = apiErr?.code;
      if (code === "ALREADY_MEMBER") {
        setMembers((prev) => {
          const next = new Set(prev);
          next.add(friendId);
          return next;
        });
      } else if (code === "INVITE_ALREADY_PENDING") {
        setInvited((prev) => {
          const next = new Set(prev);
          next.add(friendId);
          return next;
        });
      } else {
        setError(apiErr?.message || t("common:error"));
      }
    } finally {
      setPending(null);
    }
  };

  return (
    <div className="modal-overlay" onMouseDown={onClose}>
      <div className="modal" onMouseDown={(e) => e.stopPropagation()}>
        <div className="m-head">
          <h3>{t("inviteFriends.title", { community: community.name })}</h3>
          <button type="button" className="close" onClick={onClose} title={t("common:close")}>
            <Icon name="x" size={14} />
          </button>
        </div>
        <div className="m-body">
          {friends.length === 0 ? (
            <div className="pf-empty">
              {t("inviteFriends.noFriends")}
            </div>
          ) : (
            <>
              <div className="m-field" style={{ marginBottom: 10 }}>
                <input
                  type="text"
                  autoFocus
                  placeholder={t("inviteFriends.search")}
                  value={query}
                  onChange={(e) => setQuery(e.target.value)}
                />
              </div>
              {visible.length === 0 ? (
                <div className="pf-empty">{t("inviteFriends.noFriends")}</div>
              ) : (
                <div className="pf-section">
                  {visible.map((f) => {
                    const isMember = members.has(f.friendId);
                    const sent = invited.has(f.friendId);
                    const busy = pending === f.friendId;
                    const label = isMember
                      ? t("common:active")
                      : sent
                      ? t("inviteFriends.invited")
                      : busy
                      ? t("addFriend.sending")
                      : t("inviteFriends.invite");
                    return (
                      <div key={f.friendId} className="pf-row">
                        <UserAvatar
                          userId={f.friendId}
                          name={f.friendName}
                          className="pf-ava"
                        />
                        <div className="pf-name">{f.friendName}</div>
                        <button
                          type="button"
                          className={"invite-friend-btn" + (sent || isMember ? " sent" : "")}
                          disabled={sent || isMember || busy}
                          onClick={() => handleInvite(f.friendId)}
                        >
                          {isMember ? (
                            label
                          ) : sent ? (
                            <>
                              <Icon name="check" size={12} /> {label}
                            </>
                          ) : (
                            label
                          )}
                        </button>
                      </div>
                    );
                  })}
                </div>
              )}
              {error && (
                <div
                  style={{
                    fontSize: 12,
                    color: "var(--danger)",
                    background: "var(--danger-tint)",
                    padding: "8px 12px",
                    borderRadius: 8,
                    marginTop: 10,
                  }}
                >
                  {error}
                </div>
              )}
            </>
          )}
        </div>
      </div>
    </div>
  );
};

export default InviteFriendsModal;
