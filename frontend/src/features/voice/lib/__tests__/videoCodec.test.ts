import { SCREEN_CODEC, contentHintFor } from "../videoCodec";

describe("the content hint", () => {
  it("keeps text sharp even at sixty frames", () => {
    expect(contentHintFor("text", 60)).toBe("detail");
  });

  it("keeps motion smooth when asked", () => {
    expect(contentHintFor("motion", 30)).toBe("motion");
  });

  it("guesses from the frame rate when left alone", () => {
    expect(contentHintFor("auto", 60)).toBe("motion");
    expect(contentHintFor("auto", 30)).toBe("detail");
  });
});

describe("the screen codec", () => {
  it("is vp9 everywhere", () => {
    expect(SCREEN_CODEC).toBe("vp9");
  });
});
