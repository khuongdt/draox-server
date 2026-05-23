using System;
using Cysharp.Threading.Tasks;
using Draox.Client;
using TMPro;
using UnityEngine;
using UnityEngine.UI;

namespace Draox.Demo
{
    /// <summary>
    /// Panel: configure host/port/protocol and connect/disconnect.
    /// </summary>
    public class ConnectionPanel : MonoBehaviour
    {
        [Header("Input Fields")]
        [SerializeField] private TMP_InputField hostInput;
        [SerializeField] private TMP_InputField portInput;
        [SerializeField] private TMP_Dropdown   protocolDropdown;
        [SerializeField] private Toggle         tlsToggle;
        [SerializeField] private TMP_InputField timeoutInput;

        [Header("Buttons")]
        [SerializeField] private Button connectButton;
        [SerializeField] private Button disconnectButton;

        [Header("Reconnect")]
        [SerializeField] private Toggle         reconnectToggle;
        [SerializeField] private TMP_InputField maxAttemptsInput;
        [SerializeField] private TMP_InputField baseDelayInput;

        private DraoxClient _client;

        private void Awake()
        {
            // Set sensible defaults.
            if (hostInput      != null) hostInput.text      = "127.0.0.1";
            if (portInput      != null) portInput.text      = "9002";
            if (timeoutInput   != null) timeoutInput.text   = "10000";
            if (maxAttemptsInput != null) maxAttemptsInput.text = "5";
            if (baseDelayInput  != null) baseDelayInput.text   = "1";

            if (protocolDropdown != null)
            {
                protocolDropdown.ClearOptions();
                protocolDropdown.AddOptions(new System.Collections.Generic.List<string>
                    { "WebSocket", "TCP", "gRPC" });
            }
        }

        private void Start()
        {
            _client = DemoManager.Instance.Client;

            _client.OnStateChanged += state =>
            {
                var connected = state == ClientState.Connected;
                if (connectButton)    connectButton.interactable    = !connected;
                if (disconnectButton) disconnectButton.interactable  = connected;
            };

            connectButton?.onClick.AddListener(() => ConnectAsync().Forget());
            disconnectButton?.onClick.AddListener(() => DisconnectAsync().Forget());
        }

        private async UniTaskVoid ConnectAsync()
        {
            ApplyConfigToClient();

            DemoManager.Instance.Log($"Connecting to {hostInput?.text}:{portInput?.text} …");
            try
            {
                await _client.ConnectAsync();
            }
            catch (Exception ex)
            {
                DemoManager.Instance.Log($"Connect failed: {ex.Message}", LogLevel.Error);
            }
        }

        private async UniTaskVoid DisconnectAsync()
        {
            await _client.DisconnectAsync("user_request");
        }

        private void ApplyConfigToClient()
        {
            // DraoxClient exposes its config via [SerializeField] — we reach it
            // through the public Config property added for demo use.
            var cfg = _client.Config;

            if (!string.IsNullOrEmpty(hostInput?.text))
                cfg.Host = hostInput.text.Trim();

            if (int.TryParse(portInput?.text, out var port))
                cfg.Port = port;

            cfg.UseTls = tlsToggle != null && tlsToggle.isOn;

            if (int.TryParse(timeoutInput?.text, out var ms))
                cfg.TimeoutMs = ms;

            cfg.Protocol = (DraoxProtocol)(protocolDropdown?.value ?? 0);

            cfg.Reconnect.Enabled = reconnectToggle != null && reconnectToggle.isOn;
            if (int.TryParse(maxAttemptsInput?.text, out var maxAttempts))
                cfg.Reconnect.MaxAttempts = maxAttempts;
            if (float.TryParse(baseDelayInput?.text, out var baseDelay))
                cfg.Reconnect.BaseDelaySeconds = baseDelay;
        }
    }
}
