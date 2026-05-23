using System;
using Cysharp.Threading.Tasks;
using Draox.Client.Plugins;
using TMPro;
using UnityEngine;
using UnityEngine.UI;

namespace Draox.Demo
{
    /// <summary>
    /// Panel: exercise the PresencePlugin — set status, get/watch users.
    /// </summary>
    public class PresencePanel : MonoBehaviour
    {
        [Header("Set Status")]
        [SerializeField] private TMP_Dropdown   statusDropdown;
        [SerializeField] private TMP_InputField customTextInput;
        [SerializeField] private Button         setStatusButton;

        [Header("Get Presence")]
        [SerializeField] private TMP_InputField userIdsInput;   // comma-separated
        [SerializeField] private Button         getButton;
        [SerializeField] private TextMeshProUGUI presenceResultText;

        [Header("Watch / Unwatch")]
        [SerializeField] private Button watchButton;
        [SerializeField] private Button unwatchButton;

        private PresencePlugin _presence;

        private void Start()
        {
            _presence = DemoManager.Instance.Presence;

            _presence.OnPresenceChanged += e =>
                Log($"PRESENCE {e.UserId} → {e.Status}  \"{e.CustomText}\"", LogLevel.Event);

            if (statusDropdown != null)
            {
                statusDropdown.ClearOptions();
                statusDropdown.AddOptions(new System.Collections.Generic.List<string>
                    { "online", "away", "busy", "invisible" });
            }

            if (userIdsInput != null) userIdsInput.text = "user_001,user_002";

            setStatusButton?.onClick.AddListener(() => SetStatusAsync().Forget());
            getButton?.onClick.AddListener(() => GetPresenceAsync().Forget());
            watchButton?.onClick.AddListener(() => WatchAsync().Forget());
            unwatchButton?.onClick.AddListener(() => UnwatchAsync().Forget());
        }

        private async UniTaskVoid SetStatusAsync()
        {
            var status     = statusDropdown?.options[statusDropdown.value].text ?? "online";
            var customText = customTextInput?.text?.Trim();

            Log($"Setting status → {status} …");
            try
            {
                await _presence.SetStatusAsync(status, string.IsNullOrEmpty(customText) ? null : customText);
                Log($"Status set to {status}.", LogLevel.Success);
            }
            catch (Exception ex) { Log($"SetStatus error: {ex.Message}", LogLevel.Error); }
        }

        private async UniTaskVoid GetPresenceAsync()
        {
            var ids = ParseIds(userIdsInput?.text);
            if (ids.Length == 0) return;

            Log($"Getting presence for {string.Join(", ", ids)} …");
            try
            {
                var res = await _presence.GetPresenceAsync(ids);
                if (presenceResultText != null)
                {
                    var sb = new System.Text.StringBuilder();
                    if (res.Users != null)
                        foreach (var u in res.Users)
                            sb.AppendLine($"  {u.UserId} ({u.Username}): {u.Status}  \"{u.CustomText}\"  last={u.LastSeenAt}");
                    presenceResultText.text = sb.ToString();
                }
                Log($"Got {res.Users?.Length ?? 0} user(s).", LogLevel.Success);
            }
            catch (Exception ex) { Log($"Get error: {ex.Message}", LogLevel.Error); }
        }

        private async UniTaskVoid WatchAsync()
        {
            var ids = ParseIds(userIdsInput?.text);
            if (ids.Length == 0) return;

            Log($"Watching {string.Join(", ", ids)} …");
            try
            {
                await _presence.WatchAsync(ids);
                Log("Watch registered.", LogLevel.Success);
            }
            catch (Exception ex) { Log($"Watch error: {ex.Message}", LogLevel.Error); }
        }

        private async UniTaskVoid UnwatchAsync()
        {
            var ids = ParseIds(userIdsInput?.text);
            if (ids.Length == 0) return;

            Log($"Unwatching {string.Join(", ", ids)} …");
            try
            {
                await _presence.UnwatchAsync(ids);
                Log("Unwatch done.", LogLevel.Success);
            }
            catch (Exception ex) { Log($"Unwatch error: {ex.Message}", LogLevel.Error); }
        }

        private static string[] ParseIds(string input)
        {
            if (string.IsNullOrWhiteSpace(input)) return Array.Empty<string>();
            return input.Split(',', StringSplitOptions.RemoveEmptyEntries);
        }

        private void Log(string msg, LogLevel level = LogLevel.Info) =>
            DemoManager.Instance.Log(msg, level);
    }
}
