/** Shortest new password or vault passphrase. Matches `MIN_SECRET_LEN` in the API. */
export const MIN_SECRET_LENGTH = 12

export type PasswordPairResult = { ok: true } | { ok: false; message: string }

export type PasswordPairMessages = {
  tooShort: string
  mismatch: string
}

const DEFAULT_MESSAGES: PasswordPairMessages = {
  tooShort: `Password min ${MIN_SECRET_LENGTH} chars`,
  mismatch: 'Passwords mismatch',
}

/** Shared password / passphrase pair checks (minimum length, match). */
export function validatePasswordPair(
  password: string,
  confirm: string,
  messages: PasswordPairMessages = DEFAULT_MESSAGES
): PasswordPairResult {
  if (password.length < MIN_SECRET_LENGTH) {
    return { ok: false, message: messages.tooShort }
  }
  if (password !== confirm) {
    return { ok: false, message: messages.mismatch }
  }
  return { ok: true }
}
