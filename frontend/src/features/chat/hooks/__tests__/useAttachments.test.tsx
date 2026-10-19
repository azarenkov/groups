import { act, renderHook, waitFor } from "@testing-library/react";

import { attachmentAPI } from "../../api/attachment";
import { useAttachments } from "../useAttachments";

let mockLimit = 1024;

jest.mock("../../api/attachment", () => ({
  maxAttachmentsPerMessage: () => 10,
  maxAttachmentBytes: () => mockLimit,
  attachmentAPI: { upload: jest.fn(), delete: jest.fn() },
}));

const upload = attachmentAPI.upload as jest.Mock;
const remove = attachmentAPI.delete as jest.Mock;

const file = (name: string, size = 10) => {
  const f = new File(["x"], name);
  Object.defineProperty(f, "size", { value: size });
  return f;
};

describe("useAttachments", () => {
  beforeEach(() => {
    mockLimit = 1024;
    upload.mockReset();
    remove.mockReset();
    upload.mockResolvedValue({ id: "att-1" });
    remove.mockResolvedValue(undefined);
  });

  it("uploads a picked file and keeps the result", async () => {
    const { result } = renderHook(() => useAttachments());
    act(() => result.current.addFiles([file("a.png")]));
    await waitFor(() => expect(result.current.pendingFiles[0].attachment).toEqual({ id: "att-1" }));
    expect(upload).toHaveBeenCalledTimes(1);
  });

  it("marks a file too large for upload as failed without calling the api", () => {
    const { result } = renderHook(() => useAttachments());
    act(() => result.current.addFiles([file("big.png", 4096)]));
    expect(result.current.pendingFiles[0].error).toBe(true);
    expect(upload).not.toHaveBeenCalled();
  });

  it("deletes the uploaded attachment when the file is removed", async () => {
    const { result } = renderHook(() => useAttachments());
    act(() => result.current.addFiles([file("a.png")]));
    await waitFor(() => expect(result.current.pendingFiles[0].attachment).toBeTruthy());
    const localId = result.current.pendingFiles[0].localId;
    act(() => result.current.removePendingFile(localId));
    expect(result.current.pendingFiles).toHaveLength(0);
    expect(remove).toHaveBeenCalledWith("att-1");
  });

  it("tracks removed attachments while editing", () => {
    const { result } = renderHook(() => useAttachments());
    act(() => result.current.toggleRemoveAttachment("att-9"));
    expect(result.current.editRemovedIds).toEqual(["att-9"]);
    act(() => result.current.toggleRemoveAttachment("att-9"));
    expect(result.current.editRemovedIds).toEqual([]);
  });
});
