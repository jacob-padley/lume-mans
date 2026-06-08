import { invoke } from '@tauri-apps/api/core';
import * as z from 'zod';

const VideoInputSchema = z.object({
  id: z.number().nonnegative(),
  name: z.string(),
  is_primary: z.boolean(),
  source_type: z.union([z.literal('Window'), z.literal('Monitor')]),
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
          // TODO: properly support selecting windows
          availableInputs.value = parsedInputs.data.filter(
            (input) => input.source_type === 'Monitor',
          );
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
