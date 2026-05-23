using System;
using Cysharp.Threading.Tasks;
using Draox.Client;
using Draox.Client.Plugins;
using TMPro;
using UnityEngine;
using UnityEngine.UI;

namespace Draox.Demo
{
    /// <summary>
    /// Root controller for the Draox Demo scene.
    /// Holds the DraoxClient instance and exposes plugin helpers
    /// that individual panels use.
    /// </summary>
    public class DemoManager : MonoBehaviour
    {
        public static DemoManager Instance { get; private set; }

        // ── Inspector wiring ─────────────────────────────────────────────────

        [Header("Client")]
        [SerializeField] private DraoxClient draoxClient;

        [Header("Status Bar")]
        [SerializeField] private TextMeshProUGUI statusLabel;
        [SerializeField] private Image           statusIndicator;

        [Header("Tab Panels")]
        [SerializeField] private GameObject connectionPanelGO;
        [SerializeField] private GameObject authPanelGO;
        [SerializeField] private GameObject requestPanelGO;
        [SerializeField] private GameObject clansPanelGO;
        [SerializeField] private GameObject messagingPanelGO;
        [SerializeField] private GameObject presencePanelGO;

        // ── Public accessors ─────────────────────────────────────────────────

        public DraoxClient Client => draoxClient;

        public ClansPlugin    Clans     { get; private set; }
        public MessagingPlugin Messaging { get; private set; }
        public PresencePlugin  Presence  { get; private set; }

        // ── Colors ───────────────────────────────────────────────────────────

        private static readonly Color ColorConnected    = new Color(0.33f, 0.86f, 0.33f);
        private static readonly Color ColorConnecting   = new Color(1.00f, 0.80f, 0.20f);
        private static readonly Color ColorReconnecting = new Color(1.00f, 0.50f, 0.10f);
        private static readonly Color ColorDisconnected = new Color(0.86f, 0.33f, 0.33f);

        // ── Unity lifecycle ──────────────────────────────────────────────────

        private void Awake()
        {
            if (Instance != null) { Destroy(gameObject); return; }
            Instance = this;

            Clans     = new ClansPlugin(draoxClient);
            Messaging = new MessagingPlugin(draoxClient);
            Presence  = new PresencePlugin(draoxClient);
        }

        private void Start()
        {
            draoxClient.OnStateChanged  += OnStateChanged;
            draoxClient.OnConnected     += OnConnected;
            draoxClient.OnDisconnected  += OnDisconnected;
            draoxClient.OnAuthenticated += OnAuthenticated;
            draoxClient.OnError         += OnError;

            UpdateStatusBar(ClientState.Disconnected);
            ShowPanel(connectionPanelGO);
        }

        private void OnDestroy()
        {
            Clans?.UnregisterListeners();
            Messaging?.UnregisterListeners();
            Presence?.UnregisterListeners();
        }

        // ── Tab navigation ───────────────────────────────────────────────────

        public void ShowConnection()  => ShowPanel(connectionPanelGO);
        public void ShowAuth()        => ShowPanel(authPanelGO);
        public void ShowRequest()     => ShowPanel(requestPanelGO);
        public void ShowClans()       => ShowPanel(clansPanelGO);
        public void ShowMessaging()   => ShowPanel(messagingPanelGO);
        public void ShowPresence()    => ShowPanel(presencePanelGO);

        private void ShowPanel(GameObject target)
        {
            connectionPanelGO?.SetActive(connectionPanelGO == target);
            authPanelGO?.SetActive(authPanelGO == target);
            requestPanelGO?.SetActive(requestPanelGO == target);
            clansPanelGO?.SetActive(clansPanelGO == target);
            messagingPanelGO?.SetActive(messagingPanelGO == target);
            presencePanelGO?.SetActive(presencePanelGO == target);
        }

        // ── DraoxClient event handlers ───────────────────────────────────────

        private void OnStateChanged(ClientState state)
        {
            UpdateStatusBar(state);
            Log($"State → {state}");
        }

        private void OnConnected()
        {
            Log("Connected", LogLevel.Success);
        }

        private void OnDisconnected(string reason)
        {
            Log($"Disconnected: {reason}", LogLevel.Warning);
        }

        private void OnAuthenticated()
        {
            Log($"Authenticated  session={Client.SessionId}", LogLevel.Success);

            // Register plugin listeners once authenticated.
            Clans.RegisterListeners();
            Messaging.RegisterListeners();
            Presence.RegisterListeners();
        }

        private void OnError(string error)
        {
            Log($"Error: {error}", LogLevel.Error);
        }

        // ── Helpers ──────────────────────────────────────────────────────────

        public void Log(string msg, LogLevel level = LogLevel.Info)
        {
            EventLog.Instance?.Append(msg, level);
        }

        private void UpdateStatusBar(ClientState state)
        {
            if (statusLabel != null)
                statusLabel.text = state.ToString().ToUpper();

            if (statusIndicator != null)
                statusIndicator.color = state switch
                {
                    ClientState.Connected    => ColorConnected,
                    ClientState.Connecting   => ColorConnecting,
                    ClientState.Reconnecting => ColorReconnecting,
                    _                        => ColorDisconnected,
                };
        }
    }
}
