import React, { useEffect } from "react";
import { useTranslation } from "react-i18next";
import Icon from "./rift/Icon";
import { useFriendshipStore } from "../store/friendshipStore";
import { UserAvatar } from "./rift/Avatar";

interface PendingFriendsModalProps {
  isOpen: boolean;
  onClose: () => void;
}

const PendingFriendsModal: React.FC<PendingFriendsModalProps> = ({
  isOpen,
  onClose,
}) => {
  const { t } = useTranslation(["modals", "common"]);
  const friends = useFriendshipStore((s) => s.friends);
  const accept = useFriendshipStore((s) => s.accept);
  const remove = useFriendshipStore((s) => s.remove);

  useEffect(() => {
    if (!isOpen) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  const incoming = friends.filter((f) => f.status === "pending_incoming");
  const outgoing = friends.filter((f) => f.status === "pending_outgoing");
  const declined = friends.filter((f) => f.status === "declined");

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
          {incoming.length === 0 &&
            outgoing.length === 0 &&
            declined.length === 0 && (
              <div className="pf-empty">{t("pendingFriends.empty")}</div>
            )}

          {incoming.length > 0 && (
            <div className="pf-section">
              <div className="pf-section-title">
                {t("pendingFriends.incoming")} · {incoming.length}
              </div>
              {incoming.map((f) => (
                <div key={f.friendId} className="pf-row">
                  <UserAvatar
                    userId={f.friendId}
                    name={f.friendName}
                    className="pf-ava"
                  />
                  <div className="pf-name">{f.friendName}</div>
                  <button
                    type="button"
                    className="pf-btn accept"
                    title={t("pendingFriends.accept")}
                    onClick={() => accept(f.friendId)}
                  >
                    <Icon name="check" size={14} />
                  </button>
                  <button
                    type="button"
                    className="pf-btn decline"
                    title={t("pendingFriends.decline")}
                    onClick={() => remove(f.friendId)}
                  >
                    <Icon name="x" size={14} />
                  </button>
                </div>
              ))}
            </div>
          )}

          {outgoing.length > 0 && (
            <div className="pf-section">
              <div className="pf-section-title">
                {t("pendingFriends.outgoing")} · {outgoing.length}
              </div>
              {outgoing.map((f) => (
                <div key={f.friendId} className="pf-row">
                  <UserAvatar
                    userId={f.friendId}
                    name={f.friendName}
                    className="pf-ava"
                  />
                  <div className="pf-name">{f.friendName}</div>
                  <span className="pf-tag">sent</span>
                  <button
                    type="button"
                    className="pf-btn decline"
                    title={t("pendingFriends.cancel")}
                    onClick={() => remove(f.friendId)}
                  >
                    <Icon name="x" size={14} />
                  </button>
                </div>
              ))}
            </div>
          )}

          {declined.length > 0 && (
            <div className="pf-section">
              <div className="pf-section-title">
                Declined · {declined.length}
              </div>
              {declined.map((f) => (
                <div key={f.friendId} className="pf-row">
                  <UserAvatar
                    userId={f.friendId}
                    name={f.friendName}
                    className="pf-ava"
                  />
                  <div className="pf-name">{f.friendName}</div>
                  <span className="pf-tag pf-tag-danger">declined</span>
                  <button
                    type="button"
                    className="pf-btn decline"
                    title="Dismiss"
                    onClick={() => remove(f.friendId)}
                  >
                    <Icon name="x" size={14} />
                  </button>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

export default PendingFriendsModal;
