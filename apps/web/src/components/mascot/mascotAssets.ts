/** Committed WebP under `public/mascot/` (2× display). Sources stay in untracked brand/graphics. */

export const MASCOT = {
  standing: '/mascot/dean-standing.webp',
  standingLg: '/mascot/dean-standing-lg.webp',
  thinking: '/mascot/dean-thinking.webp',
  lookingUp: '/mascot/dean-looking-up.webp',
  smiling: '/mascot/dean-smiling.webp',
  approves: '/mascot/deen-approves.webp',
} as const

export type MascotSrc = (typeof MASCOT)[keyof typeof MASCOT]

/** CSS display width for EmptyState illustrations (asset is ~2×). */
export const MASCOT_EMPTY_DISPLAY_PX = 180

/** CSS display width for the login side panel (asset is ~2×). */
export const MASCOT_LOGIN_DISPLAY_PX = 280
