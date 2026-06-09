export default defineAppConfig({
  ui: {
    pageHeader: {
      slots: {
        root: 'border-none',
        title: 'font-mono uppercase tracking-widest text-4xl mx-auto',
      },
    },

    toast: {
      slots: {
        root: 'font-mono rounded-lg shadow-lg shadow-black/35',
        title: 'font-semibold uppercase',
      },
    },
  },
});
