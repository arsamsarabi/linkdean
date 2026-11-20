# Agent Skills

Coding and research agents keep asking for the same references. Your folders already hold them. Linkdean exposes the library to agents in two ways: a live **MCP** connection for search, read and scoped writes, and a downloadable [Agent Skill](https://agentskills.io) zip for offline use. Capability summary: [features.md](../product/features.md).

**Experimental.** Both paths are off unless the server sets `EXPERIMENTAL_AGENT_SKILLS=true` ([environment variables](../self-hosting/environment.md#experimental-features)). On Linkdean's hosted beta they are not switched on yet.

## Connect with MCP

Point an MCP client at your instance with an API key. Create a key under **Config options → API keys** (use **Save bookmarks** or custom scopes with `library:write` if the agent should create or edit). The **MCP** card under **Config options → Import / Export** shows the endpoint for this host.

Endpoint: `https://your-linkdean/api/mcp` (Streamable HTTP). Send the key on every request:

```http
Authorization: Bearer ldk_…
```

Illustrative client config (field names follow your client's docs):

```json
{
  "mcpServers": {
    "linkdean": {
      "url": "https://your-linkdean/api/mcp",
      "headers": {
        "Authorization": "Bearer ldk_…"
      }
    }
  }
}
```

Tools (each call needs the matching API key scope):

| Tool | Scope | What it does |
| --- | --- | --- |
| `search_bookmarks` | `library:read` | Search or browse by text, folder and/or tag (folder includes subfolders) |
| `get_bookmark` | `library:read` | One bookmark: title, URL, notes, tags, folders, dates |
| `list_folders` | `library:read` | Folder tree you can see (shared included; vault excluded) |
| `list_tags` | `library:read` | Tags in your library |
| `create_bookmark` | `library:write` | Create a bookmark (url required; optional title, notes, tags, folders) |
| `update_bookmark` | `library:write` | Update fields on a bookmark by id |
| `delete_bookmark` | `library:write` | Soft-delete (trash) a bookmark by id |
| `get_archive_status` | `archive:read` | Whether an archive copy exists and which formats — metadata only |
| `get_archive_text` | `archive:read` | Reader-mode Markdown from an HTML archive when a copy exists |

Never included: vault folders and bookmarks, or raw archive file bytes (PDF/HTML/file). Reader text is available only through `get_archive_text`. A missing scope returns a clear tool error.

When the flag is off, `/api/mcp` answers `404`.

## Download a skill (offline)

A folder, a tag, or a saved library search becomes an Agent Skill: a small folder with a `SKILL.md` that Claude Code, Codex, Cursor and other agents load when your task matches it.

### What is in a skill

- A **name** and a **description** that say what the links cover and when to use them. The description lists the subfolders and the most used tags, so the agent picks the skill for the right tasks.
- Short instructions: prefer these links over a web search, cite the link used, and treat your notes as context.
- The **links**, grouped by subfolder, each with its title, description, tags and your notes.
- More than 100 links move to `references/LINKS.md`. The agent reads that file only when a task needs it.

A skill holds up to 2,000 bookmarks and says how many it left out.

Never included:

- vault folders and vault bookmarks;
- bookmarks in Deleted items;
- someone else's "Only me" bookmarks, or their notes, in a folder shared with you.

### Download

- **From a folder:** in the library's Explorer view, open a folder's **⋯** menu and choose **Agent Skill (.zip)**. Subfolders are included.
- **From a folder, tag, or saved search:** go to **Config options → Import / Export → Agent Skill**, choose **Folder**, **Tag**, or **Saved search**, pick one and select **Download skill**. A tag skill covers your own bookmarks. A saved-search skill uses the search's live matches (vault bookmarks stay out); you can also download from a saved search's controls in the library toolbar or sidebar.

Unzip it into your agent's skills folder. For Claude Code that is `~/.claude/skills` for every project, or `.claude/skills` inside one project:

```bash
unzip rust-web.zip -d ~/.claude/skills
```

### Keep it current with an API key

Create a key under **Config options → API keys**, then fetch the skill from a script or a scheduled job. The Agent Skill card shows the command with the folder or tag filled in.

```bash
curl -H "Authorization: Bearer ldk_…" \
  "https://your-linkdean/api/agent-skill?folder=<folder id>" -o skill.zip
unzip -o skill.zip -d ~/.claude/skills
```

| Parameter | Meaning |
| --- | --- |
| `folder` | A folder id. Subfolders are included. |
| `tag` | A tag name. Give `folder` or `tag`, not both. |
| `format` | `zip` (default): a `<name>/` folder. `md`: the `SKILL.md` alone, with every link inline. |

The answer is `404` when the folder is not yours or shared with you, or when no bookmark has the tag, and `400` for a vault folder.
