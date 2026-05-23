// Editor-only script — builds the full Draox Demo scene via Unity APIs.
// Menu: Draox > Build Demo Scene
#if UNITY_EDITOR
using System.Linq;
using Draox.Client;
using Draox.Demo;
using UnityEditor;
using UnityEditor.Events;
using UnityEditor.SceneManagement;
using UnityEngine;
using UnityEngine.EventSystems;
using UnityEngine.UI;
using TMPro;

namespace Draox.Demo.Editor
{
    public static class DemoSceneBuilder
    {
        // ─── Colors ──────────────────────────────────────────────────────────

        private static readonly Color BgDark        = new Color(0.12f, 0.12f, 0.14f);
        private static readonly Color BgPanel       = new Color(0.16f, 0.16f, 0.18f);
        private static readonly Color AccentBlue    = new Color(0.20f, 0.50f, 0.85f);
        private static readonly Color AccentGreen   = new Color(0.20f, 0.72f, 0.42f);
        private static readonly Color TextLight      = new Color(0.90f, 0.90f, 0.92f);
        private static readonly Color TextMuted      = new Color(0.55f, 0.55f, 0.60f);

        // ─── Entry point ─────────────────────────────────────────────────────

        [MenuItem("Draox/Build Demo Scene")]
        public static void BuildScene()
        {
            if (!EditorSceneManager.SaveCurrentModifiedScenesIfUserWantsTo()) return;

            var scene = EditorSceneManager.OpenScene("Assets/Scenes/DemoScene.unity");

            // Clear any pre-existing objects.
            foreach (var go in scene.GetRootGameObjects())
                Object.DestroyImmediate(go);

            // ── Standard scene setup ──────────────────────────────────────────
            var camera = CreateCamera();
            var es     = CreateEventSystem();

            // ── DemoRoot ──────────────────────────────────────────────────────
            var demoRoot   = new GameObject("DemoRoot");
            var draoxClient = demoRoot.AddComponent<DraoxClient>();
            var manager     = demoRoot.AddComponent<DemoManager>();

            // ── Canvas ────────────────────────────────────────────────────────
            var canvas = CreateCanvas();

            // ── Status Bar (top strip) ────────────────────────────────────────
            var (statusBar, statusDot, statusLabel) = CreateStatusBar(canvas);

            // ── Tab Bar ───────────────────────────────────────────────────────
            var tabBar = CreateTabBar(canvas);

            // ── Panel Container ───────────────────────────────────────────────
            var panelParent = CreatePanelParent(canvas);

            var connPanelGO = CreatePanel(panelParent, "ConnectionPanel");
            var authPanelGO = CreatePanel(panelParent, "AuthPanel");
            var reqPanelGO  = CreatePanel(panelParent, "RequestPanel");
            var clanPanelGO = CreatePanel(panelParent, "ClansPanel");
            var msgPanelGO  = CreatePanel(panelParent, "MessagingPanel");
            var prsPanelGO  = CreatePanel(panelParent, "PresencePanel");

            // Hide all except first.
            authPanelGO.SetActive(false);
            reqPanelGO.SetActive(false);
            clanPanelGO.SetActive(false);
            msgPanelGO.SetActive(false);
            prsPanelGO.SetActive(false);

            // ── Panel scripts ─────────────────────────────────────────────────
            connPanelGO.AddComponent<ConnectionPanel>();
            authPanelGO.AddComponent<AuthPanel>();
            reqPanelGO.AddComponent<RequestPanel>();
            clanPanelGO.AddComponent<ClansPanel>();
            msgPanelGO.AddComponent<MessagingPanel>();
            prsPanelGO.AddComponent<PresencePanel>();

            // ── Populate panels with UI ───────────────────────────────────────
            PopulateConnectionPanel(connPanelGO, connPanelGO.GetComponent<ConnectionPanel>());
            PopulateAuthPanel(authPanelGO, authPanelGO.GetComponent<AuthPanel>());
            PopulateRequestPanel(reqPanelGO, reqPanelGO.GetComponent<RequestPanel>());
            PopulateClansPanel(clanPanelGO, clanPanelGO.GetComponent<ClansPanel>());
            PopulateMessagingPanel(msgPanelGO, msgPanelGO.GetComponent<MessagingPanel>());
            PopulatePresencePanel(prsPanelGO, prsPanelGO.GetComponent<PresencePanel>());

            // ── Event Log (right column) ──────────────────────────────────────
            var (logGO, scrollRect, logText) = CreateEventLogPanel(canvas);
            var eventLog = logGO.AddComponent<EventLog>();

            // ── Wire EventLog ─────────────────────────────────────────────────
            var elSo = new SerializedObject(eventLog);
            elSo.FindProperty("scrollRect").objectReferenceValue = scrollRect;
            elSo.FindProperty("logText").objectReferenceValue    = logText;
            elSo.ApplyModifiedProperties();

            // ── Wire DemoManager ──────────────────────────────────────────────
            var mSo = new SerializedObject(manager);
            mSo.FindProperty("draoxClient").objectReferenceValue         = draoxClient;
            mSo.FindProperty("statusLabel").objectReferenceValue         = statusLabel;
            mSo.FindProperty("statusIndicator").objectReferenceValue     = statusDot;
            mSo.FindProperty("connectionPanelGO").objectReferenceValue   = connPanelGO;
            mSo.FindProperty("authPanelGO").objectReferenceValue         = authPanelGO;
            mSo.FindProperty("requestPanelGO").objectReferenceValue      = reqPanelGO;
            mSo.FindProperty("clansPanelGO").objectReferenceValue        = clanPanelGO;
            mSo.FindProperty("messagingPanelGO").objectReferenceValue    = msgPanelGO;
            mSo.FindProperty("presencePanelGO").objectReferenceValue     = prsPanelGO;
            mSo.ApplyModifiedProperties();

            // ── Wire Tab Bar buttons ──────────────────────────────────────────
            var tabs = tabBar.GetComponentsInChildren<Button>();
            string[] tabNames = { "Connection", "Auth", "Request", "Clans", "Messaging", "Presence" };
            string[] methods  = { "ShowConnection", "ShowAuth", "ShowRequest", "ShowClans", "ShowMessaging", "ShowPresence" };
            for (int i = 0; i < tabs.Length && i < methods.Length; i++)
            {
                var method = typeof(DemoManager).GetMethod(methods[i]);
                if (method != null)
                    UnityEventTools.AddVoidPersistentListener(
                        tabs[i].onClick,
                        System.Delegate.CreateDelegate(typeof(UnityEngine.Events.UnityAction), manager, method)
                            as UnityEngine.Events.UnityAction);

                // Set button label.
                var label = tabs[i].GetComponentInChildren<TextMeshProUGUI>();
                if (label != null) label.text = tabNames[i];
            }

            EditorSceneManager.SaveScene(scene);
            Debug.Log("[DraoxDemo] Scene built successfully. Press Play to test.");
        }

