/**
 * Draox Messaging CLI Demo
 *
 * Usage:
 *   npm start                              # channel = "general"
 *   npm start -- --channel team-alpha      # specify channel
 *   HOST=my.server PORT=9002 npm start         # custom server
 *   DRAOX_USER=admin DRAOX_PASS=secret npm start  # custom credentials
 *
 * Commands while running:
 *   /history         - reload last 20 messages
 *   /delete <id>     - delete a message by ID
 *   /edit <id> <txt> - edit a message
 *   /react <id> <e>  - add emoji reaction
 *   /quit            - exit
 *   <any text>       - send message to current channel
 */

import * as readline from 'readline';
import { DraoxClient, MessagingPlugin } from '../../draox-client/src/index';
import type { MessageDto } from '../../draox-client/src/plugins/MessagingPlugin';

// ── Config from env ───────────────────────────────────────────────────────────

const HOST      = process.env['HOST']       ?? 'localhost';
const PORT      = Number(process.env['PORT']      ?? 9002);
const ADMIN_PORT = Number(process.env['ADMIN_PORT'] ?? 9100);
const USERNAME  = process.env['DRAOX_USER'] ?? process.env['DRAOX_USERNAME'] ?? 'admin';
const PASSWORD  = process.env['DRAOX_PASS'] ?? process.env['DRAOX_PASSWORD'] ?? 'draox@Admin#2024';

const channelArgIdx = process.argv.indexOf('--channel');
const CHANNEL = channelArgIdx >= 0 ? (process.argv[channelArgIdx + 1] ?? 'general') : 'general';

// ── Colours ───────────────────────────────────────────────────────────────────

const C = {
  reset:   '\x1b[0m',
  bold:    '\x1b[1m',
  dim:     '\x1b[2m',
  green:   '\x1b[32m',
  blue:    '\x1b[34m',
  cyan:    '\x1b[36m',
  yellow:  '\x1b[33m',
  red:     '\x1b[31m',
  magenta: '\x1b[35m',
  gray:    '\x1b[90m',
};

function colorize(s: string, color: string): string { return `${color}${s}${C.reset}`; }

function printMsg(sender: string, text: string, time: string, own = false): void {
  const senderColor = own ? C.green : C.blue;
  const timeStr     = colorize(time, C.gray);
  console.log(`${colorize(sender, senderColor + C.bold)} ${timeStr}`);
  console.log(`  ${text}`);
}

function printSystem(text: string): void {
  console.log(colorize(`  ⟩ ${text}`, C.gray));
}

function printError(text: string): void {
  console.log(colorize(`  ✗ ${text}`, C.red));
}

function formatTime(iso?: string): string {
  if (!iso) return new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  return new Date(iso).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
}

// ── Main ──────────────────────────────────────────────────────────────────────

