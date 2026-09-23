import React from "react";
import { useTranslation } from "react-i18next";
import type { TFunction } from "i18next";
import type { InvitePreview, InviteStatus } from "../../../shared/types/community";

export function inviteStatusMessage(status: InviteStatus, t: TFunction): string {
  switch (status) {
    case "expired":
      return t("communityModal.inviteExpired");
    case "exhausted":
      return t("communityModal.inviteExhausted");
    case "revoked":
      return t("communityModal.inviteRevoked");
    default:
      return "";
  }
}

const InvitePreviewCard: React.FC<{
  community: NonNullable<InvitePreview["community"]>;
}> = ({ community }) => {
  const { t } = useTranslation("modals");
  const initial = (community.name.trim()[0] || "?").toUpperCase();
  return (
    <div className="srvm-preview">
      <div
        className="srvm-preview-ava"
        style={{ background: community.accentColor ?? "var(--accent)" }}
      >
        {initial}
      </div>
      <div className="srvm-preview-main">
        <div className="srvm-preview-name">{community.name}</div>
        <div className="srvm-preview-meta">
          {t("communityModal.members", { count: community.memberCount })}
        </div>
        {community.description && (
          <div className="srvm-preview-desc">{community.description}</div>
        )}
      </div>
    </div>
  );
};

export default InvitePreviewCard;
