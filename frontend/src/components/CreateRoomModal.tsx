import React, { useState, useEffect } from "react";
import { RoomType } from "../types/room";
import Icon from "./rift/Icon";
import "../styles/rift.css";

interface CreateRoomModalProps {
  isOpen: boolean;
  onClose: () => void;
  onCreate: (
    name: string,
    roomType: RoomType,
    password?: string,
  ) => Promise<void>;
}

const MAX_ROOM_NAME_LENGTH = 100;

const CreateRoomModal: React.FC<CreateRoomModalProps> = ({
  isOpen,
  onClose,
  onCreate,
}) => {
  const [name, setName] = useState("");
  const [roomType, setRoomType] = useState<RoomType>(RoomType.Public);
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
    setRoomType(RoomType.Public);
    setPassword("");
    setError(null);
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) {
      setError("Room name is required");
      return;
    }
    if (roomType === RoomType.Private && !password.trim()) {
      setError("Private rooms need a password");
      return;
    }
    setIsLoading(true);
    setError(null);
    try {
      await onCreate(
        name.trim(),
        roomType,
        roomType === RoomType.Private ? password.trim() : undefined,
      );
      reset();
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Could not create room");
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
          <h3>Create a room</h3>
          <button type="button" className="close" onClick={handleClose} title="Close">
            <Icon name="x" size={14} />
          </button>
        </div>
        <div className="m-body">
          <div className="m-field">
            <label>Room name</label>
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="e.g. Lounge"
              disabled={isLoading}
              maxLength={MAX_ROOM_NAME_LENGTH}
              autoFocus
            />
          </div>

          <div className="m-field">
            <label>Privacy</label>
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 }}>
              {(
                [
                  { type: RoomType.Public, icon: "globe" as const, ttl: "Public", sub: "Anyone with the link" },
                  { type: RoomType.Private, icon: "lock" as const, ttl: "Private", sub: "Password required" },
                ]
              ).map((opt) => {
                const active = roomType === opt.type;
                return (
                  <button
                    key={opt.type}
                    type="button"
                    onClick={() => !isLoading && setRoomType(opt.type)}
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

          {roomType === RoomType.Private && (
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
            {isLoading ? <span className="loading-spinner" /> : "Create room"}
          </button>
        </div>
      </form>
    </div>
  );
};

export default CreateRoomModal;