        // ─── Helpers ─────────────────────────────────────────────────────────

        private static GameObject CreateCamera()
        {
            var go = new GameObject("Main Camera");
            go.tag = "MainCamera";
            var cam = go.AddComponent<Camera>();
            cam.clearFlags       = CameraClearFlags.SolidColor;
            cam.backgroundColor  = BgDark;
            cam.orthographic     = false;
            go.AddComponent<AudioListener>();
            go.transform.position = new Vector3(0, 1, -10);
            return go;
        }

        private static GameObject CreateEventSystem()
        {
            var go = new GameObject("EventSystem");
            go.AddComponent<EventSystem>();
            go.AddComponent<StandaloneInputModule>();
            return go;
        }

        private static GameObject CreateCanvas()
        {
            var go = new GameObject("Canvas");
            var canvas = go.AddComponent<Canvas>();
            canvas.renderMode = RenderMode.ScreenSpaceOverlay;
            canvas.sortingOrder = 0;

            var scaler = go.AddComponent<CanvasScaler>();
            scaler.uiScaleMode            = CanvasScaler.ScaleMode.ScaleWithScreenSize;
            scaler.referenceResolution    = new Vector2(1920, 1080);
            scaler.matchWidthOrHeight     = 0.5f;

            go.AddComponent<GraphicRaycaster>();
            return go;
        }

