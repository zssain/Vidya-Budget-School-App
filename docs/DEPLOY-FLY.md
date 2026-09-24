# Deploying Vidya's two cloud services to Fly.io

This guide walks a non-expert owner through hosting the two small Rust services
that Vidya needs online:

- **Relay** (`cloud/relay/`) — a stateless WebSocket relay. Binds `0.0.0.0:8788`,
  health endpoint `/healthz`. Target host: **`wss://relay.vidya.zuhairhussain.com`**.
- **Licence API** (`cloud/licence/`) — the licence service (binary `vidya-licence`),
  serves on port **8787**, and needs **persistent storage** for its code/licence
  JSON store. Target host: **`https://api.vidya.zuhairhussain.com`**.

You will create **two separate Fly.io apps** (one per service). This document is
only about hosting these two services. Wiring the hostnames into the app's release
build (GitHub Actions secrets/variables) is done elsewhere.

> **About the commands below.** Every Fly-specific detail here (region codes, CLI
> flags, config keys) was checked against Fly's official docs on 2026-09-24, with
> the doc links given in each section. Fly changes flags and defaults from time to
> time — if a command errors, run it with `--help` (e.g. `fly launch --help`) and
> check the linked page. Fly's CLI accepts both `fly` and `flyctl` as the command
> name; this guide uses `fly`.

