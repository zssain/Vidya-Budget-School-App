# Deploying the Vidya static site (`site/`)

The marketing / download website lives in **`site/`**. It is plain HTML + CSS with a
small amount of vanilla JS on the Downloads page (to read `releases.json`). There is
**no build step** and **no company server** — the whole site is static files served by
free static hosting, so the running cost to the developer is **₹0 / month**
(docs/00-SYSTEM-CONTEXT §0 zero-monthly-cost rule, §10.5).

Host it on GitHub Pages **or** Cloudflare Pages (both are covered below — you only need
one; you may run both, one as a backup). Point `vidya.zuhairhussain.com` at whichever you
publish.

> Nothing here deploys anything automatically. These are the manual steps.

---

## 0. What is in `site/`

| File | Purpose |
|---|---|
| `index.html` | Home — what Vidya is, platforms, honest size text, `[PRICE]`. |
| `how-to-buy.html` | Pay by UPI → send UTR + school name + machine code by prefilled email → get key. |
| `downloads.html` | Reads `releases.json` (vanilla JS) and renders the download table. |
| `privacy.html`, `terms.html`, `refund.html` | Draft legal pages (marked "Draft — owner review"). |
| `style.css` | Shared stylesheet; brand tokens (docs/01-MOCK-SPEC §2), Welcome-screen look. |
| `releases.json` | **Sample** release list. The release workflow overwrites it (see §5). |
| `assets/*.svg` | Brand logos + door mark (copied from `design/assets/`). |
| `assets/upi-qr.png` | **[OWNER] placeholder** — replace with your real UPI QR before publishing. |
| `assets/fonts/` | Where the bundled Geist + Newsreader `.woff2` files go (see its README). |
| `assets/README-assets.txt`, `assets/fonts/README.txt` | Notes for the owner. |

**Before you publish, do these once:**

1. **[OWNER]** Replace `site/assets/upi-qr.png` with your own UPI QR image.
2. **[OWNER]** Replace every `[PRICE]` placeholder in `index.html`, `how-to-buy.html`
   and `terms.html` with your actual price.
3. **[OWNER]** Fill the `[OWNER: number of days …]` refund window in `refund.html`.
4. Copy the six bundled font `.woff2` files into `site/assets/fonts/`
   (see `site/assets/fonts/README.txt`). Without them the site falls back to
   Georgia / system-ui — it still works, but the serif headings won't match the app.
5. Have the three draft legal pages reviewed.

---

## 1. Test locally first

The Downloads page uses `fetch('releases.json')`, which browsers block on `file://`.
Serve the folder over HTTP to test it:

```bash
cd site
python3 -m http.server 8080
# then open http://localhost:8080/
```

Click through Home → How to buy → Downloads (the table should render from the sample
`releases.json`) → Privacy / Terms / Refund. Check the prefilled email button on
How to buy opens your mail client with the subject and body filled in.

---

## 2. Option A — GitHub Pages

The repo is `zssain/Vidya-Budget-School-App`. Because the site is a **subfolder**
(`site/`), publish it with a small GitHub Actions workflow (Pages can't point at an
arbitrary subfolder from the UI alone).

1. **Repo → Settings → Pages → Build and deployment → Source: GitHub Actions.**
2. Add `.github/workflows/pages.yml` (the parent engineer wires this; shape shown here):

   ```yaml
   name: Deploy site to GitHub Pages
   on:
     push:
       branches: [main]
       paths: ['site/**']
     workflow_dispatch:
   permissions:
     contents: read
     pages: write
     id-token: write
   concurrency:
     group: pages
     cancel-in-progress: true
   jobs:
     deploy:
       runs-on: ubuntu-latest
       environment:
         name: github-pages
         url: ${{ steps.deploy.outputs.page_url }}
       steps:
         - uses: actions/checkout@v4
         - uses: actions/configure-pages@v5
         - uses: actions/upload-pages-artifact@v3
           with:
             path: site           # publish the site/ folder as-is
         - id: deploy
           uses: actions/deploy-pages@v4
   ```

3. Push to `main`. The workflow publishes `site/` to
   `https://zssain.github.io/Vidya-Budget-School-App/`.
4. **Custom domain:** Settings → Pages → Custom domain → enter
   `vidya.zuhairhussain.com` → Save. GitHub writes a `CNAME` file into the published
   site and provisions a free HTTPS certificate once DNS is set (§4). Leave
   "Enforce HTTPS" ticked once it is available.

> Note: paths in the site are all **relative** (`style.css`, `assets/…`, `downloads.html`),
> so it works both at the custom domain root and at the `…github.io/<repo>/` project path.

---

## 3. Option B — Cloudflare Pages

