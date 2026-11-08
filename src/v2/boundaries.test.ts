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

function collectSources(directory: string): Array<{ name: string; content: string }> {
  const sources: Array<{ name: string; content: string }> = [];
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const fullPath = join(directory, entry.name);
    if (entry.isDirectory()) {
      sources.push(...collectSources(fullPath));
      continue;
    }
    if (!['.ts', '.tsx'].includes(extname(entry.name))) {
      continue;
    }
    if (entry.name.endsWith('.test.ts') || entry.name.endsWith('.test.tsx')) {
      continue;
    }
    sources.push({
      name: fullPath.slice(V2_DIRECTORY.length + 1),
      content: readFileSync(fullPath, 'utf8'),
    });
  }
  return sources;
}

function importsV1Module(content: string, moduleName: string): boolean {
  return [`from '../lib/${moduleName}'`, `from '../../lib/${moduleName}'`].some((pattern) =>
    content.includes(pattern),
  );
}

describe('V2 frontend boundaries', () => {
  it('does not import V1 data API modules from V2 screens', () => {
    const sources = collectSources(V2_DIRECTORY);

    for (const moduleName of V1_DATA_API_MODULES) {
      for (const source of sources) {
        expect(
          importsV1Module(source.content, moduleName),
          `${source.name} must not import ${moduleName}`,
        ).toBe(false);
      }
    }
  });

  it('detects V1 imports at both one- and two-level relative depths', () => {
    expect(importsV1Module("import { x } from '../lib/settingsApi';", 'settingsApi')).toBe(true);
    expect(importsV1Module("import { x } from '../../lib/settingsApi';", 'settingsApi')).toBe(true);
    expect(importsV1Module("import { x } from './settingsApi';", 'settingsApi')).toBe(false);
  });
});
