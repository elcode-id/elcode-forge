/** @type {import('tailwindcss').Config} */
export default {
  content: ["./src/**/*.{html,js,svelte,ts}"],
  darkMode: "class",
  theme: {
    extend: {
      colors: {
        background: "#0b0d13",
        surface: "#12151f",
        border: "#262c3f",
      },
    },
  },
  plugins: [],
};
