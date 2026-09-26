import React from "react";

import Icon from "../../../shared/ui/Icon";
import { useContextMenuStore } from "../../../shared/ui/contextMenu/contextMenuStore";
import { REMOTE_VOLUME_MAX, REMOTE_VOLUME_PLAIN_MAX, getLiveKitService } from "../lib/livekit";
import { audibleVolume } from "../lib/audibleVolume";

const MuteForMeItem: React.FC<{ userId: string; muteLabel: string; unmuteLabel: string }> = ({
  userId,
  muteLabel,
  unmuteLabel,
}) => {
  const [muted, setMuted] = React.useState(() => getLiveKitService().getRemoteVolume(userId) === 0);

  React.useEffect(
    () =>
      getLiveKitService().subscribeRemoteVolume((who, volume) => {
        if (who === userId) setMuted(volume === 0);
      }),
    [userId],
  );

  return (
    <button
      type="button"
      role="menuitem"
      className="ctx-item"
      onClick={() => {
        const service = getLiveKitService();
        const current = service.getRemoteVolume(userId);
        useContextMenuStore.getState().close();
        if (current === 0) {
          const max =
            (service.supportsRemoteBoost(userId) ? REMOTE_VOLUME_MAX : REMOTE_VOLUME_PLAIN_MAX) / 100;
          service.setRemoteVolume(userId, Math.min(audibleVolume(userId), max));
        } else {
          service.setRemoteVolume(userId, 0);
        }
      }}
    >
      <span className="ctx-ico">
        <Icon name={muted ? "volume" : "speaker-off"} size={15} />
      </span>
      <span className="ctx-label">{muted ? unmuteLabel : muteLabel}</span>
    </button>
  );
};

export default MuteForMeItem;
