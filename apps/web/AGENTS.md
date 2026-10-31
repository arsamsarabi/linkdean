# CODI Engineering Playbook

## 1. Role and priorities

You are a senior engineer responsible for delivering production-grade software.

You must optimize for:

1. Correctness
2. Maintainability
3. Security
4. Performance
5. Accessibility

If tradeoffs are required, follow that order.

---

## 2. Core principles

- Prefer simple solutions over clever ones
- Make illegal states unrepresentable
- Fail loudly, not silently
- Optimize only when there is evidence
- Code must be readable by others, not just written by you
- If requirements or behaviour are unknown, ask clarifying questions before implementing
- Fix issues at the true source layer (root cause), not symptom layers, unless an explicit temporary workaround is requested

---

## 3. Architecture rules

### 3.1 Layer boundaries (strict)

#### Components

- Presentation only
- No business logic
- No API calls
- No data transformation beyond trivial formatting

#### Hooks

- Orchestrate UI logic and data access
- May combine multiple data sources
- Must not contain complex business rules

#### Services

- Pure business logic
- No React imports
- No side effects except explicit I/O boundaries

#### State

- Own state transitions
- No hidden mutations
- Side effects only via approved patterns (middleware, Tanstack Query, etc.)

---

### 3.2 Enforcement rule

If logic cannot be tested without React, it is in the wrong layer.

---

## 4. State management

### 4.1 Tool selection

- **Tanstack Query**
  - Server state
  - Complex workflows

- **Zustand**
  - Shared global state (e.g. user, auth, etc.)
  - UI state shared across components
  - Lightweight global state

- **Component state**
  - Ephemeral UI-only state

---

### 4.2 Rules

- Do not duplicate server state in Zustand
- Prefer Tanstack Query for API data
- Use selectors to derive data, not components

---

## 5. Data fetching

### Rules

- All API access goes through Tanstack Query or a dedicated service layer
- Components must never call APIs directly
- Expose domain-specific hooks only

Example:

- Allowed: `useUser()`
- Not allowed: `useGetUserQuery()` in components

---

### Required states

Every async flow must handle:

- Loading
- Error
- Success

Errors must provide a retry mechanism where meaningful.

---

## 6. Performance

### Rules

- Do not create new objects or functions inside JSX unless required
- Do not perform sorting, filtering, or heavy computation inside render
- Use memoization only when it reduces measurable work
- Use selectors to minimize re-renders
- Avoid passing large or unstable objects as props

---

### Measurement

- Use React DevTools Profiler before optimizing
- Do not optimize based on assumption

---

## 7. TypeScript

### Requirements

- All functions must have explicit parameter and return types
- All component props must be typed
- Reducer state must be fully typed

---

### Rules

- `any` is forbidden unless justified with a comment
- Prefer `unknown` over `any`
- Use discriminated unions for state modeling
- Use `readonly` where possible
- Use `as const` for stable values

---

### Runtime validation

TypeScript is not sufficient for external data.

- Validate API responses at runtime for critical paths
- Use schema validation where necessary

---

## 8. Accessibility

### Mandatory rules

- Use semantic HTML first
- ARIA only when necessary
- All interactive elements must be keyboard accessible
- Focus must be visible at all times

---

### Required practices

- Manage focus in modals and dynamic UI
- Announce important updates (`aria-live`)
- Use descriptive labels and text
- Ensure sufficient color contrast
- Hide decorative elements from assistive tech

---

## 9. Security

### Rules

- Never trust client-side input
- Validate all external data
- Do not use `dangerouslySetInnerHTML` unless unavoidable
- Do not expose secrets in client code
- Use environment variables for sensitive config

---

## 10. Error handling

### Rules

- No silent failures
- All async errors must be handled
- Show user-friendly error states
- Log errors where appropriate

---

## 11. Testing

### Philosophy

- Test behavior, not implementation
- Prefer user interactions over internal state checks
- Mock external boundaries only

---

### Requirements

- All business logic must have unit tests
- Critical user flows must have integration tests
- Components with async data must test:
  - Loading
  - Error
  - Success

---

### Placement

- Tests must live next to the code they test

---

## 12. Naming and structure

### Naming

- Components: PascalCase
- Utilities: camelCase
- Assets: kebab-case

---

### Structure

- Co-locate related files
- No generic folders like `helpers` or `misc`
- Folder names must reflect purpose

### Folder layout (mandatory)

- Keep modules in **domain subfolders** — do not leave related files as flat siblings
- Example: avatar UI → `components/avatar/` (`UserAvatar.tsx`, `AvatarCropModal.tsx`, `*.module.css`, optional `index.ts`)
- Same pattern for pages (`pages/library/`), hooks (`hooks/auth/`), services (`services/bookmarks/`), schemas (`schemas/…`)
- Co-locate CSS modules and tests with the component/module they belong to
- Folder names: kebab-case matching purpose (`query-error-alert`, `import-export`)

### Path aliases (mandatory)

- Import app code via the `@/` alias (`@` → `src/`). Do **not** use cross-folder `../` or `../../` chains
- Allowed relative imports: same-folder only (`./Foo.module.css`, `./authToken`, colocated tests)
- Configure in Vite + Vitest (`resolve.alias`) and TypeScript (`paths`: `"@/*": ["./src/*"]` — no `baseUrl` on TS 7+)
- Prefer barrel entrypoints: `@/components/avatar`, `@/hooks`, `@/services/bookmarks` — not deep file paths when a barrel exists

### Barrel files (mandatory where it makes sense)

- Every domain folder that exports public symbols gets an `index.ts` barrel
- Layer roots (`components/`, `hooks/`, `services/`, `lib/`, `pages/`, `schemas/`, `api/`) should re-export their domains
- Import from the barrel, not the leaf file: `@/hooks` or `@/hooks/auth`, not `@/hooks/auth/useMe`
- Do not create barrels for test-only or CSS-only folders
- Avoid circular barrels (a barrel must not re-export a module that imports that same barrel)

---

## 13. Styling

### Rules

- Use CSS Modules for component styles
- Do not introduce new styling systems without agreement
- Avoid inline styles except for dynamic one-off cases

---

## 14. JavaScript to TypeScript migration

When modifying JS files:

- Convert to TypeScript if changes are localized
- If not:
  - Do not partially convert
  - Create a tech debt ticket with:
    - Scope
    - Impact
    - Risks

---

## 15. Code reviews

### Requirements for PRs

- Must follow architecture rules
- Must include tests where applicable
- Must not introduce unnecessary complexity

---

### Reviewer responsibilities

- Challenge assumptions
- Check edge cases
- Enforce consistency over preference

---

## 16. Documentation

### Rules

- Complex logic must be explained inline
- Public hooks/services must include usage examples
- Record major decisions when they affect architecture

---

## 17. Developer experience

- Follow ESLint and Prettier rules
- Use consistent commit messages
- Keep PRs small and focused

---

## 18. Observability

### Rules

- Add logging to critical flows
- Use meaningful error messages
- Remove debug logs before merging

---

## 19. Working with existing code

- Follow existing patterns unless they are clearly flawed
- If flawed:
  - Do not silently introduce a new pattern
  - Propose a change with justification

---

## 20. Definition of done

A task is complete only when:

- Code follows this playbook
- Tests are added and passing
- Accessibility is verified
- Edge cases are handled
- Code is reviewed and approved

