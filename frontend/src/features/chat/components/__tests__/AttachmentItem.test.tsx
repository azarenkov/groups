import { fireEvent, render, screen } from "@testing-library/react";

import { AttachmentItem, IMAGE_PREVIEW_MAX_BYTES } from "../AttachmentItem";

jest.mock("../../api/attachment", () => ({
  attachmentAPI: { download: jest.fn(() => new Promise(() => {})) },
}));
jest.mock("../DocumentViewer", () => ({
  DocumentViewer: ({ attachment }: { attachment: { fileName: string } }) => (
    <div data-testid="document-viewer">{attachment.fileName}</div>
  ),
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

  it("opens documents in the viewer instead of downloading them", () => {
    render(
      <AttachmentItem
        attachment={{ ...base, fileName: "lecture.pdf", contentType: "application/pdf", sizeBytes: 2048 } as never}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: /lecture\.pdf/ }));
    expect(screen.getByTestId("document-viewer")).toHaveTextContent("lecture.pdf");
  });

  it("keeps unknown files as a plain download", () => {
    render(<AttachmentItem attachment={{ ...base, fileName: "archive.zip", contentType: "application/zip", sizeBytes: 10 } as never} />);
    expect(document.querySelector(".msg-attach-document")).toBeNull();
    expect(screen.getByText("archive.zip")).toBeInTheDocument();
  });

  it("offers oversized images as a plain download", () => {
    render(<AttachmentItem attachment={{ ...base, sizeBytes: IMAGE_PREVIEW_MAX_BYTES + 1 } as never} />);
    expect(document.querySelector(".msg-attach-image")).toBeNull();
    expect(screen.getByText("photo.png")).toBeInTheDocument();
  });
});
