import { listen } from '@tauri-apps/api/event';

export function useMetrics() {
  const lastFrameTime = ref(0);

  let unlisten: (() => void) | undefined;

  onMounted(async () => {
    unlisten = await listen<number>(
      'last-frame-time',
      (event) => (lastFrameTime.value = event.payload),
    );
  });

  onUnmounted(() => {
    unlisten?.();
  });

  return { lastFrameTime };
}
