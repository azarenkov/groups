import React from "react";

import { formatCallDuration } from "../../chat/lib/messageFormat";

const elapsedSeconds = (startedAt: string) =>
  Math.max(0, Math.floor((Date.now() - Date.parse(startedAt)) / 1000));

const VoiceSessionTimer: React.FC<{ startedAt: string; className?: string }> = ({
  startedAt,
  className,
}) => {
  const [seconds, setSeconds] = React.useState(() => elapsedSeconds(startedAt));
  React.useEffect(() => {
    setSeconds(elapsedSeconds(startedAt));
    const id = setInterval(() => setSeconds(elapsedSeconds(startedAt)), 1000);
    return () => clearInterval(id);
  }, [startedAt]);
  return (
    <time className={className} dateTime={startedAt}>
      {formatCallDuration(seconds)}
    </time>
  );
};

export default VoiceSessionTimer;