1. Log in to the **Cloudflare dashboard → Workers & Pages → Create → Pages →
   Connect to Git** and pick `zssain/Vidya-Budget-School-App`.
2. Build settings:
   - **Framework preset:** None.
   - **Build command:** *(leave empty — no build step).*
   - **Build output directory:** `site`
   - **Root directory:** *(leave as repo root; output dir `site` selects the folder).*
3. **Save and Deploy.** Cloudflare serves the files from `site/` at a
   `*.pages.dev` URL.
4. **Custom domain:** the Pages project → **Custom domains → Set up a custom domain →**
   `vidya.zuhairhussain.com`. If `zuhairhussain.com` is already on Cloudflare DNS,
   Cloudflare adds the record and the certificate for you.

---

## 4. DNS for `vidya.zuhairhussain.com` — **[OWNER] to confirm the domain/registrar**

`vidya.zuhairhussain.com` is a **subdomain** of `zuhairhussain.com`. Add **one**
`CNAME` record for the `vidya` host at wherever `zuhairhussain.com`'s DNS is managed
(**[OWNER]** to confirm the registrar / DNS provider):

### If you host on GitHub Pages

| Type | Name (host) | Value | TTL |
|---|---|---|---|
| CNAME | `vidya` | `zssain.github.io.` | Auto / 3600 |

(GitHub serves the custom domain from that CNAME; the free certificate is issued
automatically once the CNAME resolves. Apex/`A` records are only needed if you ever
point the bare `zuhairhussain.com` at Pages — not required for this subdomain.)

### If you host on Cloudflare Pages

| Type | Name (host) | Value | Proxy | TTL |
|---|---|---|---|---|
| CNAME | `vidya` | `<your-project>.pages.dev.` | Proxied (orange cloud) | Auto |

If `zuhairhussain.com` already uses Cloudflare nameservers, adding the custom domain in
the Pages UI creates this record for you — you usually don't add it by hand.

**[OWNER] to confirm:**
- Where `zuhairhussain.com` DNS is managed (registrar or Cloudflare).
- That `vidya` is not already in use.
- Which host you're publishing (GitHub Pages vs Cloudflare Pages) so only the matching
  CNAME above is added.

After the CNAME propagates (minutes to a few hours), the site is live at
`https://vidya.zuhairhussain.com` with free HTTPS. **No company server is involved.**

---

## 5. `releases.json` — how the download list stays current

`site/releases.json` shipped in the repo is a **sample** so the Downloads page renders
locally and on first deploy. The **release workflow overwrites it** with the real list.

- Schema: identical to `cloud/licence/releases.example.json` (see §6 below).
- The parent engineer wires `.github/workflows/release.yml` (P12 Step 9) to write
  `site/releases.json` from the published (non-draft) GitHub Release: version,
  `released_on`, `notes_url`, and per-platform `os / label / file / url /
  download_bytes / installed_bytes / sha256 / requirements / signed_by`, then commit or
  re-deploy the site. **Draft releases are never listed.**
- Because the file is committed in `site/`, a Pages/Cloudflare deploy that publishes
  `site/` automatically picks up the new list — no extra step.
- Sizes are **decimal bytes** (docs/00-SYSTEM-CONTEXT §2: 40 MB = 40,000,000). The
  page formats them as `x.x MB` (bytes ÷ 1,000,000).

---

## 6. `releases.json` schema (consumed by `downloads.html`)

```jsonc
{
  "generated_at": "2026-09-24T00:00:00Z",   // ISO-8601, informational
  "releases": [
    {
      "version": "1.0.0",
      "released_on": "2026-09-24",            // YYYY-MM-DD
      "draft": false,                         // draft releases are NEVER shown
      "notes_url": "https://…/releases/tag/v1.0.0",
      "platforms": [
        {
          "os": "windows",                    // windows | macos | android
          "label": "Windows 10/11 (64-bit)",
          "file": "Vidya_1.0.0_x64-setup.exe",
          "url":  "https://…/download/…exe",  // optional; "Link pending" if absent
          "download_bytes": 6100000,          // decimal bytes
          "installed_bytes": 12000000,        // decimal bytes
          "sha256": "…64 hex chars…",         // from SHA256SUMS.txt
          "requirements": "Windows 10 or 11, 64-bit",
          "signed_by": null                   // null = unsigned → shown as
                                              // "Unsigned (verify the SHA-256)"
        }
      ]
    }
  ]
}
```

The page shows the first non-draft release, sorted Windows → macOS → Android, with
version, date, file, size (download + installed), SHA-256 and signing status.

---

## 7. Updating the site later

Edit files under `site/` and push to `main`:
- **GitHub Pages:** the `pages.yml` workflow re-publishes automatically.
- **Cloudflare Pages:** the Git integration re-deploys automatically on push.

No rebuild, no server restart, no cost.
