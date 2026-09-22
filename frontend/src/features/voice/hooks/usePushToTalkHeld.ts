import { useEffect, useState } from "react";

import { isPushToTalkHeld, subscribePushToTalk } from "../lib/pushToTalk";

export function usePushToTalkHeld(): boolean {
  const [held, setHeld] = useState<boolean>(isPushToTalkHeld);

  useEffect(() => subscribePushToTalk(setHeld), []);

  return held;
}
