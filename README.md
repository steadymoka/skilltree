# skilltree

Manage and visualize AI coding agent skill trees from the terminal.

A CLI tool for centrally managing skills for AI coding agents (Claude Code, Codex) and linking them per project.

## Install

```bash
npm i -g skilltree
```

Or build from source:

```bash
cargo build --release
```

## Usage

### Initialize

```bash
skilltree init
```

Sets up the central skill repository at `~/.skilltree/` and migrates any existing skills.

### Interactive TUI

```bash
skilltree
```

Browse and manage skills interactively in the terminal.

### Link skills

```bash
# Link by tags
skilltree link authentication logging

# Link a single skill by name
skilltree link-skill my-skill
```

### Unlink skills

```bash
# Unlink a specific skill
skilltree unlink my-skill

# Unlink all skills from the project
skilltree unlink --all
```

### View & Explore

```bash
# Print skill tree in the terminal
skilltree tree

# Open the web dashboard
skilltree serve
```

### Options

| Flag | Description |
|------|-------------|
| `--path <dir>` | Target project path (defaults to current directory) |
| `--tool claude\|codex` | Target agent tool (defaults to `claude`) |

## How it works

```
~/.skilltree/            # Central skill repository
├── skills.yaml          # Skill → tags mapping
└── <skill-name>/        # Skill directories

~/.claude/skills/        # Symlinks for Claude Code
~/.codex/skills/         # Symlinks for Codex
```

1. `skilltree init` — Initializes the central repository (`~/.skilltree/`) and migrates existing skills
2. Browse skills by tag using the TUI or web dashboard
3. `skilltree link <tags>` — Symlinks skills matching the given tags to the current project
4. The agent automatically recognizes and uses the linked skills

## Development

```bash
# Rust CLI
cargo run

# Web dashboard
cd web && pnpm install && pnpm dev
```

## License

MIT
