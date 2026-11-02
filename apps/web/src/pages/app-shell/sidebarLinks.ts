/**
 * The router marks a link `aria-current` when its path matches, ignoring search, and Mantine
 * styles that as active; so Library would light up inside every folder. Match exactly, and let
 * each link's own `active` decide.
 */
export const EXACT = { exact: true, includeSearch: true } as const
