-- Operator template (NOT a migration): create the LOGIN roles each service connects as.
-- The migrations create the NOLOGIN privilege roles vidya_portal / vidya_worker /
-- vidya_signer; each deployment gets its own login role that only inherits one of them.
-- Generate passwords in a password manager; never commit them. Run as the DB owner.
--
--   psql "$ADMIN_URL" -v portal_pw="'…'" -v worker_pw="'…'" -v signer_pw="'…'" -f login-roles.example.sql

CREATE ROLE vidya_portal_login LOGIN PASSWORD :portal_pw IN ROLE vidya_portal;
CREATE ROLE vidya_worker_login LOGIN PASSWORD :worker_pw IN ROLE vidya_worker;
CREATE ROLE licence_signer     LOGIN PASSWORD :signer_pw IN ROLE vidya_signer;

-- Every statement in the apps is schema-qualified (billing.*); no search_path needed.
-- Supabase: these roles connect through the pooler/direct connection string with
-- their own user name; they are NOT exposed through the Data API (PostgREST). Keep the
-- `billing` schema OUT of "Exposed schemas" in the Supabase API settings.
