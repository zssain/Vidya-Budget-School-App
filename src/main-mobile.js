// Mobile (Android) entry point. In later prompts this registers the
// client-only views. Not wired into any build yet (see P8.1).
import { getVersion } from '@tauri-apps/api/app';

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
  title.textContent = 'Vidya mobile';

  const ver = document.createElement('p');
  ver.textContent = `Version ${version}`;

  app.replaceChildren(title, ver);
}

main();
