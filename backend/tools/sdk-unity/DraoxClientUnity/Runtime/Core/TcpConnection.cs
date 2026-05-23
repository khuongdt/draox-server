// TCP is not supported on WebGL — DraoxClient.Awake() falls back to WebSocket on that platform.
#if !UNITY_WEBGL || UNITY_EDITOR

using System;
using System.IO;
using System.Net.Sockets;
using System.Text;
using System.Threading;
using Cysharp.Threading.Tasks;
using UnityEngine;

namespace Draox.Client
{
    // TCP transport using newline-delimited JSON framing.
    // Each message is a single JSON object terminated by '\n'.
    internal class TcpConnection : IConnection
    {
        private TcpClient            _tcp;
        private StreamReader         _reader;
        private StreamWriter         _writer;
        private CancellationTokenSource _readCts;

        public event Action<string> MessageReceived;
        public event Action         Opened;
        public event Action<string> Closed;

        public bool IsConnected => _tcp?.Connected ?? false;

        public async UniTask ConnectAsync(DraoxConfig config, CancellationToken ct = default)
        {
            _tcp = new TcpClient();
            await _tcp.ConnectAsync(config.Host, config.Port).AsUniTask().AttachExternalCancellation(ct);

            var stream = _tcp.GetStream();
            _reader = new StreamReader(stream, Encoding.UTF8);
            _writer = new StreamWriter(stream, Encoding.UTF8) { AutoFlush = true };

            Opened?.Invoke();

            _readCts = new CancellationTokenSource();
            ReadLoopAsync(_readCts.Token).Forget();
        }

        public async UniTask DisconnectAsync()
        {
            _readCts?.Cancel();
            _tcp?.Close();
            await UniTask.CompletedTask;
        }

        public async UniTask SendTextAsync(string json, CancellationToken ct = default)
        {
            if (!IsConnected) throw new DraoxException("TCP is not connected");
            await _writer.WriteLineAsync(json).AsUniTask().AttachExternalCancellation(ct);
        }

        private async UniTaskVoid ReadLoopAsync(CancellationToken ct)
        {
            try
            {
                while (!ct.IsCancellationRequested)
                {
                    var line = await _reader.ReadLineAsync().AsUniTask().AttachExternalCancellation(ct);
                    if (line == null) break; // server closed the connection
                    if (!string.IsNullOrWhiteSpace(line))
                        MessageReceived?.Invoke(line);
                }
            }
            catch (OperationCanceledException) { }
            catch (Exception ex)
            {
                Debug.LogError($"[Draox] TCP read error: {ex.Message}");
            }
            finally
            {
                Closed?.Invoke("tcp_closed");
            }
        }
    }
}

#endif
