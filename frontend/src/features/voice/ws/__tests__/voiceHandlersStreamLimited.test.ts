import { voiceHandlers } from "../voiceHandlers";
import { useCallStore } from "../../store/callStore";
import { useStreamCeilingStore } from "../../../subscription/store/streamCeilingStore";

const setScreenShareEnabled = jest.fn(() => Promise.resolve());

jest.mock("../../lib/livekit", () => ({
  getLiveKitService: () => ({ setScreenShareEnabled }),
}));

beforeEach(() => {
  setScreenShareEnabled.mockClear();
  useCallStore.setState({ callError: null });
  useStreamCeilingStore.setState({ ceiling: null });
});

it("stops the screen share, adopts the server ceiling and explains why", () => {
  voiceHandlers["voice.stream_limited"]!({
    roomId: "room",
    width: 1280,
    height: 720,
    frameRate: 60,
    maxBitrate: 4_000_000,
  });
  expect(setScreenShareEnabled).toHaveBeenCalledWith(false);
  expect(useStreamCeilingStore.getState().ceiling).toEqual({
    width: 1280,
    height: 720,
    frameRate: 60,
    maxBitrate: 4_000_000,
  });
  expect(useCallStore.getState().callError).toBeTruthy();
});
