# Agent Skills: how it works

**What it is.** A folder or a tag in Linkdean, packaged as an [Agent Skill](https://agentskills.io) that AI agents (Claude Code, Codex, Cursor and others) can load.

**What it does.** It gives the agent your curated links as trusted context. When a task matches the skill's description, the agent loads it and uses your links before searching the web.

**How it works.**

1. **Pick a source:** a folder (subfolders included), a tag (your own bookmarks), or a saved library search (live filter matches).
2. **Collect the links** (`crates/db/src/repo/agent_skills.rs`). Linkdean checks that you can read the folder, then gathers the bookmarks. Vault and deleted items are left out, and so are other people's "Only me" bookmarks and their notes. Saved searches ignore vault filters on the snapshot. The limit is 2,000 bookmarks.
3. **Write the skill** (`crates/core/src/agent_skill`). This produces a `SKILL.md` with:
   - a spec-valid `name`;
   - a `description` built from the folder or tag, its subfolders and top tags, so the agent knows when to use it;
   - short usage notes;
   - the links grouped by subfolder, with tags and notes.

   Over 100 links go to `references/LINKS.md`, which the agent reads only when needed.
4. **Deliver it** (`GET /api/agent-skill?folder=|tag=|search=&format=zip|md`). By default this is a zip with a `<name>/` folder, built by a small built-in zip writer. `md` returns just the `SKILL.md`. The endpoint works with API keys, so a script can keep a skill current.
5. **Use it:** unzip into `~/.claude/skills` (or the agent's skills folder).

**Experimental flag.** Off by default. `EXPERIMENTAL_AGENT_SKILLS=true` merges the zip route and the MCP endpoint (otherwise `404`) and reports `experimental.agent_skills` in `GET /api/auth/config`, which the web app reads (`useExperimentalFlags`) to show the MCP and Agent Skill cards and the folder menu item. The dev compose file turns it on.

**Where in the app:** **Config options → Import / Export** leads with **MCP**; zip download stays secondary there and on a folder's **⋯ → Agent Skill (.zip)**.

User guide: [guides/agent-skills.md](../guides/agent-skills.md).

**Shipped:** read-only remote MCP for live search/read — [mcp-readonly.md](./mcp-readonly.md); HTML archive reader text via MCP — [mcp-archive-text.md](./mcp-archive-text.md); saved-search skills — [saved-searches.md](./saved-searches.md). Write tools remain a later roadmap item.
