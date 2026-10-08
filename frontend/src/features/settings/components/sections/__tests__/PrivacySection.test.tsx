import React from "react";
import { act, fireEvent, render, screen } from "@testing-library/react";

import i18n from "../../../../../i18n";
import { PrivacySection } from "../PrivacySection";
import { homeAPI } from "../../../../home/api/home";
import { queryClient, queryKeys } from "../../../../../shared/query/queryClient";

jest.mock("../../../../home/api/home", () => ({ homeAPI: { privacy: jest.fn(), updatePrivacy: jest.fn() } }));
jest.mock("../../../../social/store/blockStore", () => {
  const actual = jest.requireActual("../../../../social/store/blockStore");
  actual.useBlockStore.setState({ fetch: () => Promise.resolve(), loaded: true, blocked: {} });
  return actual;
});

const privacy = homeAPI.privacy as jest.Mock;
const update = homeAPI.updatePrivacy as jest.Mock;
const settings = (dmPolicy: string) => ({ dmPolicy, acceptFriendRequests: true, showActivity: true, showLocalTime: false });

it("keeps a saved change when an older read finishes later", async () => {
  await i18n.changeLanguage("en");
  privacy.mockResolvedValueOnce(settings("all"));
  render(<PrivacySection />);
  await screen.findAllByText(i18n.t("settings:privacy.friends"));
  let finishRead: (v: unknown) => void = () => undefined;
  privacy.mockImplementationOnce(() => new Promise((resolve) => (finishRead = resolve)));
  act(() => {
    void queryClient.invalidateQueries({ queryKey: queryKeys.privacySettings() });
  });
  update.mockResolvedValueOnce(settings("friends"));
  await act(async () => {
    fireEvent.click(screen.getAllByText(i18n.t("settings:privacy.friends"))[0]);
  });
  await act(async () => {
    finishRead(settings("all"));
    await new Promise((resolve) => setTimeout(resolve, 0));
  });
  expect(queryClient.getQueryData<{ dmPolicy: string }>(queryKeys.privacySettings())?.dmPolicy).toBe("friends");
});
