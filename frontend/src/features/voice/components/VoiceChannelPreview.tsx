import React from "react";
import { useTranslation } from "react-i18next";
import Icon from "../../../shared/ui/Icon";
import { UserAvatar } from "../../../shared/ui/Avatar";
import { colorForId } from "../../../shared/lib/avatar";
import VoiceRoomSplash from "./VoiceRoomSplash";
import type { ChannelParticipant } from "../../../shared/types/channel";

interface VoiceChannelPreviewProps {
  channelName: string;
  participants: ChannelParticipant[];
  onConnect: () => void;
  onInvite: () => void;
}

const VoiceChannelPreview: React.FC<VoiceChannelPreviewProps> = ({
  channelName,
  participants,
  onConnect,
  onInvite,
}) => {
  const { t: tVoice } = useTranslation("voice");
  const sizeClass =
    participants.length === 1
      ? "solo"
      : participants.length <= 4
        ? "row"
        : "grid";
  return (
    <div className="voice-stage-wrap">
      <header className="main-header">
        <span className="ch-ico">
          <Icon name="volume" size={16} />
        </span>
        <span className="ch-name">{channelName}</span>
        <span className="sep" />
        <span className="topic">{tVoice("disconnected")}</span>
      </header>
      <div className="voice-preview">
        {participants.length === 0 ? (
          <VoiceRoomSplash
            channelName={channelName}
            onConnect={onConnect}
            onInvite={onInvite}
          />
        ) : (
          <div className={`voice-stage-orbs ${sizeClass}`}>
            {participants.map((p) => (
              <div key={p.id} className="voice-stage-orb-cell">
                <UserAvatar
                  userId={p.id}
                  name={p.name}
                  className="voice-stage-orb"
                  style={
                    {
                      "--orb-color": colorForId(p.id),
                    } as React.CSSProperties
                  }
                />
                <div className="voice-stage-orb-name">{p.name}</div>
              </div>
            ))}
          </div>
        )}
        {participants.length > 0 && (
          <button
            type="button"
            className="btn primary voice-preview-join"
            onClick={onConnect}
          >
            {tVoice("connect")}
          </button>
        )}
      </div>
    </div>
  );
};

export default VoiceChannelPreview;
