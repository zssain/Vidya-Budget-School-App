# KNOWN_ISSUES.md

Agents list here anything unfinished, any placeholder the user can reach, and any risk found. Remove items when fixed.

| # | Found in | Issue | Impact | Plan |
|---|---|---|---|---|
| 1 | P1.1 (deny.toml) | cargo-deny reports 6 `unmaintained` advisories on transitive Tauri deps (rust-unic family, proc-macro-error). Scoped `unmaintained = "workspace"` so they don't fail CI. | Low — unmaintained, not vulnerabilities/unsound; those still fail the build. | Drop the scope override when Tauri stops pulling these crates. Revisit at P10.4 security review. |
| 2 | P1.1 (prettier) | Kit source-of-truth markdown (`AGENTS.md`, `CLAUDE.md`, `README-FIRST.md`, `VIDYA_PROMPTS_BOOK.md`) added to `.prettierignore` beyond the P1.1 spec list, because AGENTS.md rule #5 forbids reformatting them. | Low — cosmetic; these files are not prettier-checked. | Keep as-is; revisit only if the kit files are re-authored. |
| 3 | P1.3 (CSP) | Frontend templates still use inline `style` attributes, so the CSP permits them with `style-src-attr 'unsafe-inline'`. Inline scripts and style elements remain blocked. | Low — HTML interpolation is escaped, but removing the exception would further tighten defence in depth. | Move inline styles to CSS classes during UI cleanup. |
| 4 | P1.3 (icons) | The current app icon is a temporary blue rounded square generated for build testing. | Low — app bundling works, but the icon is not final branding. | Replace `app-icon.png` and regenerate icons when final artwork is supplied. |
