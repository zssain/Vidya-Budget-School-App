// `npm run migrate` — apply infra/licensing/migrations/*.sql in order.
// Use an ADMIN connection (database owner), never an application role:
//   BILLING_MIGRATION_DATABASE_URL=postgres://… npm run migrate
import { applyMigrations } from "@vidya/billing-core/migrations";
import pg from "pg";

const url = process.env.BILLING_MIGRATION_DATABASE_URL;
if (!url) {
  process.stderr.write("BILLING_MIGRATION_DATABASE_URL is required (admin connection)\n");
  process.exit(2);
}
const client = new pg.Client({ connectionString: url });
await client.connect();
try {
  const applied = await applyMigrations(client);
  process.stdout.write(applied.length ? `applied: ${applied.join(", ")}\n` : "up to date\n");
} finally {
  await client.end();
}
