import i18n from "../../i18n";

const DELETED_NAME = /^deleted_[0-9a-f]{32}$/;

export const isDeletedUserName = (name: string | null | undefined): boolean => !!name && DELETED_NAME.test(name);

export const displayUserName = (name: string): string =>
  isDeletedUserName(name) ? i18n.t("deletedUser", { ns: "profile" }) : name;
