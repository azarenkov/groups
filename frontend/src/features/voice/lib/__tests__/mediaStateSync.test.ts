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

it("keeps a change that failed to save and tries again", async () => {
  save.mockRejectedValueOnce(new Error("offline"));
  syncMediaState({ micMuted: true, deafened: false });
  jest.advanceTimersByTime(300);
  await Promise.resolve();
  await Promise.resolve();
  expect(save).toHaveBeenCalledTimes(1);
  jest.advanceTimersByTime(5000);
  expect(save).toHaveBeenCalledTimes(2);
  expect(save).toHaveBeenLastCalledWith({ micMuted: true, deafened: false }, { keepalive: false });
});

it("never retries an older state over a newer one", async () => {
  let failFirst: (reason: Error) => void = () => {};
  save.mockImplementationOnce(() => new Promise((_, reject) => (failFirst = reject)));
  syncMediaState({ micMuted: false, deafened: false });
  jest.advanceTimersByTime(300);
  syncMediaState({ micMuted: true, deafened: false });
  jest.advanceTimersByTime(300);
  expect(save).toHaveBeenCalledTimes(1);
  failFirst(new Error("offline"));
  await Promise.resolve();
  await Promise.resolve();
  jest.advanceTimersByTime(5000);
  expect(save).toHaveBeenCalledTimes(2);
  expect(save).toHaveBeenLastCalledWith({ micMuted: true, deafened: false }, { keepalive: false });
});
