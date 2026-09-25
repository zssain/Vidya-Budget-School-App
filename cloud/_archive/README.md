# cloud/_archive — retired services (kept for reference)

Code here is **retired in v2 and is NOT built, shipped, or part of CI/release.**
It is kept only for reference and history. Each crate keeps its own `[workspace]`
and the root `Cargo.toml` `exclude`s `cloud/_archive`.

## licence/ — the online licence server (retired in Phase 12)

`cloud/_archive/licence/` was the Phase 10 online licence + checkout service
(`/v1/activate`, `/v1/check`, `/v1/transfer`, the admin panel and the
server-rendered download site). v2 replaces it entirely (prompts/P12, docs/00
§0/§10):

- Licences are now **offline files** minted on the owner's laptop with
  [`tools/licence-maker`](../../tools/licence-maker/README.md) and verified in the
  app against the build-config public key(s). There is no `LICENCE_API` and no
  online re-check.
- The checkout website is now the static [`site/`](../../site/) (UPI QR + a
  prefilled `mailto:`), and downloads come from `site/releases.json`.

Do not revive this service for production — it would reintroduce a company-run
server and break the zero-monthly-cost rule (docs/00 §0). It may still be run
locally (`--dev`) only to sign **v1** dev licences if ever needed for regression
testing of the v1 verification path.
