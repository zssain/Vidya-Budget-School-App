Bundled fonts for the Vidya static site
========================================

The site NEVER loads fonts from a CDN (no Google Fonts). style.css @font-face rules
reference the files below, which must be copied here from the app's own bundled faces
(the exact same Geist and Newsreader used in the app, docs/00-SYSTEM-CONTEXT §13 and
docs/01-MOCK-SPEC §3). Until they are copied the site falls back to Georgia / system-ui
so it still renders.

Files style.css expects (place them in this folder):

  Geist-Regular.woff2         Geist 400
  Geist-Medium.woff2          Geist 500
  Geist-SemiBold.woff2        Geist 600
  Geist-Bold.woff2            Geist 700
  Newsreader-opsz.woff2       Newsreader variable, upright, optical-size axis (opsz 6..72)
  Newsreader-opsz-italic.woff2  Newsreader variable, italic, optical-size axis

Where to get them (same sources the app uses):

  - Geist: @fontsource/geist (or Geist's official woff2 files). Latin subset only.
  - Newsreader: the VARIABLE package with the opsz axis
    (@fontsource-variable/newsreader — the opsz + opsz-italic .woff2 files).
    A static 400 file will NOT match the big serif headings; use the opsz variable file.

The family names in style.css ('Geist', 'Newsreader') deliberately match the app so the
site's look is identical to the Welcome screen. Latin subset is enough for this English
marketing site.

[OWNER / build] Copy these six .woff2 files into this folder before publishing (or wire
the release/site build to copy them from node_modules). No network fetch is used.
