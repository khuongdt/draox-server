using System;
using System.Text;
using System.Threading;
using Cysharp.Threading.Tasks;
using NativeWebSocket;
using UnityEngine;

namespace Draox.Client
{
    // WebSocket transport using NativeWebSocket (supports Android, iOS, WebGL, Standalone).
    internal class WebSocketConnection : IConnection
    {
        private WebSocket    _ws;
        private DraoxConfig  _config;

        public event Action<string> MessageReceived;
        public event Action         Opened;
        public event Action<string> Closed;

        public bool IsConnected => _ws?.State == WebSocketState.Open;

        public async UniTask ConnectAsync(DraoxConfig config, CancellationToken ct = default)
        {
            _config = config;
            var scheme = config.UseTls ? "wss" : "ws";
            var url    = $"{scheme}://{config.Host}:{config.Port}";

            _ws = WebSocketFactory.CreateInstance(url);

            var openTcs  = new UniTaskCompletionSource();
            var errorMsg = (string)null;

            _ws.OnOpen    += () =>
            {
                openTcs.TrySetResult();
                Opened?.Invoke();
            };
            _ws.OnMessage += bytes =>
            {
                var json = Encoding.UTF8.GetString(bytes);
                MessageReceived?.Invoke(json);
            };
            _ws.OnError   += msg =>
            {
                errorMsg = msg;
                openTcs.TrySetException(new DraoxException($"WebSocket error: {msg}"));
                Debug.LogError($"[Draox] WebSocket error: {msg}");
            };
            _ws.OnClose   += code => Closed?.Invoke(((int)code).ToString());

            // Connect() initiates the handshake; we await OnOpen via the TCS.
            _ws.Connect();

            using (ct.Register(() => openTcs.TrySetCanceled()))
                await openTcs.Task;
        }

        public async UniTask DisconnectAsync()
        {
            if (_ws != null && IsConnected)
                await _ws.Close();
        }

        public async UniTask SendTextAsync(string json, CancellationToken ct = default)
        {
            if (!IsConnected) throw new DraoxException("WebSocket is not connected");
            await _ws.SendText(json);
        }

        // Must be called from MonoBehaviour.Update() on WebGL to pump the message queue.
        public void DispatchMessageQueue() => _ws?.DispatchMessageQueue();
    }
}
