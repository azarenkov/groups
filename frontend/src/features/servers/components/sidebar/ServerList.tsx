import React from "react";
import { useTranslation } from "react-i18next";
import Icon from "../../../../shared/ui/Icon";
import { ServerLogo } from "../../../../shared/ui/Avatar";
import { colorForId } from "../../../../shared/lib/avatar";
import type { Server } from "../../../../shared/types/server";
import type { Channel } from "../../../../shared/types/channel";
import type { SidebarVoiceMember } from "./types";

interface ServerListProps {
  servers: Server[];
  isFiltering: boolean;
  channelsByServer: Record<string, Channel[]>;
  voiceMembersByChannel?: Record<string, SidebarVoiceMember[]>;
  unreadFor: (serverId: string) => number;
  visibleChannelsFor: (serverId: string) => Channel[];
  onCreateServer: () => void;
  onJoinServer: () => void;
  onPick: (server: Server) => void;
  renderChannel: (server: Server, channel: Channel) => React.ReactNode;
}

const ServerList: React.FC<ServerListProps> = ({
  servers,
  isFiltering,
  channelsByServer,
  voiceMembersByChannel,
  unreadFor,
  visibleChannelsFor,
  onCreateServer,
  onJoinServer,
  onPick,
  renderChannel,
}) => {
  const { t } = useTranslation(["sidebar", "common"]);
  return (
    <div className="sb-view">
      {!isFiltering && (
        <>
          <button type="button" className="sb-add-row" onClick={onCreateServer}>
            <span className="sb-add-row-ico">
              <Icon name="plus" size={13} />
            </span>
            <span>{t("createServer")}</span>
          </button>
          <button
            type="button"
            className="sb-add-row secondary"
            onClick={onJoinServer}
          >
            <span className="sb-add-row-ico">
              <Icon name="logout" size={13} />
            </span>
            <span>{t("joinServer")}</span>
          </button>
        </>
      )}
      <div className="sb-vhead">
        <span className="sb-vhead-label">{t("servers")}</span>
      </div>
      {servers.length === 0 && (
        <div className="sb-empty">
          {isFiltering ? t("noMatches") : t("noServers")}
        </div>
      )}
      {servers.map((server) => {
        const accent = server.accentColor ?? colorForId(server.id);
        const allServerChannels = channelsByServer[server.id] || [];
        const serverUnread = unreadFor(server.id);
        const hasVoiceActivity = allServerChannels.some(
          (c) =>
            c.kind === "voice" &&
            (voiceMembersByChannel?.[c.id]?.length ?? 0) > 0,
        );
        const channels = isFiltering ? visibleChannelsFor(server.id) : [];
        return (
          <div key={server.id}>
            <div
              role="button"
              tabIndex={0}
              className="sb-srv"
              onClick={() => onPick(server)}
              onKeyDown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  onPick(server);
                }
              }}
            >
              <ServerLogo
                serverId={server.id}
                name={server.name}
                className="sb-srv-ava"
                style={{ background: accent }}
              />
              <span className="sb-srv-name">{server.name}</span>
              {hasVoiceActivity && (
                <span
                  className="sb-server-voice-dot"
                  title="Someone is in voice"
                  aria-label="Voice activity"
                />
              )}
              {serverUnread > 0 && (
                <span className="sb-server-unread">{serverUnread}</span>
              )}
              <Icon name="chev" size={13} className="sb-srv-chev collapsed" />
            </div>
            {isFiltering && channels.length > 0 && (
              <div className="sb-srv-channels">
                {channels.map((c) => renderChannel(server, c))}
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
};

export default ServerList;
