import { useEffect, useState } from "react";

import {
  getPushToTalkGlobalStatus,
  subscribePushToTalkGlobal,
  type PushToTalkGlobalStatus,
} from "../lib/pushToTalk";

export function usePushToTalkGlobalStatus(): PushToTalkGlobalStatus {
  const [status, setStatus] = useState<PushToTalkGlobalStatus>(
    getPushToTalkGlobalStatus,
  );

  useEffect(() => subscribePushToTalkGlobal(setStatus), []);

  return status;
}
