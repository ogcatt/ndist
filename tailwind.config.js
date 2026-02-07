/** @type {import('tailwindcss').Config} */
module.exports = {
  mode: "all",
  content: ["`$process.env.HOME/.cargo/registry/src/**/dioxus-tw-components-*/src/**/*.{rs,html,css}`", "./src/**/*.{rs,html,css}", "./dist/**/*.html"],
  theme: {
    extend: {},
  },
  plugins: [],
  ux: {
    themes: {
      light: {
        primary: 'oklch(49.12% 0.3096 275.75)',
        secondary: 'oklch(69.71% 0.329 342.55)',
        'secondary-content': 'oklch(98.71% 0.0106 342.55)',
        accent: 'oklch(76.76% 0.184 183.61)',
        neutral: '#2B3440',
        'neutral-content': '#D7DDE4',
        'surface-100': 'oklch(100% 0 0)',
        'surface-200': '#F2F2F2',
        'surface-300': '#E5E6E6',
        'surface-content': '#1f2937',
      },
    }
  }
};