> **Heads-up on the region code.** Fly's Mumbai region is **`bom`**. It has had
> capacity limits in the past (new machines occasionally rejected). If `bom` is
> ever unavailable when you launch, use **`sin`** (Singapore) instead — it is the
> next closest region to India. Both codes are confirmed in Fly's regions list
> (https://fly.io/docs/reference/regions/). You can always list current regions
> with `fly platform regions`.

---

## 0. Before you start

You will need:

- A **credit/debit card** (Fly requires one even for small apps; there is no
  free tier anymore — see §7).
- The **two service folders** in this repo: `cloud/relay/` and `cloud/licence/`.
  The relay already has a `Dockerfile`. If the licence folder has no `Dockerfile`,
  `fly launch` will offer to generate one (accept it) or you can copy the relay's
  Dockerfile as a starting point and change the binary name to `vidya-licence`
  and the port to `8787`.
- Your **domain registrar login** for `zuhairhussain.com` (to add DNS records in §6).

---

## 1. Install `flyctl` and sign in

`flyctl` is Fly's command-line tool. Install the one for your operating system.
Source: https://fly.io/docs/flyctl/install/

### macOS

If you have Homebrew:

```sh
brew install flyctl
```

If you do not have Homebrew, use the install script:

```sh
curl -L https://fly.io/install.sh | sh
```

> After the curl method, the script prints two lines to add to your shell startup
> file (e.g. `~/.zshrc`) so your terminal can find `fly`. Copy-paste those lines
> as instructed, then open a new terminal.

### Windows (PowerShell)

```powershell
pwsh -Command "iwr https://fly.io/install.ps1 -useb | iex"
```

If Windows says `pwsh` is not found, replace `pwsh` with `powershell`:

```powershell
powershell -Command "iwr https://fly.io/install.ps1 -useb | iex"
```

### Check it installed

```sh
fly version
```

### Create an account (first time only) / log in

```sh
fly auth signup      # create a new Fly account (opens your browser)
```

If you already have an account:

```sh
fly auth login       # log in (opens your browser)
```

Sources: https://fly.io/docs/flyctl/auth/ , https://fly.io/docs/flyctl/install/

---

## 2. Launch each app (do this twice — once per service)

`fly launch --no-deploy` creates the Fly app and a `fly.toml` config file **without
deploying yet**, so you can set the region, machine size, secrets, and (for the
licence app) a volume first.

Source for `fly launch`: https://fly.io/docs/flyctl/launch/ and
https://fly.io/docs/launch/ . The default machine when nothing is specified is
**`shared-cpu-1x` with 1 GB RAM** — the smallest shared-CPU size. We pin it
explicitly below with `--vm-size shared-cpu-1x`.

### 2a. Relay

```sh
cd cloud/relay
fly launch --no-deploy --region bom --vm-size shared-cpu-1x
```

When prompted:

- **App name:** choose something like `vidya-relay` (must be globally unique on
  Fly; if taken, add a suffix, e.g. `vidya-relay-zh`). Write the name down.
- It will detect the `Dockerfile` — accept it.
- If it asks to tweak settings in a browser, you can decline; we edit `fly.toml`
  by hand next.

Now open the generated `cloud/relay/fly.toml` in a text editor and make sure the
service block points at the relay's port **8788** and keeps one machine always
running. It should look like this (edit the `internal_port` and the three
auto-stop/min lines; leave everything else Fly generated):

```toml
[http_service]
  internal_port = 8788
  force_https = true
  auto_stop_machines = "off"
  auto_start_machines = false
  min_machines_running = 1

[[http_service.checks]]
  method = "GET"
  path = "/healthz"
  interval = "15s"
  timeout = "5s"
  grace_period = "10s"
```

> **Why these three lines?** `min_machines_running = 1` plus `auto_stop_machines = "off"`
> keeps exactly one machine on 24/7 so the relay is always reachable (WebSocket
> tunnels must stay up). Source:
> https://fly.io/docs/launch/autostop-autostart/ and
> https://fly.io/docs/reference/configuration/ (`[http_service]` section).
>
> WebSocket upgrades pass through Fly's HTTPS front end automatically — you do
> **not** need to configure anything special for `wss://`.

### 2b. Licence API

```sh
cd ../licence
fly launch --no-deploy --region bom --vm-size shared-cpu-1x
```

- **App name:** e.g. `vidya-licence`. Write it down.
- If there is no `Dockerfile`, let `fly launch` generate one, or add one that
  builds and runs the `vidya-licence` binary on port `8787`.

Edit `cloud/licence/fly.toml` so its service points at port **8787** and stays
always-on the same way:

```toml
[http_service]
  internal_port = 8787
  force_https = true
  auto_stop_machines = "off"
  auto_start_machines = false
  min_machines_running = 1
```

Do **not** deploy yet — the licence app needs a volume first (§3) and both apps
need secrets (§4).

---

## 3. Persistent storage for the LICENCE app only

The licence service stores its codes/licences in a JSON file on disk. On Fly, a
machine's own disk is wiped on every redeploy, so that file must live on a **Fly
volume** (persistent disk). The relay is stateless and needs **no** volume.

Source: https://fly.io/docs/volumes/volume-manage/ and
https://fly.io/docs/launch/volume-storage/

### 3a. Create a small volume (from `cloud/licence/`)

A 1 GB volume is plenty for a JSON store (1 GB is the default size). Create it in
the **same region** as the app (`bom`):

```sh
fly volumes create vidya_licence_data --region bom --size 1
```

> **Note on redundancy.** A single volume lives on one physical host. For a small
> licence store, one volume + regular snapshots (below) is the simplest reliable
> setup. (Fly recommends 2+ volumes for high availability; that also means running
> 2+ machines, which doubles compute cost — not needed here.)

### 3b. Mount the volume in `fly.toml`

Add a `[mounts]` section to `cloud/licence/fly.toml`. `source` is the volume name
you just created; `destination` is where it appears inside the machine:

```toml
[mounts]
  source = "vidya_licence_data"
  destination = "/data"
```

**DB path.** The licence service must be told to read/write its store under the
mounted path, i.e. at **`/data/store.json`** (and keep its keys under `/data/`
too, e.g. `/data/keys/`). Set this via whatever config the binary uses — commonly
an environment variable in `[env]` of `fly.toml`, for example:

```toml
[env]
  LICENCE_DATA_DIR = "/data"
```

> **Confirmed (P10).** The `vidya-licence` binary reads **`LICENCE_DATA_DIR`** and
> keeps its SQLite store at **`$LICENCE_DATA_DIR/vidya-licence.db`** (so with the
> mount above, `/data/vidya-licence.db`). It no longer uses a JSON `store.json`;
> the store is SQLite. `.dev-keys/` is dev-only and never used in production. The
> important rule stands: **the store must live under `/data` (the volume), never on
> the machine's ephemeral disk.** (You can override the exact file with
> `LICENCE_DB=/data/vidya-licence.db` if you prefer.)

### 3c. Back up the volume

Fly takes **daily block-level snapshots automatically** and keeps them for 5 days
by default (retention configurable 1–60 days). You can also snapshot on demand.
Source: https://fly.io/docs/volumes/snapshots/ and
https://fly.io/docs/flyctl/volumes-snapshots/

