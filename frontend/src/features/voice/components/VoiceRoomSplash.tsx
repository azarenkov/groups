import React from "react";
import { useTranslation } from "react-i18next";
import Icon from "../../../shared/ui/Icon";
import VoiceRoomLabel from "./VoiceRoomLabel";

interface VoiceRoomSplashProps {
  communityName?: string;
  channelName: string;
  onConnect: () => void;
  onInvite?: () => void;
}

const VoiceRoomSplash: React.FC<VoiceRoomSplashProps> = ({
  communityName,
  channelName,
  onConnect,
  onInvite,
}) => {
  const { t } = useTranslation("voice");
  return (
    <div className="vrs">
      <VoiceRoomLabel communityName={communityName} channelName={channelName} />
      <div className="vrs-center">
        <div className="vrs-motif">
          <span className="vrs-ripple" />
          <span className="vrs-ripple" />
          <span className="vrs-ripple" />
          <div className="vrs-core">
            <Icon name="mic" size={30} />
          </div>
        </div>
        <h3 className="vrs-title">{t("splashTitle")}</h3>
        <p className="vrs-sub">{t("splashSub")}</p>
        <div className="vrs-actions">
          <button type="button" className="vrs-connect" onClick={onConnect}>
            <Icon name="mic" size={18} />
            {t("connect")}
          </button>
          {onInvite && (
            <button type="button" className="vrs-invite" onClick={onInvite}>
              <Icon name="user-plus" size={15} />
              {t("splashInvite")}
            </button>
          )}
        </div>
      </div>
    </div>
  );
};

export default VoiceRoomSplash;
