#!/usr/bin/env node
/**
 * Render `static/openapi.json` (written by the API's `committed_spec_is_current` test) into
 * `docs/api-reference.md`. Runs as part of `pnpm sync`. The raw spec is published next to the
 * page at /openapi.json for client generators.
 */
import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const appRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const spec = JSON.parse(readFileSync(join(appRoot, 'static/openapi.json'), 'utf8'));
const schemas = spec.components?.schemas ?? {};
const schemes = spec.components?.securitySchemes ?? {};

const esc = (s) => String(s ?? '').replace(/\|/g, '\\|').replace(/\n+/g, ' ').trim();
const anchor = (name) => `schema-${name.toLowerCase()}`;

/** A short type label, linking named schemas. */
function typeOf(s) {
  if (!s) return 'any';
  if (s.$ref) {
    const name = s.$ref.split('/').pop();
    return `[${name}](#${anchor(name)})`;
  }
  if (s.oneOf || s.anyOf) {
    const parts = (s.oneOf ?? s.anyOf).map(typeOf).filter((t) => t !== 'null');
    return parts.join(' or ') || 'any';
  }
  if (s.allOf) return s.allOf.map(typeOf).join(' & ');
  if (s.enum) return s.enum.map((v) => `\`${v}\``).join(', ');
  let t = s.type;
  if (Array.isArray(t)) t = t.filter((x) => x !== 'null').join(' or ');
  if (t === 'array') return `${typeOf(s.items)}[]`;
  if (t === 'object' && s.additionalProperties && typeof s.additionalProperties === 'object')
    return `map of ${typeOf(s.additionalProperties)}`;
  if (!t) return 'any';
  return s.format ? `${t} (${s.format})` : t;
}

function propsTable(s) {
  const props = s.properties ?? {};
  const req = new Set(s.required ?? []);
  const rows = Object.entries(props).map(
    ([k, v]) => `| \`${k}\` | ${typeOf(v)} | ${req.has(k) ? 'yes' : ''} | ${esc(v.description)} |`,
  );
  if (!rows.length) return '';
  return ['| Field | Type | Required | Notes |', '| --- | --- | --- | --- |', ...rows].join('\n');
}

function bodySchema(content) {
  if (!content) return null;
  const [type, media] = Object.entries(content)[0];
  return { type, label: media.schema ? typeOf(media.schema) : null };
}

function authLabel(op) {
  if (!op.security?.length) return 'None (public).';
  const names = op.security.flatMap((r) => Object.keys(r));
  if (names.includes('api_key')) return 'API key or sign-in session (bearer token or cookie).';
  return 'Sign-in session only (bearer session token or cookie); API keys are refused.';
}

const out = [];
out.push('---', 'format: md', 'id: api-reference', 'title: API reference', 'sidebar_position: 7', '---', '');
out.push('# API reference', '');
out.push(
  `Every endpoint of the Linkdean HTTP API (version ${spec.info.version}), generated from the routes. ` +
    'The same description as an OpenAPI 3.1 document: [openapi.json](pathname:///openapi.json). ' +
    'A running instance also serves its own copy at `/api/openapi.json`.',
  '',
);
out.push('## Basics', '');
out.push(
  '- **Base URL**: `https://<your instance>/api`, for example `https://app.linkdean.app/api`.',
  '- **Authentication**: send an API key (create one under **Config options → API keys**) as `Authorization: Bearer ldk_…`. ' +
    'Endpoints marked *sign-in session only* refuse API keys, so a leaked key cannot mint more keys or use admin powers.',
  '- **Errors**: every error has the same JSON body, `{ "code": "not_found", "message": "not found", "details": { … }, "error": "not found" }`. ' +
    'Match on `code`, show `message`. `details` appears only for some codes (`banned`, `quota_exceeded`, `vault_move_blocked`). ' +
    '`error` repeats `message` for older clients.',
  '- **Caching**: successful JSON reads carry an `ETag` and `Cache-Control: private, no-cache`. Send the tag back in `If-None-Match` to get `304 Not Modified` with no body.',
  '- **Rate limits**: sign-in, registration and some writes answer `429` with code `too_many_requests` when called too often.',
  '- **Request IDs**: every response has an `X-Request-Id`. Send your own (letters, digits, `_`, `-`, `.`, up to 128) to follow a request through the server logs and traces.',
  '',
);
const codes = [
  ['bad_request', 400, 'The input was refused; `message` says why.'],
  ['invalid_body', 422, 'The JSON body did not match the expected fields.'],
  ['unauthorized', 401, 'Wrong password, or no credentials where they are needed.'],
  ['session_expired', 401, 'The session or API key is missing, expired or revoked.'],
  ['forbidden', 403, 'Signed in, but not allowed.'],
  ['banned', 403, 'The account is banned; `details.ban` has the reason and end.'],
  ['email_unverified', 403, 'The instance requires a verified email address first.'],
  ['quota_exceeded', 403, '`details` has `meter`, `limit` and `used`.'],
  ['not_found', 404, 'No such thing, or not visible to you.'],
  ['method_not_allowed', 405, 'The path exists with other methods.'],
  ['conflict', 409, 'Already exists or changed meanwhile.'],
  ['vault_move_blocked', 409, '`details.blockers` lists what to undo first.'],
  ['payload_too_large', 413, 'The upload is over the limit.'],
  ['unsupported_media_type', 415, 'Send `Content-Type: application/json`.'],
  ['vault_locked', 423, 'Unlock the vault first.'],
  ['vault_unlock_failed', 401, 'Wrong vault passphrase.'],
  ['too_many_requests', 429, 'Slow down and retry later.'],
  ['internal', 500, 'Server error; quote the `X-Request-Id`.'],
];
out.push('### Error codes', '', '| Code | Status | Meaning |', '| --- | --- | --- |');
for (const [c, s, m] of codes) out.push(`| \`${c}\` | ${s} | ${m} |`);
out.push('');