Find the volume's ID:

```sh
fly volumes list
```

Take an on-demand snapshot right now (replace `<volume-id>` with the `vol_...` ID):

```sh
fly volumes snapshots create <volume-id>
```

List a volume's snapshots:

```sh
fly volumes snapshots list <volume-id>
```

Restore a snapshot into a **new** volume (equal or larger size):

```sh
fly volumes create vidya_licence_restore --snapshot-id <snapshot-id> --size 1 --region bom
```

> These snapshot commands are confirmed in Fly's docs. If you also want a plain
> file copy off the volume, run the licence service's own export/backup routine
> (if it has one) or `fly ssh console` into the machine and copy `/data/store.json`
> out with `fly ssh sftp get /data/store.json ./store-backup.json`. Verify
> `fly ssh sftp` behavior with `fly ssh sftp --help` — I could not fully verify
> the exact `sftp get` syntax against a current docs page, so treat that one as
> "try it, and fall back to snapshots which are the documented path."

---

## 4. Set the secrets

Secrets are encrypted values Fly injects as environment variables at runtime. They
are **never** stored in `fly.toml` or in git. Setting a secret restarts the app's
machines. Source: https://fly.io/docs/apps/secrets/ and
https://fly.io/docs/flyctl/secrets-set/

### 4a. Generate `RELAY_SHARED_KEY` (one value, used on BOTH apps)

This is a long random string that the relay and the licence service both use to
verify relay secrets. Generate a strong random base64 value:

**macOS / Linux:**

```sh
openssl rand -base64 48
```

**Windows (PowerShell):**

```powershell
[Convert]::ToBase64String((1..48 | ForEach-Object {Get-Random -Max 256}))
```

Copy the output. You will paste the **same** value into both apps below.

### 4b. Set it on the RELAY app (run from `cloud/relay/`)

```sh
fly secrets set RELAY_SHARED_KEY="PASTE_THE_SAME_VALUE_HERE"
```

### 4c. Set it on the LICENCE app (run from `cloud/licence/`)

```sh
fly secrets set RELAY_SHARED_KEY="PASTE_THE_SAME_VALUE_HERE"
```

> It **must be byte-for-byte identical** on both apps, or the relay will reject
> the school server's tunnel.

### 4d. Set the production ed25519 PRIVATE key on the LICENCE app ONLY

The licence service signs licences with a production ed25519 **private** key. This
key must **never** be committed to git and lives only as a Fly secret on the
licence app. Set it from `cloud/licence/`:

```sh
fly secrets set LICENCE_SIGNING_KEY="PASTE_PRODUCTION_ED25519_PRIVATE_KEY_HERE"
```

> **Confirmed (P10).** The secret name is exactly **`LICENCE_SIGNING_KEY`**, and its
> value is a **base64 32-byte ed25519 seed** (not PEM). Generate the keypair with
> `cd cloud/licence && cargo run -- gen-prod-key` — it prints the private seed (set
> as this secret) and the matching public key (put into the app's release
> build-config as `LICENCE_PUBLIC_KEY`). Do **not** set this on the relay app.

### 4e. Set the remaining LICENCE app secrets

The licence app needs three more secrets (set from `cloud/licence/`):

```sh
# Encrypts activation codes at rest so the Account page can re-display them.
# Generate with:  cd cloud/licence && cargo run -- gen-enc-key
fly secrets set LICENCE_CODE_ENC_KEY="PASTE_BASE64_32_BYTES"

# The first admin account (created on first start). Use a strong password.
fly secrets set LICENCE_ADMIN_EMAIL="you@example.com"
fly secrets set LICENCE_ADMIN_PASSWORD="A_STRONG_PASSWORD"
```

Optional business text (else `[placeholder]` shows on the site): `COMPANY_NAME`,
`SUPPORT_EMAIL`, `SUPPORT_PHONE`, `UPI_ID`, `PRICE_TEXT`, `TERMS_URL`, plus a QR
image path `UPI_QR_PATH` and the downloads feed `RELEASES_JSON` (both point under
`/data`). See `cloud/licence/.env.example` for the full list.

Verify which secrets are set (this shows names and digests only, never values):

```sh
fly secrets list
```

---

## 5. Deploy and health-check

Deploy each app from its own folder. `fly deploy` builds the Docker image and
starts the machine. Source: https://fly.io/docs/launch/deploy/

