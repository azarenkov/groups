import { fireEvent, render, screen, waitFor } from "@testing-library/react";

import "../../../../i18n";
import ReportModal from "../ReportModal";
import { reportAPI } from "../../api/report";
import {
  MAX_REPORT_DETAILS,
  SERVER_REPORT_REASONS,
  USER_REPORT_REASONS,
} from "../../lib/reasons";
import { useReportModalStore } from "../../store/reportModalStore";

jest.mock("../../api/report", () => ({
  reportAPI: {
    reportUser: jest.fn().mockResolvedValue(undefined),
    reportServer: jest.fn().mockResolvedValue(undefined),
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

    useReportModalStore.getState().open({ type: "server", id: "s1", name: "den" });
    rerender(<ReportModal />);
    expect(screen.getAllByRole("radio")).toHaveLength(SERVER_REPORT_REASONS.length);
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