        private static (GameObject bar, Image dot, TextMeshProUGUI label) CreateStatusBar(GameObject canvas)
        {
            var bar = CreateUIElement("StatusBar", canvas.transform);
            SetAnchors(bar, new Vector2(0, 1), new Vector2(1, 1), new Vector2(0, -40), new Vector2(0, 0));
            AddImage(bar, new Color(0.10f, 0.10f, 0.12f));

            var dotGO = CreateUIElement("StatusDot", bar.transform);
            SetAnchors(dotGO, new Vector2(0, 0), new Vector2(0, 1), new Vector2(8, 0), new Vector2(32, 0));
            var dot = AddImage(dotGO, AccentGreen);

            var lblGO = CreateUIElement("StatusLabel", bar.transform);
            SetAnchors(lblGO, new Vector2(0, 0), new Vector2(1, 1), new Vector2(48, 0), new Vector2(-16, 0));
            var lbl = AddTMPText(lblGO, "DISCONNECTED", 14, TextAlignmentOptions.MidlineLeft);

            return (bar, dot, lbl);
        }

        private static GameObject CreateTabBar(GameObject canvas)
        {
            var bar = CreateUIElement("TabBar", canvas.transform);
            SetAnchors(bar, new Vector2(0, 1), new Vector2(1, 1), new Vector2(0, -80), new Vector2(0, -40));
            AddImage(bar, new Color(0.14f, 0.14f, 0.16f));

            var hlg = bar.AddComponent<HorizontalLayoutGroup>();
            hlg.childControlWidth  = true;
            hlg.childControlHeight = true;
            hlg.childForceExpandWidth = true;
            hlg.childForceExpandHeight = true;
            hlg.spacing = 2;
            hlg.padding = new RectOffset(4, 4, 4, 4);

            string[] names = { "Connection", "Auth", "Request", "Clans", "Messaging", "Presence" };
            foreach (var name in names)
            {
                var btnGO = CreateUIElement($"Btn{name}", bar.transform);
                var img   = AddImage(btnGO, new Color(0.22f, 0.22f, 0.26f));
                var btn   = btnGO.AddComponent<Button>();
                var cs    = btn.colors;
                cs.normalColor      = new Color(0.22f, 0.22f, 0.26f);
                cs.highlightedColor = AccentBlue;
                cs.pressedColor     = new Color(0.15f, 0.35f, 0.65f);
                btn.colors = cs;
                btn.targetGraphic = img;

                var lblGO = CreateUIElement("Text", btnGO.transform);
                SetAnchors(lblGO, Vector2.zero, Vector2.one, Vector2.zero, Vector2.zero);
                AddTMPText(lblGO, name, 13, TextAlignmentOptions.Center);
            }

            return bar;
        }

        private static GameObject CreatePanelParent(GameObject canvas)
        {
            // Left 70% below tab bar, above bottom.
            var go = CreateUIElement("PanelParent", canvas.transform);
            SetAnchors(go, new Vector2(0, 0), new Vector2(0.70f, 1), new Vector2(0, 0), new Vector2(0, -80));
            AddImage(go, BgPanel);
            return go;
        }

        private static GameObject CreatePanel(GameObject parent, string name)
        {
            var go = CreateUIElement(name, parent.transform);
            SetAnchors(go, Vector2.zero, Vector2.one, Vector2.zero, Vector2.zero);
            return go;
        }

