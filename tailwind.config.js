/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    "./src/**/*.{js,jsx,ts,tsx,html}",
  ],
  theme: {
    extend: {
      gridTemplateAreas: {
      'layout': [
        'a b',
      ],
      },
    },
  },
  plugins: [
    function ({ addUtilities }) {
      addUtilities({
        '.grid-areas-layout': {
          'grid-template-areas': '"a b"',
        },
        '.grid-area-a': {
          'grid-area': 'a',
        },
        '.grid-area-b': {
          'grid-area': 'b',
        },
      })
    }
  ],
}