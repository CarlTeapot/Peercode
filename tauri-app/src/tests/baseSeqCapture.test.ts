import { describe, expect, test, vi } from "vitest";
import { createEnqueueOp, createIpcSenders } from "../opQueue";

function defer<T>(): { promise: Promise<T>; resolve: (value: T) => void } {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((r) => {
    resolve = r;
  });
  return { promise, resolve };
}

describe("IPC senders preserve caller-supplied baseSeq", () => {
  test("sendInsert passes through the caller's baseSeq even when external state mutates between enqueue and task execution", async () => {
    const invoke = vi.fn<(cmd: string, args: unknown) => Promise<void>>();
    const opChainRef = { current: Promise.resolve() as Promise<unknown> };
    const enqueueOp = createEnqueueOp(opChainRef);
    const { sendInsert } = createIpcSenders(enqueueOp, invoke);

    const slow = defer<void>();
    enqueueOp(() => slow.promise);

    const sendPromise = sendInsert(10, "A", 0);

    slow.resolve();
    await sendPromise;

    expect(invoke).toHaveBeenCalledWith(
      "insert",
      expect.objectContaining({
        position: 10,
        content: "A",
        baseSeq: 0,
      }),
    );
  });

  test("sendDelete passes through the caller's baseSeq under the same conditions", async () => {
    const invoke = vi.fn<(cmd: string, args: unknown) => Promise<void>>();
    const opChainRef = { current: Promise.resolve() as Promise<unknown> };
    const enqueueOp = createEnqueueOp(opChainRef);
    const { sendDelete } = createIpcSenders(enqueueOp, invoke);

    const slow = defer<void>();
    enqueueOp(() => slow.promise);

    const sendPromise = sendDelete(10, 3, 0);

    slow.resolve();
    await sendPromise;

    expect(invoke).toHaveBeenCalledWith(
      "delete",
      expect.objectContaining({
        position: 10,
        length: 3,
        baseSeq: 0,
      }),
    );
  });
});
