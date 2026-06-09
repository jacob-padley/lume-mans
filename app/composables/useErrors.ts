import { listen } from '@tauri-apps/api/event';

export function useErrors(callback: (error: string) => void) {
  let unlisten: (() => void) | undefined;

  onMounted(async () => {
    unlisten = await listen<string>('error', (error) => callback(error.payload));
  });

  onUnmounted(() => {
    unlisten?.();
  });
}
