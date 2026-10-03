import { fireEvent, render, screen, waitFor } from "@testing-library/react";

import i18n from "../../../../../i18n";
import { homeAPI } from "../../../../home/api/home";
import { PrivacySection } from "../PrivacySection";

jest.mock("../../../../home/api/home", () => ({
  homeAPI: { privacy: jest.fn(), updatePrivacy: jest.fn() },
}));

jest.mock("../../../../social/store/blockStore", () => {
  const state = {
    page: [],
    pageLoaded: true,
    pageError: false,
    pageCursor: null,
    pageLoading: false,
    fetchPage: () => Promise.resolve(),
    unblock: () => Promise.resolve(),
  };
  const useBlockStore = (select: (s: typeof state) => unknown) => select(state);
  useBlockStore.getState = () => state;
  return { useBlockStore };
});

const settings = { dmPolicy: "friends", callPolicy: "friends", acceptFriendRequests: true, showLocalTime: false };

describe("PrivacySection", () => {
  beforeAll(async () => {
    await i18n.changeLanguage("ru");
  });

  beforeEach(() => {
    (homeAPI.privacy as jest.Mock).mockResolvedValue(settings);
    (homeAPI.updatePrivacy as jest.Mock).mockResolvedValue({ ...settings, showLocalTime: true });
  });

  it("keeps local time hidden by default and turns it on", async () => {
    render(<PrivacySection />);
    const label = await screen.findByText("Показывать моё местное время");
    const toggle = label.closest(".s-row")!.querySelector<HTMLButtonElement>(".s-switch")!;
    expect(toggle).toHaveAttribute("aria-pressed", "false");
    fireEvent.click(toggle);
    await waitFor(() => expect(homeAPI.updatePrivacy).toHaveBeenCalledWith({ showLocalTime: true }));
    await waitFor(() => expect(toggle).toHaveAttribute("aria-pressed", "true"));
  });
});
