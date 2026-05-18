import React, { useEffect } from "react";
import Icon from "./rift/Icon";
import { useFriendshipStore } from "../store/friendshipStore";

interface PendingFriendsModalProps {
  isOpen: boolean;
  onClose: () => void;
}

const PALETTE = ["#0071e3", "#ff6b35", "#30a46c", "#bf5af2", "#ff9f0a", "#5e6ad2"];
const colorFor = (id: string): string => {
  let h = 0;
  for (let i = 0; i < id.length; i++) h = (h * 31 + id.charCodeAt(i)) >>> 0;
  return PALETTE[h % PALETTE.length];
};
const initialsFor = (s: string) => (s ? s.charAt(0).toUpperCase() : "?");

const PendingFriendsModal: React.FC<PendingFriendsModalProps> = ({
  isOpen,
  onClose,
}) => {
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
          <h3>Friend requests</h3>
          <button
            type="button"
            className="close"
            onClick={onClose}
            title="Close"
          >
            <Icon name="x" size={14} />
          </button>
        </div>
        <div className="m-body">
          {incoming.length === 0 &&
            outgoing.length === 0 &&
            declined.length === 0 && (
              <div className="pf-empty">No pending friend requests.</div>
            )}

          {incoming.length > 0 && (
            <div className="pf-section">
              <div className="pf-section-title">
                Incoming · {incoming.length}
              </div>
              {incoming.map((f) => (
                <div key={f.friendId} className="pf-row">
                  <div
                    className="pf-ava"
                    style={{ background: colorFor(f.friendId) }}
                  >
                    {initialsFor(f.friendName)}
                  </div>
                  <div className="pf-name">{f.friendName}</div>
                  <button
                    type="button"
                    className="pf-btn accept"
                    title="Accept"
                    onClick={() => accept(f.friendId)}
                  >
                    <Icon name="check" size={14} />
                  </button>
                  <button
                    type="button"
                    className="pf-btn decline"
                    title="Decline"
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
                Outgoing · {outgoing.length}
              </div>
              {outgoing.map((f) => (
                <div key={f.friendId} className="pf-row">
                  <div
                    className="pf-ava"
                    style={{ background: colorFor(f.friendId) }}
                  >
                    {initialsFor(f.friendName)}
                  </div>
                  <div className="pf-name">{f.friendName}</div>
                  <span className="pf-tag">sent</span>
                  <button
                    type="button"
                    className="pf-btn decline"
                    title="Cancel"
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
                  <div
                    className="pf-ava"
                    style={{ background: colorFor(f.friendId) }}
                  >
                    {initialsFor(f.friendName)}
                  </div>
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
