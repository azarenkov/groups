import { voiceHandlers } from "../voiceHandlers";
import { useCallStore } from "../../store/callStore";
import { useStreamCeilingStore } from "../../../subscription/store/streamCeilingStore";

const mockSetScreenShareEnabled = jest.fn();
let mockCurrentRoom: string | null = "room";

jest.mock("../../lib/livekit", () => ({
  getLiveKitService: () => ({ setScreenShareEnabled: mockSetScreenShareEnabled, getRoomName: () => mockCurrentRoom }),
}));

beforeEach(() => {
  mockCurrentRoom = "room";
  mockSetScreenShareEnabled.mockImplementation(() => Promise.resolve());
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
  expect(mockSetScreenShareEnabled).toHaveBeenCalledWith(false);
  expect(useStreamCeilingStore.getState().ceiling).toEqual({
    width: 1280,
    height: 720,
    frameRate: 60,
    maxBitrate: 4_000_000,
  });
  expect(useCallStore.getState().callError).toBeTruthy();
});

it("ignores a limit for a room the user has already left", () => {
  mockCurrentRoom = "other";
  voiceHandlers["voice.stream_limited"]!({
    roomId: "room",
    width: 1280,
    height: 720,
    frameRate: 60,
    maxBitrate: 4_000_000,
  });
  expect(mockSetScreenShareEnabled).not.toHaveBeenCalled();
  expect(useStreamCeilingStore.getState().ceiling).toBeNull();
  expect(useCallStore.getState().callError).toBeNull();
});
