import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";

type Registrar = <T>(event: string, handler: (payload: T) => void) => void;

export function useTauriEvents(subscribe: (on: Registrar) => void) {
  useEffect(() => {
    const unlisten: (() => void)[] = [];
    let cancelled = false;

    subscribe(<T>(event: string, handler: (payload: T) => void) => {
      void listen<T>(event, (e) => handler(e.payload)).then((fn) => {
        if (cancelled) fn();
        else unlisten.push(fn);
      });
    });

    return () => {
      cancelled = true;
      unlisten.forEach((fn) => fn());
    };
  }, [subscribe]);
}
