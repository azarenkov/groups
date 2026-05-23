import React, { useEffect, useState } from "react";
import Icon from "./rift/Icon";
import "../styles/rift.css";
import { AVATAR_PALETTE as PALETTE } from "../utils/avatar";

interface CreateServerModalProps {
  isOpen: boolean;
  onClose: () => void;
  onCreate: (name: string, description?: string) => Promise<void>;
}
const MAX_SERVER_NAME_LENGTH = 100;
const MAX_DESCRIPTION_LENGTH = 500;

const CreateServerModal: React.FC<CreateServerModalProps> = ({
  isOpen,
  onClose,
  onCreate,
}) => {
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [accent, setAccent] = useState(PALETTE[0]);
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

  const trimmed = name.trim();
  const glyph =
    trimmed
      ? trimmed.split(/\s+/).slice(0, 2).map((w) => w[0]).join("").toUpperCase()
      : "?";

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!trimmed) {
      setError("Server name is required");
      return;
    }
    setIsLoading(true);
    setError(null);
    try {
      await onCreate(trimmed, description.trim() || undefined);
      setName("");
      setDescription("");
      setAccent(PALETTE[0]);
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Could not create server");
    } finally {
      setIsLoading(false);
    }
  };

  const handleClose = () => {
    if (isLoading) return;
    setName("");
    setDescription("");
    setAccent(PALETTE[0]);
    setError(null);
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
          <h3>Create a server</h3>
          <button type="button" className="close" onClick={handleClose} title="Close">
            <Icon name="x" size={14} />
          </button>
        </div>
        <div className="m-body">
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
            disabled={isLoading || !trimmed}
          >
            {isLoading ? <span className="loading-spinner" /> : "Create server"}
          </button>
        </div>
      </form>
    </div>
  );
};

export default CreateServerModal;
