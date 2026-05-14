import React, { useState, useEffect } from "react";
import { ChannelKind, ChannelVisibility } from "../types/channel";
import Icon from "./rift/Icon";
import "../styles/rift.css";

interface CreateChannelModalProps {
  isOpen: boolean;
  onClose: () => void;
  onCreate: (
    name: string,
    kind: ChannelKind,
    visibility: ChannelVisibility,
    password?: string,
  ) => Promise<void>;
}

const MAX_CHANNEL_NAME_LENGTH = 100;

const CreateChannelModal: React.FC<CreateChannelModalProps> = ({
  isOpen,
  onClose,
  onCreate,
}) => {
  const [name, setName] = useState("");
  const [kind, setKind] = useState<ChannelKind>("voice");
  const [visibility, setVisibility] = useState<ChannelVisibility>("public");
  const [password, setPassword] = useState("");
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!isOpen) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [isOpen, onClose]);

  const reset = () => {
    setName("");
    setKind("voice");
    setVisibility("public");
    setPassword("");
    setError(null);
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) {
      setError("Channel name is required");
      return;
    }
    if (visibility === "private" && !password.trim()) {
      setError("Private channels need a password");
      return;
    }
    setIsLoading(true);
    setError(null);
    try {
      await onCreate(
        name.trim(),
        kind,
        visibility,
        visibility === "private" ? password.trim() : undefined,
      );
      reset();
      onClose();
    } catch (err) {
      setError(
        err instanceof Error ? err.message : "Could not create channel",
      );
    } finally {
      setIsLoading(false);
    }
  };

  const handleClose = () => {
    if (isLoading) return;
    reset();
    onClose();
  };

  if (!isOpen) return null;

  return (
    <div className="modal-overlay" onMouseDown={handleClose}>
      <form
        className="modal"
        onMouseDown={(e) => e.stopPropagation()}
        onSubmit={handleSubmit}
      >
        <div className="m-head">
          <h3>Create a channel</h3>
          <button type="button" className="close" onClick={handleClose} title="Close">
            <Icon name="x" size={14} />
          </button>
        </div>
        <div className="m-body">
          <div className="m-field">
            <label>Channel name</label>
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="e.g. Lounge"
              disabled={isLoading}
              maxLength={MAX_CHANNEL_NAME_LENGTH}
              autoFocus
            />
          </div>

          <div className="m-field">
            <label>Type</label>
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 }}>
              {(
                [
                  { value: "voice" as const, icon: "headphones" as const, ttl: "Voice", sub: "Talk in real time" },
                  { value: "text" as const, icon: "hash" as const, ttl: "Text", sub: "Chat with messages" },
                ]
              ).map((opt) => {
                const active = kind === opt.value;
                return (
                  <button
                    key={opt.value}
                    type="button"
                    onClick={() => {
                      if (isLoading) return;
                      setKind(opt.value);
                      if (opt.value === "text") {
                        setVisibility("public");
                        setPassword("");
                      }
                    }}
                    style={{
                      display: "flex",
                      alignItems: "center",
                      gap: 10,
                      padding: 12,
                      background: active ? "var(--accent-tint)" : "var(--surface-canvas)",
                      border: `1px solid ${active ? "var(--accent)" : "var(--hairline)"}`,
                      borderRadius: 8,
                      cursor: isLoading ? "not-allowed" : "pointer",
                      textAlign: "left",
                      transition: "all 160ms",
                    }}
                  >
                    <Icon name={opt.icon} size={18} style={{ color: active ? "var(--accent)" : "var(--fg-2)" }} />
                    <div>
                      <div style={{ fontSize: 13, fontWeight: 600, color: "var(--fg-1)" }}>
                        {opt.ttl}
                      </div>
                      <div style={{ fontSize: 11, color: "var(--fg-3)" }}>{opt.sub}</div>
                    </div>
                  </button>
                );
              })}
            </div>
          </div>

          {kind === "voice" && (
          <div className="m-field">
            <label>Privacy</label>
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 }}>
              {(
                [
                  { value: "public" as const, icon: "globe" as const, ttl: "Public", sub: "Anyone with the link" },
                  { value: "private" as const, icon: "lock" as const, ttl: "Private", sub: "Password required" },
                ]
              ).map((opt) => {
                const active = visibility === opt.value;
                return (
                  <button
                    key={opt.value}
                    type="button"
                    onClick={() => !isLoading && setVisibility(opt.value)}
                    style={{
                      display: "flex",
                      alignItems: "center",
                      gap: 10,
                      padding: 12,
                      background: active ? "var(--accent-tint)" : "var(--surface-canvas)",
                      border: `1px solid ${active ? "var(--accent)" : "var(--hairline)"}`,
                      borderRadius: 8,
                      cursor: isLoading ? "not-allowed" : "pointer",
                      textAlign: "left",
                      transition: "all 160ms",
                    }}
                  >
                    <Icon name={opt.icon} size={18} style={{ color: active ? "var(--accent)" : "var(--fg-2)" }} />
                    <div>
                      <div style={{ fontSize: 13, fontWeight: 600, color: "var(--fg-1)" }}>
                        {opt.ttl}
                      </div>
                      <div style={{ fontSize: 11, color: "var(--fg-3)" }}>{opt.sub}</div>
                    </div>
                  </button>
                );
              })}
            </div>
          </div>
          )}

          {kind === "voice" && visibility === "private" && (
            <div className="m-field">
              <label>Password</label>
              <input
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                placeholder="Set a password"
                disabled={isLoading}
                maxLength={50}
              />
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
                marginBottom: 4,
              }}
            >
              {error}
            </div>
          )}
        </div>
        <div className="m-foot">
          <button type="button" onClick={handleClose} disabled={isLoading}>
            Cancel
          </button>
          <button
            type="submit"
            className="primary"
            disabled={isLoading || !name.trim()}
          >
            {isLoading ? <span className="loading-spinner" /> : "Create channel"}
          </button>
        </div>
      </form>
    </div>
  );
};

export default CreateChannelModal;
