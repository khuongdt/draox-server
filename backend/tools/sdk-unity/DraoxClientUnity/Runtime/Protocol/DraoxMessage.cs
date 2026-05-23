using System;
using UnityEngine;

namespace Draox.Client
{
    public enum DraoxProtocol { WebSocket, Tcp, Grpc }
    public enum ConnectionRole { Primary, Notification, Control, Streaming }
    public enum ClientState { Disconnected, Connecting, Connected, Reconnecting }

    [Serializable]
    public class DraoxConfig
    {
        public string Host = "localhost";
        public int Port = 9002;
        public DraoxProtocol Protocol = DraoxProtocol.WebSocket;
        public bool UseTls = false;
        public int TimeoutMs = 10_000;
        public int HeartbeatIntervalSeconds = 30;
        public ReconnectConfig Reconnect = new ReconnectConfig();
    }

    [Serializable]
    public class ReconnectConfig
    {
        public bool Enabled = true;
        public int MaxAttempts = 5;
        public float BaseDelaySeconds = 1f;
        public float MaxDelaySeconds = 30f;
    }

    // ── Public SDK types ─────────────────────────────────────────────────────

    public class DraoxRequest
    {
        public string Id      { get; set; }
        public string Action  { get; set; }
        public object Payload { get; set; }
    }

    public class DraoxResponse
    {
        public string Id      { get; set; }
        public bool   Success { get; set; }
        public string RawData { get; set; }
        public string Error   { get; set; }

        public T Data<T>() => Serializer.Deserialize<T>(RawData);
    }

    public class DraoxEvent
    {
        public string Category  { get; set; }
        public string Name      { get; set; }
        public string RawData   { get; set; }
        public string Timestamp { get; set; }

        public T Data<T>() => Serializer.Deserialize<T>(RawData);
    }

    public class DraoxException : Exception
    {
        public DraoxException(string message) : base(message) { }
    }

    public class DraoxAuthException : DraoxException
    {
        public DraoxAuthException(string message) : base(message) { }
    }

    public class DraoxTimeoutException : DraoxException
    {
        public DraoxTimeoutException(string requestId)
            : base($"Request '{requestId}' timed out") { }
    }

    // ── Internal wire protocol types ─────────────────────────────────────────
    // These mirror the JSON envelope format used by Draox Server.

    [Serializable]
    internal class WireRequest
    {
        public string id;
        public string type    = "request";
        public string action;
        public object payload;
    }

    [Serializable]
    internal class BindMessage
    {
        public string type       = "bind";
        public string session_id;
        public string role;
    }

    [Serializable]
    internal class PingMessage
    {
        public string type = "ping";
        public long   ts;
    }

    // Used internally after auth to extract session_id from data field.
    [Serializable]
    internal class AuthResponseData
    {
        [Newtonsoft.Json.JsonProperty("session_id")]
        public string SessionId;
    }
}
