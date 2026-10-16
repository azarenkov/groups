import React from "react";
import { useTranslation } from "react-i18next";

import Icon from "../../../shared/ui/Icon";

export type UnavailableReason = "missing" | "failed";

const UnavailableView: React.FC<{ reason: UnavailableReason; onGoHome: () => void; onRetry: () => void }> = ({
  reason,
  onGoHome,
  onRetry,
}) => {
  const { t } = useTranslation("chat");
  const failed = reason === "failed";
  return (
    <div className="empty-state" role="status">
      <div className="icon-wrap">
        <Icon name={failed ? "refresh" : "lock"} size={22} />
      </div>
      <h3>{t(failed ? "unavailableFailedTitle" : "unavailableTitle")}</h3>
      <p>{t(failed ? "unavailableFailedHint" : "unavailableHint")}</p>
      {failed && (
        <button type="button" className="btn primary" onClick={onRetry}>
          {t("unavailableRetry")}
        </button>
      )}
      <button type="button" className={failed ? "btn" : "btn primary"} onClick={onGoHome}>
        {t("unavailableHome")}
      </button>
    </div>
  );
};

export default UnavailableView;
