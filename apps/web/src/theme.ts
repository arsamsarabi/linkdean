import { createTheme, type MantineColorsTuple, type MantineThemeOverride } from '@mantine/core'

/**
 * Brand —
 * Ice Cold #A0D2EB · Freeze Purple #E5EAF5 · Medium Purple #D0BDF4
 * Purple Pain #8458B3 · Heavy Purple #A28089
 */

/** Purple Pain — primary; #8458B3 @ 5 */
const violet: MantineColorsTuple = [
  '#F7F2FC',
  '#EDE4F7',
  '#D9C6EB',
  '#C1A3DE',
  '#A57DCD',
  '#8458B3',
  '#6E4796',
  '#58387A',
  '#432A5C',
  '#2E1D40',
]

/** Ice Cold — cool accent; #A0D2EB @ 3 */
const ice: MantineColorsTuple = [
  '#F0F8FC',
  '#E0F1F8',
  '#C5E5F3',
  '#A0D2EB',
  '#7ABFE0',
  '#5AABD0',
  '#458EAF',
  '#35708C',
  '#28556B',
  '#1C3C4C',
]

/** Medium Purple — soft accent; #D0BDF4 @ 3 */
const orchid: MantineColorsTuple = [
  '#F8F5FD',
  '#F0EAFB',
  '#E4D8F8',
  '#D0BDF4',
  '#B89AEB',
  '#9F7ADF',
  '#8560C4',
  '#6B4BA3',
  '#523A7F',
  '#3A295A',
]

/** Neutrals: Freeze Purple → Heavy Purple */
const mist: MantineColorsTuple = [
  '#E5EAF5',
  '#D8DDE8',
  '#C5CAD6',
  '#ADB3C0',
  '#949AA8',
  '#A28089',
  '#7A5F66',
  '#5C484E',
  '#3F3236',
  '#2A2225',
]

/**
 * Dark-mode surfaces — neutral charcoal from Heavy Purple (not violet).
 * Mantine: dark[6] ≈ Paper/Card, dark[7] ≈ body.
 */
const charcoal: MantineColorsTuple = [
  '#E5EAF5',
  '#C5CAD6',
  '#A28089',
  '#7A5F66',
  '#5C484E',
  '#3F3236',
  '#2A2225',
  '#2A2225',
  '#1A1618',
  '#121011',
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
  primaryColor: 'violet',
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
    violet,
    ice,
    orchid,
    mist,
    /** @deprecated alias — prefer violet */
    harbor: violet,
    /** @deprecated alias — prefer orchid */
    mauve: orchid,
    green: softGreen,
    red: softCoral,
    yellow: softApricot,
    blue: ice,
    gray: mist,
    dark: charcoal,
  },
  primaryShade: { light: 5, dark: 5 },
  white: '#E5EAF5',
  black: '#121011',
  other: {
    brandIce: '#A0D2EB',
    brandFreeze: '#E5EAF5',
    brandOrchid: '#D0BDF4',
    brandPain: '#8458B3',
    brandHeavy: '#A28089',
    brandNight: '#1A1618',
    pageMaxWidth: 1140,
  },
  components: {
    Anchor: {
      defaultProps: { c: 'ice' },
    },
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
