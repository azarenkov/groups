import React from "react";
import { useTranslation } from "react-i18next";
import Icon from "../../../shared/ui/Icon";

interface NoTextChannelProps {
  serverName: string;
  onCreate: () => void;
}

const NoTextChannel: React.FC<NoTextChannelProps> = ({ serverName, onCreate }) => {
  const { t } = useTranslation("sidebar");
  return (
    <div className="empty-state" style={{ padding: 60 }}>
      <div className="icon-wrap" style={{ width: 64, height: 64 }}>
        <Icon name="hash" size={26} />
      </div>
      <h3>{t("noTextChannelsTitle")}</h3>
      <p>
        {serverName
          ? t("noTextChannelsBody", { server: serverName })
          : t("noTextChannelsBodyGeneric")}
      </p>
      <div style={{ marginTop: 20 }}>
        <button
          type="button"
          className="btn primary"
          onClick={onCreate}
          style={{ height: 38, padding: "0 18px", whiteSpace: "nowrap" }}
        >
          <Icon name="plus" size={14} />
          {t("createChannelCta")}
        </button>
      </div>
    </div>
  );
};

export default NoTextChannel;
