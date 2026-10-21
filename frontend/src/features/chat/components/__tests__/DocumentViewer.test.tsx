import { fireEvent, render, screen } from "@testing-library/react";

import { attachmentAPI } from "../../api/attachment";
import { openPdf } from "../../lib/pdf";
import { saveAttachment } from "../../lib/saveAttachment";
import { DocumentViewer } from "../DocumentViewer";

jest.mock("../../api/attachment", () => ({
  attachmentAPI: { download: jest.fn(), preview: jest.fn() },
}));
jest.mock("../../lib/pdf", () => ({ openPdf: jest.fn(), loadPdfJs: jest.fn() }));
jest.mock("../../lib/saveAttachment", () => ({ saveAttachment: jest.fn(() => Promise.resolve()) }));

const download = attachmentAPI.download as jest.Mock;
const preview = attachmentAPI.preview as jest.Mock;
const open = openPdf as jest.Mock;

const textBlob = (text: string) => ({
  size: text.length,
  slice: () => textBlob(text),
  text: () => Promise.resolve(text),
});

const attachment = (fileName: string, contentType: string) =>
  ({ id: "att-1", fileName, contentType, sizeBytes: 64, createdAt: "2026-10-21T00:00:00Z" }) as never;

describe("DocumentViewer", () => {
  beforeAll(() => {
    (window as unknown as { IntersectionObserver: unknown }).IntersectionObserver = class {
      observe() {}
      disconnect() {}
      unobserve() {}
    };
  });

  beforeEach(() => jest.clearAllMocks());

  it("shows plain text files as they are", async () => {
    download.mockResolvedValue(textBlob("line one\nline two"));
    render(<DocumentViewer attachment={attachment("notes.txt", "text/plain")} onClose={() => {}} />);
    expect(await screen.findByText(/line one/)).toBeInTheDocument();
    expect(preview).not.toHaveBeenCalled();
  });

  it("indents JSON and lays CSV out as a table", async () => {
    download.mockResolvedValueOnce(textBlob('{"a":1}'));
    const { unmount } = render(<DocumentViewer attachment={attachment("a.json", "application/json")} onClose={() => {}} />);
    expect(await screen.findByText(/"a": 1/)).toBeInTheDocument();
    unmount();
    download.mockResolvedValueOnce(textBlob("city,people\nAlmaty,2000000"));
    render(<DocumentViewer attachment={attachment("a.csv", "text/csv")} onClose={() => {}} />);
    expect(await screen.findByRole("columnheader", { name: "city" })).toBeInTheDocument();
    expect(screen.getByRole("cell", { name: "Almaty" })).toBeInTheDocument();
  });

  it("opens office documents through the server rendition and shows its pages", async () => {
    preview.mockResolvedValue({ arrayBuffer: () => Promise.resolve(new ArrayBuffer(8)) });
    const page = {
      getViewport: ({ scale }: { scale: number }) => ({ width: 600 * scale, height: 800 * scale, scale }),
      render: () => ({ promise: Promise.resolve(), cancel: () => {} }),
      streamTextContent: () => ({}),
    };
    open.mockResolvedValue({ numPages: 3, getPage: jest.fn(() => Promise.resolve(page)), destroy: jest.fn() });
    render(
      <DocumentViewer
        attachment={attachment("Essay.docx", "application/vnd.openxmlformats-officedocument.wordprocessingml.document")}
        onClose={() => {}}
      />,
    );
    expect(screen.getByText("documentPreparing")).toBeInTheDocument();
    expect(await screen.findByText("documentPage")).toBeInTheDocument();
    expect(preview).toHaveBeenCalledWith("att-1", 64, expect.anything());
    expect(document.querySelectorAll(".doc-viewer-page")).toHaveLength(3);
  });

  it("falls back to a download when no preview can be made", async () => {
    preview.mockRejectedValue(new Error("422"));
    render(<DocumentViewer attachment={attachment("broken.pptx", "application/vnd.ms-powerpoint")} onClose={() => {}} />);
    expect(await screen.findByText("documentUnavailable")).toBeInTheDocument();
    const buttons = screen.getAllByRole("button", { name: /download/ });
    fireEvent.click(buttons[buttons.length - 1]);
    expect(saveAttachment).toHaveBeenCalled();
  });

  it("closes with Escape and with the close button", () => {
    download.mockReturnValue(new Promise(() => {}));
    const onClose = jest.fn();
    render(<DocumentViewer attachment={attachment("a.md", "text/markdown")} onClose={onClose} />);
    fireEvent.keyDown(window, { key: "Escape" });
    fireEvent.click(screen.getByRole("button", { name: "documentClose" }));
    expect(onClose).toHaveBeenCalledTimes(2);
  });
});