        private static (GameObject go, ScrollRect sr, TextMeshProUGUI text) CreateEventLogPanel(GameObject canvas)
        {
            var logRoot = CreateUIElement("EventLogPanel", canvas.transform);
            SetAnchors(logRoot, new Vector2(0.70f, 0), new Vector2(1, 1), new Vector2(4, 0), new Vector2(0, -80));
            AddImage(logRoot, new Color(0.10f, 0.10f, 0.12f));

            // Header
            var header = CreateUIElement("LogHeader", logRoot.transform);
            SetAnchors(header, new Vector2(0, 1), new Vector2(1, 1), new Vector2(0, -28), new Vector2(0, 0));
            AddImage(header, new Color(0.14f, 0.14f, 0.16f));
            var hText = CreateUIElement("HText", header.transform);
            SetAnchors(hText, Vector2.zero, Vector2.one, new Vector2(8, 0), new Vector2(0, 0));
            AddTMPText(hText, "Event Log", 13, TextAlignmentOptions.MidlineLeft);

            // Clear button
            var clearGO = CreateUIElement("BtnClear", header.transform);
            SetAnchors(clearGO, new Vector2(1, 0), new Vector2(1, 1), new Vector2(-60, 4), new Vector2(-4, -4));
            var clearImg = AddImage(clearGO, new Color(0.30f, 0.30f, 0.34f));
            var clearBtn = clearGO.AddComponent<Button>();
            clearBtn.targetGraphic = clearImg;
            var clearLbl = CreateUIElement("Text", clearGO.transform);
            SetAnchors(clearLbl, Vector2.zero, Vector2.one, Vector2.zero, Vector2.zero);
            AddTMPText(clearLbl, "Clear", 11, TextAlignmentOptions.Center);

            // Scroll view
            var scrollGO = CreateUIElement("ScrollView", logRoot.transform);
            SetAnchors(scrollGO, Vector2.zero, Vector2.one, new Vector2(0, 0), new Vector2(0, -28));
            var sr = scrollGO.AddComponent<ScrollRect>();
            sr.horizontal = false;
            sr.vertical   = true;

            // Viewport
            var viewport = CreateUIElement("Viewport", scrollGO.transform);
            SetAnchors(viewport, Vector2.zero, Vector2.one, Vector2.zero, Vector2.zero);
            viewport.AddComponent<RectMask2D>();
            sr.viewport = viewport.GetComponent<RectTransform>();

            // Content
            var content = CreateUIElement("Content", viewport.transform);
            var contentRT = content.GetComponent<RectTransform>();
            contentRT.anchorMin = new Vector2(0, 1);
            contentRT.anchorMax = new Vector2(1, 1);
            contentRT.pivot     = new Vector2(0.5f, 1);
            contentRT.offsetMin = new Vector2(0, 0);
            contentRT.offsetMax = new Vector2(0, 0);
            var csf = content.AddComponent<ContentSizeFitter>();
            csf.verticalFit = ContentSizeFitter.FitMode.PreferredSize;
            var vlg = content.AddComponent<VerticalLayoutGroup>();
            vlg.padding = new RectOffset(6, 6, 6, 6);
            vlg.spacing = 2;
            vlg.childControlWidth  = true;
            vlg.childForceExpandWidth = true;
            sr.content = contentRT;

            // Log text
            var textGO = CreateUIElement("LogText", content.transform);
            var logText = textGO.AddComponent<TextMeshProUGUI>();
            logText.fontSize    = 11;
            logText.color       = TextLight;
            logText.richText    = true;
            logText.overflowMode = TextOverflowModes.Overflow;
            logText.enableWordWrapping = true;
            logText.text = "<color=#888888>[ Draox Demo — event log ]</color>";

            // Wire clear button
            var eventLogComp = logRoot.GetComponent<EventLog>(); // may be null here, wired later
            UnityEventTools.AddVoidPersistentListener(
                clearBtn.onClick,
                () => { EventLog.Instance?.Clear(); });

            return (logRoot, sr, logText);
        }

        // ─── Panel population ─────────────────────────────────────────────────

        private static void PopulateConnectionPanel(GameObject parent, ConnectionPanel comp)
        {
            var vlg = AddVLG(parent, 8);
            AddPanelTitle(parent, "Connection");

            var host    = AddLabeledInput(parent, "Host",       "127.0.0.1");
            var port    = AddLabeledInput(parent, "Port",       "9002");
            var protocol = AddLabeledDropdown(parent, "Protocol", new[] { "WebSocket", "TCP", "gRPC" });
            var tls     = AddLabeledToggle(parent, "Use TLS");
            var timeout = AddLabeledInput(parent, "Timeout (ms)", "10000");
            var reconToggle  = AddLabeledToggle(parent, "Reconnect");
            var maxAttempts  = AddLabeledInput(parent, "Max Attempts", "5");
            var baseDelay    = AddLabeledInput(parent, "Base Delay (s)", "1");

            var (connectBtn, disconnectBtn) = AddButtonRow(parent, "Connect", "Disconnect");

            var so = new SerializedObject(comp);
            so.FindProperty("hostInput").objectReferenceValue        = host;
            so.FindProperty("portInput").objectReferenceValue        = port;
            so.FindProperty("protocolDropdown").objectReferenceValue = protocol;
            so.FindProperty("tlsToggle").objectReferenceValue        = tls;
            so.FindProperty("timeoutInput").objectReferenceValue     = timeout;
            so.FindProperty("reconnectToggle").objectReferenceValue  = reconToggle;
            so.FindProperty("maxAttemptsInput").objectReferenceValue = maxAttempts;
            so.FindProperty("baseDelayInput").objectReferenceValue   = baseDelay;
            so.FindProperty("connectButton").objectReferenceValue    = connectBtn;
            so.FindProperty("disconnectButton").objectReferenceValue = disconnectBtn;
            so.ApplyModifiedProperties();
        }

