import React from "react";

import Icon from "../../../shared/ui/Icon";
import {
  REMOTE_VOLUME_MAX,
  REMOTE_VOLUME_PLAIN_MAX,
  getLiveKitService,
} from "../lib/livekit";

const MemberVolumeSlider: React.FC<{ userId: string; label: string }> = ({ userId, label }) => {
  const service = getLiveKitService();
  const max = service.supportsRemoteBoost(userId) ? REMOTE_VOLUME_MAX : REMOTE_VOLUME_PLAIN_MAX;
  const [vol, setVol] = React.useState(() =>
    Math.min(max, Math.round(service.getRemoteVolume(userId) * 100)),
  );

  React.useEffect(
    () =>
      getLiveKitService().subscribeRemoteVolume((who, volume) => {
        if (who === userId) setVol(Math.round(volume * 100));
      }),
    [userId],
  );

  return (
    <div className="cs-menu-vol">
      <Icon name={vol === 0 ? "speaker-off" : vol < 40 ? "speaker-lo" : "speaker-hi"} size={15} />
      <input
        type="range"
        min={0}
        max={max}
        step={1}
        value={vol}
        aria-label={label}
        onChange={(e) => {
          const next = Number(e.target.value);
          setVol(next);
          getLiveKitService().setRemoteVolume(userId, next / 100);
        }}
        style={{ ["--pct" as unknown as string]: `${(vol / max) * 100}%` } as React.CSSProperties}
      />
      <span className="cs-menu-pct">{vol}%</span>
    </div>
  );
};

export default MemberVolumeSlider;
