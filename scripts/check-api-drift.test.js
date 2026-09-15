import { describe, expect, it } from 'vitest';
import { camelToSnake, parseApiCommands, parseJsCommands, parseRustCommands } from './check-api-drift.mjs';

describe('parseRustCommands', () => {
  it('strips module paths and ignores non-command tokens', () => {
    const src = `
      .invoke_handler(tauri::generate_handler![
        commands::auth::sign_in,
        commands::fees::collect_fee,
        current_user,
      ])
    `;
    expect(parseRustCommands(src).sort()).toEqual(['collect_fee', 'current_user', 'sign_in']);
  });

  it('returns [] when there is no generate_handler', () => {
    expect(parseRustCommands('pub fn run() {}')).toEqual([]);
  });
});

describe('camelToSnake', () => {
  it('converts camelCase to snake_case', () => {
    expect(camelToSnake('collectFee')).toBe('collect_fee');
    expect(camelToSnake('getReportCard')).toBe('get_report_card');
    expect(camelToSnake('signIn')).toBe('sign_in');
  });
});

describe('parseJsCommands', () => {
  it('finds exported functions and consts and snake_cases them', () => {
    const src = `
      export async function collectFee(input) {}
      export function signIn(input) {}
      export const currentUser = async () => {};
      function notExported() {}
    `;
    expect(parseJsCommands(src).sort()).toEqual(['collect_fee', 'current_user', 'sign_in']);
  });
});

describe('parseApiCommands', () => {
  it('reads snake_case first-column cells with optional D/M markers', () => {
    const src = [
      '| Command | Permission |',
      '|---|---|',
      '| `app_status` | no session |',
      '| `get_device_id` **D** | no session |',
      '| `discover_server` **M** | no session |',
      '',
      '| Event | Payload |',
      '| `session-expired` | {} |',
      '',
      '| Method and path | Purpose |',
      '| `POST /auth/sign-in` | login |',
    ].join('\n');
    expect(parseApiCommands(src).sort()).toEqual(['app_status', 'discover_server', 'get_device_id']);
  });
});
