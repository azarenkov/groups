import React, { useState } from "react";
import "../styles/CreateServerModal.css";

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
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!name.trim()) {
      setError("Server name is required");
      return;
    }

    setIsLoading(true);
    setError(null);

    try {
      await onCreate(name.trim(), description.trim() || undefined);
      setName("");
      setDescription("");
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to create server");
    } finally {
      setIsLoading(false);
    }
  };

  const handleClose = () => {
    if (!isLoading) {
      setName("");
      setDescription("");
      setError(null);
      onClose();
    }
  };

  if (!isOpen) return null;

  return (
    <div className="modal-overlay" onClick={handleClose}>
      <div className="modal-content" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h2>Create Your Server</h2>
          <p className="modal-subtitle">
            Your server is where you and your friends hang out. Make yours and
            start talking.
          </p>
          <button
            className="modal-close-btn"
            onClick={handleClose}
            disabled={isLoading}
          >
            ×
          </button>
        </div>

        <form onSubmit={handleSubmit} className="modal-form">
          <div className="form-group">
            <label htmlFor="server-name">
              Server Name
              {name.length > 0 && (
                <span className="char-count">
                  {name.length} / {MAX_SERVER_NAME_LENGTH}
                </span>
              )}
            </label>
            <input
              id="server-name"
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="Enter server name"
              disabled={isLoading}
              maxLength={MAX_SERVER_NAME_LENGTH}
              autoFocus
            />
          </div>

          <div className="form-group">
            <label htmlFor="server-description">
              Description (Optional)
              {description.length > 0 && (
                <span className="char-count">
                  {description.length} / {MAX_DESCRIPTION_LENGTH}
                </span>
              )}
            </label>
            <textarea
              id="server-description"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="What's your server about?"
              disabled={isLoading}
              maxLength={MAX_DESCRIPTION_LENGTH}
              rows={3}
            />
          </div>

          {error && <div className="error-message">{error}</div>}

          <div className="modal-actions">
            <button
              type="button"
              className="btn-secondary"
              onClick={handleClose}
              disabled={isLoading}
            >
              Cancel
            </button>
            <button
              type="submit"
              className="btn-primary"
              disabled={isLoading || !name.trim()}
            >
              {isLoading ? "Creating..." : "Create Server"}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};

export default CreateServerModal;
