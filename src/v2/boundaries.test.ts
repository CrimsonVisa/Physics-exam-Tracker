import { readFileSync, readdirSync } from 'node:fs';
import { extname, join } from 'node:path';
import { describe, expect, it } from 'vitest';

const V2_DIRECTORY = join(process.cwd(), 'src', 'v2');

const V1_DATA_API_MODULES = [
  'settingsApi',
  'examsApi',
  'backupApi',
  'plannerApi',
  'revisionApi',
  'pyqApi',
  'mockTestApi',
  'sessionsApi',
  'timerApi',
  'syllabusApi',
  'resourcesApi',
  'analyticsApi',
  'preparationApi',
  'csvImportApi',
];

describe('V2 frontend boundaries', () => {
  it('does not import V1 data API modules from V2 screens', () => {
    const sources = readdirSync(V2_DIRECTORY)
      .filter((name) => ['.ts', '.tsx'].includes(extname(name)))
      .filter((name) => !name.endsWith('.test.ts') && !name.endsWith('.test.tsx'))
      .map((name) => ({
        name,
        content: readFileSync(join(V2_DIRECTORY, name), 'utf8'),
      }));

    for (const moduleName of V1_DATA_API_MODULES) {
      for (const source of sources) {
        expect(source.content, `${source.name} must not import ${moduleName}`).not.toContain(
          `from '../lib/${moduleName}'`,
        );
      }
    }
  });
});