async function main(): Promise<void> {
  console.log(colorize('\n  Draox Messaging Demo (TypeScript)\n', C.cyan + C.bold));
  console.log(colorize(`  Server:  ${HOST}:${PORT}`, C.dim));
  console.log(colorize(`  User:    ${USERNAME}`, C.dim));
  console.log(colorize(`  Channel: #${CHANNEL}\n`, C.dim));

  // Connect & auth
  const client = new DraoxClient({ host: HOST, port: PORT, adminPort: ADMIN_PORT });

  client.on('disconnected', (reason) => {
    printSystem(`Disconnected: ${reason}`);
  });
  client.on('stateChanged', (s) => {
    if (s === 'reconnecting') printSystem('Reconnecting…');
    if (s === 'connected')    printSystem('Reconnected.');
  });

  process.stdout.write(colorize('  Connecting… ', C.yellow));
  await client.connect();
  console.log(colorize('OK', C.green));

  process.stdout.write(colorize('  Logging in… ', C.yellow));
  await client.login(USERNAME, PASSWORD);
  console.log(colorize(`OK  (session: ${client.sessionId?.substring(0, 8)}…)`, C.green));

  // Messaging plugin
  const messaging = new MessagingPlugin(client);

  messaging.onMessage = (evt) => {
    const msg = evt.message;
    if (msg.sender_id === USERNAME) return; // already printed on send
    readline.clearLine(process.stdout, 0);
    readline.cursorTo(process.stdout, 0);
    printMsg(msg.sender_id, msg.text, formatTime(msg.sent_at));
    rl.prompt(true);
  };

  messaging.onMessageDeleted = (evt) => {
    readline.clearLine(process.stdout, 0);
    readline.cursorTo(process.stdout, 0);
    printSystem(`Message ${evt.message_id.substring(0, 8)}… deleted in #${evt.channel_id}`);
    rl.prompt(true);
  };

  messaging.onTyping = (evt) => {
    if (evt.is_typing && evt.user_id !== USERNAME) {
      readline.clearLine(process.stdout, 0);
      readline.cursorTo(process.stdout, 0);
      printSystem(`${evt.username} is typing…`);
      rl.prompt(true);
    }
  };

  messaging.registerListeners();

  // Load history
  console.log(colorize(`\n  #${CHANNEL}`, C.cyan + C.bold));
  console.log(colorize('  ' + '─'.repeat(50), C.gray));

  try {
    const history = await messaging.getHistory(CHANNEL, 20);
    const msgs = [...(history.messages ?? [])].reverse();
    if (msgs.length > 0) {
      for (const m of msgs)
        printMsg(m.sender_id, m.text, formatTime(m.sent_at), m.sender_id === USERNAME);
    } else {
      printSystem('No messages yet. Be the first to say something!');
    }
  } catch (err) {
    printError(`Could not load history: ${String(err)}`);
  }

  console.log(colorize('  ' + '─'.repeat(50), C.gray));
  console.log(colorize('  Type a message or /help for commands.\n', C.dim));

  // CLI input loop
  const rl = readline.createInterface({
    input:  process.stdin,
    output: process.stdout,
    prompt: colorize(`  [${USERNAME}] `, C.magenta),
  });

  rl.prompt();

  rl.on('line', async (line) => {
    const input = line.trim();
    if (!input) { rl.prompt(); return; }

    // Commands
    if (input === '/help') {
      console.log([
        '',
        '  Commands:',
        '    /history            - reload last 20 messages',
        '    /delete <id>        - delete a message by ID',
        '    /edit <id> <text>   - edit a message',
        '    /react <id> <emoji> - add emoji reaction',
        '    /quit               - exit',
        '',
      ].join('\n'));
      rl.prompt(); return;
    }

    if (input === '/history') {
      await reloadHistory(messaging, CHANNEL, USERNAME);
      rl.prompt(); return;
    }

    if (input === '/quit') {
      rl.close(); return;
    }

    const [cmd, arg1, ...rest] = input.split(' ');

    if (cmd === '/delete' && arg1) {
      try {
        await messaging.deleteMessage(arg1);
        printSystem(`Deleted ${arg1.substring(0, 8)}…`);
      } catch (e) { printError(String(e)); }
      rl.prompt(); return;
    }

    if (cmd === '/edit' && arg1 && rest.length > 0) {
      try {
        const updated = await messaging.editMessage(arg1, rest.join(' '));
        printSystem(`Edited → ${updated.text}`);
      } catch (e) { printError(String(e)); }
      rl.prompt(); return;
    }

    if (cmd === '/react' && arg1 && rest.length > 0) {
      try {
        await messaging.react(arg1, rest[0]!);
        printSystem(`Reacted ${rest[0]} to ${arg1.substring(0, 8)}…`);
      } catch (e) { printError(String(e)); }
      rl.prompt(); return;
    }

    // Plain message
    try {
      // Send typing indicator (fire-and-forget)
      void messaging.sendTyping(CHANNEL);

      const resp = await messaging.sendMessage(CHANNEL, input);
      const m: MessageDto = resp.message;
      printMsg(m.sender_id, m.text, formatTime(m.sent_at), true);
    } catch (e) {
      printError(`Send failed: ${String(e)}`);
    }

    rl.prompt();
  });

  rl.on('close', async () => {
    printSystem('Goodbye!');
    messaging.unregisterListeners();
    await client.disconnect();
    process.exit(0);
  });

  process.on('SIGINT', () => rl.close());
}

async function reloadHistory(
  messaging: MessagingPlugin,
  channel: string,
  myUserId: string,
): Promise<void> {
  try {
    const history = await messaging.getHistory(channel, 20);
    const msgs = [...(history.messages ?? [])].reverse();
    console.log(colorize('  ' + '─'.repeat(50), C.gray));
    for (const m of msgs)
      printMsg(m.sender_id, m.text, formatTime(m.sent_at), m.sender_id === myUserId);
    console.log(colorize('  ' + '─'.repeat(50), C.gray));
  } catch (e) {
    printError(`History failed: ${String(e)}`);
  }
}

main().catch((err) => {
  console.error(colorize(`\n  Fatal: ${String(err)}`, C.red));
  process.exit(1);
});
