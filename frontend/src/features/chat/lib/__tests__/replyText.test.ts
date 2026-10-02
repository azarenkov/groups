import "../../../../i18n";
import i18n from "../../../../i18n";
import { previewOf } from "../replies";
import { replyText } from "../replyText";

const sticker = { id: "s", packId: "p", emoji: "😺", animated: true, width: 320, height: 320 };

describe("replies to stickers", () => {
  beforeAll(async () => {
    await i18n.changeLanguage("ru");
  });

  it("quotes a sticker by its emoji", () => {
    expect(replyText({ id: "m", content: "", hasAttachments: false, deleted: false, sticker })).toBe("😺 Стикер");
  });

  it("keeps the sticker in a local reply preview", () => {
    const preview = previewOf({
      id: "m",
      channelId: "c",
      authorId: "a",
      content: "",
      sticker,
      createdAt: "2026-10-01T10:00:00Z",
      editedAt: null,
    });
    expect(preview.sticker).toEqual(sticker);
  });
});
