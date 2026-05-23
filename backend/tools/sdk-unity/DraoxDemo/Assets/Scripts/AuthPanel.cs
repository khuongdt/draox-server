using System;
using Cysharp.Threading.Tasks;
using TMPro;
using UnityEngine;
using UnityEngine.UI;

namespace Draox.Demo
{
    /// <summary>
    /// Panel: authenticate with userId + token, show session state.
    /// </summary>
    public class AuthPanel : MonoBehaviour
    {
        [Header("Inputs")]
        [SerializeField] private TMP_InputField userIdInput;
        [SerializeField] private TMP_InputField tokenInput;

        [Header("Buttons")]
        [SerializeField] private Button authenticateButton;
        [SerializeField] private Button addConnectionButton;

        [Header("Status")]
        [SerializeField] private TextMeshProUGUI sessionIdLabel;
        [SerializeField] private TMP_Dropdown    roleDropdown;

        private void Start()
        {
            if (userIdInput != null) userIdInput.text = "user_001";
            if (tokenInput  != null) tokenInput.text  = "test_token";

            if (roleDropdown != null)
            {
                roleDropdown.ClearOptions();
                roleDropdown.AddOptions(new System.Collections.Generic.List<string>
                    { "Notification", "Control", "Streaming" });
            }

            authenticateButton?.onClick.AddListener(() => AuthenticateAsync().Forget());
            addConnectionButton?.onClick.AddListener(() => AddConnectionAsync().Forget());

            DemoManager.Instance.Client.OnAuthenticated += UpdateSessionLabel;
        }

        private async UniTaskVoid AuthenticateAsync()
        {
            var userId = userIdInput?.text?.Trim() ?? string.Empty;
            var token  = tokenInput?.text?.Trim()  ?? string.Empty;

            if (string.IsNullOrEmpty(userId) || string.IsNullOrEmpty(token))
            {
                DemoManager.Instance.Log("User ID and Token are required.", LogLevel.Warning);
                return;
            }

            DemoManager.Instance.Log($"Authenticating user={userId} …");
            try
            {
                await DemoManager.Instance.Client.AuthenticateAsync(userId, token);
            }
            catch (Exception ex)
            {
                DemoManager.Instance.Log($"Auth failed: {ex.Message}", LogLevel.Error);
            }
        }

        private async UniTaskVoid AddConnectionAsync()
        {
            if (!DemoManager.Instance.Client.IsAuthenticated)
            {
                DemoManager.Instance.Log("Authenticate first before adding a connection.", LogLevel.Warning);
                return;
            }

            var role = (Draox.Client.ConnectionRole)(roleDropdown?.value + 1 ?? 1);
            DemoManager.Instance.Log($"Adding {role} connection …");
            try
            {
                await DemoManager.Instance.Client.AddConnectionAsync(role);
                DemoManager.Instance.Log($"{role} connection added.", LogLevel.Success);
            }
            catch (Exception ex)
            {
                DemoManager.Instance.Log($"AddConnection failed: {ex.Message}", LogLevel.Error);
            }
        }

        private void UpdateSessionLabel()
        {
            if (sessionIdLabel != null)
                sessionIdLabel.text = $"Session: {DemoManager.Instance.Client.SessionId}";
        }
    }
}
