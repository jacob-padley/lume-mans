import { listen } from '@tauri-apps/api/event';

export function useSessionTime() {
  const hours = ref(24);
  const minutes = ref(0);
  const seconds = ref(0);

  let unlisten: (() => void) | undefined;

  onMounted(async () => {
    unlisten = await listen<{
      hours: number;
      minutes: number;
      seconds: number;
    }>(
      'session-time',
      (event) =>
        ({ hours: hours.value, minutes: minutes.value, seconds: seconds.value } = event.payload),
    );
  });

  onUnmounted(() => {
    unlisten?.();
  });

  return { hours, minutes, seconds };
}