// Group operations by tag, in path order.
const byTag = new Map();
for (const [path, item] of Object.entries(spec.paths)) {
  for (const [method, op] of Object.entries(item)) {
    if (method === 'parameters') continue;
    const tag = op.tags?.[0] ?? 'other';
    if (!byTag.has(tag)) byTag.set(tag, []);
    byTag.get(tag).push({ path, method, op });
  }
}
const titles = { me: 'Me', meta: 'This document' };
out.push('## Endpoints', '');
for (const tag of [...byTag.keys()].sort()) {
  const title = titles[tag] ?? tag.charAt(0).toUpperCase() + tag.slice(1).replace(/-/g, ' ');
  out.push(`### ${title}`, '');
  for (const { path, method, op } of byTag.get(tag)) {
    out.push(`#### \`${method.toUpperCase()} ${path}\``, '');
    const text = [op.summary, op.description].filter(Boolean).join('\n\n');
    if (text) out.push(text, '');
    out.push(`**Auth:** ${authLabel(op)}`, '');
    const params = op.parameters ?? [];
    if (params.length) {
      out.push('| Parameter | In | Type | Required | Notes |', '| --- | --- | --- | --- | --- |');
      for (const p of params)
        out.push(
          `| \`${p.name}\` | ${p.in} | ${typeOf(p.schema)} | ${p.required ? 'yes' : ''} | ${esc(p.description)} |`,
        );
      out.push('');
    }
    const body = bodySchema(op.requestBody?.content);
    if (body) out.push(`**Body** (\`${body.type}\`): ${body.label ?? esc(op.requestBody.description)}`, '');
    const ok = Object.entries(op.responses ?? {}).filter(([s]) => !s.endsWith('XX'));
    for (const [status, r] of ok) {
      const b = bodySchema(r.content);
      out.push(`**${status}**${b?.label ? `: ${b.label}` : r.description ? `: ${esc(r.description)}` : ''}`, '');
    }
  }
}

out.push('## Schemas', '');
for (const name of Object.keys(schemas).sort()) {
  const s = schemas[name];
  out.push(`### ${name} {#${anchor(name)}}`, '');
  if (s.description) out.push(esc(s.description), '');
  const table = propsTable(s);
  if (table) out.push(table, '');
  else out.push(`Type: ${typeOf(s)}`, '');
  for (const variant of s.oneOf ?? []) {
    const t = propsTable(variant);
    if (t) out.push(t, '');
  }
}

writeFileSync(join(appRoot, 'docs/api-reference.md'), out.join('\n') + '\n');
console.log(`api-reference.md: ${Object.keys(spec.paths).length} paths, ${Object.keys(schemas).length} schemas`);
