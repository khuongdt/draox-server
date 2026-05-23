import dayjs from 'dayjs';
import relativeTime from 'dayjs/plugin/relativeTime';
import numeral from 'numeral';

dayjs.extend(relativeTime);

/** Convert raw byte count to human-readable string (B / KB / MB / GB / TB). */
export function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return `${(bytes / Math.pow(1024, i)).toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
}

/** Format a number with thousands separator (e.g. 1,234,567). */
export function formatNumber(n: number): string {
  return numeral(n).format('0,0');
}

/** Convert seconds to a human-readable duration string (e.g. 2h 14m). */
export function formatDuration(seconds: number): string {
  if (seconds < 60) return `${seconds}s`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ${seconds % 60}s`;
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  return `${h}h ${m}m`;
}

/** Format an ISO date string to local display format (YYYY-MM-DD HH:mm:ss). */
export function formatDate(dateStr: string): string {
  return dayjs(dateStr).format('YYYY-MM-DD HH:mm:ss');
}

/** Format an ISO date string as a relative time label (e.g. "3 minutes ago"). */
export function formatRelative(dateStr: string): string {
  return dayjs(dateStr).fromNow();
}
