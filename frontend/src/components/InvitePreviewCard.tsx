import React from "react";
import { useTranslation } from "react-i18next";
import type { TFunction } from "i18next";
import type { InvitePreview, InviteStatus } from "../types/server";

export function inviteStatusMessage(status: InviteStatus, t: TFunction): string {
  switch (status) {
    case "expired":
      return t("serverModal.inviteExpired");
    case "exhausted":
      return t("serverModal.inviteExhausted");
    case "revoked":
      return t("serverModal.inviteRevoked");
    default:
      return "";
  }
}

const InvitePreviewCard: React.FC<{
  server: NonNullable<InvitePreview["server"]>;
}> = ({ server }) => {
  const { t } = useTranslation("modals");
  const initial = (server.name.trim()[0] || "?").toUpperCase();
  return (
    <div className="srvm-preview">
      <div
        className="srvm-preview-ava"
        style={{ background: server.accentColor ?? "var(--accent)" }}
      >
        {initial}
      </div>
      <div className="srvm-preview-main">
        <div className="srvm-preview-name">{server.name}</div>
        <div className="srvm-preview-meta">
          {t("serverModal.members", { count: server.memberCount })}
        </div>
        {server.description && (
          <div className="srvm-preview-desc">{server.description}</div>
        )}
      </div>
    </div>
  );
};

export default InvitePreviewCard;
