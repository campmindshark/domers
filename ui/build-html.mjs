import { mkdir, writeFile } from 'node:fs/promises';

const page = ({ title, simulator = false }) => `<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>${title}</title>
    <link rel="stylesheet" href="/assets/styles.css" />
  </head>
  <body${simulator ? ' data-page="simulator"' : ''}>
    <div id="root"></div>
    <script type="module" src="/assets/main.js"></script>
  </body>
</html>
`;

await mkdir(new URL('./dist', import.meta.url), { recursive: true });
await writeFile(new URL('./dist/index.html', import.meta.url), page({ title: 'MindShark Dome Control Panel' }));
await writeFile(new URL('./dist/simulator.html', import.meta.url), page({ title: 'MindShark Dome Simulator', simulator: true }));
