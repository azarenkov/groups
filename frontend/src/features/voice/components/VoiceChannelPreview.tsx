import React from "react";
import { useTranslation } from "react-i18next";
import Icon from "../../../shared/ui/Icon";
import { UserAvatar } from "../../../shared/ui/Avatar";
import { colorForId } from "../../../shared/lib/avatar";
import VoiceRoomSplash from "./VoiceRoomSplash";
import VoiceSessionTimer from "./VoiceSessionTimer";
import { useVoiceSessionStore } from "../store/voiceSessionStore";
import { useSpeakingStore } from "../store/speakingStore";

export interface VoicePreviewParticipant {
  id: string;
  name: string;
  isMuted?: boolean;
  cameraOn?: boolean;
  screenOn?: boolean;
}

interface VoiceChannelPreviewProps {
  channelId: string;
  channelName: string;
  participants: VoicePreviewParticipant[];
  onConnect: () => void;
  onInvite: () => void;
}

const VoiceChannelPreview: React.FC<VoiceChannelPreviewProps> = ({
  channelId,
  channelName,
  participants,
  onConnect,
  onInvite,
}) => {
  const { t: tVoice } = useTranslation("voice");
  const startedAt = useVoiceSessionStore((st) => st.startedAt[channelId]);
  const speakingIds = useSpeakingStore((st) => st.ids);
  return (
    <div className="voice-stage-wrap">
      <div className="voice-preview">
        {participants.length === 0 ? (
          <VoiceRoomSplash
            channelName={channelName}
            onConnect={onConnect}
            onInvite={onInvite}
          />
        ) : (
          <div className="vrs vcp">
            <div className="vrs-label-wrap">
              <div className="vrs-label">
                <Icon name="volume" size={15} />
                <span>{tVoice("splashRoomLabel", { name: channelName })}</span>
              </div>
            </div>
            <div className="vcp-center">
              <div className="vcp-live">
                <span className="vcp-live-dot" />
                <span>{tVoice("previewLive")}</span>
                {startedAt && (
                  <>
                    <span className="vcp-live-sep" />
                    <VoiceSessionTimer startedAt={startedAt} className="vcp-live-time" />
                  </>
                )}
              </div>
              <h3 className="vcp-title">
                {tVoice("membersConnected", { count: participants.length })}
              </h3>
              <div className="vcp-cards">
                {participants.map((p) => (
                  <div
                    key={p.id}
                    className={"vcp-card" + (speakingIds.has(p.id) ? " speaking" : "")}
                  >
                    <UserAvatar
                      userId={p.id}
                      name={p.name}
                      className="vcp-ava"
                      style={{ "--orb-color": colorForId(p.id) } as React.CSSProperties}
                    />
                    <div className="vcp-name">{p.name}</div>
                    {(p.isMuted || p.cameraOn || p.screenOn) && (
                      <div className="vcp-flags">
                        {p.screenOn && <span className="vcp-flag live">LIVE</span>}
                        {p.cameraOn && (
                          <span className="vcp-flag">
                            <Icon name="camera" size={12} />
                          </span>
                        )}
                        {p.isMuted && (
                          <span className="vcp-flag muted">
                            <Icon name="mic-off" size={12} />
                          </span>
                        )}
                      </div>
                    )}
                  </div>
                ))}
              </div>
              <div className="vrs-actions">
                <button type="button" className="vrs-connect" onClick={onConnect}>
                  <Icon name="mic" size={18} />
                  {tVoice("connect")}
                </button>
                <button type="button" className="vrs-invite" onClick={onInvite}>
                  <Icon name="user-plus" size={15} />
                  {tVoice("splashInvite")}
                </button>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

export default VoiceChannelPreview;