### 5a. Deploy the relay

```sh
cd cloud/relay
fly deploy
```

### 5b. Deploy the licence API

```sh
cd ../licence
fly deploy
```

### 5c. Check status and health

For each app (run from its folder):

```sh
fly status          # shows the machine(s) and whether they're started/passing
fly checks list     # shows the health-check results
fly logs            # live logs — watch for the service reporting it's listening
```

Sources: https://fly.io/docs/flyctl/status/ ,
https://fly.io/docs/reference/health-checks/

### 5d. Hit the health endpoints

Before you add your custom domains, every Fly app gets a free
`https://<app-name>.fly.dev` address. Test the relay's health endpoint:

```sh
curl https://vidya-relay.fly.dev/healthz
```

(Replace `vidya-relay` with the actual relay app name.) You should get `ok`.

The licence app **does** expose `/healthz` (P10) — the same as the relay — so the
`[[http_service.checks]]` block in `cloud/licence/fly.toml` pointing at `/healthz`
is correct. Test it directly:

```sh
curl https://vidya-licence.fly.dev/healthz     # → ok
curl https://vidya-licence.fly.dev/            # → the website home page
```

`fly checks list` and `fly logs` are the real signal that it started cleanly.

---

## 6. Point your domains at the Fly apps (DNS + TLS)

You want:

- `api.vidya.zuhairhussain.com`   → the **licence** app
- `relay.vidya.zuhairhussain.com` → the **relay** app

Fly issues free TLS certificates via Let's Encrypt once DNS is pointed correctly.
The key command is **`fly certs add <hostname>`**, which prints the **exact DNS
records** you must create at your registrar — always follow that printed output,
not a guessed record. Source: https://fly.io/docs/networking/custom-domain/

### 6a. Make sure each app has a public IP (usually already done at launch)

```sh
fly ips list        # run from each app's folder
```

If an app has no public IPv4/IPv6, allocate a **free shared IPv4** plus an IPv6:

```sh
fly ips allocate-v4 --shared
fly ips allocate-v6
```

Source: https://fly.io/docs/flyctl/ips/ (a shared IPv4 and an IPv6 are free; a
**dedicated** IPv4 is billed monthly — you do not need one for these two apps).

### 6b. Add the certificate for each hostname

From the **licence** app folder:

```sh
fly certs add api.vidya.zuhairhussain.com
```

From the **relay** app folder:

```sh
fly certs add relay.vidya.zuhairhussain.com
```

Each command **prints the exact DNS records to create** — this is the important
part. Depending on your setup it will tell you to add either:

- an **A record** and an **AAAA record** pointing the subdomain at the app's IPs
  (the values are shown in the output / from `fly ips list`), **and/or**
- a **CNAME** record (common for subdomains) pointing the subdomain at
  `<app-name>.fly.dev`, **and/or**
- an **`_acme-challenge` CNAME** record used to prove you own the domain and to
  issue the certificate.

**Do exactly what the command prints.** Different apps get different values, so
copy them from your own `fly certs add` output.

### 6c. Create those records at your registrar

Log in to the registrar that holds `zuhairhussain.com`, open its DNS editor, and
create each record from the `fly certs add` output. Typical for a subdomain:

