import { fireEvent, render, screen } from "@testing-library/react";

import i18n from "../../../../../i18n";
import { MembersSection } from "../MembersSection";
import type { Community } from "../../../../../shared/types/community";

const community = { id: "c1", name: "404", ownerId: "owner" } as Community;

beforeAll(async () => {
  await i18n.changeLanguage("en");
});

it("asks before kicking and kicks only after confirming", () => {
  const kickMember = jest.fn().mockResolvedValue(undefined);
  render(
    <MembersSection
      community={community}
      members={[{ id: "u2", name: "Bob" }]}
      currentUserId="owner"
      rolesData={undefined}
      canKick={() => true}
      kickBlocked={() => false}
      kickMember={kickMember}
      kickError={null}
    />,
  );
  fireEvent.click(screen.getByRole("button", { name: i18n.t("communitySettings:kickMember") }));
  expect(screen.getByText(i18n.t("communitySettings:kickConfirmTitle", { name: "Bob" }))).toBeInTheDocument();
  expect(kickMember).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole("button", { name: i18n.t("communitySettings:kickConfirm") }));
  expect(kickMember).toHaveBeenCalledWith("u2");
});

it("shows why a member of equal or higher rank cannot be kicked", () => {
  render(
    <MembersSection
      community={community}
      members={[{ id: "u2", name: "Bob" }]}
      currentUserId="me"
      rolesData={undefined}
      canKick={() => false}
      kickBlocked={() => true}
      kickMember={jest.fn()}
      kickError={null}
    />,
  );
  expect(screen.getByLabelText(i18n.t("communitySettings:kickBlockedRank"))).toBeInTheDocument();
});
