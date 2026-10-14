import React from "react";
import { act, render } from "@testing-library/react";

import { bustAvatar, useAvatarUrl } from "../avatarCache";
import { memoryBlobBackend, setBlobBackend, writeCachedBlob } from "../blobCache";
import { userAPI } from "../../../features/social/api/user";

jest.mock("../../../features/social/api/user", () => ({ userAPI: { downloadAvatar: jest.fn() } }));
jest.mock("../../../features/communities/api/community", () => ({ communityAPI: {} }));

const download = userAPI.downloadAvatar as jest.Mock;
let urls = 0;
let seen: (string | null)[] = [];
const Probe: React.FC = () => {
  seen.push(useAvatarUrl("user", "u1"));
  return null;
};

beforeAll(() => {
  Object.assign(URL, { createObjectURL: () => `blob:${++urls}`, revokeObjectURL: () => undefined });
});

it("does not bring back the old avatar from disk after it changed", async () => {
  const memory = memoryBlobBackend();
  let releaseDelete: () => void = () => undefined;
  setBlobBackend({
    ...memory,
    delete: async (key) => {
      await new Promise<void>((resolve) => (releaseDelete = resolve));
      await memory.delete(key);
    },
  });
  await writeCachedBlob("avatar:user:u1", new Blob([new Uint8Array(1)]));
  download.mockRejectedValue(new Error("offline"));
  render(<Probe />);
  await act(async () => {
    await new Promise((resolve) => setTimeout(resolve, 0));
  });
  const restored = seen[seen.length - 1];
  expect(restored).not.toBeNull();
  seen = [];
  act(() => bustAvatar("user", "u1"));
  await act(async () => {
    await new Promise((resolve) => setTimeout(resolve, 0));
  });
  expect(seen.every((url) => url === null)).toBe(true);
  releaseDelete();
  setBlobBackend(null);
});
