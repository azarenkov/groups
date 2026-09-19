import { render, screen } from "@testing-library/react";

import { AttachmentItem, IMAGE_PREVIEW_MAX_BYTES } from "../AttachmentItem";

jest.mock("../../api/attachment", () => ({
  attachmentAPI: { download: jest.fn(() => new Promise(() => {})) },
}));

const base = {
  id: "a1",
  fileName: "photo.png",
  contentType: "image/png",
  createdAt: "2026-09-19T00:00:00Z",
};

describe("AttachmentItem", () => {
  beforeAll(() => {
    (window as unknown as { IntersectionObserver: unknown }).IntersectionObserver = class {
      observe() {}
      disconnect() {}
      unobserve() {}
    };
  });

  it("previews images that are small enough", () => {
    render(<AttachmentItem attachment={{ ...base, sizeBytes: 1024 } as never} />);
    expect(document.querySelector(".msg-attach-image")).not.toBeNull();
  });

  it("offers oversized images as a plain download", () => {
    render(<AttachmentItem attachment={{ ...base, sizeBytes: IMAGE_PREVIEW_MAX_BYTES + 1 } as never} />);
    expect(document.querySelector(".msg-attach-image")).toBeNull();
    expect(screen.getByText("photo.png")).toBeInTheDocument();
  });
});
