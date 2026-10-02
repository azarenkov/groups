import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";

import "../../../../i18n";
import { ApiError } from "../../../../shared/lib/apiError";
import ReportModal from "../ReportModal";
import { reportAPI } from "../../api/report";
import {
  MAX_REPORT_DETAILS,
  COMMUNITY_REPORT_REASONS,
  STICKER_PACK_REPORT_REASONS,
  USER_REPORT_REASONS,
} from "../../lib/reasons";
import { useReportModalStore } from "../../store/reportModalStore";

jest.mock("../../api/report", () => ({
  reportAPI: {
    reportUser: jest.fn().mockResolvedValue(undefined),
    reportCommunity: jest.fn().mockResolvedValue(undefined),
    reportStickerPack: jest.fn().mockResolvedValue(undefined),
  },
}));

const mockedApi = reportAPI as jest.Mocked<typeof reportAPI>;

const submitButton = () =>
  screen.getAllByRole("button").find((b) => b.classList.contains("primary"))!;

describe("ReportModal", () => {
  beforeEach(() => {
    jest.clearAllMocks();
    useReportModalStore.getState().close();
  });

  it("stays closed until a target is set", () => {
    const { container } = render(<ReportModal />);
    expect(container).toBeEmptyDOMElement();
  });

  it("offers the reasons that fit the target", () => {
    useReportModalStore.getState().open({ type: "user", id: "u1", name: "spammer" });
    const { rerender } = render(<ReportModal />);
    expect(screen.getAllByRole("radio")).toHaveLength(USER_REPORT_REASONS.length);

    act(() => {
      useReportModalStore.getState().open({ type: "community", id: "s1", name: "den" });
    });
    rerender(<ReportModal />);
    expect(screen.getAllByRole("radio")).toHaveLength(COMMUNITY_REPORT_REASONS.length);
  });

  it("reports a sticker pack with the pack reasons", async () => {
    useReportModalStore.getState().open({ type: "stickerPack", id: "p1", name: "Cats" });
    render(<ReportModal />);
    expect(screen.getAllByRole("radio")).toHaveLength(STICKER_PACK_REPORT_REASONS.length);
    expect(screen.getByText("Report sticker pack")).toBeInTheDocument();
    fireEvent.click(screen.getByDisplayValue("nsfw"));
    fireEvent.click(submitButton());
    await waitFor(() =>
      expect(mockedApi.reportStickerPack).toHaveBeenCalledWith("p1", {
        reason: "nsfw",
        details: undefined,
        messageId: undefined,
      }),
    );
    expect(mockedApi.reportUser).not.toHaveBeenCalled();
  });

  it("sends the picked reason for the reported user", async () => {
    useReportModalStore.getState().open({
      type: "user",
      id: "u1",
      name: "spammer",
      messageId: "m1",
    });
    render(<ReportModal />);

    expect(submitButton()).toBeDisabled();
    fireEvent.click(screen.getByDisplayValue("spam"));
    fireEvent.click(submitButton());

    await waitFor(() => expect(mockedApi.reportUser).toHaveBeenCalledTimes(1));
    expect(mockedApi.reportUser).toHaveBeenCalledWith("u1", {
      reason: "spam",
      details: undefined,
      messageId: "m1",
    });
  });

  it("counts details the way the backend does", () => {
    useReportModalStore.getState().open({ type: "user", id: "u1", name: "spammer" });
    render(<ReportModal />);

    fireEvent.click(screen.getByDisplayValue("spam"));
    fireEvent.change(screen.getByRole("textbox"), {
      target: { value: "🙂🙂🙂" },
    });

    expect(screen.getByText(`3 / ${MAX_REPORT_DETAILS}`)).toBeInTheDocument();
    expect(submitButton()).not.toBeDisabled();
  });

  it("lets the report go without a message that has vanished", async () => {
    mockedApi.reportUser
      .mockRejectedValueOnce(
        new ApiError("gone", 400, "REPORT_CONTEXT_MISMATCH"),
      )
      .mockResolvedValueOnce(undefined);
    useReportModalStore.getState().open({
      type: "user",
      id: "u1",
      name: "spammer",
      messageId: "m1",
    });
    render(<ReportModal />);

    fireEvent.click(screen.getByDisplayValue("spam"));
    fireEvent.click(submitButton());
    await waitFor(() => expect(mockedApi.reportUser).toHaveBeenCalledTimes(1));
    expect(mockedApi.reportUser.mock.calls[0][1].messageId).toBe("m1");
    await waitFor(() => expect(submitButton()).not.toBeDisabled());

    fireEvent.click(submitButton());
    await waitFor(() => expect(mockedApi.reportUser).toHaveBeenCalledTimes(2));
    expect(mockedApi.reportUser.mock.calls[1][1].messageId).toBeUndefined();
  });

  it("does not land a slow answer on the next report", async () => {
    let settle: () => void = () => {};
    mockedApi.reportUser.mockImplementationOnce(
      () =>
        new Promise<void>((resolve) => {
          settle = resolve;
        }),
    );
    useReportModalStore.getState().open({ type: "user", id: "u1", name: "spammer" });
    render(<ReportModal />);

    fireEvent.click(screen.getByDisplayValue("spam"));
    fireEvent.click(submitButton());

    act(() => {
      useReportModalStore.getState().open({ type: "community", id: "s1", name: "den" });
      settle();
    });

    await waitFor(() =>
      expect(screen.getAllByRole("radio")).toHaveLength(COMMUNITY_REPORT_REASONS.length),
    );
    expect(useReportModalStore.getState().target?.id).toBe("s1");
    expect(submitButton()).toBeDisabled();
  });

  it("does not land a slow answer on a reopened report", async () => {
    let settle: () => void = () => {};
    mockedApi.reportUser.mockImplementationOnce(
      () =>
        new Promise<void>((resolve) => {
          settle = resolve;
        }),
    );
    const same = { type: "user" as const, id: "u1", name: "spammer", messageId: "m1" };
    useReportModalStore.getState().open(same);
    render(<ReportModal />);

    fireEvent.click(screen.getByDisplayValue("spam"));
    fireEvent.click(submitButton());

    act(() => {
      useReportModalStore.getState().close();
      useReportModalStore.getState().open(same);
      settle();
    });

    await waitFor(() => expect(submitButton()).toBeDisabled());
    expect(screen.getAllByRole("radio")).toHaveLength(USER_REPORT_REASONS.length);
    expect(useReportModalStore.getState().target?.id).toBe("u1");
  });

  it("keeps escape to itself so the screen underneath stays open", () => {
    const underneath = jest.fn();
    window.addEventListener("keydown", underneath);
    useReportModalStore.getState().open({ type: "community", id: "s1", name: "den" });
    render(<ReportModal />);

    fireEvent.keyDown(document.body, { key: "Escape" });

    expect(useReportModalStore.getState().target).toBeNull();
    expect(underneath).not.toHaveBeenCalled();
    window.removeEventListener("keydown", underneath);
  });

  it("keeps other unsendable until something is written", async () => {
    useReportModalStore.getState().open({ type: "user", id: "u1", name: "spammer" });
    render(<ReportModal />);

    fireEvent.click(screen.getByDisplayValue("other"));
    expect(submitButton()).toBeDisabled();

    fireEvent.change(screen.getByRole("textbox"), {
      target: { value: "  he keeps doing it  " },
    });
    expect(submitButton()).not.toBeDisabled();
    fireEvent.click(submitButton());

    await waitFor(() => expect(mockedApi.reportUser).toHaveBeenCalledTimes(1));
    expect(mockedApi.reportUser.mock.calls[0][1].details).toBe("he keeps doing it");
  });
});
