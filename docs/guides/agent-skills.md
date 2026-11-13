# Agent Skills

Coding and research agents keep asking for the same references. Your folders already hold them. Linkdean turns a folder or a tag into an [Agent Skill](https://agentskills.io): a small folder with a `SKILL.md` file that Claude Code, Codex, Cursor and other agents load when your task matches it. Capability summary: [features.md](../product/features.md).

## What is in a skill

- A **name** and a **description** that say what the links cover and when to use them. The description lists the subfolders and the most used tags, so the agent picks the skill for the right tasks.
- Short instructions: prefer these links over a web search, cite the link used, and treat your notes as context.
- The **links**, grouped by subfolder, each with its title, description, tags and your notes.
- More than 100 links move to `references/LINKS.md`. The agent reads that file only when a task needs it.

A skill holds up to 2,000 bookmarks and says how many it left out.

Never included:

- vault folders and vault bookmarks;
- bookmarks in Deleted items;
- someone else's "Only me" bookmarks, or their notes, in a folder shared with you.

## Download a skill

- **From a folder:** in the library's Explorer view, open a folder's **⋯** menu and choose **Agent Skill (.zip)**. Subfolders are included.
- **From a folder or a tag:** go to **Config options → Import / Export → Agent Skill**, choose **Folder** or **Tag**, pick one and select **Download skill**. A tag skill covers your own bookmarks.

Unzip it into your agent's skills folder. For Claude Code that is `~/.claude/skills` for every project, or `.claude/skills` inside one project:

```bash
unzip rust-web.zip -d ~/.claude/skills
```

## Keep it current with an API key

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
