import React from "react";
import { useTranslation } from "react-i18next";
import Icon from "../../../../shared/ui/Icon";
import { UserAvatar } from "../../../../shared/ui/Avatar";
import type { Server } from "../../../../shared/types/server";
import type { Channel } from "../../../../shared/types/channel";
import type { SidebarVoiceMember } from "./types";
import { moderatorDeafened, type SidebarPermissions } from "./permissions";
import { moderateDeafen, moderateMute } from "./moderation";
import { useSpeakingStore } from "../../../voice/store/speakingStore";

interface ChannelRowProps {
  server: Server;
  channel: Channel;
  perms: SidebarPermissions;
  selectedChannelId: string | null;
  joinedChannelId: string | null;
  unreadByChannel: Record<string, number>;
  voiceMembers?: SidebarVoiceMember[];
  voiceMembersByChannel?: Record<string, SidebarVoiceMember[]>;
  currentUserId?: string;
  onPick: (server: Server, channel: Channel) => void;
  onDeleteChannel: (server: Server, channel: Channel) => void;
}

const ChannelRow: React.FC<ChannelRowProps> = ({
  server,
  channel,
  perms,
  selectedChannelId,
  joinedChannelId,
  unreadByChannel,
  voiceMembers,
  voiceMembersByChannel,
  currentUserId,
  onPick,
  onDeleteChannel,
}) => {
  const speakingIds = useSpeakingStore((state) => state.ids);
  const { t } = useTranslation(["sidebar", "common"]);
  return (
    <>
      <div
        role="button"
        tabIndex={0}
        className={
          "sb-item" +
          (selectedChannelId === channel.id ? " active" : "") +
          (joinedChannelId === channel.id ? " in-voice" : "") +
          (channel.kind === "text" && (unreadByChannel[channel.id] ?? 0) > 0
            ? " unread"
            : "")
        }
        onClick={() => onPick(server, channel)}
        onKeyDown={(e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            onPick(server, channel);
          }
        }}
        onContextMenu={(e) => {
          e.preventDefault();
          if (channel.isDefault) return;
          if (perms.canManageChannelsIn(server)) onDeleteChannel(server, channel);
        }}
      >
        <span className="sb-icon">
          <Icon name={channel.kind === "voice" ? "volume" : "hash"} size={14} />
        </span>
        <span className="sb-label">{channel.name}</span>
        {channel.isDefault && (
          <span className="sb-default-badge" title={t("defaultChannel")}>
            <Icon name="pin" size={10} />
          </span>
        )}
        {channel.kind === "text" && (unreadByChannel[channel.id] ?? 0) > 0 && (
          <span className="meta">{unreadByChannel[channel.id]}</span>
        )}
        {channel.kind === "voice" &&
          (voiceMembersByChannel?.[channel.id]?.length ?? 0) > 0 && (
            <span className="meta">
              {voiceMembersByChannel?.[channel.id]?.length}
            </span>
          )}
        {channel.visibility === "private" && (
          <span className="lock">
            <Icon name="lock" size={11} />
          </span>
        )}
      </div>
      {channel.kind === "voice" &&
        (() => {
          const base = voiceMembersByChannel?.[channel.id] ?? [];
          const isJoined = joinedChannelId === channel.id;
          let membersForChannel = base;
          if (isJoined && voiceMembers && voiceMembers.length > 0) {
            const merged = new Map(base.map((m) => [m.id, m]));
            for (const live of voiceMembers) {
              const existing = merged.get(live.id);
              merged.set(live.id, existing ? { ...existing, ...live } : live);
            }
            membersForChannel = Array.from(merged.values());
          }
          if (membersForChannel.length === 0) return null;
          return (
            <div className="sb-voice-members">
              {membersForChannel.map((m) => (
                <div key={m.id} className="sb-voice-member">
                  <UserAvatar
                    userId={m.id}
                    name={m.name}
                    className={
                      "sb-voice-member-ava" + (speakingIds.has(m.id) ? " speaking" : "")
                    }
                    style={{ background: m.color, display: "inline-flex" }}
                  />
                  <span className="sb-voice-member-name">{m.name}</span>
                  {m.isCameraOn && (
                    <span className="sb-voice-member-badge" title="Camera on">
                      <Icon name="camera" size={11} />
                    </span>
                  )}
                  {m.isSharingScreen && (
                    <span className="sb-voice-member-badge" title="Sharing screen">
                      <Icon name="monitor" size={11} />
                    </span>
                  )}
                  {m.isMuted && (
                    <span className="sb-voice-member-badge muted" title="Muted">
                      <Icon name="mic-off" size={11} />
                    </span>
                  )}
                  {m.isDeafened && (
                    <span className="sb-voice-member-badge muted" title="Deafened">
                      <Icon name="headphones-off" size={11} />
                    </span>
                  )}
                  {m.id !== currentUserId &&
                    perms.canMuteIn(server) &&
                    perms.outranksIn(server, m.id) && (
                    <button
                      type="button"
                      className="sb-voice-mute-btn"
                      title={
                        m.isMuted && m.mutedBy && m.mutedBy !== m.id
                          ? t("unmuteMember")
                          : t("muteMember")
                      }
                      onClick={(e) => {
                        e.stopPropagation();
                        moderateMute(server, channel.id, m);
                      }}
                    >
                      <Icon
                        name={
                          m.isMuted && m.mutedBy && m.mutedBy !== m.id
                            ? "mic"
                            : "mic-off"
                        }
                        size={12}
                      />
                    </button>
                  )}
                  {m.id !== currentUserId &&
                    perms.canDeafenIn(server) &&
                    perms.outranksIn(server, m.id) &&
                    perms.canLiftDeafenIn(server, m) && (
                    <button
                      type="button"
                      className="sb-voice-mute-btn"
                      title={moderatorDeafened(m) ? t("undeafenMember") : t("deafenMember")}
                      onClick={(e) => {
                        e.stopPropagation();
                        moderateDeafen(server, channel.id, m);
                      }}
                    >
                      <Icon
                        name={moderatorDeafened(m) ? "headphones" : "headphones-off"}
                        size={12}
                      />
                    </button>
                  )}
                </div>
              ))}
            </div>
          );
        })()}
    </>
  );
};

export default ChannelRow;
