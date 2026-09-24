# Google OAuth Setup for Vidya (Google Drive backup)

This guide walks you — step by step — through creating the Google sign-in
credentials ("OAuth clients") that Vidya needs so it can use **Google Drive as an
offline backup / exchange fallback**.

You do **not** need to be a programmer to follow this. Everything happens in a
website called the **Google Cloud Console** (`https://console.cloud.google.com`).
You just click buttons and copy a few text values.

> **Heads-up on wording.** Google changes the look and the button labels of this
> website fairly often. This guide was checked against Google's current
> documentation (September 2026), but if a button is named slightly differently
> from what you see below, this guide tells you *what the button does* so you can
> find the right one anyway. When in doubt, look for the option that does the
> thing described, not the exact word.

---

## What you are building (the big picture)

Vidya needs **two** separate OAuth clients — think of each as a numbered ID card
that tells Google "this request is coming from the Vidya app":

| # | Client | For which version of Vidya | When to make it |
|---|--------|---------------------------|-----------------|
| A | **Desktop app** client | The Windows / macOS installer | Now |
| B | **Android** client | The Android phone/tablet app | **Later** — only after the app's package name and the release signing key are decided (see Part 8) |

Both live inside **one** Google Cloud project called **"Vidya"**.

You will finish with two long text strings called **client IDs**. They go into
Vidya's build configuration under these two names:

- `google_client_id_desktop`
- `google_client_id_android`

(How they get plugged in is covered in Part 9.)

### Key facts you'll enter (keep these handy)

| Field | Value |
|-------|-------|
| App name | **Vidya** |
| Publisher / owner | Zuhair Hussain |
| User support email | `mohammedzuhairhussain28@gmail.com` |
| Developer contact email | `mohammedzuhairhussain28@gmail.com` |
| Website | `https://zuhairhussain.com` |
| Privacy policy URL | `https://zuhairhussain.com/vidya/privacy` |
| Drive permission (scope) | `https://www.googleapis.com/auth/drive.file` |

> ⚠️ **The privacy policy page must really exist.** `https://zuhairhussain.com/vidya/privacy`
> is a placeholder. You can enter it now while the app is in *Testing*, but Google
> will **not** let you publish the app to the public until that page is actually
> live on the internet. Put up a real privacy-policy page before Part 7.

---

## Part 1 — Create the Google Cloud project "Vidya"

A "project" is just a folder that holds all of Vidya's Google settings.

1. Sign in to Google using the account you want to own this app
   (`mohammedzuhairhussain28@gmail.com` is a sensible choice).
2. Go to **https://console.cloud.google.com**.
3. At the very **top of the page**, next to the "Google Cloud" logo, there is a
   **project picker** — a drop-down that usually shows the name of whatever
   project is currently selected (or "Select a project"). Click it.
4. In the little window that opens, click **New project** (top-right of that window).
5. For **Project name**, type: `Vidya`.
6. Leave **Location / Organization** as "No organization" (this is normal for a
   personal Google account).
7. Click **Create**.
8. Wait a few seconds. A notification appears when it's ready. Then open the
   project picker again and select **Vidya** so that it is the *active* project.
   (Every step after this assumes "Vidya" is the selected project — always check
   the name shown at the top.)

> You do **not** need to add a credit card or billing account. Creating a project
> and setting up OAuth is free.

---

## Part 2 — Turn on the Google Drive API

Right now the project can't talk to Google Drive yet. You have to switch that on.

1. With **Vidya** selected at the top, open the left-hand menu (the ☰ "hamburger"
   icon, top-left). Go to **APIs & Services → Library**.
   - *(If you don't see "Library", the current menu may instead say
     "Enabled APIs & services" — from there click **+ Enable APIs and services**,
     which opens the same Library.)*
2. In the search box on the Library page, type: `Google Drive API`.
3. Click the result named **Google Drive API**.
4. Click the blue **Enable** button.
5. Wait for it to finish. When it says the API is enabled, you're done with this part.

---

## Part 3 — Set up the "OAuth consent screen" (the permission pop-up)

The **consent screen** is the little "Vidya wants to access your Google Drive"
window that people will see when they connect their Google account. You have to
describe your app here before Google will hand out any client IDs.

> **Where it lives (2026 UI).** Google recently reorganized this area. In the
> current console it's under **APIs & Services → OAuth consent screen**, which
> opens a section now called the **Google Auth Platform**. It is split into tabs
> named **Branding**, **Audience**, **Data access**, and **Clients**. If you don't
> see the tabs yet, there will be a **Get started** button that opens a short
> wizard covering the same fields — either path is fine.

