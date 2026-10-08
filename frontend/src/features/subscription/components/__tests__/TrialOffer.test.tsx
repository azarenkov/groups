import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";

import i18n from "../../../../i18n";
import TrialOffer from "../TrialOffer";
import { entitlementsAPI } from "../../api/entitlements";
import { useEntitlementsStore } from "../../store/entitlementsStore";
import { ApiError } from "../../../../shared/lib/apiError";

const TRIAL_END = new Date(Date.now() + 3 * 24 * 60 * 60 * 1000).toISOString();

jest.mock("../../api/entitlements", () => ({
  entitlementsAPI: {
    getMine: jest.fn(),
    startTrial: jest.fn(),
    setBadgeStyle: jest.fn(),
    lookupBadges: jest.fn().mockResolvedValue([]),
  },
}));

const mockedApi = entitlementsAPI as jest.Mocked<typeof entitlementsAPI>;

describe("TrialOffer", () => {
  beforeAll(async () => {
    await i18n.changeLanguage("ru");
  });

  beforeEach(() => {
    useEntitlementsStore.getState().clear();
    mockedApi.startTrial.mockReset();
    mockedApi.getMine.mockReset();
  });

  it("stays hidden when the trial is not available", () => {
    const { container } = render(<TrialOffer />);
    expect(container).toBeEmptyDOMElement();
  });

  it("starts the trial and switches the account to Pro until the end date", async () => {
    act(() => useEntitlementsStore.setState({ trialAvailable: true }));
    mockedApi.startTrial.mockResolvedValue({ tier: "pro", validUntil: TRIAL_END });
    render(<TrialOffer />);
    fireEvent.click(screen.getByRole("button", { name: "Попробовать Pro бесплатно на 7 дней" }));
    await waitFor(() => expect(useEntitlementsStore.getState().tier).toBe("pro"));
    const state = useEntitlementsStore.getState();
    expect(state.trial).toBe(true);
    expect(state.trialAvailable).toBe(false);
    expect(state.validUntil).toBe(TRIAL_END);
  });

  it("explains why the trial could not start", async () => {
    act(() => useEntitlementsStore.setState({ trialAvailable: true }));
    mockedApi.startTrial.mockRejectedValue(
      new ApiError("used", 409, "TRIAL_ALREADY_USED"),
    );
    render(<TrialOffer compact />);
    fireEvent.click(screen.getByRole("button"));
    expect(await screen.findByRole("alert")).toHaveTextContent("Пробный период уже использован.");
    expect(useEntitlementsStore.getState().tier).toBe("free");
  });
});

describe("applyTier", () => {
  beforeEach(() => {
    useEntitlementsStore.getState().clear();
    mockedApi.getMine.mockReset();
    mockedApi.getMine.mockResolvedValue({
      tier: "free",
      badgeStyle: "tint",
      memberNumber: null,
      validUntil: null,
      trial: false,
      trialAvailable: false,
    });
  });

  it("keeps the trial window from a realtime update and clears it on Free", () => {
    const store = useEntitlementsStore.getState();
    store.applyTier("pro", null, { validUntil: TRIAL_END, trial: true });
    expect(useEntitlementsStore.getState()).toMatchObject({
      tier: "pro",
      trial: true,
      validUntil: TRIAL_END,
      trialAvailable: false,
    });
    store.applyTier("free", null, { validUntil: null, trial: false });
    expect(useEntitlementsStore.getState()).toMatchObject({ tier: "free", trial: false, validUntil: null });
    expect(mockedApi.getMine).toHaveBeenCalled();
  });
});
