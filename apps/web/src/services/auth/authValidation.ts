export type PasswordPairResult =
  | { ok: true }
  | { ok: false; message: string }

export type PasswordPairMessages = {
  tooShort: string
  mismatch: string
}

const DEFAULT_MESSAGES: PasswordPairMessages = {
  tooShort: 'Password min 8 chars',
  mismatch: 'Passwords mismatch',
}

/** Shared password / passphrase pair checks (min 8, match). */
export function validatePasswordPair(
  password: string,
  confirm: string,
  messages: PasswordPairMessages = DEFAULT_MESSAGES,
): PasswordPairResult {
  if (password.length < 8) {
    return { ok: false, message: messages.tooShort }
  }
  if (password !== confirm) {
    return { ok: false, message: messages.mismatch }
  }
  return { ok: true }
}
