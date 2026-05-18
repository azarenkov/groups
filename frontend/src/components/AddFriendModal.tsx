import React, { useEffect, useState } from "react";
import { ApiError } from "../utils/apiError";
import { useFriendshipStore } from "../store/friendshipStore";

interface AddFriendModalProps {
  isOpen: boolean;
  onClose: () => void;
}

const AddFriendModal: React.FC<AddFriendModalProps> = ({ isOpen, onClose }) => {
  const [username, setUsername] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const sendRequest = useFriendshipStore((s) => s.sendRequest);

  useEffect(() => {
    if (!isOpen) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [isOpen, onClose]);

  useEffect(() => {
    if (!isOpen) {
      setUsername("");
      setError(null);
      setSubmitting(false);
    }
  }, [isOpen]);

  if (!isOpen) return null;

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    const trimmed = username.trim();
    if (!trimmed) return;
    setSubmitting(true);
    setError(null);
    try {
      await sendRequest(trimmed);
      onClose();
    } catch (err) {
      const apiErr = err instanceof ApiError ? err : null;
      setError(apiErr?.message || "Failed to send friend request");
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="m-head">
          <h3>Add friend</h3>
        </div>
        <form onSubmit={handleSubmit}>
          <div className="m-body">
            <div className="m-field">
              <label>Username</label>
              <input
                autoFocus
                type="text"
                placeholder="Enter exact username"
                value={username}
                onChange={(e) => setUsername(e.target.value)}
                maxLength={100}
                disabled={submitting}
              />
            </div>
            {error && <div className="m-error">{error}</div>}
          </div>
          <div className="m-foot">
            <button type="button" onClick={onClose} disabled={submitting}>
              Cancel
            </button>
            <button
              type="submit"
              className="primary"
              disabled={submitting || username.trim().length === 0}
            >
              {submitting ? "Sending…" : "Send request"}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};

export default AddFriendModal;
