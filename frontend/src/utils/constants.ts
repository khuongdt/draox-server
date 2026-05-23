/** Primary brand and UI color palette for Draox dark theme. */
export const COLORS = {
  primary: '#e05d10',
  primaryLight: '#ff8c42',
  bgDark: '#1a1a2e',
  bgCard: '#16213e',
  bgSection: '#0f3460',
  text: '#e0e0e0',
  textMuted: '#a0a0b0',
  accent: '#53c28b',
  border: '#2a2a4a',
  warning: '#f5a623',
  error: '#d32f2f',
} as const;

/** Color per network protocol for connection type badges and charts. */
export const PROTOCOL_COLORS: Record<string, string> = {
  tcp: '#90caf9',
  udp: '#a5d6a7',
  websocket: '#ef9a9a',
  http: '#ff8a65',
};

/** Color per audit severity level. */
export const SEVERITY_COLORS: Record<string, string> = {
  critical: '#d32f2f',
  high: '#e05d10',
  medium: '#f5a623',
  low: '#53c28b',
};

/** Color per server event category for the event stream view. */
export const EVENT_CATEGORY_COLORS: Record<string, string> = {
  connection: '#1890ff',
  session: '#13c2c2',
  guard: '#f5222d',
  plugin: '#722ed1',
  server: '#fa8c16',
  custom: '#8c8c8c',
};

/** IP reputation risk zones — ordered by ascending risk (max score inclusive). */
export const REPUTATION_ZONES = [
  { max: 30, color: '#53c28b', label: 'Low Risk' },
  { max: 60, color: '#f5a623', label: 'Medium Risk' },
  { max: 80, color: '#e05d10', label: 'High Risk' },
  { max: 100, color: '#d32f2f', label: 'Critical' },
];