        private static void PopulateAuthPanel(GameObject parent, AuthPanel comp)
        {
            AddPanelTitle(parent, "Authentication");

            var userId = AddLabeledInput(parent, "User ID", "user_001");
            var token  = AddLabeledInput(parent, "Token",   "test_token");
            var sessionLabel = AddLabel(parent, "Session: (none)");
            var roleDropdown = AddLabeledDropdown(parent, "Role", new[] { "Notification", "Control", "Streaming" });
            var (authBtn, addConnBtn) = AddButtonRow(parent, "Authenticate", "Add Connection");

            var so = new SerializedObject(comp);
            so.FindProperty("userIdInput").objectReferenceValue       = userId;
            so.FindProperty("tokenInput").objectReferenceValue        = token;
            so.FindProperty("sessionIdLabel").objectReferenceValue    = sessionLabel;
            so.FindProperty("roleDropdown").objectReferenceValue      = roleDropdown;
            so.FindProperty("authenticateButton").objectReferenceValue = authBtn;
            so.FindProperty("addConnectionButton").objectReferenceValue = addConnBtn;
            so.ApplyModifiedProperties();
        }

        private static void PopulateRequestPanel(GameObject parent, RequestPanel comp)
        {
            AddPanelTitle(parent, "Raw Request / Send");

            var action    = AddLabeledInput(parent, "Action",  "echo");
            var payload   = AddLabeledInput(parent, "Payload (JSON)", "{\"message\":\"hello\"}");
            var eventName = AddLabeledInput(parent, "Event Name", "system.notice");

            var (sendBtn, requestBtn) = AddButtonRow(parent, "Send (fire-and-forget)", "Request (await response)");
            var (subBtn, unsubBtn)    = AddButtonRow(parent, "Subscribe", "Unsubscribe");
            var pingBtn = AddSingleButton(parent, "Ping (RTT)");

            var so = new SerializedObject(comp);
            so.FindProperty("actionInput").objectReferenceValue    = action;
            so.FindProperty("payloadInput").objectReferenceValue   = payload;
            so.FindProperty("eventNameInput").objectReferenceValue = eventName;
            so.FindProperty("sendButton").objectReferenceValue     = sendBtn;
            so.FindProperty("requestButton").objectReferenceValue  = requestBtn;
            so.FindProperty("subscribeButton").objectReferenceValue   = subBtn;
            so.FindProperty("unsubscribeButton").objectReferenceValue = unsubBtn;
            so.FindProperty("pingButton").objectReferenceValue     = pingBtn;
            so.ApplyModifiedProperties();
        }

        private static void PopulateClansPanel(GameObject parent, ClansPanel comp)
        {
            AddPanelTitle(parent, "Clans Plugin");

            var clanName = AddLabeledInput(parent, "Clan Name",   "MyTestClan");
            var clanTag  = AddLabeledInput(parent, "Tag",         "MTC");
            var clanDesc = AddLabeledInput(parent, "Description", "");
            var clanId   = AddLabeledInput(parent, "Clan ID",     "");
            var targetUser   = AddLabeledInput(parent, "Target User ID", "");
            var promoteRole  = AddLabeledInput(parent, "Promote Role", "officer");
            var listResult   = AddLabel(parent, "");

            var (listBtn, createBtn)   = AddButtonRow(parent, "List Clans", "Create Clan");
            var (joinBtn, leaveBtn)    = AddButtonRow(parent, "Join", "Leave");
            var (kickBtn, promoteBtn)  = AddButtonRow(parent, "Kick", "Promote");

            var so = new SerializedObject(comp);
            so.FindProperty("clanNameInput").objectReferenceValue  = clanName;
            so.FindProperty("clanTagInput").objectReferenceValue   = clanTag;
            so.FindProperty("clanDescInput").objectReferenceValue  = clanDesc;
            so.FindProperty("clanIdInput").objectReferenceValue    = clanId;
            so.FindProperty("targetUserInput").objectReferenceValue  = targetUser;
            so.FindProperty("promoteRoleInput").objectReferenceValue = promoteRole;
            so.FindProperty("listResultText").objectReferenceValue = listResult;
            so.FindProperty("listButton").objectReferenceValue     = listBtn;
            so.FindProperty("createButton").objectReferenceValue   = createBtn;
            so.FindProperty("joinButton").objectReferenceValue     = joinBtn;
            so.FindProperty("leaveButton").objectReferenceValue    = leaveBtn;
            so.FindProperty("kickButton").objectReferenceValue     = kickBtn;
            so.FindProperty("promoteButton").objectReferenceValue  = promoteBtn;
            so.ApplyModifiedProperties();
        }

