import { mediaAPI } from "../../api/media";
import { installMediaStateFlushOnUnload, resetMediaStateSync, syncMediaState } from "../mediaStateSync";

jest.mock("../../api/media", () => ({ mediaAPI: { save: jest.fn(), get: jest.fn() } }));

const save = mediaAPI.save as jest.Mock;

beforeEach(() => {
  jest.useFakeTimers();
  resetMediaStateSync();
  save.mockResolvedValue({ micMuted: false, deafened: false });
});

afterEach(() => jest.useRealTimers());

it("saves only the last change once the toggles settle", () => {
  syncMediaState({ micMuted: true, deafened: false });
  syncMediaState({ micMuted: false, deafened: true });
  jest.advanceTimersByTime(300);
  expect(save).toHaveBeenCalledTimes(1);
  expect(save).toHaveBeenCalledWith({ micMuted: false, deafened: true }, { keepalive: false });
});

it("sends a change that has not been saved yet when the tab goes away", () => {
  installMediaStateFlushOnUnload();
  syncMediaState({ micMuted: true, deafened: true });
  window.dispatchEvent(new Event("pagehide"));
  expect(save).toHaveBeenCalledWith({ micMuted: true, deafened: true }, { keepalive: true });
  jest.advanceTimersByTime(300);
  expect(save).toHaveBeenCalledTimes(1);
});
