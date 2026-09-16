import React from "react";
import { useTranslation } from "react-i18next";


export const AuthLegal: React.FC = () => {
  const { t } = useTranslation("auth");
  return (
    <div className="auth-legal">
      <span>© 2026 Rift</span>
      <button type="button" className="auth-link" onClick={() => {}}>{t("privacy")}</button>
      <button type="button" className="auth-link" onClick={() => {}}>{t("terms")}</button>
      <button type="button" className="auth-link" onClick={() => {}}>{t("status")}</button>
    </div>
  );
};