### 3a. Start the wizard / open Branding

1. Left menu → **APIs & Services → OAuth consent screen**.
2. If you see a **Get started** button, click it. (If instead you see the tabbed
   layout, click the **Branding** tab.)

### 3b. App information

3. **App name:** type `Vidya`.
4. **User support email:** pick / type `mohammedzuhairhussain28@gmail.com`.
   (This is the address users can contact for help; it must be an email attached
   to your Google account, so choose it from the drop-down if offered.)

### 3c. Audience (who can use the app) — choose **External**

5. For **Audience** / **User type**, choose **External**.
   - **External** means "anyone with a Google account can be allowed to use it."
   - **Internal** only exists if you have a Google Workspace *organization* — you
     don't, so **External** is the right (and usually only) choice.
   - The current label may read **External** or describe it as *"Available to any
     test user with a Google Account"* — pick the one that is **not** limited to
     your own organization.

> Note: choosing External starts the app in **Testing** mode automatically. That's
> expected — Part 7 explains Testing vs. Published.

### 3d. Contact information

6. **Developer contact information / email:** enter `mohammedzuhairhussain28@gmail.com`.
   (Google uses this to notify you about your project — e.g. policy changes.)

### 3e. Finish and, if asked, agree to the policy

7. If the wizard shows Google's **User Data Policy** checkbox, tick it, then click
   **Create** / **Save** / **Continue** to finish the wizard.

### 3f. Add the privacy policy link (Branding tab)

