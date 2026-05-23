export interface ITransport {
  readonly isConnected: boolean;
  onMessage: ((data: string) => void) | null;
  onClose:   ((reason: string) => void) | null;

  connect(host: string, port: number, useTls: boolean): Promise<void>;
  disconnect(): void;
  send(json: string): Promise<void>;
}