- **Type:** as printed (A / AAAA / CNAME / the `_acme-challenge` CNAME)
- **Name / Host:** the subdomain part, e.g. `api.vidya` or `relay.vidya`
  (some registrars want the full `api.vidya.zuhairhussain.com`, some want just
  `api.vidya` — follow the registrar's convention)
- **Value / Target:** exactly as printed by Fly
- **TTL:** default is fine

### 6d. Wait for the certificate to go green

DNS can take a few minutes to a couple of hours to propagate. Check status:

```sh
fly certs show api.vidya.zuhairhussain.com
fly certs check api.vidya.zuhairhussain.com
```

(and the same for `relay.vidya.zuhairhussain.com`.) Once Fly reports the
certificate as issued, test the real URLs:

```sh
curl https://api.vidya.zuhairhussain.com/          # licence API is up over TLS
curl https://relay.vidya.zuhairhussain.com/healthz # relay health → ok
```

The relay's `wss://relay.vidya.zuhairhussain.com` tunnel uses the **same**
hostname and certificate — `https` working means `wss` works.

---

## 7. What it will cost (read the live pricing page)

**Fly no longer has a free tier**, and prices change, so do **not** trust any
dollar figure from this document. Read the current numbers here before you commit:

- **Pricing page:** https://fly.io/pricing/
- **Resource pricing details:** https://fly.io/docs/about/pricing/

For these two services, your monthly bill is driven by exactly three things:

1. **Two always-on `shared-cpu-1x` machines** (one per app), billed by the second
   while running. Because we set `min_machines_running = 1` and
   `auto_stop_machines = "off"`, each machine runs 24/7 — this is the main cost.
   Smaller RAM = cheaper; `shared-cpu-1x` is the smallest shared-CPU size.
2. **One small volume** (the 1 GB licence volume), billed per GB of **provisioned**
   size per month (not per GB used). Snapshots have a small monthly free allowance;
   beyond that they're billed too.
3. **Outbound bandwidth**, billed per GB after a monthly free allowance. **Note:**
   bandwidth out of the India region is in Fly's **highest** price band, so if you
   serve heavy traffic from `bom` this line can matter. For a licence API and a
   relay carrying small sealed frames, bandwidth should be modest.

> I did not copy exact dollar amounts into this doc on purpose — the pricing page
> above is the source of truth. (There have also been pricing updates around
> late 2026; the live page reflects whatever is current.)

---

## 8. Troubleshooting

**A machine won't stay up / keeps restarting.**
- Run `fly logs` and read the last lines — most often it's a missing secret
  (e.g. `RELAY_SHARED_KEY` not set) or the service crashing on start. Fix and
  redeploy with `fly deploy`.
- Confirm the health check path/port match the service: relay uses `/healthz` on
  `8788`; the licence app listens on `8787`. If `internal_port` in `fly.toml`
  doesn't match the port the binary actually binds, the health check fails and
  Fly restarts the machine.
- Make sure `min_machines_running = 1` and `auto_stop_machines = "off"` are set,
  or the machine may scale to zero and look "down."
- Check status with `fly status` and `fly checks list`.

**The certificate stays "pending" / "awaiting configuration."**
- The DNS records from `fly certs add` aren't in place yet or are wrong. Re-run
  `fly certs show <hostname>` to see exactly which record Fly is still waiting on,
  and compare it character-for-character with what's at your registrar.
- DNS propagation can take up to a couple of hours. Re-check with
  `fly certs check <hostname>`.
- Make sure you didn't accidentally add the record for the wrong subdomain, and
  that the registrar didn't append the domain twice (a common gotcha:
  `api.vidya.zuhairhussain.com.zuhairhussain.com`).

**The volume is full (licence app).**
- Check usage: `fly ssh console` into the machine, then `df -h /data`.
- Expand the volume (volumes can be grown, not shrunk):
  `fly volumes list` to get the ID, then `fly volumes extend <volume-id> --size <new-GB>`.
  Verify the exact flag with `fly volumes extend --help`. Source:
  https://fly.io/docs/volumes/volume-manage/
- A JSON licence store is tiny; a "full" 1 GB volume usually means logs or
  something unexpected is writing to `/data` — check what's there via
  `fly ssh console` before just growing it.

**General.** `fly logs`, `fly status`, and `fly checks list` are your three main
diagnostic commands. `fly config validate` (run in an app folder) catches
`fly.toml` mistakes before a deploy.

---

## Could-not-fully-verify notes

- **`fly ssh sftp get` exact syntax** (§3c, for a plain file copy off the volume)
  — I could not confirm the precise command form against a current docs page.
  The **documented, verified** backup path is volume **snapshots** (§3c); use
  those as the primary method and treat `sftp` as a convenience to test locally.
- **`fly volumes extend` exact flag** (§8) — confirm with `--help`; the command
  exists but flag names occasionally change.
- **Exact prices** — deliberately not stated; read https://fly.io/pricing/ (§7).
- **Licence-service env var names** — now **confirmed** against the `vidya-licence`
  binary (P10): `LICENCE_DATA_DIR`, `LICENCE_SIGNING_KEY`, `LICENCE_CODE_ENC_KEY`,
  `RELAY_SHARED_KEY`, `LICENCE_ADMIN_EMAIL`, `LICENCE_ADMIN_PASSWORD` (+ the
  optional business text). Full list with descriptions: `cloud/licence/.env.example`.
