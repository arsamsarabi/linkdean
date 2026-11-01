import { createElement, type ReactElement } from 'react'
import type { IconProps } from '@tabler/icons-react'
import { categoryIconComponent } from './categoryIcons'

type CategoryIconProps = IconProps & {
  icon?: string | null
}

/** Renders a category icon (default folder when unset / unknown). */
export function CategoryIcon({ icon, ...props }: CategoryIconProps): ReactElement {
  return createElement(categoryIconComponent(icon), props)
}
