const required = [
  '../README.md',
  '../docs/architecture.md',
  '../docs/testing.md',
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

console.log('ui smoke check ok');
