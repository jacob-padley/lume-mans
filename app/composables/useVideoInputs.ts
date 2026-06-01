import { invoke } from '@tauri-apps/api/core';
import * as z from 'zod';

const VideoInputSchema = z.object({
  id: z.number().nonnegative(),
  name: z.string(),
  isPrimary: z.boolean(),
});
export type VideoInput = z.infer<typeof VideoInputSchema>;

const VideoInputListSchema = z.array(VideoInputSchema);
export type VideoInputList = z.infer<typeof VideoInputListSchema>;

export function useVideoInputs() {
  const availableInputs = ref<VideoInputList>([]);

  onMounted(() => {
    invoke('list_inputs')
      .then((inputs) => {
        const parsedInputs = VideoInputListSchema.safeParse(inputs);
        if (parsedInputs.success) {
          availableInputs.value = parsedInputs.data;
        } else {
          console.error(parsedInputs.error);
        }
      })
      .catch((e) => {
        console.error(e);
      });
  });

  return { availableInputs };
}
