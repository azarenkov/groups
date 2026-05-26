import React, { useState, useEffect } from "react";
import { Trans, useTranslation } from "react-i18next";
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
  const { t } = useTranslation(["modals", "common"]);
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
          <h3>{t("passwordPrompt.title")}</h3>
          <button type="button" className="close" onClick={onClose} title={t("common:close")}>
            <Icon name="x" size={14} />
          </button>
        </div>
        <div className="m-body">
          <div style={{ fontSize: 13, color: "var(--fg-2)", marginBottom: 12 }}>
            <Trans i18nKey="passwordPrompt.subtitle" t={t} values={{ name: channelName }} components={[<b key="n" style={{ color: "var(--fg-1)" }} />]} />
          </div>
          <div className="m-field">
            <label>{t("passwordPrompt.placeholder")}</label>
            <input
              type="password"
              autoFocus
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder={t("passwordPrompt.placeholder")}
              maxLength={50}
            />
          </div>
        </div>
        <div className="m-foot">
          <button type="button" onClick={onClose}>
            {t("common:cancel")}
          </button>
          <button type="submit" className="primary" disabled={!password.trim()}>
            {t("passwordPrompt.confirm")}
          </button>
        </div>
      </form>
    </div>
  );
};

export default PasswordPromptModal;
