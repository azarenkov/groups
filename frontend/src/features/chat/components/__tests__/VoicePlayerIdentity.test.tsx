import React from "react";
import { render } from "@testing-library/react";

import VoicePlayer from "../VoicePlayer";
import { useVoicePlaybackStore } from "../../store/voicePlaybackStore";

jest.mock("../../api/attachment", () => ({
  attachmentAPI: { download: jest.fn(() => new Promise(() => undefined)) },
}));

const voice = {
  id: "a1",
  fileName: "voice.ogg",
  contentType: "audio/ogg",
  sizeBytes: 10,
  createdAt: "2026-10-01T00:00:00Z",
  voice: { durationMs: 1000, waveform: [1, 2, 3] },
};

it("keeps the chat copy playing when a shared-panel copy goes away", () => {
  useVoicePlaybackStore.getState().setCurrent("m1");
  const { unmount } = render(
    <VoicePlayer messageId="m1" playbackId="shared:m1:a1" attachment={voice} isMine={false} listened showListenDot={false} />,
  );
  unmount();
  expect(useVoicePlaybackStore.getState().currentId).toBe("m1");
});
