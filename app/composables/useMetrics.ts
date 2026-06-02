import { listen } from '@tauri-apps/api/event';

export function useMetrics() {
  const lastFrameTime = ref(0);

  listen<number>('last-frame-time', (event) => (lastFrameTime.value = event.payload));

  return { lastFrameTime };
}
