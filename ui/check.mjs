import { readFile } from 'node:fs/promises';

const required = [
  '../README.md',
  '../docs/architecture.md',
  '../docs/configuration.md',
  '../docs/fixture-capture.md',
  '../docs/hardware-mapping.md',
  '../docs/hardware-readiness.md',
  '../docs/images/.gitkeep',
  '../docs/intentional-deviations.md',
  '../docs/porting-inventory.md',
  '../docs/testing.md',
  '../docs/ui-expectations.md',
  './index.html',
  './main.mjs',
];

for (const path of required) {
  const full = new URL(path, import.meta.url);
  try {
    await import('node:fs/promises').then(fs => fs.access(full));
  } catch {
    console.error(`Missing required UI-adjacent document: ${path}`);
    process.exit(1);
  }
}

const html = await readFile(new URL('./index.html', import.meta.url), 'utf8');
for (const marker of [
  'data-domers-app',
  'id="start-engine"',
  'id="stop-engine"',
  'name="domeActiveVis"',
  'id="flash-speed"',
  'id="dome-simulator"',
  'data-frame-source="websocket"',
]) {
  if (!html.includes(marker)) {
    console.error(`Missing required UI marker: ${marker}`);
    process.exit(1);
  }
}

console.log('ui smoke check ok');
