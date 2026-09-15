// Desktop entry point. In later prompts this registers every desktop view.
// For P1.1 it only renders a placeholder so the Tauri window shows something.
import { getVersion } from '@tauri-apps/api/app';

// getVersion() only resolves inside a Tauri window; in a plain browser
// (e.g. `npm run dev` without Tauri) it rejects, so we fall back to "dev".
async function appVersion() {
  try {
    return await getVersion();
  } catch {
    return 'dev';
  }
}

async function main() {
  const app = document.getElementById('app');
  const version = await appVersion();

  const title = document.createElement('h1');
  title.textContent = 'Vidya';

  const ver = document.createElement('p');
  ver.textContent = `Version ${version}`;

  app.replaceChildren(title, ver);
}

main();
