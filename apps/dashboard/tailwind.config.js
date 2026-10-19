/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./src/**/*.{html,js,svelte,ts}",
    "../../packages/forge-ui/src/**/*.{html,js,svelte,ts}",
  ],
  theme: {
    extend: {
      colors: {
        elcode: {
          bg: "#090a0f",
          surface: "#12151f",
          subtle: "#1a1e2e",
          border: "#262c3f",
        },
      },
      fontFamily: {
        mono: [
          "JetBrains Mono",
          "ui-monospace",
          "SFMono-Regular",
          "Menlo",
          "Monaco",
          "Consolas",
          "monospace",
        ],
      },
    },
  },
  plugins: [],
};
