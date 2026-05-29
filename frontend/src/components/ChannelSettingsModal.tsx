import React, { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import Icon from "./rift/Icon";

interface ChannelSettingsModalProps {
  isOpen: boolean;
  channel: { id: string; name: string; topic?: string };
  onClose: () => void;
  onSave: (patch: { name: string; topic: string }) => void;
  onDelete: () => void;
}

const ChannelSettingsModal: React.FC<ChannelSettingsModalProps> = ({
  isOpen,
  channel,
  onClose,
  onSave,
  onDelete,
}) => {
  const { t } = useTranslation(["modals", "common"]);
  const [name, setName] = useState(channel.name);
  const [topic, setTopic] = useState(channel.topic || "");
  const nameInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    setName(channel.name);
    setTopic(channel.topic || "");
  }, [channel.id, channel.name, channel.topic]);

  useEffect(() => {
    if (!isOpen) return;
    nameInputRef.current?.focus();
    nameInputRef.current?.select();
  }, [isOpen]);

  useEffect(() => {
    if (!isOpen) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [isOpen, onClose]);

  const modalRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!isOpen) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Tab" || !modalRef.current) return;
      const focusables = modalRef.current.querySelectorAll<HTMLElement>(
        'input, textarea, button:not([tabindex="-1"])',
      );
      if (focusables.length === 0) return;
      const first = focusables[0];
      const last = focusables[focusables.length - 1];
      const active = document.activeElement as HTMLElement | null;
      if (e.shiftKey && active === first) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && active === last) {
        e.preventDefault();
        first.focus();
      } else if (active && !modalRef.current.contains(active)) {
        e.preventDefault();
        first.focus();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [isOpen]);

  if (!isOpen) return null;

  return (
    <div className="modal-overlay" onMouseDown={onClose}>
      <div ref={modalRef} className="modal" onMouseDown={(e) => e.stopPropagation()}>
        <div className="m-head">
          <h3>#{channel.name} · {t("channelSettings.title").toLowerCase()}</h3>
          <button
            type="button"
            tabIndex={-1}
            className="close"
            onClick={onClose}
            title={t("common:close")}
          >
            <Icon name="x" size={14} />
          </button>
        </div>
        <div className="m-body">
          <div className="m-field">
            <label>{t("createChannel.nameLabel")}</label>
            <input
              ref={nameInputRef}
              value={name}
              onChange={(e) =>
                setName(e.target.value.replace(/\s+/g, "-").toLowerCase())
              }
            />
          </div>
          <div className="m-field">
            <label>Topic</label>
            <textarea
              rows={3}
              placeholder=""
              value={topic}
              onChange={(e) => setTopic(e.target.value)}
            />
          </div>
        </div>
        <div className="m-foot">
          <button type="button" className="danger" onClick={onDelete}>
            {t("channelSettings.delete")}
          </button>
          <button type="button" onClick={onClose}>
            {t("common:cancel")}
          </button>
          <button
            type="button"
            className="primary"
            onClick={() =>
              onSave({ name: name.trim() || channel.name, topic: topic.trim() })
            }
          >
            {t("common:save")}
          </button>
        </div>
      </div>
    </div>
  );
};

export default ChannelSettingsModal;
