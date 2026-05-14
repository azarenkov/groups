import React, { useState, useEffect } from "react";
import Icon from "./rift/Icon";
import "../styles/rift.css";

interface PasswordPromptModalProps {
  isOpen: boolean;
  channelName: string;
  onClose: () => void;
  onSubmit: (password: string) => void;
}

const PasswordPromptModal: React.FC<PasswordPromptModalProps> = ({
  isOpen,
  channelName,
  onClose,
  onSubmit,
}) => {
  const [password, setPassword] = useState("");

  useEffect(() => {
    if (!isOpen) {
      setPassword("");
      return;
    }
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!password.trim()) return;
    onSubmit(password);
  };

  return (
    <div className="modal-overlay" onMouseDown={onClose}>
      <form
        className="modal"
        onMouseDown={(e) => e.stopPropagation()}
        onSubmit={handleSubmit}
      >
        <div className="m-head">
          <h3>Private channel</h3>
          <button type="button" className="close" onClick={onClose} title="Close">
            <Icon name="x" size={14} />
          </button>
        </div>
        <div className="m-body">
          <div style={{ fontSize: 13, color: "var(--fg-2)", marginBottom: 12 }}>
            Enter password to join <b style={{ color: "var(--fg-1)" }}>{channelName}</b>.
          </div>
          <div className="m-field">
            <label>Password</label>
            <input
              type="password"
              autoFocus
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder="Password"
              maxLength={50}
            />
          </div>
        </div>
        <div className="m-foot">
          <button type="button" onClick={onClose}>
            Cancel
          </button>
          <button type="submit" className="primary" disabled={!password.trim()}>
            Join
          </button>
        </div>
      </form>
    </div>
  );
};

export default PasswordPromptModal;