        private static void PopulateMessagingPanel(GameObject parent, MessagingPanel comp)
        {
            AddPanelTitle(parent, "Messaging Plugin");

            var channel   = AddLabeledInput(parent, "Channel", "general");
            var message   = AddLabeledInput(parent, "Message", "");
            var msgId     = AddLabeledInput(parent, "Message ID", "");
            var editText  = AddLabeledInput(parent, "Edit Text", "");
            var reactMsgId = AddLabeledInput(parent, "React Message ID", "");
            var emoji     = AddLabeledInput(parent, "Emoji", "👍");
            var limit     = AddLabeledInput(parent, "History Limit", "20");
            var histText  = AddLabel(parent, "");

            var (sendBtn, typingBtn)   = AddButtonRow(parent, "Send", "Send Typing");
            var (histBtn, deleteBtn)   = AddButtonRow(parent, "History", "Delete");
            var (editBtn, reactBtn)    = AddButtonRow(parent, "Edit", "React");

            var so = new SerializedObject(comp);
            so.FindProperty("channelInput").objectReferenceValue      = channel;
            so.FindProperty("messageInput").objectReferenceValue      = message;
            so.FindProperty("messageIdInput").objectReferenceValue    = msgId;
            so.FindProperty("editTextInput").objectReferenceValue     = editText;
            so.FindProperty("reactMessageIdInput").objectReferenceValue = reactMsgId;
            so.FindProperty("emojiInput").objectReferenceValue        = emoji;
            so.FindProperty("historyLimitInput").objectReferenceValue = limit;
            so.FindProperty("historyText").objectReferenceValue       = histText;
            so.FindProperty("sendButton").objectReferenceValue        = sendBtn;
            so.FindProperty("typingButton").objectReferenceValue      = typingBtn;
            so.FindProperty("historyButton").objectReferenceValue     = histBtn;
            so.FindProperty("deleteButton").objectReferenceValue      = deleteBtn;
            so.FindProperty("editButton").objectReferenceValue        = editBtn;
            so.FindProperty("reactButton").objectReferenceValue       = reactBtn;
            so.ApplyModifiedProperties();
        }

        private static void PopulatePresencePanel(GameObject parent, PresencePanel comp)
        {
            AddPanelTitle(parent, "Presence Plugin");

            var statusDropdown = AddLabeledDropdown(parent, "Status",
                new[] { "online", "away", "busy", "invisible" });
            var customText = AddLabeledInput(parent, "Custom Text", "");
            var userIds    = AddLabeledInput(parent, "User IDs (comma)", "user_001,user_002");
            var presResult = AddLabel(parent, "");

            var setBtn  = AddSingleButton(parent, "Set Status");
            var getBtn  = AddSingleButton(parent, "Get Presence");
            var (watchBtn, unwatchBtn) = AddButtonRow(parent, "Watch", "Unwatch");

            var so = new SerializedObject(comp);
            so.FindProperty("statusDropdown").objectReferenceValue       = statusDropdown;
            so.FindProperty("customTextInput").objectReferenceValue      = customText;
            so.FindProperty("userIdsInput").objectReferenceValue         = userIds;
            so.FindProperty("presenceResultText").objectReferenceValue   = presResult;
            so.FindProperty("setStatusButton").objectReferenceValue      = setBtn;
            so.FindProperty("getButton").objectReferenceValue            = getBtn;
            so.FindProperty("watchButton").objectReferenceValue          = watchBtn;
            so.FindProperty("unwatchButton").objectReferenceValue        = unwatchBtn;
            so.ApplyModifiedProperties();
        }

        // ─── UI factory helpers ───────────────────────────────────────────────

