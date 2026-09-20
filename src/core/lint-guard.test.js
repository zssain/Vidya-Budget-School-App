import { ESLint } from 'eslint';
import { describe, expect, it } from 'vitest';

describe('React rendering lint guard', () => {
  it('rejects raw DOM writes and raw HTML JSX', async () => {
    const eslint = new ESLint({ cwd: process.cwd() });
    const rawDom = await eslint.lintText('const el = document.body; el.innerHTML = value;', {
      filePath: 'src/lint-fixture.js',
    });
    const rawHtml = await eslint.lintText(
      'const Example = () => <div dangerouslySetInnerHTML={{ __html: value }} />;',
      { filePath: 'src/lint-fixture.jsx' },
    );

    expect(rawDom[0].messages.some(({ ruleId }) => ruleId === 'no-restricted-properties')).toBe(true);
    expect(rawHtml[0].messages.some(({ ruleId }) => ruleId === 'react/no-danger')).toBe(true);
  });
});
