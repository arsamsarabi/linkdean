import {
  createTheme,
  type CSSVariablesResolver,
  type MantineColorsTuple,
  type MantineThemeOverride,
} from '@mantine/core'

/*
 * Brand: Ice Cold #A0D2EB · Freeze Purple #E5EAF5 · Medium Purple #D0BDF4
 * Purple Pain #8458B3 · Heavy Purple #A28089
 *
 * Every scale is generated in OKLCH on one lightness ramp, so the same shade index has the
 * same lightness across colours, and the brand hex is pinned at its index. Filled shade is 6
 * in both schemes, and every 6 carries white text at WCAG AA.
 */

/** Purple Pain, the primary. Brand #8458B3 sits at shade 6, the filled shade in both schemes (5.2:1 with white text). */
const violet: MantineColorsTuple = [
  '#F8F3FE',
  '#EFE4FE',
  '#E1CAFF',
  '#CEABF9',
  '#B78BE9',
  '#9E6FD2',
  '#8458B3',
  '#6C4297',
  '#543078',
  '#3C2058',
]

/** Ice Cold, the link and info accent. Brand #A0D2EB at 3. Links use 7 in light (6.7:1 on white) and 3 in dark. */
const ice: MantineColorsTuple = [
  '#EFF7FB',
  '#DAEDF7',
  '#B5DDF1',
  '#A0D2EB',
  '#60AED2',
  '#3E95BB',
  '#1D7BA0',
  '#046384',
  '#004C66',
  '#023549',
]

/** Medium Purple, a soft accent for tints and washes. Brand #D0BDF4 at 3. */
const orchid: MantineColorsTuple = [
  '#F7F4FD',
  '#EDE6FB',
  '#DCCEF8',
  '#D0BDF4',
  '#AE92DE',
  '#9678C7',
  '#7D5FAC',
  '#654A8E',
  '#4E3770',
  '#372552',
]

/** Heavy Purple, a warm muted accent. Brand #A28089 at 5. */
const mauve: MantineColorsTuple = [
  '#F9F4F5',
  '#F2E6E9',
  '#E7CFD5',
  '#D6B3BC',
  '#BF96A1',
  '#A28089',
  '#8D646F',
  '#724E59',
  '#593B44',
  '#40292F',
]

/** Danger. Shade 6 carries white text at AA; 4 is the readable tint text in dark. */
const red: MantineColorsTuple = [
  '#FFF3F1',
  '#FFE2DF',
  '#FEC7C1',
  '#FFA098',
  '#EE7D76',
  '#D55F5A',
  '#B84542',
  '#993230',
  '#782222',
  '#581415',
]

/** Success. Same rules as red. */
const green: MantineColorsTuple = [
  '#F0F8F3',
  '#DDEFE2',
  '#BBE2C7',
  '#92CEA6',
  '#6CB687',
  '#4C9D6C',
  '#308354',
  '#1D6A40',
  '#0F522F',
  '#053B1F',
]

/** Warning, a soft apricot. Same rules as red. */
const yellow: MantineColorsTuple = [
  '#FBF4ED',
  '#F7E7D6',
  '#F1D0AC',
  '#E5B37D',
  '#D19551',
  '#B97A2C',
  '#9E6101',
  '#7F4E04',
  '#623B01',
  '#462900',
]

/** Light-scheme neutrals, tinted toward Freeze Purple (#E5EAF5). 6 is dimmed text (5.7:1 on white), 9 is body text. */
const gray: MantineColorsTuple = [
  '#F6F8FC',
  '#EDEFF5',
  '#E1E4EB',
  '#D3D7E0',
  '#C2C5CF',
  '#9A9EA9',
  '#636772',
  '#484B56',
  '#31343D',
  '#1B1E25',
]

/** Dark-scheme neutrals, near-black with a faint violet tint. 0 text, 2 dimmed, 4 border, 6 inputs, 7 surfaces, 8 page canvas. */
const dark: MantineColorsTuple = [
  '#ECECF1',
  '#CAC9D1',
  '#A9A8B2',
  '#86858F',
  '#414049',
  '#313038',
  '#26252C',
  '#1C1B21',
  '#121217',
  '#0B0A0E',
]

export const linkdeanTheme: MantineThemeOverride = createTheme({
  primaryColor: 'violet',
  primaryShade: { light: 6, dark: 6 },
  autoContrast: true,
  white: '#FFFFFF',
  black: gray[9],
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
    mauve,
    red,
    green,
    yellow,
    gray,
    dark,
    blue: ice,
  },
  other: {
    pageMaxWidth: 1440,
  },
  components: {
    Modal: {
      defaultProps: { centered: true },
    },
    Drawer: {
      // Side panels keep the page behind them readable, so you can see what you are editing.
      defaultProps: { overlayProps: { backgroundOpacity: 0.2, blur: 0 } },
    },
    Tooltip: {
      defaultProps: { withArrow: true, openDelay: 400 },
    },
    Skeleton: {
      defaultProps: { radius: 'sm' },
    },
  },
})

/**
 * The primary's light variant (active nav item, toggled filters, tinted badges). Mantine's
 * default is shade 1 in light, almost white, and a 10% wash in dark, almost black. These sit
 * one step stronger in light and clearly purple in dark; text stays at AA on both.
 */
const violetLight = {
  light: {
    '--mantine-color-violet-light': 'var(--mantine-color-violet-2)',
    '--mantine-color-violet-light-hover': 'var(--mantine-color-violet-3)',
    '--mantine-color-violet-light-color': 'var(--mantine-color-violet-8)',
  },
  dark: {
    '--mantine-color-violet-light': 'color-mix(in srgb, var(--mantine-color-violet-5) 32%, transparent)',
    '--mantine-color-violet-light-hover': 'color-mix(in srgb, var(--mantine-color-violet-5) 42%, transparent)',
    '--mantine-color-violet-light-color': 'var(--mantine-color-violet-1)',
  },
}

/**
 * Semantic colours. `--mantine-color-body` is the surface of cards, modals and menus;
 * `--ld-canvas` is the page behind them, one step deeper so surfaces read as raised.
 */
export const linkdeanCssVariables: CSSVariablesResolver = () => ({
  variables: {},
  light: {
    ...violetLight.light,
    '--ld-canvas': 'var(--mantine-color-gray-1)',
    '--mantine-color-anchor': 'var(--mantine-color-ice-7)',
    '--mantine-color-dimmed': 'var(--mantine-color-gray-6)',
    '--mantine-color-placeholder': 'var(--mantine-color-gray-6)',
    '--mantine-color-default-border': 'var(--mantine-color-gray-3)',
    '--mantine-color-error': 'var(--mantine-color-red-6)',
    '--mantine-color-success': 'var(--mantine-color-green-7)',
  },
  dark: {
    ...violetLight.dark,
    '--ld-canvas': 'var(--mantine-color-dark-8)',
    '--mantine-color-anchor': 'var(--mantine-color-ice-3)',
    '--mantine-color-error': 'var(--mantine-color-red-4)',
    '--mantine-color-success': 'var(--mantine-color-green-4)',
  },
})
