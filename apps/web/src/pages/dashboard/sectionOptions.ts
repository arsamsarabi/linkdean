import type { DashboardSectionDisplay } from '@/schemas'

export const DISPLAY_OPTIONS: { value: DashboardSectionDisplay; label: string }[] = [
  { value: 'icons', label: 'Icons' },
  { value: 'small', label: 'Small' },
  { value: 'medium', label: 'Medium' },
  { value: 'large', label: 'Large' },
]
