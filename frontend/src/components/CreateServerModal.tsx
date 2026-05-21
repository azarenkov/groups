import React, { useEffect, useState } from "react";
import Icon from "./rift/Icon";
import "../styles/rift.css";

interface CreateServerModalProps {
  isOpen: boolean;
  onClose: () => void;
  onCreate: (name: string, description?: string) => Promise<void>;
  onJoin: (code: string) => Promise<void>;
}

const PALETTE = ["#0071e3", "#30a46c", "#ff9f0a", "#bf5af2", "#ff6b35", "#5b6cd9", "#d97557", "#30d158"];
const MAX_SERVER_NAME_LENGTH = 100;
const MAX_DESCRIPTION_LENGTH = 500;

type Mode = "create" | "join";

const CreateServerModal: React.FC<CreateServerModalProps> = ({
  isOpen,
  onClose,
  onCreate,
  onJoin,
}) => {
  const [mode, setMode] = useState<Mode>("create");
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [accent, setAccent] = useState(PALETTE[0]);
  const [code, setCode] = useState("");
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
    setMode("create");
    setName("");
    setDescription("");
    setAccent(PALETTE[0]);
    setCode("");
    setError(null);
  };

  const trimmed = name.trim();
  const trimmedCode = code.trim();
  const glyph =
    trimmed
      ? trimmed.split(/\s+/).slice(0, 2).map((w) => w[0]).join("").toUpperCase()
      : "?";

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    if (mode === "create") {
      if (!trimmed) {
        setError("Server name is required");
        return;
      }
      setIsLoading(true);
      try {
        await onCreate(trimmed, description.trim() || undefined);
        reset();
        onClose();
      } catch (err) {
        setError(err instanceof Error ? err.message : "Could not create server");
      } finally {
        setIsLoading(false);
      }
    } else {
      if (!trimmedCode) {
        setError("Invite code is required");
        return;
      }
      setIsLoading(true);
      try {
        await onJoin(trimmedCode);
        reset();
        onClose();
      } catch (err) {
        setError(err instanceof Error ? err.message : "Could not join server");
      } finally {
        setIsLoading(false);
      }
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
        className="modal server-modal"
        onMouseDown={(e) => e.stopPropagation()}
        onSubmit={handleSubmit}
      >
        <div className="m-head">
          <h3>{mode === "create" ? "Create a server" : "Join a server"}</h3>
          <button type="button" className="close" onClick={handleClose} title="Close">
            <Icon name="x" size={14} />
          </button>
        </div>
        <div className="m-body">
          <div className="cs-tabs">
            <button
              type="button"
              className={"cs-tab" + (mode === "create" ? " active" : "")}
              onClick={() => {
                setMode("create");
                setError(null);
              }}
              disabled={isLoading}
            >
              Create
            </button>
            <button
              type="button"
              className={"cs-tab" + (mode === "join" ? " active" : "")}
              onClick={() => {
                setMode("join");
                setError(null);
              }}
              disabled={isLoading}
            >
              Join
            </button>
          </div>

          {mode === "create" ? (
            <>
              <div className="cs-glyph-row">
                <div className="cs-glyph-preview" style={{ background: accent }}>
                  {glyph}
                </div>
                <div className="cs-glyph-hint">
                  Servers are where teammates hang out. Pick a name and a color —
                  channels and members come next.
                </div>
              </div>

              <div className="m-field">
                <label>Server name</label>
                <input
                  type="text"
                  autoFocus
                  placeholder="The Studio, Garden, Signal…"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  disabled={isLoading}
                  maxLength={MAX_SERVER_NAME_LENGTH}
                />
              </div>

              <div className="m-field">
                <label>Description (optional)</label>
                <textarea
                  placeholder="What is this server about?"
                  value={description}
                  onChange={(e) => setDescription(e.target.value)}
                  disabled={isLoading}
                  maxLength={MAX_DESCRIPTION_LENGTH}
                  rows={3}
                />
              </div>

              <div className="m-field">
                <label>Accent</label>
                <div className="cs-accent-swatches">
                  {PALETTE.map((c) => (
                    <button
                      key={c}
                      type="button"
                      className={"cs-accent-swatch" + (accent === c ? " selected" : "")}
                      style={{ background: c }}
                      onClick={() => setAccent(c)}
                      title={c}
                    />
                  ))}
                </div>
              </div>
            </>
          ) : (
            <div className="m-field">
              <label>Invite code</label>
              <input
                type="text"
                autoFocus
                placeholder="Enter an invite code"
                value={code}
                onChange={(e) => setCode(e.target.value)}
                disabled={isLoading}
                maxLength={16}
              />
              <div className="cs-glyph-hint" style={{ marginTop: 8 }}>
                Paste an invite code shared by a server member to join.
              </div>
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
            disabled={
              isLoading || (mode === "create" ? !trimmed : !trimmedCode)
            }
          >
            {isLoading ? (
              <span className="loading-spinner" />
            ) : mode === "create" ? (
              "Create server"
            ) : (
              "Join server"
            )}
          </button>
        </div>
      </form>
    </div>
  );
};

export default CreateServerModal;