        private static GameObject CreateUIElement(string name, Transform parent)
        {
            var go = new GameObject(name);
            go.transform.SetParent(parent, false);
            go.AddComponent<RectTransform>();
            return go;
        }

        private static void SetAnchors(GameObject go,
            Vector2 anchorMin, Vector2 anchorMax,
            Vector2 offsetMin, Vector2 offsetMax)
        {
            var rt = go.GetComponent<RectTransform>();
            rt.anchorMin = anchorMin;
            rt.anchorMax = anchorMax;
            rt.offsetMin = offsetMin;
            rt.offsetMax = offsetMax;
        }

        private static Image AddImage(GameObject go, Color color)
        {
            var img = go.AddComponent<Image>();
            img.color = color;
            return img;
        }

        private static TextMeshProUGUI AddTMPText(
            GameObject go, string text, float size,
            TextAlignmentOptions align = TextAlignmentOptions.MidlineLeft)
        {
            var tmp = go.AddComponent<TextMeshProUGUI>();
            tmp.text      = text;
            tmp.fontSize  = size;
            tmp.color     = TextLight;
            tmp.alignment = align;
            tmp.enableWordWrapping = false;
            return tmp;
        }

        private static VerticalLayoutGroup AddVLG(GameObject go, int spacing = 4)
        {
            var vlg = go.AddComponent<VerticalLayoutGroup>();
            vlg.padding = new RectOffset(8, 8, 8, 8);
            vlg.spacing = spacing;
            vlg.childControlWidth  = true;
            vlg.childForceExpandWidth = true;
            vlg.childControlHeight = false;
            return vlg;
        }

        private static void AddPanelTitle(GameObject parent, string title)
        {
            var go = CreateUIElement("Title", parent.transform);
            var rt = go.GetComponent<RectTransform>();
            rt.sizeDelta = new Vector2(0, 30);
            var tmp = go.AddComponent<TextMeshProUGUI>();
            tmp.text     = title;
            tmp.fontSize = 16;
            tmp.color    = TextLight;
            tmp.fontStyle = FontStyles.Bold;
        }

        private static TMP_InputField AddLabeledInput(
            GameObject parent, string label, string defaultValue)
        {
            var row = CreateRow(parent, 28);

            var lblGO = CreateUIElement("Label", row.transform);
            var lblRT = lblGO.GetComponent<RectTransform>();
            lblRT.anchorMin = new Vector2(0, 0);
            lblRT.anchorMax = new Vector2(0.35f, 1);
            lblRT.offsetMin = Vector2.zero;
            lblRT.offsetMax = Vector2.zero;
            AddTMPText(lblGO, label, 12);

            var inputGO = CreateUIElement("Input", row.transform);
            var inputRT = inputGO.GetComponent<RectTransform>();
            inputRT.anchorMin = new Vector2(0.36f, 0);
            inputRT.anchorMax = new Vector2(1, 1);
            inputRT.offsetMin = Vector2.zero;
            inputRT.offsetMax = Vector2.zero;
            AddImage(inputGO, new Color(0.08f, 0.08f, 0.10f));

            var field = inputGO.AddComponent<TMP_InputField>();
            var textArea = CreateUIElement("TextArea", inputGO.transform);
            SetAnchors(textArea, Vector2.zero, Vector2.one, new Vector2(4, 2), new Vector2(-4, -2));
            textArea.AddComponent<RectMask2D>();
            var textComp = CreateUIElement("Text", textArea.transform);
            SetAnchors(textComp, Vector2.zero, Vector2.one, Vector2.zero, Vector2.zero);
            var txt = AddTMPText(textComp, defaultValue, 12);
            field.textComponent = txt;
            field.text = defaultValue;

            return field;
        }

        private static TMP_Dropdown AddLabeledDropdown(
            GameObject parent, string label, string[] options)
        {
            var row = CreateRow(parent, 28);

            var lblGO = CreateUIElement("Label", row.transform);
            var lblRT = lblGO.GetComponent<RectTransform>();
            lblRT.anchorMin = new Vector2(0, 0);
            lblRT.anchorMax = new Vector2(0.35f, 1);
            lblRT.offsetMin = Vector2.zero;
            lblRT.offsetMax = Vector2.zero;
            AddTMPText(lblGO, label, 12);

            var ddGO = CreateUIElement("Dropdown", row.transform);
            var ddRT = ddGO.GetComponent<RectTransform>();
            ddRT.anchorMin = new Vector2(0.36f, 0);
            ddRT.anchorMax = new Vector2(1, 1);
            ddRT.offsetMin = Vector2.zero;
            ddRT.offsetMax = Vector2.zero;
            AddImage(ddGO, new Color(0.08f, 0.08f, 0.10f));

            var dd = ddGO.AddComponent<TMP_Dropdown>();
            dd.ClearOptions();
            dd.AddOptions(options.ToList());

            return dd;
        }

