import type { DashboardSectionDisplay, DashboardSectionWidth } from '@/schemas'

export const DISPLAY_OPTIONS: { value: DashboardSectionDisplay; label: string }[] = [
  { value: 'icons', label: 'Icons' },
  { value: 'small', label: 'Small' },
  { value: 'medium', label: 'Medium' },
  { value: 'large', label: 'Large' },
]

export const WIDTH_OPTIONS: { value: DashboardSectionWidth; label: string }[] = [
  { value: 'full', label: 'Full' },
  { value: 'half', label: 'Half' },
  { value: 'third', label: 'Third' },
]
