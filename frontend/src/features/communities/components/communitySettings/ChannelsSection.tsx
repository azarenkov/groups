import React from "react";
import { useTranslation } from "react-i18next";
import Icon from "../../../../shared/ui/Icon";
import { ApiError } from "../../../../shared/lib/apiError";
import type { Community } from "../../../../shared/types/community";
import type { Channel, ChannelKind, ChannelVisibility } from "../../../../shared/types/channel";


interface ChannelsSectionProps {
  community: Community;
  channels: Channel[];
  canManageChannels: boolean;
  onCreateChannel: (
    name: string,
    kind: ChannelKind,
    visibility: ChannelVisibility,
    password?: string,
  ) => Promise<void>;
  onDeleteChannel: (channel: Channel) => void;
}

export const ChannelsSection: React.FC<ChannelsSectionProps> = ({
  community,
  channels,
  canManageChannels,
  onCreateChannel,
  onDeleteChannel,
}) => {
  const { t } = useTranslation(["communitySettings", "common", "modals"]);
  const [newChName, setNewChName] = React.useState("");
  const [newChKind, setNewChKind] = React.useState<ChannelKind>("text");
  const [newChVisibility, setNewChVisibility] =
    React.useState<ChannelVisibility>("public");
  const [newChPassword, setNewChPassword] = React.useState("");
  const [newChBusy, setNewChBusy] = React.useState(false);
  const [newChError, setNewChError] = React.useState<string | null>(null);

  const submitNewChannel = async () => {
    const name = newChName.trim();
    if (!name) {
      setNewChError(t("channelNameRequired"));
      return;
    }
    if (newChVisibility === "private" && !newChPassword.trim()) {
      setNewChError(t("channelPasswordRequired"));
      return;
    }
    setNewChBusy(true);
    setNewChError(null);
    try {
      await onCreateChannel(
        name,
        newChKind,
        newChVisibility,
        newChVisibility === "private" ? newChPassword : undefined,
      );
      setNewChName("");
      setNewChPassword("");
      setNewChVisibility("public");
    } catch (e) {
      setNewChError(e instanceof ApiError ? e.message : t("common:error"));
    } finally {
      setNewChBusy(false);
    }
  };


  return (
            <>
              <h1>{t("nav.channels")}</h1>
              <p className="lead">
                {t("channelsLead", { count: channels.length, community: community.name })}
              </p>
              {canManageChannels && (
              <form
                className="ssv-invite-card"
                onSubmit={(e) => {
                  e.preventDefault();
                  void submitNewChannel();
                }}
              >
                <div className="ssv-invite-head">
                  <Icon name="hash" size={16} />
                  <span>{t("createChannel")}</span>
                </div>
                <div className="ssv-invite-row">
                  <input
                    className="ssv-input ssv-create-input"
                    placeholder={t("modals:createChannel.namePlaceholder")}
                    value={newChName}
                    maxLength={100}
                    disabled={newChBusy}
                    onChange={(e) => {
                      setNewChName(e.target.value);
                      setNewChError(null);
                    }}
                  />
                  <button
                    type="submit"
                    className="ssv-btn primary"
                    disabled={newChBusy}
                  >
                    <Icon name="plus" size={14} />
                    {t("createChannel")}
                  </button>
                </div>
                <div className="ssv-invite-opts">
                  <label className="ssv-invite-opt">
                    <span className="ssv-invite-opt-label">{t("channelKind")}</span>
                    <select
                      value={newChKind}
                      disabled={newChBusy}
                      onChange={(e) => setNewChKind(e.target.value as ChannelKind)}
                    >
                      <option value="text">{t("kindText")}</option>
                      <option value="voice">{t("kindVoice")}</option>
                    </select>
                  </label>
                  <label className="ssv-invite-opt">
                    <span className="ssv-invite-opt-label">{t("channelVisibility")}</span>
                    <select
                      value={newChVisibility}
                      disabled={newChBusy}
                      onChange={(e) =>
                        setNewChVisibility(e.target.value as ChannelVisibility)
                      }
                    >
                      <option value="public">{t("modals:createChannel.public")}</option>
                      <option value="private">{t("modals:createChannel.private")}</option>
                    </select>
                  </label>
                  {newChVisibility === "private" && (
                    <label className="ssv-invite-opt">
                      <span className="ssv-invite-opt-label">{t("channelPassword")}</span>
                      <input
                        className="ssv-create-password"
                        type="password"
                        value={newChPassword}
                        disabled={newChBusy}
                        onChange={(e) => {
                          setNewChPassword(e.target.value);
                          setNewChError(null);
                        }}
                      />
                    </label>
                  )}
                </div>
                {newChError && <div className="ssv-error">{newChError}</div>}
              </form>
              )}
              <div className="ssv-section-label">
                {t("channelList", { count: channels.length })}
              </div>
              <div className="ssv-card">
                {channels.length === 0 && (
                  <div className="ssv-empty">{t("noChannels")}</div>
                )}
                {channels.map((c, i) => (
                  <React.Fragment key={c.id}>
                    {i > 0 && <div className="ssv-divider" />}
                    <div className="ssv-row">
                      <span className="ssv-channel-ico">
                        <Icon name={c.kind === "voice" ? "volume" : "hash"} size={15} />
                      </span>
                      <div className="ssv-row-main">
                        <div className="ssv-row-name">
                          {c.name}
                          {c.isDefault && (
                            <span className="ssv-default-badge">{t("defaultChannelBadge")}</span>
                          )}
                        </div>
                        <div className="ssv-row-sub">
                          {c.kind === "voice" ? t("kindVoice") : t("kindText")}
                          {c.visibility === "private" && ` · ${t("privateChannel")}`}
                        </div>
                        {c.isDefault && (
                          <div className="ssv-row-note">{t("defaultChannelNote")}</div>
                        )}
                      </div>
                      {c.visibility === "private" && (
                        <span className="ssv-tag">
                          <Icon name="lock" size={11} />
                        </span>
                      )}
                      {canManageChannels && !c.isDefault && (
                        <button
                          type="button"
                          className="ssv-row-delete"
                          title={t("deleteChannel")}
                          onClick={() => onDeleteChannel(c)}
                        >
                          <Icon name="trash" size={14} />
                        </button>
                      )}
                    </div>
                  </React.Fragment>
                ))}
              </div>
            </>
  );
};

export default ChannelsSection;
