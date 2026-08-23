import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import Icon from "./rift/Icon";
import { SEED_MEMBERS, type MockMember, type MockDirect } from "../data/mockData";
import { UserAvatar } from "./rift/Avatar";

interface NewDmModalProps {
  isOpen: boolean;
  existingDirects: MockDirect[];
  onClose: () => void;
  onPick: (person: MockMember) => void;
  onAddFriend?: () => void;
}

const NewDmModal: React.FC<NewDmModalProps> = ({
  isOpen,
  existingDirects,
  onClose,
  onPick,
  onAddFriend,
}) => {
  const { t } = useTranslation(["modals", "common"]);
  const [query, setQuery] = useState("");

  useEffect(() => {
    if (!isOpen) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  const dmNameSet = new Set(existingDirects.map((d) => d.name));
  const dmIdSet = new Set(existingDirects.map((d) => d.id.replace(/^dm-/, "u-")));
  const people = SEED_MEMBERS.map((m) => ({
    ...m,
    hasDm: dmIdSet.has(m.id) || dmNameSet.has(m.name),
  }));

  const q = query.trim().toLowerCase();
  const filtered = q ? people.filter((p) => p.name.toLowerCase().includes(q)) : people;

  const submitFreeform = () => {
    const name = query.trim();
    if (!name) return;
    const known = people.find((p) => p.name.toLowerCase() === name.toLowerCase());
    if (known) {
      onPick(known);
      return;
    }
    const initials = name
      .split(/\s+/)
      .slice(0, 2)
      .map((w) => w[0])
      .join("")
      .toUpperCase();
    onPick({
      id: "u-custom-" + Date.now().toString(36),
      name,
      initials,
      color: "#0071e3",
      role: "Member",
      status: "offline",
    });
  };

  return (
    <div className="modal-overlay" onMouseDown={onClose}>
      <div className="modal newdm-modal" onMouseDown={(e) => e.stopPropagation()}>
        <div className="m-head">
          <h3>{t("newDm.title")}</h3>
          <button type="button" className="close" onClick={onClose} title={t("close", { ns: "common" })}>
            <Icon name="x" size={14} />
          </button>
        </div>
        <div className="newdm-search">
          <Icon name="search" size={14} />
          <input
            autoFocus
            placeholder={t("newDm.search")}
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                if (filtered.length > 0) onPick(filtered[0]);
                else submitFreeform();
              }
            }}
          />
        </div>
        <div className="newdm-list">
          {people.length === 0 && !q && (
            <div className="m-empty">
              <span className="m-empty-ico">
                <Icon name="users" size={22} />
              </span>
              <div className="m-empty-title">{t("newDm.emptyTitle")}</div>
              <div className="m-empty-sub">{t("newDm.noFriends")}</div>
              {onAddFriend && (
                <button
                  type="button"
                  className="m-empty-cta"
                  onClick={() => {
                    onClose();
                    onAddFriend();
                  }}
                >
                  <Icon name="user-plus" size={14} />
                  {t("newDm.emptyCta")}
                </button>
              )}
            </div>
          )}
          {filtered.length === 0 && q && (
            <button type="button" className="newdm-create" onClick={submitFreeform}>
              <span className="newdm-ava placeholder">
                <Icon name="plus" size={14} />
              </span>
              <span className="newdm-text">
                <span className="newdm-name">Start chat with "{query.trim()}"</span>
                <span className="newdm-sub">
                  No matches in your network — we'll create a new conversation.
                </span>
              </span>
            </button>
          )}
          {filtered.map((p) => (
            <button
              type="button"
              key={p.id}
              className="newdm-row"
              onClick={() => onPick(p)}
            >
              <UserAvatar
                userId={p.id}
                name={p.name}
                className="newdm-ava"
                style={{ background: p.color, display: "inline-flex" }}
              >
                {p.status === "online" && <span className="presence" />}
              </UserAvatar>
              <span className="newdm-text">
                <span className="newdm-name">{p.name}</span>
                <span className="newdm-sub">
                  {p.role || (p.status === "online" ? "Active now" : "Offline")}
                </span>
              </span>
              {p.hasDm && <span className="newdm-tag">Open chat</span>}
            </button>
          ))}
        </div>
        <div className="newdm-foot">
          <span className="hint">
            <kbd>↵</kbd> to start chat &nbsp;·&nbsp; <kbd>Esc</kbd> to close
          </span>
        </div>
      </div>
    </div>
  );
};

export default NewDmModal;
