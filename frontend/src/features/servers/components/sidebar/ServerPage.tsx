import React from "react";
import { useTranslation } from "react-i18next";
import Icon from "../../../../shared/ui/Icon";
import { ServerLogo } from "../../../../shared/ui/Avatar";
import { colorForId } from "../../../../shared/lib/avatar";
import type { Server } from "../../../../shared/types/server";
import type { Channel, ChannelKind } from "../../../../shared/types/channel";
import type { SidebarPermissions } from "./permissions";

interface ServerPageProps {
  server: Server;
  channels: Channel[];
  perms: SidebarPermissions;
  onInviteServer: (server: Server) => void;
  onOpenServerSettings: (server: Server) => void;
  onAddChannel: (server: Server, kind: ChannelKind) => void;
  renderChannel: (server: Server, channel: Channel) => React.ReactNode;
}

const ServerPage: React.FC<ServerPageProps> = ({
  server,
  channels,
  perms,
  onInviteServer,
  onOpenServerSettings,
  onAddChannel,
  renderChannel,
}) => {
  const { t } = useTranslation(["sidebar", "common"]);
  const textChannels = [...channels.filter((c) => c.kind === "text")].sort(
    (a, b) => Number(!!b.isDefault) - Number(!!a.isDefault),
  );
  const voiceChannels = channels.filter((c) => c.kind === "voice");
  const accent = server.accentColor ?? colorForId(server.id);
  return (
    <div className="sb-view sb-server-page">
      <div className="sb-server-head">
        <ServerLogo
          serverId={server.id}
          name={server.name}
          className="sb-srv-ava"
          style={{ background: accent }}
        />
        <span className="sb-srv-name">{server.name}</span>
        {perms.canInviteIn(server) && (
          <button
            type="button"
            className="sb-section-add"
            title={t("inviteFriends")}
            onClick={() => onInviteServer(server)}
          >
            <Icon name="user-plus" size={14} />
          </button>
        )}
        <button
          type="button"
          className="sb-section-add"
          title={t("serverSettings")}
          onClick={() => onOpenServerSettings(server)}
        >
          <Icon name="settings" size={14} />
        </button>
      </div>
      <div className="sb-vhead">
        <span className="sb-vhead-label">{t("textChannels")}</span>
        {perms.canManageChannelsIn(server) && (
          <button
            type="button"
            className="sb-vhead-add"
            title={t("addChannel")}
            onClick={() => onAddChannel(server, "text")}
          >
            <Icon name="plus" size={14} />
          </button>
        )}
      </div>
      <div className="sb-srv-channels">
        {textChannels.map((c) => renderChannel(server, c))}
      </div>
      <div className="sb-vhead">
        <span className="sb-vhead-label">{t("voiceChannels")}</span>
        {perms.canManageChannelsIn(server) && (
          <button
            type="button"
            className="sb-vhead-add"
            title={t("addChannel")}
            onClick={() => onAddChannel(server, "voice")}
          >
            <Icon name="plus" size={14} />
          </button>
        )}
      </div>
      <div className="sb-srv-channels">
        {voiceChannels.map((c) => renderChannel(server, c))}
      </div>
    </div>
  );
};

export default ServerPage;
