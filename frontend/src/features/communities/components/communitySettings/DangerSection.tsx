import React from "react";
import { useTranslation } from "react-i18next";
import type { Community } from "../../../../shared/types/community";
import { openReport } from "../../../moderation/store/reportModalStore";


interface DangerSectionProps {
  community: Community;
  isOwner: boolean;
  onClose: () => void;
  onDeleteCommunity: (community: Community) => void;
}

export const DangerSection: React.FC<DangerSectionProps> = ({
  community,
  isOwner,
  onClose,
  onDeleteCommunity,
}) => {
  const { t } = useTranslation(["communitySettings", "common", "modals", "report"]);
  return (
            <>
              <h1>{t("nav.danger")}</h1>
              <p className="lead">{t("dangerLead")}</p>
              <div className="ssv-danger-card">
                <div className="ssv-danger-info">
                  <div className="ssv-danger-title">
                    {isOwner ? t("deleteCommunity") : t("leaveCommunity")}
                  </div>
                  <div className="ssv-danger-sub">
                    {isOwner ? t("deleteCommunityHint") : t("leaveCommunityHint")}
                  </div>
                </div>
                <button
                  type="button"
                  className="ssv-btn danger"
                  onClick={() => {
                    onClose();
                    onDeleteCommunity(community);
                  }}
                >
                  {isOwner ? t("common:delete") : t("leave")}
                </button>
              </div>
              {!isOwner && (
                <div className="ssv-danger-card">
                  <div className="ssv-danger-info">
                    <div className="ssv-danger-title">{t("report:titleCommunity")}</div>
                    <div className="ssv-danger-sub">{t("reportCommunityHint")}</div>
                  </div>
                  <button
                    type="button"
                    className="ssv-btn"
                    onClick={() =>
                      openReport({
                        type: "community",
                        id: community.id,
                        name: community.name,
                      })
                    }
                  >
                    {t("report:action")}
                  </button>
                </div>
              )}
            </>
  );
};

export default DangerSection;
