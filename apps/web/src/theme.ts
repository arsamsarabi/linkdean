import { createTheme, type MantineColorsTuple, type MantineThemeOverride } from '@mantine/core'

/**
 * Brand — #213C51 · #6594B1 · #DDAED3 · #EEEEEE
 * Dark UI: near-black navy (not mid #213C51) + harbor highlights.
 */

/** Steel blue — primary; #6594B1 @ 5 */
const harbor: MantineColorsTuple = [
  '#F2F7FA',
  '#E4EEF4',
  '#C8DCE8',
  '#A5C5D6',
  '#84AEC6',
  '#6594B1',
  '#527A94',
  '#416178',
  '#324A5C',
  '#1A3040',
]

/** Soft mauve accent — #DDAED3 @ 4 */
const mauve: MantineColorsTuple = [
  '#FBF5FA',
  '#F5E8F3',
  '#EED5E9',
  '#E6C2DF',
  '#DDAED3',
  '#C994C0',
  '#B07AA8',
  '#916289',
  '#724B6B',
  '#54364F',
]

/** Neutrals from #EEEEEE → navy ink (light theme) */
const mist: MantineColorsTuple = [
  '#EEEEEE',
  '#E2E2E2',
  '#D0D0D0',
  '#B8B8B8',
  '#9A9A9A',
  '#7A7A7A',
  '#5C6A75',
  '#3F4A55',
  '#2C3D4A',
  '#213C51',
]

/**
 * Dark-mode surfaces — deep near-black navy.
 * Mantine: dark[6] ≈ Paper/Card, dark[7] ≈ body.
 */
const navyNight: MantineColorsTuple = [
  '#EEEEEE',
  '#C5D0DA',
  '#8FA8B8',
  '#6594B1',
  '#3D5F78',
  '#213C51',
  '#0B1520',
  '#0B1520',
  '#080F18',
  '#060C14',
]

/** Soft semantic — keep calm, not neon */
const softGreen: MantineColorsTuple = [
  '#F1F7F3',
  '#DDECE2',
  '#C2DCCB',
  '#A3C9B0',
  '#84B595',
  '#68997A',
  '#527C62',
  '#3F614B',
  '#2F4A39',
  '#21352A',
]

const softCoral: MantineColorsTuple = [
  '#FBF3F2',
  '#F5E0DD',
  '#ECC4BF',
  '#E0A49C',
  '#D1847A',
  '#B86860',
  '#9A504A',
  '#7A3E3A',
  '#5C2F2C',
  '#402220',
]

const softApricot: MantineColorsTuple = [
  '#FAF6F0',
  '#F0E6D6',
  '#E4D0B4',
  '#D5B890',
  '#C49F70',
  '#A88355',
  '#8A6A42',
  '#6C5233',
  '#4F3C26',
  '#352919',
]

export const linkdeanTheme: MantineThemeOverride = createTheme({
  primaryColor: 'harbor',
  fontFamily: 'Satoshi, system-ui, sans-serif',
  headings: {
    fontFamily: 'Rajdhani, system-ui, sans-serif',
    fontWeight: '700',
  },
  defaultRadius: 'md',
  spacing: {
    xs: '0.5rem',
    sm: '0.75rem',
    md: '1rem',
    lg: '1.5rem',
    xl: '2.25rem',
  },
  colors: {
    harbor,
    mauve,
    mist,
    green: softGreen,
    red: softCoral,
    yellow: softApricot,
    blue: harbor,
    gray: mist,
    dark: navyNight,
  },
  primaryShade: { light: 5, dark: 5 },
  white: '#EEEEEE',
  black: '#060C14',
  other: {
    brandLight: '#EEEEEE',
    brandBlue: '#6594B1',
    brandMauve: '#DDAED3',
    brandInk: '#213C51',
    brandNight: '#0B1520',
    pageMaxWidth: 1140,
  },
  components: {
    Button: {
      defaultProps: { radius: 'md' },
    },
    Card: {
      defaultProps: { radius: 'md' },
    },
    Paper: {
      defaultProps: { radius: 'md' },
    },
    Modal: {
      defaultProps: { radius: 'md', centered: true },
    },
    Drawer: {
      defaultProps: { radius: 'md' },
    },
    Tooltip: {
      defaultProps: { withArrow: true, openDelay: 400 },
    },
    Skeleton: {
      defaultProps: { radius: 'sm' },
    },
    Notification: {
      defaultProps: { radius: 'md' },
    },
  },
})
