Vidya static site — assets
===========================

Brand logos (copied from design/assets/, do not recolour or edit — see
docs/01-MOCK-SPEC §4 brand rules):

  vidya-horizontal-on-light.svg   Logo lockup for the light top nav / light pages.
  vidya-horizontal-on-dark.svg    Logo lockup for navy backgrounds (spare).
  dwaar-mark-on-dark.svg          The door mark, used on the navy hero.

Fonts:

  fonts/   Bundled Geist + Newsreader .woff2 files. See fonts/README.txt.
           NO web-font CDN is used.

UPI QR:

  upi-qr.png   [OWNER] PLACEHOLDER. This is a coloured placeholder image, NOT a real
               QR code. The owner MUST replace it with their own UPI QR image (240x240
               or larger, PNG) before publishing. how-to-buy.html points its <img> at
               this exact path (assets/upi-qr.png).

releases.json (in the site root, not here) is overwritten by the release workflow.
