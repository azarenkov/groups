import React from "react";
import { useTranslation } from "react-i18next";
import Icon from "../../../../shared/ui/Icon";


export const ResetSuccess: React.FC<{ onContinue: () => void }> = ({ onContinue }) => {
  const { t } = useTranslation("auth");
  return (
    <>
      <div className="auth-hero success">
        <Icon name="check" size={28} />
      </div>
      <h1>{t("passwordUpdated")}</h1>
      <p className="sub">{t("passwordUpdatedHint")}</p>
      <button type="button" className="btn primary" onClick={onContinue}>
        {t("continueToSignIn")}
      </button>
    </>
  );
};
