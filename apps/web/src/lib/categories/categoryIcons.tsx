import { createElement, type ComponentType, type ReactElement } from 'react'
import {
  IconBook,
  IconBookmark,
  IconBriefcase,
  IconCar,
  IconCode,
  IconCoffee,
  IconDeviceGamepad2,
  IconFlask,
  IconFolderFilled,
  IconHeart,
  IconHome,
  IconLock,
  IconMovie,
  IconMusic,
  IconNews,
  IconPhoto,
  IconPizza,
  IconPlane,
  IconSchool,
  IconShoppingCart,
  IconStar,
  IconTools,
  IconWorld,
  type IconProps,
} from '@tabler/icons-react'

export type CategoryIconKey =
  | 'bookmark'
  | 'star'
  | 'heart'
  | 'code'
  | 'book'
  | 'briefcase'
  | 'home'
  | 'music'
  | 'photo'
  | 'world'
  | 'gamepad'
  | 'cart'
  | 'school'
  | 'flask'
  | 'plane'
  | 'car'
  | 'pizza'
  | 'coffee'
  | 'movie'
  | 'news'
  | 'tools'
  | 'lock'

type IconEntry = {
  key: CategoryIconKey
  label: string
  Icon: ComponentType<IconProps>
}

export const CATEGORY_ICON_OPTIONS: readonly IconEntry[] = [
  { key: 'bookmark', label: 'Bookmark', Icon: IconBookmark },
  { key: 'star', label: 'Star', Icon: IconStar },
  { key: 'heart', label: 'Heart', Icon: IconHeart },
  { key: 'code', label: 'Code', Icon: IconCode },
  { key: 'book', label: 'Book', Icon: IconBook },
  { key: 'briefcase', label: 'Work', Icon: IconBriefcase },
  { key: 'home', label: 'Home', Icon: IconHome },
  { key: 'music', label: 'Music', Icon: IconMusic },
  { key: 'photo', label: 'Photos', Icon: IconPhoto },
  { key: 'world', label: 'Web', Icon: IconWorld },
  { key: 'gamepad', label: 'Games', Icon: IconDeviceGamepad2 },
  { key: 'cart', label: 'Shopping', Icon: IconShoppingCart },
  { key: 'school', label: 'School', Icon: IconSchool },
  { key: 'flask', label: 'Science', Icon: IconFlask },
  { key: 'plane', label: 'Travel', Icon: IconPlane },
  { key: 'car', label: 'Car', Icon: IconCar },
  { key: 'pizza', label: 'Food', Icon: IconPizza },
  { key: 'coffee', label: 'Coffee', Icon: IconCoffee },
  { key: 'movie', label: 'Movies', Icon: IconMovie },
  { key: 'news', label: 'News', Icon: IconNews },
  { key: 'tools', label: 'Tools', Icon: IconTools },
  { key: 'lock', label: 'Private', Icon: IconLock },
] as const

const BY_KEY = new Map(CATEGORY_ICON_OPTIONS.map((o) => [o.key, o]))

export function isCategoryIconKey(v: string | null | undefined): v is CategoryIconKey {
  return !!v && BY_KEY.has(v as CategoryIconKey)
}

export function categoryIconComponent(icon: string | null | undefined): ComponentType<IconProps> {
  if (isCategoryIconKey(icon)) return BY_KEY.get(icon)!.Icon
  return IconFolderFilled
}

type CategoryIconProps = IconProps & {
  icon?: string | null
}

/** Renders a category icon (default folder when unset / unknown). */
export function CategoryIcon({ icon, ...props }: CategoryIconProps): ReactElement {
  return createElement(categoryIconComponent(icon), props)
}
