import { noteAppIntent, openAppLocationOnceResolved, registerAppNavigator } from "../appNavigator";

const peer = "22222222-2222-2222-2222-222222222222";

afterEach(() => registerAppNavigator(null));

it("opens a resolved location while the user stays where the request started", async () => {
  const go = jest.fn();
  registerAppNavigator(go);
  noteAppIntent("a");
  await openAppLocationOnceResolved(Promise.resolve({ kind: "dm", peerId: peer }));
  expect(go).toHaveBeenCalledWith(`/app/dm/${peer}`);
});

it("drops a resolved location once the user has navigated elsewhere", async () => {
  const go = jest.fn();
  registerAppNavigator(go);
  noteAppIntent("a");
  let resolve: (value: { kind: "dm"; peerId: string }) => void = () => {};
  const opening = openAppLocationOnceResolved(new Promise((done) => (resolve = done)));
  noteAppIntent("b");
  resolve({ kind: "dm", peerId: peer });
  await opening;
  expect(go).not.toHaveBeenCalled();
});
