import type { Preview } from "@storybook/react";
import "@wms/ui/styles/globals.css";

const preview: Preview = {
  parameters: {
    controls: { matchers: { color: /(background|color)$/i, date: /Date$/i } },
    layout: "padded",
  },
};

export default preview;
