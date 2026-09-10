import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import Icon from "./rift/Icon";
import { ApiError } from "../utils/apiError";
import { serverAPI } from "../api/server";
import type { InvitePreview } from "../types/server";
import { extractInviteToken } from "../utils/inviteLinks";
import InvitePreviewCard, { inviteStatusMessage } from "./InvitePreviewCard";
import "../styles/rift.css";

export type ServerModalView = "choose" | "create" | "join";

const ACCENTS = [
  "#0071e3",
  "#2f9e8f",
  "#4f9d5b",
  "#c68a2e",
  "#c1694a",
  "#c05a78",
  "#5566cc",
  "#6e6e73",
];

interface CreateServerModalProps {
  isOpen: boolean;
  initialView?: ServerModalView;
  onClose: () => void;
  onCreate: (
    name: string,
    description?: string,
    accentColor?: string,
    logoFile?: File,
  ) => Promise<void | { logoError?: string }>;
  onJoinByLink?: (token: string) => Promise<void>;
  initialToken?: string | null;
}

const MAX_SERVER_NAME_LENGTH = 100;
const MAX_DESCRIPTION_LENGTH = 500;

const CreateServerModal: React.FC<CreateServerModalProps> = ({
  isOpen,
  initialView = "choose",
  onClose,
  onCreate,
  onJoinByLink,
  initialToken = null,
}) => {
  const { t } = useTranslation(["modals", "common"]);
  const [view, setView] = useState<ServerModalView>(initialView);
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [accentIdx, setAccentIdx] = useState(0);
  const [linkInput, setLinkInput] = useState("");
  const [preview, setPreview] = useState<InvitePreview | null>(null);
  const [previewToken, setPreviewToken] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [joining, setJoining] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [logoFile, setLogoFile] = useState<File | null>(null);
  const [logoPreview, setLogoPreview] = useState<string | null>(null);
  const [warning, setWarning] = useState<string | null>(null);
  const logoInputRef = React.useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (isOpen) {
      setView(initialView);
      setError(null);
    }
  }, [isOpen, initialView]);

  const loadPreview = React.useCallback(async (raw: string) => {
    const token = extractInviteToken(raw);
    if (!token) {
      setError(t("serverModal.linkInvalid"));
      return;
    }
    setJoining(true);
    setError(null);
    setPreview(null);
    setPreviewToken(null);
    try {
      const res = await serverAPI.previewInvite(token);
      setPreview(res);
      setPreviewToken(token);
      if (res.status !== "active") setError(inviteStatusMessage(res.status, t));
    } catch (err) {
      const apiErr = err instanceof ApiError ? err : null;
      setError(
        apiErr?.code === "INVITE_NOT_FOUND"
          ? t("serverModal.inviteNotFound")
          : err instanceof Error
            ? err.message
            : t("common:error", { defaultValue: "Error" }),
      );
    } finally {
      setJoining(false);
    }
  }, [t]);

  useEffect(() => {
    if (!isOpen || !initialToken) return;
    setView("join");
    setLinkInput(initialToken);
    void loadPreview(initialToken);
  }, [isOpen, initialToken, loadPreview]);

  useEffect(() => {
    if (!isOpen) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [isOpen, onClose]);

  const trimmed = name.trim();
  const initial = (trimmed[0] || "?").toUpperCase();
  const accent = ACCENTS[accentIdx];

  const resetAll = () => {
    setName("");
    setDescription("");
    setAccentIdx(0);
    setLinkInput("");
    setPreview(null);
    setPreviewToken(null);
    setError(null);
    setWarning(null);
    setLogoFile(null);
    setLogoPreview((prev) => {
      if (prev) URL.revokeObjectURL(prev);
      return null;
    });
  };

  const handleClose = () => {
    if (isLoading || joining) return;
    resetAll();
    onClose();
  };

  const goTo = (v: ServerModalView) => {
    setError(null);
    setView(v);
  };

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!trimmed) return;
    setIsLoading(true);
    setError(null);
    try {
      const result = await onCreate(
        trimmed,
        description.trim() || undefined,
        accent,
        logoFile ?? undefined,
      );
      if (result && result.logoError) {
        setLogoFile(null);
        setLogoPreview((prev) => {
          if (prev) URL.revokeObjectURL(prev);
          return null;
        });
        setWarning(result.logoError);
        return;
      }
      resetAll();
      onClose();
    } catch (err) {
      const apiErr = err instanceof ApiError ? err : null;
      if (apiErr?.code === "SERVER_NAME_NOT_ALLOWED") {
        setError(t("createServer.nameNotAllowed"));
      } else if (apiErr?.code === "SERVER_DESCRIPTION_NOT_ALLOWED") {
        setError(t("createServer.descriptionNotAllowed"));
      } else if (apiErr?.code === "MODERATION_UNAVAILABLE") {
        setError(t("createServer.moderationUnavailable"));
      } else {
        setError(
          err instanceof Error
            ? err.message
            : t("common:error", { defaultValue: "Error" }),
        );
      }
    } finally {
      setIsLoading(false);
    }
  };

  const handleJoin = async () => {
    if (!previewToken || !onJoinByLink) return;
    setJoining(true);
    setError(null);
    try {
      await onJoinByLink(previewToken);
      resetAll();
      onClose();
    } catch (err) {
      const apiErr = err instanceof ApiError ? err : null;
      const known: Record<string, string> = {
        INVITE_NOT_FOUND: t("serverModal.inviteNotFound"),
        INVITE_EXPIRED: t("serverModal.inviteExpired"),
        INVITE_EXHAUSTED: t("serverModal.inviteExhausted"),
        INVITE_REVOKED: t("serverModal.inviteRevoked"),
      };
      setError(
        (apiErr?.code ? known[apiErr.code] : undefined) ??
          (err instanceof Error
            ? err.message
            : t("common:error", { defaultValue: "Error" })),
      );
    } finally {
      setJoining(false);
    }
  };

  if (!isOpen) return null;

  const closeBtn = (
    <button
      type="button"
      className="srvm-icon-btn"
      title={t("close", { ns: "common" })}
      onClick={handleClose}
    >
      <Icon name="x" size={16} />
    </button>
  );

  const errorBox = error && <div className="srvm-error">{error}</div>;

  if (warning) {
    return (
      <div className="modal-overlay" onMouseDown={handleClose}>
        <div className="srvm" onMouseDown={(e) => e.stopPropagation()}>
          <div className="srvm-head">
            <span className="srvm-head-spacer" />
            <h4 className="srvm-head-title">{t("createServer.title")}</h4>
            {closeBtn}
          </div>
          <div className="srvm-body">
            <div className="srvm-error">{warning}</div>
          </div>
          <div className="srvm-foot">
            <button type="button" className="srvm-btn primary" onClick={handleClose}>
              {t("close", { ns: "common" })}
            </button>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="modal-overlay" onMouseDown={handleClose}>
      {view === "choose" && (
        <div
          className="srvm srvm-choose"
          onMouseDown={(e) => e.stopPropagation()}
        >
          <div className="srvm-choose-close">{closeBtn}</div>
          <div className="srvm-choose-body">
            <h3 className="srvm-title">{t("serverModal.addTitle")}</h3>
            <p className="srvm-sub">{t("serverModal.addSub")}</p>
            <button
              type="button"
              className="srvm-option"
              onClick={() => goTo("create")}
            >
              <span className="srvm-option-icon accent">
                <Icon name="plus" size={22} />
              </span>
              <span className="srvm-option-body">
                <span className="srvm-option-title">
                  {t("serverModal.createOwn")}
                </span>
                <span className="srvm-option-sub">
                  {t("serverModal.createOwnSub")}
                </span>
              </span>
              <Icon name="chev-left" size={18} className="srvm-option-chev" />
            </button>
            <button
              type="button"
              className="srvm-option"
              onClick={() => goTo("join")}
            >
              <span className="srvm-option-icon">
                <Icon name="link" size={22} />
              </span>
              <span className="srvm-option-body">
                <span className="srvm-option-title">
                  {t("serverModal.haveInvite")}
                </span>
                <span className="srvm-option-sub">
                  {t("serverModal.haveInviteSub")}
                </span>
              </span>
              <Icon name="chev-left" size={18} className="srvm-option-chev" />
            </button>
          </div>
        </div>
      )}

      {view === "create" && (
        <form
          className="srvm srvm-create"
          onMouseDown={(e) => e.stopPropagation()}
          onSubmit={handleCreate}
        >
          <div className="srvm-head">
            <h4 className="srvm-head-title">{t("createServer.title")}</h4>
            {closeBtn}
          </div>
          <div className="srvm-body">
            <p className="srvm-sub">{t("serverModal.createBlurb")}</p>
            <div className="srvm-identity-row">
              <input
                ref={logoInputRef}
                type="file"
                accept="image/png,image/jpeg,image/webp"
                hidden
                onChange={(e) => {
                  const f = e.target.files?.[0];
                  if (f) {
                    setLogoFile(f);
                    setLogoPreview((prev) => {
                      if (prev) URL.revokeObjectURL(prev);
                      return URL.createObjectURL(f);
                    });
                  }
                  e.target.value = "";
                }}
              />
              <div
                className={"srvm-avatar" + (trimmed ? " named" : "")}
                style={{
                  background: accent,
                  position: "relative",
                  cursor: "pointer",
                }}
                role="button"
                tabIndex={0}
                title={t("serverModal.logoPick")}
                onClick={() => logoInputRef.current?.click()}
                onKeyDown={(e) => {
                  if (e.key === "Enter" || e.key === " ") {
                    e.preventDefault();
                    logoInputRef.current?.click();
                  }
                }}
              >
                {initial}
                {logoPreview && (
                  <img className="avatar-img" src={logoPreview} alt="" />
                )}
                <span className="edit-badge srvm-avatar-badge">
                  <Icon name="camera" size={11} />
                </span>
              </div>
              <div className="srvm-field">
                <label>{t("createServer.nameLabel")}</label>
                <input
                  autoFocus
                  value={name}
                  maxLength={MAX_SERVER_NAME_LENGTH}
                  placeholder={t("serverModal.namePlaceholder")}
                  disabled={isLoading}
                  onChange={(e) => setName(e.target.value)}
                />
              </div>
            </div>
            <div className="srvm-field">
              <label>
                {t("serverModal.descLabel")}{" "}
                <span className="srvm-optional">
                  {t("serverModal.descOptional")}
                </span>
              </label>
              <textarea
                rows={3}
                value={description}
                maxLength={MAX_DESCRIPTION_LENGTH}
                placeholder={t("serverModal.descPlaceholder")}
                disabled={isLoading}
                onChange={(e) => setDescription(e.target.value)}
              />
            </div>
            <div className="srvm-field">
              <label>{t("serverModal.accentLabel")}</label>
              <div className="srvm-accents">
                {ACCENTS.map((c, i) => (
                  <button
                    key={c}
                    type="button"
                    className={"srvm-accent" + (accentIdx === i ? " selected" : "")}
                    style={{ background: c, color: c }}
                    onClick={() => setAccentIdx(i)}
                  >
                    {accentIdx === i && <Icon name="check" size={15} />}
                  </button>
                ))}
              </div>
            </div>
            {errorBox}
          </div>
          <div className="srvm-foot">
            <button
              type="button"
              className="srvm-btn"
              disabled={isLoading}
              onClick={handleClose}
            >
              {t("common:cancel")}
            </button>
            <button
              type="submit"
              className="srvm-btn primary"
              disabled={isLoading || !trimmed}
            >
              {isLoading ? (
                <span className="loading-spinner" />
              ) : (
                t("createServer.create")
              )}
            </button>
          </div>
        </form>
      )}

      {view === "join" && (
        <div
          className="srvm srvm-join"
          onMouseDown={(e) => e.stopPropagation()}
        >
          <div className="srvm-head">
            <h4 className="srvm-head-title">{t("serverModal.joinTitle")}</h4>
            {closeBtn}
          </div>
          <div className="srvm-body">
            <p className="srvm-sub">{t("serverModal.joinBlurb")}</p>
            <div className="srvm-field">
              <label>{t("serverModal.linkLabel")}</label>
              <input
                autoFocus
                className="srvm-code-input"
                value={linkInput}
                placeholder="https://rift.app/join/…"
                disabled={joining}
                onChange={(e) => {
                  setLinkInput(e.target.value);
                  setPreview(null);
                  setPreviewToken(null);
                }}
                onKeyDown={(e) => {
                  if (e.key === "Enter") {
                    e.preventDefault();
                    if (preview?.status === "active") void handleJoin();
                    else void loadPreview(linkInput);
                  }
                }}
              />
            </div>
            {preview?.server ? (
              <InvitePreviewCard server={preview.server} />
            ) : (
              <div className="srvm-hint">
                <Icon name="info" size={15} />
                <span>
                  {t("serverModal.linkHint")}{" "}
                  <span className="srvm-hint-mono">https://rift.app/join/x7Kd9m…</span>
                </span>
              </div>
            )}
            {errorBox}
          </div>
          <div className="srvm-foot">
            {preview?.status === "active" ? (
              <button
                type="button"
                className="srvm-btn primary"
                disabled={joining}
                onClick={handleJoin}
              >
                {joining ? (
                  <span className="loading-spinner" />
                ) : (
                  t("serverModal.joinCta")
                )}
              </button>
            ) : (
              <button
                type="button"
                className="srvm-btn primary"
                disabled={joining || !linkInput.trim()}
                onClick={() => void loadPreview(linkInput)}
              >
                {joining ? (
                  <span className="loading-spinner" />
                ) : (
                  t("serverModal.continue")
                )}
              </button>
            )}
          </div>
        </div>
      )}
    </div>
  );
};

export default CreateServerModal;