8. Go to the **Branding** tab (if you're not already there).
9. Find **App domain → Privacy policy link** (wording may be "Privacy policy URL").
10. Enter: `https://zuhairhussain.com/vidya/privacy`
11. *(Optional but recommended)* fill in the **Application home page** with
    `https://zuhairhussain.com` and, if there is an **Authorized domains** box,
    add `zuhairhussain.com`.
12. Click **Save**.

> ⚠️ Reminder from earlier: this privacy URL can be saved now, but the page must be
> **live and reachable** before you publish the app in Part 7.

---

## Part 4 — Add the Drive permission (the `drive.file` scope)

A **scope** is the exact permission you're asking each user for. Vidya needs only
the **smallest** Drive permission that exists — access to *only the files Vidya
itself creates or that the user hands to Vidya* — never the user's whole Drive.

That permission is:

```
https://www.googleapis.com/auth/drive.file
```

This is the **minimal** scope for what Vidya does (create and manage its own backup
files). Do **not** add any broader Drive scope (like plain `.../auth/drive` or
`drive.readonly`) — those are "restricted" scopes that would drag you into a much
heavier Google security review.

**Steps:**

1. Left menu → **APIs & Services → OAuth consent screen**, then open the
   **Data access** tab.
   - *(In the older layout this same button was labeled **Scopes** — look for the
     place where you "Add or remove scopes.")*
2. Click **Add or remove scopes**.
3. A panel opens with a big list and a search box. In the filter/search box, type:
   `drive.file`
4. Tick the checkbox for the row whose **Scope** column is exactly
   `.../auth/drive.file` and whose description says it lets the app create/modify
   only files the user opens or shares with the app. Be careful **not** to tick
   the broader `.../auth/drive` row.
5. Click **Update** (bottom of the panel).
6. Back on the Data access tab, click **Save**.

> Why the smallest scope matters: `drive.file` is a **non-sensitive** scope. Apps
> that use only non-sensitive scopes avoid Google's lengthy *sensitive-scope*
> verification. (See the important nuance in Part 7.)

---

## Part 5 — Testing vs. Published, test users, and verification

This is the part people most often get confused by, so read it slowly.

### 5a. Two publishing states

Your app is always in one of two states, shown on the **Audience** tab under
**Publishing status**:

- **Testing** — the default. Only Google accounts you personally added to a
  **Test users** list can connect. Everyone else gets an *"access blocked / app not
  verified"* error. Also, in Testing, a connected account's sign-in **expires after
  7 days** and the user must reconnect. Testing allows **up to 100 test users**.
  This mode is perfect while you are the only one using the app, or during a small
  pilot.
- **In production (Published)** — anyone with a Google account can connect, and the
  7-day expiry goes away (connections last until revoked). This is what you want
  once you release Vidya to real school users.

### 5b. Add yourself as a test user (do this now)

While in Testing you must whitelist every account that will connect:

1. Left menu → **APIs & Services → OAuth consent screen → Audience** tab.
2. Find the **Test users** section and click **Add users**.
3. Type the Google email(s) that will test Vidya's backup — at minimum your own,
   e.g. `mohammedzuhairhussain28@gmail.com`. Add up to 100.
4. Click **Save**.

### 5c. When you're ready for real users — Publish

When the privacy page is live and you've tested successfully:

1. **Audience** tab → click **Publish app** (may read **Push to production**).
2. Confirm. The status changes to **In production**.

### 5d. Do you need Google "verification"? (read carefully)

Here is the nuance, stated precisely:

- Because Vidya uses **only** the **non-sensitive** `drive.file` scope, it does
  **not** need the heavy *sensitive-scope verification* (the multi-day security
  review that broader Drive scopes trigger). This is the main reason we chose
  `drive.file`.
- **However**, Google's current rules also describe a lighter **"brand
  verification"** step that can apply to *any* published app that accesses user
  data — this is mainly Google confirming your app name, logo, support email, and
  that your privacy-policy page is real. Whether Google actually prompts you for
  this, and how much it asks for, can vary.
- **Practical takeaway:** You can run entirely in **Testing** mode (with yourself
  and up to 100 whitelisted accounts) **without any verification at all**. If/when
  you **Publish** for the general public, be ready that Google *may* ask you to
  confirm your branding and a working privacy-policy page. It should **not** ask
  for the expensive third-party security assessment, precisely because Vidya sticks
  to `drive.file`.

> Bottom line: start in Testing (no verification needed). Publish only when the
> privacy page is live; expect at most a light branding check, not a full
> security audit.

---

## Part 6 — Create the **Desktop** OAuth client (client A)

This is the ID card for the **Windows / macOS** version of Vidya.

Vidya's desktop app signs in using the standard **"installed app" flow with PKCE
and a loopback redirect** — i.e. during sign-in it briefly listens on your own
computer at an address like `http://127.0.0.1:<random port>` and Google sends the
result there. **The correct client type for exactly this behaviour is
"Desktop app."** You do **not** create a "Web application" client and you do
**not** type in any redirect URLs — the Desktop app type handles loopback
automatically.

**Steps:**

1. Left menu → **APIs & Services → OAuth consent screen → Clients** tab.
   - *(Alternative path: **APIs & Services → Credentials**. Both reach the same
     place in the current console.)*
2. Click **Create client** (older wording: **Create credentials → OAuth client ID**).
3. For **Application type**, choose **Desktop app**.
   - The current label should say **Desktop app**. If your console words it
     differently, pick the type meant for *"an application installed on a computer"*
     — **not** Web application, Android, iOS, or TV/limited-input.
4. **Name:** type something you'll recognize, e.g. `Vidya Desktop`.
   (This name is only for your own reference in the console; users never see it.)
5. Click **Create**.
6. A window pops up showing **Your Client ID** (a long string ending in
   `.apps.googleusercontent.com`).
   - **Copy the Client ID** and paste it somewhere safe — you'll need it in Part 9.
   - You can always find it again later on the **Clients / Credentials** page.
   - Note: a Desktop-app client may also show a "client secret." For Vidya's
     PKCE-based flow the secret is not treated as truly secret, but there's no harm
     in saving it too. **The value you must keep is the Client ID.**

➡️ Record this as **`google_client_id_desktop`**.

---

## Part 7 — Create the **Android** OAuth client (client B) — DO THIS LAST

> 🛑 **Do not do this part yet if the two things below aren't settled.** The Android
> client is permanently tied to two values that are *not final* right now:
>
> 1. **The app's package name** — the owner is still deciding between
>    **`in.vidyabudget.app`** and **`com.zuhairhussain.vidya`**. Pick one and
>    commit to it *before* creating this client, because it's baked into the
>    published app and is painful to change afterward.
> 2. **The release keystore** — the signing key used for the Play Store release
>    **does not exist yet**. You need its **SHA-1 fingerprint**, which you can only
>    get after the keystore is created.
>
> **So: settle the package name, create the release keystore, then come back here.**
> The steps for creating the keystore and reading its SHA-1 are in
> **`docs/ANDROID-SIGNING.md`**.

An Android OAuth client is identified by the **pair** of (package name + SHA-1
fingerprint). Google requires this pair to be **unique** across all Google/Firebase
projects, which is another reason to finalize both first.

### 7a. Get the SHA-1 fingerprint

Follow **`docs/ANDROID-SIGNING.md`** to obtain the **SHA-1** of your **release**
keystore. For reference, the underlying command is the Java `keytool` (the doc
above wraps this):

```
keytool -list -v -keystore <path-to-your-release-keystore> -alias <your-key-alias>
```

In the output, find the **Certificate fingerprints** section and copy the **SHA-1**
line — it's 20 pairs of hex digits separated by colons
(e.g. `AB:CD:12:34:...`).

> Note: `~/.android/debug.keystore` (password `android`) is only the *debug* key
> used during development. For the client that ships to users you need the **release**
> keystore's SHA-1, not the debug one. (You may create a *second* Android client for
> the debug SHA-1 if you want sign-in to work in test builds too.)