        private static Toggle AddLabeledToggle(GameObject parent, string label)
        {
            var row = CreateRow(parent, 24);

            var lblGO = CreateUIElement("Label", row.transform);
            var lblRT = lblGO.GetComponent<RectTransform>();
            lblRT.anchorMin = new Vector2(0, 0);
            lblRT.anchorMax = new Vector2(0.35f, 1);
            lblRT.offsetMin = Vector2.zero;
            lblRT.offsetMax = Vector2.zero;
            AddTMPText(lblGO, label, 12);

            var tGO = CreateUIElement("Toggle", row.transform);
            var tRT = tGO.GetComponent<RectTransform>();
            tRT.anchorMin = new Vector2(0.36f, 0.1f);
            tRT.anchorMax = new Vector2(0.36f, 0.9f);
            tRT.sizeDelta = new Vector2(20, 0);
            AddImage(tGO, new Color(0.20f, 0.20f, 0.24f));

            var bg = CreateUIElement("Background", tGO.transform);
            SetAnchors(bg, Vector2.zero, Vector2.one, Vector2.zero, Vector2.zero);
            AddImage(bg, new Color(0.20f, 0.20f, 0.24f));

            var check = CreateUIElement("Checkmark", bg.transform);
            SetAnchors(check, Vector2.zero, Vector2.one, new Vector2(2, 2), new Vector2(-2, -2));
            AddImage(check, AccentBlue);

            var toggle = tGO.AddComponent<Toggle>();
            toggle.targetGraphic = bg.GetComponent<Image>();
            toggle.graphic       = check.GetComponent<Image>();
            toggle.isOn          = false;

            return toggle;
        }

        private static TextMeshProUGUI AddLabel(GameObject parent, string text)
        {
            var go = CreateRow(parent, 50);
            var tmp = go.AddComponent<TextMeshProUGUI>();
            tmp.text     = text;
            tmp.fontSize = 11;
            tmp.color    = TextMuted;
            tmp.enableWordWrapping = true;
            return tmp;
        }

        private static (Button primary, Button secondary) AddButtonRow(
            GameObject parent, string label1, string label2)
        {
            var row = CreateRow(parent, 32);
            row.AddComponent<HorizontalLayoutGroup>().spacing = 4;

            var b1 = CreateButton(row, label1, AccentBlue);
            var b2 = CreateButton(row, label2, new Color(0.25f, 0.25f, 0.30f));
            return (b1, b2);
        }

        private static Button AddSingleButton(GameObject parent, string label)
        {
            var row = CreateRow(parent, 32);
            return CreateButton(row, label, AccentGreen);
        }

        private static Button CreateButton(GameObject parent, string label, Color color)
        {
            var go = CreateUIElement(label, parent.transform);
            var le = go.AddComponent<LayoutElement>();
            le.flexibleWidth = 1;

            var img = AddImage(go, color);
            var btn = go.AddComponent<Button>();
            btn.targetGraphic = img;
            var cs = btn.colors;
            cs.normalColor      = color;
            cs.highlightedColor = Color.Lerp(color, Color.white, 0.2f);
            cs.pressedColor     = Color.Lerp(color, Color.black, 0.2f);
            btn.colors = cs;

            var txtGO = CreateUIElement("Text", go.transform);
            SetAnchors(txtGO, Vector2.zero, Vector2.one, Vector2.zero, Vector2.zero);
            AddTMPText(txtGO, label, 12, TextAlignmentOptions.Center);

            return btn;
        }

        private static GameObject CreateRow(GameObject parent, float height)
        {
            var go = CreateUIElement("Row", parent.transform);
            var le = go.AddComponent<LayoutElement>();
            le.preferredHeight = height;
            le.flexibleWidth   = 1;
            return go;
        }
    }
}
#endif
