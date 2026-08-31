import React, { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { ApiError } from "../utils/apiError";
import { friendshipAPI } from "../api/friendship";
import { useFriendshipStore } from "../store/friendshipStore";
import { useProfileModalStore } from "../store/profileModalStore";
import Icon from "./rift/Icon";

interface AddFriendModalProps {
  isOpen: boolean;
  onClose: () => void;
}

interface Suggestion {
  id: string;
  name: string;
}

const AddFriendModal: React.FC<AddFriendModalProps> = ({ isOpen, onClose }) => {
  const { t } = useTranslation(["modals", "common"]);
  const [username, setUsername] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const [suggestions, setSuggestions] = useState<Suggestion[]>([]);
  const [highlightIdx, setHighlightIdx] = useState(-1);
  const [showSuggestions, setShowSuggestions] = useState(false);
  const sendRequest = useFriendshipStore((s) => s.sendRequest);
  const inputRef = useRef<HTMLInputElement>(null);

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
      setSuggestions([]);
      setShowSuggestions(false);
      setHighlightIdx(-1);
    }
  }, [isOpen]);

  useEffect(() => {
    if (!isOpen) return;
    const trimmed = username.trim();
    if (trimmed.length === 0) {
      setSuggestions([]);
      setShowSuggestions(false);
      return;
    }
    let cancelled = false;
    const timeout = window.setTimeout(async () => {
      try {
        const users = await friendshipAPI.searchUsers(trimmed, 8);
        if (cancelled) return;
        setSuggestions(users);
        setShowSuggestions(true);
        setHighlightIdx(-1);
      } catch (err) {
        if (!cancelled) {
          console.warn("[AddFriendModal] user search failed:", err);
          setSuggestions([]);
          setShowSuggestions(false);
        }
      }
    }, 200);
    return () => {
      cancelled = true;
      window.clearTimeout(timeout);
    };
  }, [username, isOpen]);

  if (!isOpen) return null;

  const submit = async (name: string) => {
    const trimmed = name.trim();
    if (!trimmed) return;
    setSubmitting(true);
    setError(null);
    try {
      await sendRequest(trimmed);
      onClose();
    } catch (err) {
      const apiErr = err instanceof ApiError ? err : null;
      setError(apiErr?.message || t("addFriend.userNotFound"));
    } finally {
      setSubmitting(false);
    }
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    void submit(username);
  };

  const pickSuggestion = (s: Suggestion) => {
    setUsername(s.name);
    setShowSuggestions(false);
    void submit(s.name);
  };

  const onKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (!showSuggestions || suggestions.length === 0) return;
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setHighlightIdx((i) =>
        i < 0 ? 0 : Math.min(i + 1, suggestions.length - 1),
      );
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setHighlightIdx((i) =>
        i < 0 ? suggestions.length - 1 : Math.max(i - 1, 0),
      );
    } else if (e.key === "Enter") {
      if (highlightIdx < 0) return;
      e.preventDefault();
      const picked = suggestions[highlightIdx];
      if (picked) pickSuggestion(picked);
    } else if (e.key === "Tab") {
      if (highlightIdx < 0) return;
      const picked = suggestions[highlightIdx];
      if (picked) {
        e.preventDefault();
        setUsername(picked.name);
        setShowSuggestions(false);
      }
    }
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="m-head">
          <h3>{t("addFriend.title")}</h3>
        </div>

        <form onSubmit={handleSubmit}>
          <div className="m-body">
            <div className="m-blurb">
              <div>{t("addFriend.subtitle")}</div>
              <div className="m-blurb-hint">{t("addFriend.hint")}</div>
            </div>
            <div className="m-field" style={{ position: "relative" }}>
              <label>{t("addFriend.placeholder")}</label>
              <input
                ref={inputRef}
                autoFocus
                type="text"
                placeholder={t("addFriend.placeholder")}
                value={username}
                onChange={(e) => setUsername(e.target.value)}
                onKeyDown={onKeyDown}
                onFocus={() =>
                  suggestions.length > 0 && setShowSuggestions(true)
                }
                maxLength={100}
                disabled={submitting}
                autoComplete="off"
              />
              {showSuggestions && suggestions.length > 0 && (
                <div className="user-suggest">
                  {suggestions.map((s, i) => (
                    <div
                      key={s.id}
                      className={
                        "user-suggest-item" + (i === highlightIdx ? " on" : "")
                      }
                      onMouseDown={(e) => {
                        e.preventDefault();
                        pickSuggestion(s);
                      }}
                      onMouseEnter={() => setHighlightIdx(i)}
                    >
                      <span className="user-suggest-name">{s.name}</span>
                      <button
                        type="button"
                        className="user-suggest-profile"
                        title={t("profile:viewProfile")}
                        onMouseDown={(e) => {
                          e.preventDefault();
                          e.stopPropagation();
                          useProfileModalStore.getState().open(s.id);
                        }}
                      >
                        <Icon name="user" size={13} />
                      </button>
                    </div>
                  ))}
                </div>
              )}
            </div>
            {error && <div className="m-error">{error}</div>}
          </div>
          <div className="m-foot">
            <button type="button" onClick={onClose} disabled={submitting}>
              {t("common:cancel")}
            </button>
            <button
              type="submit"
              className="primary"
              disabled={submitting || username.trim().length === 0}
            >
              {submitting ? t("addFriend.sending") : t("addFriend.send")}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};

export default AddFriendModal;
