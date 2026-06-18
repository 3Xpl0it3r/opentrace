/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{ts,tsx}'],
  theme: {
    extend: {
      colors: {
        brand: {
          DEFAULT: '#6C5CE7',
          light: '#a29bfe',
          dark: '#4834d4',
        },
        accent: {
          DEFAULT: '#00CEC9',
        },
        surface: {
          DEFAULT: '#12151C',
          card: '#181C25',
          elevated: '#1E2330',
          hover: '#252A38',
        },
        root: {
          DEFAULT: '#0B0D11',
        },
        border: {
          DEFAULT: '#2A2F3E',
          light: '#363C4E',
        },
        green: {
          DEFAULT: '#00D68F',
          dim: 'rgba(0,214,143,.12)',
        },
        red: {
          DEFAULT: '#FF4D6A',
          dim: 'rgba(255,77,106,.12)',
        },
        yellow: {
          DEFAULT: '#FFB800',
          dim: 'rgba(255,184,0,.12)',
        },
        blue: {
          DEFAULT: '#3B82F6',
          dim: 'rgba(59,130,246,.12)',
        },
      },
      fontFamily: {
        sans: ['Inter', 'system-ui', 'sans-serif'],
        mono: ['JetBrains Mono', 'ui-monospace', 'Menlo', 'monospace'],
      },
    },
  },
  plugins: [],
}
