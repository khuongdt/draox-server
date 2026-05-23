using System;
using Cysharp.Threading.Tasks;
using TMPro;
using UnityEngine;
using UnityEngine.UI;

namespace Draox.Demo
{
    /// <summary>
    /// Panel: fire raw actions against the server.
    /// Send = fire-and-forget; Request = awaited response.
    /// Also provides subscribe/unsubscribe for arbitrary event names.
    /// </summary>
    public class RequestPanel : MonoBehaviour
    {
        [Header("Request / Send")]
        [SerializeField] private TMP_InputField actionInput;
        [SerializeField] private TMP_InputField payloadInput;
        [SerializeField] private Button         sendButton;
        [SerializeField] private Button         requestButton;

        [Header("Subscribe")]
        [SerializeField] private TMP_InputField eventNameInput;
        [SerializeField] private Button         subscribeButton;
        [SerializeField] private Button         unsubscribeButton;

        [Header("Ping")]
        [SerializeField] private Button pingButton;

        private void Start()
        {
            if (actionInput  != null) actionInput.text  = "echo";
            if (payloadInput != null) payloadInput.text = "{\"message\":\"hello\"}";
            if (eventNameInput != null) eventNameInput.text = "system.notice";

            sendButton?.onClick.AddListener(() => SendAsync().Forget());
            requestButton?.onClick.AddListener(() => RequestAsync().Forget());
            subscribeButton?.onClick.AddListener(Subscribe);
            unsubscribeButton?.onClick.AddListener(Unsubscribe);
            pingButton?.onClick.AddListener(() => PingAsync().Forget());
        }

        private async UniTaskVoid SendAsync()
        {
            var action = actionInput?.text?.Trim();
            if (string.IsNullOrEmpty(action)) return;

            object payload = ParsePayload(payloadInput?.text);
            DemoManager.Instance.Log($"→ Send  action={action}");
            try
            {
                await DemoManager.Instance.Client.SendAsync(action, payload);
                DemoManager.Instance.Log("Send dispatched.", LogLevel.Success);
            }
            catch (Exception ex)
            {
                DemoManager.Instance.Log($"Send error: {ex.Message}", LogLevel.Error);
            }
        }

        private async UniTaskVoid RequestAsync()
        {
            var action = actionInput?.text?.Trim();
            if (string.IsNullOrEmpty(action)) return;

            object payload = ParsePayload(payloadInput?.text);
            DemoManager.Instance.Log($"→ Request  action={action}");
            try
            {
                var result = await DemoManager.Instance.Client.RequestAsync<object>(action, payload);
                DemoManager.Instance.Log($"← Response: {Newtonsoft.Json.JsonConvert.SerializeObject(result)}", LogLevel.Success);
            }
            catch (Draox.Client.DraoxTimeoutException)
            {
                DemoManager.Instance.Log("Request timed out.", LogLevel.Warning);
            }
            catch (Exception ex)
            {
                DemoManager.Instance.Log($"Request error: {ex.Message}", LogLevel.Error);
            }
        }

        private void Subscribe()
        {
            var name = eventNameInput?.text?.Trim();
            if (string.IsNullOrEmpty(name)) return;

            DemoManager.Instance.Client.Subscribe(name, OnEvent);
            DemoManager.Instance.Log($"Subscribed to \"{name}\"", LogLevel.Success);
        }

        private void Unsubscribe()
        {
            var name = eventNameInput?.text?.Trim();
            if (string.IsNullOrEmpty(name)) return;

            DemoManager.Instance.Client.Unsubscribe(name, OnEvent);
            DemoManager.Instance.Log($"Unsubscribed from \"{name}\"", LogLevel.Warning);
        }

        private void OnEvent(Draox.Client.DraoxEvent evt)
        {
            DemoManager.Instance.Log(
                $"EVENT  {evt.Category}.{evt.Name}  data={evt.RawData}", LogLevel.Event);
        }

        private async UniTaskVoid PingAsync()
        {
            DemoManager.Instance.Log("→ ping");
            var start = DateTime.UtcNow;
            try
            {
                await DemoManager.Instance.Client.RequestAsync<object>("ping", null);
                var rtt = (DateTime.UtcNow - start).TotalMilliseconds;
                DemoManager.Instance.Log($"← pong  {rtt:F0} ms", LogLevel.Success);
            }
            catch (Exception ex)
            {
                DemoManager.Instance.Log($"Ping error: {ex.Message}", LogLevel.Error);
            }
        }

        private static object ParsePayload(string text)
        {
            if (string.IsNullOrWhiteSpace(text)) return null;
            try   { return Newtonsoft.Json.JsonConvert.DeserializeObject(text); }
            catch { return text; }
        }
    }
}
