import { LiveKitService } from "../livekit";

describe("screen view lifetime", () => {
  it("keeps the choice while a viewing surface is open", () => {
    const service = new LiveKitService();
    const release = service.acquireScreenView();
    service.watchScreen("ann");
    expect(service.getScreenShares().watching).toBe("ann");

    release();
    expect(service.getScreenShares().watching).toBeNull();
  });

  it("holds the choice until the last surface closes", () => {
    const service = new LiveKitService();
    const first = service.acquireScreenView();
    const second = service.acquireScreenView();
    service.watchScreen("bob");

    first();
    expect(service.getScreenShares().watching).toBe("bob");

    second();
    expect(service.getScreenShares().watching).toBeNull();
  });

  it("ignores a surface released twice", () => {
    const service = new LiveKitService();
    const first = service.acquireScreenView();
    const second = service.acquireScreenView();
    service.watchScreen("carol");

    first();
    first();
    expect(service.getScreenShares().watching).toBe("carol");

    second();
    expect(service.getScreenShares().watching).toBeNull();
  });
});
