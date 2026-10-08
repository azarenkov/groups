import React from "react";
import { act, render, screen } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";

import i18n from "../../../../i18n";
import JoinPage from "../JoinPage";
import { communityAPI } from "../../api/community";
import { queryClient } from "../../../../shared/query/queryClient";

jest.mock("../../api/community", () => ({ communityAPI: { previewInvite: jest.fn() } }));

const preview = communityAPI.previewInvite as jest.Mock;

it("keeps a loaded invite usable when a later refresh fails", async () => {
  await i18n.changeLanguage("en");
  preview.mockResolvedValueOnce({ status: "active", community: { id: "c1", name: "Rift Club", memberCount: 3 } });
  render(
    <MemoryRouter initialEntries={["/join/AbCdEfGhIjKlMnOpQrStUv12"]}>
      <Routes>
        <Route path="/join/:token" element={<JoinPage />} />
      </Routes>
    </MemoryRouter>,
  );
  await screen.findByText("Rift Club");
  preview.mockRejectedValueOnce(new Error("offline"));
  await act(async () => {
    await queryClient.invalidateQueries();
    await new Promise((resolve) => setTimeout(resolve, 0));
  });
  expect(screen.getByText("Rift Club")).toBeTruthy();
  expect(screen.queryByText(i18n.t("modals:communityModal.inviteNotFound"))).toBeNull();
});
