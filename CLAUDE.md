# CLAUDE.md

@AGENTS.md

## Claude Code notes

- Start a build step with `/run <prompt id>`, for example `/run P2.2`. It loads the prompt from `docs/prompts/`.
- Before finishing, run `/verify`, then `/finish`.
- Use the `spec-checker` subagent at the end of every prompt to compare what you built with the prompt and the docs.
- Use the `dependency-verifier` subagent whenever you add or first use a crate or npm package.
- Use the `security-reviewer` subagent after any change to auth, permissions, crypto, licensing, networking or backups.
- Use plan mode for any prompt marked "Size: large": show the plan, wait for approval, then build.
- Never run commands that delete files outside this repository, change global git config, or publish anything.
