'use client';
import { useCallback, useSyncExternalStore } from 'react';
export function useMediaQuery(query: string) {
  const subscribe = useCallback(
    (notify: () => void) => {
      const media = matchMedia(query);
      media.addEventListener('change', notify);
      return () => media.removeEventListener('change', notify);
    },
    [query],
  );
  return useSyncExternalStore(
    subscribe,
    () => matchMedia(query).matches,
    () => false,
  );
}
