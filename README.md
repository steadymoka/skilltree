# skill-tree

Manage and visualize AI coding agent skill trees from the terminal.

AI 코딩 에이전트(Claude Code, Codex)의 스킬을 중앙에서 관리하고, 프로젝트별로 링크하는 CLI 도구입니다.

## Install

```bash
npm i -g skill-tree
```

Or build from source:

```bash
cd crate && cargo build --release
```

## Usage

```bash
# Interactive TUI
skill-tree

# Initialize skill directory (~/.skill-tree/)
skill-tree init

# Link skills by tag
skill-tree link authentication logging

# Link a single skill
skill-tree link-skill my-skill

# Unlink
skill-tree unlink my-skill
skill-tree unlink --all

# Print skill tree
skill-tree tree

# Open web dashboard
skill-tree serve
```

### Options

| Flag | Description |
|------|-------------|
| `--path <dir>` | Target project path |
| `--tool claude\|codex` | Target agent tool |

## How it works

```
~/.skill-tree/           # Central skill repository
├── skills.yaml          # Skill → tags mapping
└── <skill-name>/        # Skill directories

~/.claude/skills/        # Symlinks for Claude Code
~/.codex/skills/         # Symlinks for Codex
```

1. `skill-tree init` — 중앙 저장소(`~/.skill-tree/`)를 초기화하고 기존 스킬을 마이그레이션
2. TUI 또는 웹 대시보드에서 스킬을 태그별로 탐색
3. `skill-tree link <tags>` — 태그에 해당하는 스킬을 현재 프로젝트에 심볼릭 링크
4. 에이전트가 링크된 스킬을 자동으로 인식하여 활용

## Development

```bash
# Rust CLI
cd crate && cargo run

# Web dashboard
pnpm install && pnpm dev
```

## License

MIT
