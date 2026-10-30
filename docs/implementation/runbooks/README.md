# Owner portal runbooks

| Runbook | When |
|---|---|
| [local-dev.md](local-dev.md) | Run the portal, worker and tests on a laptop (test mode, fake data). |
| [production-setup.md](production-setup.md) | First deployment: Supabase, owner bootstrap + MFA, Postgres roles + migrations, catalogue, Razorpay test→live, Resend, domains, worker + signer. |
| [operations.md](operations.md) | Running/monitoring the worker and signer, migrations, dead jobs, webhook retention, backups. |
| [payment-reconciliation.md](payment-reconciliation.md) | A customer paid but the dashboard disagrees; missed webhooks; refunds/disputes; manual payments. |
| [owner-recovery.md](owner-recovery.md) | Lost password, lost authenticator, suspected compromise, changing the owner account. |

Signing-key generation, rotation and backup: [../licence-signer.md](../licence-signer.md).
Architecture and security model: [../owner-portal.md](../owner-portal.md).

Never paste passwords, API secrets, private keys or MFA recovery codes into chat, tickets,
commits or screenshots. Put secrets in the hosting provider's secret store.
