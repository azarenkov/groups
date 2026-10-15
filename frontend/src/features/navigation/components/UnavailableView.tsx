import React from "react";
import { useTranslation } from "react-i18next";

import Icon from "../../../shared/ui/Icon";

const UnavailableView: React.FC<{ onGoHome: () => void }> = ({ onGoHome }) => {
  const { t } = useTranslation("chat");
  return (
    <div className="empty-state" role="status">
      <div className="icon-wrap">
        <Icon name="lock" size={22} />
      </div>
      <h3>{t("unavailableTitle")}</h3>
      <p>{t("unavailableHint")}</p>
      <button type="button" className="btn primary" onClick={onGoHome}>
        {t("unavailableHome")}
      </button>
    </div>
  );
};

export default UnavailableView;
