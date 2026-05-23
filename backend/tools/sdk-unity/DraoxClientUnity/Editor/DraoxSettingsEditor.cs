using UnityEditor;
using UnityEngine;
using Draox.Client;

namespace Draox.Client.Editor
{
    /// <summary>
    /// Custom Inspector for DraoxClient.
    /// Shows configuration foldouts and a live runtime status panel.
    /// </summary>
    [CustomEditor(typeof(DraoxClient))]
    public class DraoxSettingsEditor : UnityEditor.Editor
    {
        private bool _showConnection  = true;
        private bool _showHeartbeat   = false;
        private bool _showReconnect   = false;
        private bool _showRuntime     = true;

        private SerializedProperty _config;
        private SerializedProperty _host;
        private SerializedProperty _port;
        private SerializedProperty _protocol;
        private SerializedProperty _useTls;
        private SerializedProperty _timeoutMs;
        private SerializedProperty _heartbeatIntervalSeconds;
        private SerializedProperty _reconnect;
        private SerializedProperty _reconnectEnabled;
        private SerializedProperty _reconnectMaxAttempts;
        private SerializedProperty _reconnectBaseDelay;
        private SerializedProperty _reconnectMaxDelay;

        private void OnEnable()
        {
            _config   = serializedObject.FindProperty("config");

            _host     = _config.FindPropertyRelative("Host");
            _port     = _config.FindPropertyRelative("Port");
            _protocol = _config.FindPropertyRelative("Protocol");
            _useTls   = _config.FindPropertyRelative("UseTls");
            _timeoutMs = _config.FindPropertyRelative("TimeoutMs");
            _heartbeatIntervalSeconds = _config.FindPropertyRelative("HeartbeatIntervalSeconds");
            _reconnect            = _config.FindPropertyRelative("Reconnect");
            _reconnectEnabled     = _reconnect.FindPropertyRelative("Enabled");
            _reconnectMaxAttempts = _reconnect.FindPropertyRelative("MaxAttempts");
            _reconnectBaseDelay   = _reconnect.FindPropertyRelative("BaseDelaySeconds");
            _reconnectMaxDelay    = _reconnect.FindPropertyRelative("MaxDelaySeconds");
        }

        public override void OnInspectorGUI()
        {
            serializedObject.Update();

            DrawConnectionSection();
            DrawHeartbeatSection();
            DrawReconnectSection();

            if (Application.isPlaying)
                DrawRuntimeStatus();

            serializedObject.ApplyModifiedProperties();
        }

        private void DrawConnectionSection()
        {
            _showConnection = EditorGUILayout.BeginFoldoutHeaderGroup(_showConnection, "Connection");
            if (_showConnection)
            {
                EditorGUI.indentLevel++;
                EditorGUILayout.PropertyField(_host,     new GUIContent("Host"));
                EditorGUILayout.PropertyField(_port,     new GUIContent("Port"));
                EditorGUILayout.PropertyField(_protocol, new GUIContent("Protocol"));
                EditorGUILayout.PropertyField(_useTls,   new GUIContent("Use TLS"));
                EditorGUILayout.PropertyField(_timeoutMs, new GUIContent("Timeout (ms)"));
                EditorGUI.indentLevel--;
            }
            EditorGUILayout.EndFoldoutHeaderGroup();
        }

        private void DrawHeartbeatSection()
        {
            _showHeartbeat = EditorGUILayout.BeginFoldoutHeaderGroup(_showHeartbeat, "Heartbeat");
            if (_showHeartbeat)
            {
                EditorGUI.indentLevel++;
                EditorGUILayout.PropertyField(
                    _heartbeatIntervalSeconds,
                    new GUIContent("Interval (seconds)"));
                EditorGUI.indentLevel--;
            }
            EditorGUILayout.EndFoldoutHeaderGroup();
        }

        private void DrawReconnectSection()
        {
            _showReconnect = EditorGUILayout.BeginFoldoutHeaderGroup(_showReconnect, "Reconnect");
            if (_showReconnect)
            {
                EditorGUI.indentLevel++;
                EditorGUILayout.PropertyField(_reconnectEnabled,     new GUIContent("Enabled"));
                if (_reconnectEnabled.boolValue)
                {
                    EditorGUILayout.PropertyField(_reconnectMaxAttempts, new GUIContent("Max Attempts (0 = unlimited)"));
                    EditorGUILayout.PropertyField(_reconnectBaseDelay,   new GUIContent("Base Delay (s)"));
                    EditorGUILayout.PropertyField(_reconnectMaxDelay,    new GUIContent("Max Delay (s)"));
                }
                EditorGUI.indentLevel--;
            }
            EditorGUILayout.EndFoldoutHeaderGroup();
        }

        private void DrawRuntimeStatus()
        {
            EditorGUILayout.Space(4);
            _showRuntime = EditorGUILayout.BeginFoldoutHeaderGroup(_showRuntime, "Runtime Status");
            if (_showRuntime)
            {
                var client = (DraoxClient)target;

                EditorGUI.BeginDisabledGroup(true);

                var stateColor = client.State switch
                {
                    ClientState.Connected    => Color.green,
                    ClientState.Connecting   => Color.yellow,
                    ClientState.Reconnecting => new Color(1f, 0.5f, 0f),
                    _                        => Color.red,
                };

                var prevColor = GUI.color;
                GUI.color = stateColor;
                EditorGUILayout.TextField("State", client.State.ToString());
                GUI.color = prevColor;

                EditorGUILayout.TextField("Session ID",
                    string.IsNullOrEmpty(client.SessionId) ? "(none)" : client.SessionId);
                EditorGUILayout.Toggle("Authenticated", client.IsAuthenticated);

                EditorGUI.EndDisabledGroup();

                EditorGUI.indentLevel++;
                if (GUILayout.Button("Disconnect", GUILayout.Height(24)))
                    client.DisconnectAsync("editor_disconnect").Forget();
                EditorGUI.indentLevel--;
            }
            EditorGUILayout.EndFoldoutHeaderGroup();

            // Repaint every frame in Play mode so the status stays live.
            if (Application.isPlaying)
                Repaint();
        }
    }
}
