using System;
using System.Collections.Generic;
using System.Threading;
using Cysharp.Threading.Tasks;

namespace Draox.Client
{
    // Manages the primary connection plus any additional role-specific connections
    // (notification, control, streaming) sharing the same session.
    internal class SessionManager
    {
        private readonly DraoxConfig       _config;
        private readonly List<IConnection> _extras = new List<IConnection>();

        public string SessionId { get; set; }

        public SessionManager(DraoxConfig config) => _config = config;

        // Opens an extra connection for a given role and binds it to the current session.
        public async UniTask AddConnectionAsync(ConnectionRole role, CancellationToken ct = default)
        {
            if (string.IsNullOrEmpty(SessionId))
                throw new DraoxException("Authenticate before adding extra connections");

            IConnection conn;
#if !UNITY_WEBGL || UNITY_EDITOR
            conn = _config.Protocol == DraoxProtocol.Tcp
                ? (IConnection)new TcpConnection()
                : new WebSocketConnection();
#else
            conn = new WebSocketConnection();
#endif
            await conn.ConnectAsync(_config, ct);

            var bind = new BindMessage
            {
                session_id = SessionId,
                role       = role.ToString().ToLowerInvariant(),
            };
            await conn.SendTextAsync(Serializer.Serialize(bind), ct);

            _extras.Add(conn);
        }

        public async UniTask DisconnectAllAsync()
        {
            foreach (var conn in _extras)
            {
                try { await conn.DisconnectAsync(); }
                catch { /* best-effort */ }
            }
            _extras.Clear();
        }
    }
}
