import { invoke } from "@tauri-apps/api/core";
import type { RefObject } from "react";

export type EnqueueOp = <T>(task: () => Promise<T>) => Promise<T>;

type InvokeFn = (
  cmd: string,
  args: Record<string, unknown>,
) => Promise<unknown>;

export function createEnqueueOp(
  opChainRef: RefObject<Promise<unknown>>,
): EnqueueOp {
  return <T>(task: () => Promise<T>): Promise<T> => {
    const next = opChainRef.current.then(task, task);
    opChainRef.current = next.catch(() => undefined);
    return next;
  };
}

export interface IpcSenders {
  sendInsert: (
    position: number,
    content: string,
    baseSeq: number,
  ) => Promise<unknown>;
  sendDelete: (
    position: number,
    length: number,
    baseSeq: number,
  ) => Promise<unknown>;
  sendReplace: (
    position: number,
    deleteLength: number,
    content: string,
    baseSeq: number,
  ) => Promise<unknown>;
}

export function createIpcSenders(
  enqueueOp: EnqueueOp,
  invokeFn: InvokeFn = invoke,
): IpcSenders {
  return {
    sendInsert: (position, content, baseSeq) =>
      enqueueOp(() => invokeFn("insert", { position, content, baseSeq })),
    sendDelete: (position, length, baseSeq) =>
      enqueueOp(() => invokeFn("delete", { position, length, baseSeq })),
    sendReplace: (position, deleteLength, content, baseSeq) =>
      enqueueOp(() =>
        invokeFn("replace", { position, deleteLength, content, baseSeq }),
      ),
  };
}
