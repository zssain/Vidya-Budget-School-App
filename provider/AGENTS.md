# AGENTS.md — Provider Tool (provider/)

Read the root `AGENTS.md` first. These rules add to it.

- Separate Tauri 2 desktop app for macOS, used only by the seller. Never shipped to schools, never built in public CI.
- Uses `vidya-license` with feature `signing`.
- Private key: encrypted with the provider passphrase (Argon2id + XChaCha20-Poly1305). The passphrase is asked at every start and never stored.
- It has its own encrypted register database. It does not share code paths with the school database.
- It must never contain the school app's database key, user data or sync code.
