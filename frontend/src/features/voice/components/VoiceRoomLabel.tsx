import React from "react";
import Icon from "../../../shared/ui/Icon";

interface VoiceRoomLabelProps {
  communityName?: string;
  channelName: string;
  overlay?: boolean;
}

const VoiceRoomLabel: React.FC<VoiceRoomLabelProps> = ({ communityName, channelName, overlay = false }) => (
  <div className={"vrs-label-wrap" + (overlay ? " overlay" : "")}>
    <div className="vrs-label">
      <Icon name="volume" size={16} />
      {communityName && (
        <>
          <span className="vrs-label-community">{communityName}</span>
          <span className="vrs-label-sep" aria-hidden="true">·</span>
        </>
      )}
      <span className="vrs-label-channel">{channelName}</span>
    </div>
  </div>
);

export default VoiceRoomLabel;
