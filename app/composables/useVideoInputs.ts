import { invoke } from '@tauri-apps/api/core';
import * as z from 'zod';

const SourceTypeSchema = z.union([z.literal('Window'), z.literal('Monitor')]);
export type SourceType = z.infer<typeof SourceTypeSchema>;

const VideoInputSchema = z.object({
  id: z.number().nonnegative(),
  name: z.string(),
  is_primary: z.boolean(),
  source_type: SourceTypeSchema,
});
export type VideoInput = z.infer<typeof VideoInputSchema>;

const VideoInputListSchema = z.array(VideoInputSchema);
export type VideoInputList = z.infer<typeof VideoInputListSchema>;

const getVideoInputs = async (): Promise<VideoInputList> => {
  const inputs = await invoke('list_inputs');

  const parsedInputs = VideoInputListSchema.safeParse(inputs);

  if (!parsedInputs.success) {
    console.error(parsedInputs.error);
    throw new Error(parsedInputs.error.message);
  }

  return parsedInputs.data;
};

export function useVideoInputs() {
  const availableInputs = ref<VideoInputList>([]);
  const loading = ref(false);

  const refreshAvailableInputs = async () => {
    loading.value = true;

    try {
      availableInputs.value = await getVideoInputs();
    } catch (e) {
      console.error(e);
    } finally {
      loading.value = false;
    }
  };

  onMounted(() => {
    refreshAvailableInputs();
  });

  return { availableInputs, loading, refreshAvailableInputs };
}
