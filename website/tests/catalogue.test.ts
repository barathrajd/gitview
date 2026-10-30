import { test, expect } from 'bun:test';
import { readFileSync } from 'node:fs';
test('feature catalogue preserves anchored sections', () => {
  const page = readFileSync(new URL('../dist/features/index.html', import.meta.url), 'utf8');
  expect(page).not.toContain('data-release=');
  expect(page).toContain('Since 0.3.0');
  expect(page).toContain('Since 0.6.0');
  expect(page).toContain('Since 0.7.0');
  for (const id of ['cockpit', 'diff', 'history', 'sessions', 'control-channel']) {
    expect(page).toContain(`id="${id}"`);
    expect(page).toContain(`href="#${id}"`);
  }
});
test('release highlights link to existing catalogue sections', () => {
  const notes = readFileSync(new URL('../dist/changelog/index.html', import.meta.url), 'utf8');
  const catalogue = readFileSync(new URL('../dist/features/index.html', import.meta.url), 'utf8');
  const links = [...notes.matchAll(/href="\/features\/#([^"]+)"/g)];
  expect(links.length).toBeGreaterThanOrEqual(20);
  for (const [, id] of links) expect(catalogue.includes(`id="${id}"`)).toBe(true);
});
test('catalogue provides a searchable feature navigator', () => {
  const page = readFileSync(new URL('../dist/features/index.html', import.meta.url), 'utf8');
  expect(page.includes('id="feature-search"')).toBe(true);
  expect(page.includes('data-feature-link="sessions"')).toBe(true);
  expect(page.includes('data-feature="sessions"')).toBe(true);
  expect(page.includes('No matching features')).toBe(true);
});
