import React from "react";
import { useTranslation } from "react-i18next";
import type { Server } from "../../../../shared/types/server";
import { openReport } from "../../../moderation/store/reportModalStore";


interface DangerSectionProps {
  server: Server;
  isOwner: boolean;
  onClose: () => void;
  onDeleteServer: (server: Server) => void;
}

export const DangerSection: React.FC<DangerSectionProps> = ({
  server,
  isOwner,
  onClose,
  onDeleteServer,
}) => {
  const { t } = useTranslation(["serverSettings", "common", "modals", "report"]);
  return (
            <>
              <h1>{t("nav.danger")}</h1>
              <p className="lead">{t("dangerLead")}</p>
              <div className="ssv-danger-card">
                <div className="ssv-danger-info">
                  <div className="ssv-danger-title">
                    {isOwner ? t("deleteServer") : t("leaveServer")}
                  </div>
                  <div className="ssv-danger-sub">
                    {isOwner ? t("deleteServerHint") : t("leaveServerHint")}
                  </div>
                </div>
                <button
                  type="button"
                  className="ssv-btn danger"
                  onClick={() => {
                    onClose();
                    onDeleteServer(server);
                  }}
                >
                  {isOwner ? t("common:delete") : t("leave")}
                </button>
              </div>
              {!isOwner && (
                <div className="ssv-danger-card">
                  <div className="ssv-danger-info">
                    <div className="ssv-danger-title">{t("report:titleServer")}</div>
                    <div className="ssv-danger-sub">{t("reportServerHint")}</div>
                  </div>
                  <button
                    type="button"
                    className="ssv-btn"
                    onClick={() => {
                      onClose();
                      openReport({
                        type: "server",
                        id: server.id,
                        name: server.name,
                      });
                    }}
                  >
                    {t("report:action")}
                  </button>
                </div>
              )}
            </>
  );
};

export default DangerSection;
