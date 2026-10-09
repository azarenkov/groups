import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";

import i18n from "../../../../i18n";
import ProfileStyleSettings from "../ProfileStyleSettings";
import { entitlementsAPI } from "../../api/entitlements";
import { BADGE_STYLES } from "../../lib/plans";
import { useEntitlementsStore } from "../../store/entitlementsStore";

jest.mock("../../api/entitlements", () => ({
  entitlementsAPI: {
    getMine: jest.fn(),
    setBadgeStyle: jest.fn((style: string) => Promise.resolve(style)),
    lookupBadges: jest.fn().mockResolvedValue([]),
  },
}));

jest.mock("../../../social/api/user", () => ({
  userAPI: {
    uploadBanner: jest.fn().mockResolvedValue(undefined),
    deleteBanner: jest.fn().mockResolvedValue(undefined),
    downloadBanner: jest.fn().mockRejectedValue(new Error("none")),
    downloadAvatar: jest.fn().mockRejectedValue(new Error("none")),
  },
}));

jest.mock("../../../stickers/api/stickers", () => ({
  stickersAPI: { mine: jest.fn().mockResolvedValue([]) },
}));

const mockedApi = entitlementsAPI as jest.Mocked<typeof entitlementsAPI>;

const badgeRadios = () =>
  within(screen.getByRole("radiogroup", { name: i18n.t("subscription:badge.title") })).getAllByRole("radio");

describe("ProfileStyleSettings", () => {
  beforeEach(() => {
    mockedApi.setBadgeStyle.mockImplementation((style) => Promise.resolve(style));
    act(() => useEntitlementsStore.getState().clear());
  });

  it("shows every badge style to a free user but lets none be picked", () => {
    render(<ProfileStyleSettings />);
    const options = badgeRadios();
    expect(options).toHaveLength(BADGE_STYLES.length);
    for (const option of options) expect(option).toBeDisabled();

    fireEvent.click(options[2]);
    expect(mockedApi.setBadgeStyle).not.toHaveBeenCalled();
  });

  it("saves the style a subscriber picks", async () => {
    act(() => useEntitlementsStore.getState().applyTier("pro"));
    render(<ProfileStyleSettings />);

    const ink = badgeRadios()[BADGE_STYLES.indexOf("ink")];
    expect(ink).not.toBeDisabled();
    fireEvent.click(ink);

    await waitFor(() => expect(mockedApi.setBadgeStyle).toHaveBeenCalledWith("ink"));
    expect(useEntitlementsStore.getState().badgeStyle).toBe("ink");
  });

  it("keeps the gold badge for Premium", async () => {
    act(() => useEntitlementsStore.getState().applyTier("pro"));
    const { unmount } = render(<ProfileStyleSettings />);
    const goldForPro = badgeRadios()[BADGE_STYLES.indexOf("gold")];
    expect(goldForPro).toBeDisabled();
    expect(goldForPro).toHaveAttribute("title", expect.stringContaining("Premium"));
    expect(goldForPro.querySelector(".pro-badge-gold")).toHaveTextContent("PREMIUM");
    unmount();

    act(() => useEntitlementsStore.getState().applyTier("premium"));
    render(<ProfileStyleSettings />);
    const gold = badgeRadios()[BADGE_STYLES.indexOf("gold")];
    expect(gold).not.toBeDisabled();
    fireEvent.click(gold);
    await waitFor(() => expect(mockedApi.setBadgeStyle).toHaveBeenCalledWith("gold"));
  });

  it("puts the old style back when the community refuses", async () => {
    mockedApi.setBadgeStyle.mockRejectedValueOnce(new Error("nope"));
    act(() => useEntitlementsStore.getState().applyTier("pro"));
    render(<ProfileStyleSettings />);

    fireEvent.click(badgeRadios()[BADGE_STYLES.indexOf("solid")]);

    await waitFor(() => expect(mockedApi.setBadgeStyle).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(useEntitlementsStore.getState().badgeStyle).toBe("tint"));
  });
});