### 7b. Create the Android client

1. Left menu → **APIs & Services → OAuth consent screen → Clients** tab
   (or **APIs & Services → Credentials**).
2. Click **Create client** (older wording: **Create credentials → OAuth client ID**).
3. For **Application type**, choose **Android**.
4. **Name:** e.g. `Vidya Android` (for your reference only).
5. **Package name:** enter the final decided package name — either
   `in.vidyabudget.app` **or** `com.zuhairhussain.vidya`. This must exactly match
   the `applicationId` the Android app actually ships with.
6. **SHA-1 certificate fingerprint:** paste the SHA-1 you copied in step 7a.
7. Click **Create**.
8. Copy the resulting **Client ID** (ends in `.apps.googleusercontent.com`) and
   save it.

➡️ Record this as **`google_client_id_android`**.

---

## Part 8 — Where the two client IDs go (build config → GitHub)

You now have two values:

| Build-config key | Value | Which client |
|------------------|-------|--------------|
| `google_client_id_desktop` | `…apps.googleusercontent.com` | Part 6 (Desktop app) |
| `google_client_id_android` | `…apps.googleusercontent.com` | Part 7 (Android) |

These feed Vidya's **build configuration**. At release time they are supplied to
the automated build (GitHub Actions) as **repository secrets / variables** with the
**exact same names**:

1. On GitHub, open the Vidya repository.
2. Go to **Settings → Secrets and variables → Actions**.
3. Add each value:
   - Client IDs are not truly secret, so either the **Variables** tab or the
     **Secrets** tab works; if unsure, **Secrets** is the safe default.
   - Click **New repository secret** (or **New variable**).
   - **Name:** `GOOGLE_CLIENT_ID_DESKTOP` — **Value:** the desktop client ID.
   - Repeat: **Name:** `GOOGLE_CLIENT_ID_ANDROID` — **Value:** the android client ID.

   *(Use the exact key names the build expects. This guide's config keys are
   `google_client_id_desktop` / `google_client_id_android`; GitHub environment
   names are conventionally UPPER_CASE. Match whatever the workflow file references.)*

4. Save. The next release build will read these in and bake them into the app.

You do **not** need to create the Android secret until the Android client actually
exists (Part 7). The desktop build only needs `google_client_id_desktop`.

---

## Quick checklist

- [ ] Project **Vidya** created and selected
- [ ] **Google Drive API** enabled
- [ ] OAuth consent screen: **External**, app name **Vidya**, support +
      developer email set
- [ ] **`drive.file`** scope added (and *only* that Drive scope)
- [ ] Privacy policy URL entered **and the page is actually live before publishing**
- [ ] Yourself added under **Test users** (while in Testing)
- [ ] **Desktop app** client created → saved as `google_client_id_desktop`
- [ ] Package name + release keystore decided → **Android** client created →
      saved as `google_client_id_android` *(last)*
- [ ] Both IDs added to GitHub → **Settings → Secrets and variables → Actions**

---

## Sources (verified September 2026)

- OAuth 2.0 for Mobile & Desktop Apps (Desktop app type, PKCE, loopback):
  https://developers.google.com/identity/protocols/oauth2/native-app
- Choose Google Drive API scopes (`drive.file`, non-sensitive):
  https://developers.google.com/workspace/drive/api/guides/api-specific-auth
- Configure the OAuth consent screen (Branding/Audience/Data access/Clients):
  https://developers.google.com/workspace/guides/configure-oauth-consent
- Sensitive-scope verification (when verification is / isn't required):
  https://developers.google.com/identity/protocols/oauth2/production-readiness/sensitive-scope-verification
- Testing mode limits (100 test users, 7-day token expiry) — Google Cloud help
  on publishing status / managing app audience:
  https://support.google.com/cloud/answer/15549945
- Android OAuth client (package name + SHA-1 via keytool):
  https://developers.google.com/fit/android/get-api-key
