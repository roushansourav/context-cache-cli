import { refresh } from '../../../../index';
import { DEFAULT_TEXT_EXTENSIONS } from '../../../../constants/core/constants';

export function watchRepo(repoRoot: string): void {
  // eslint-disable-next-line @typescript-eslint/no-require-imports
  const { watch } = require('node:fs') as typeof import('node:fs');
  let timer: ReturnType<typeof setTimeout> | null = null;

  const scheduleRefresh = () => {
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => {
      try {
        const result = refresh(repoRoot);
        const { fileCount, changedCount } = result.payload;
        console.log(`[${new Date().toISOString()}] refreshed files=${fileCount} changed=${changedCount}`);
      } catch (err) {
        console.error('Refresh error:', err instanceof Error ? err.message : err);
      }
    }, 700);
  };

  refresh(repoRoot);
  console.log('Watching for changes. Ctrl+C to stop.');

  const watcher = watch(repoRoot, { recursive: true }, (_event, filename) => {
    if (!filename) return;
    const ext = '.' + String(filename).split('.').pop();
    if (!DEFAULT_TEXT_EXTENSIONS.includes(ext)) return;
    scheduleRefresh();
  });

  process.on('SIGINT', () => { watcher.close(); process.exit(0); });
}
