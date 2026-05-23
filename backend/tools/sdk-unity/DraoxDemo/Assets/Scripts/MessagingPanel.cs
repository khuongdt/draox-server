using System;
using Cysharp.Threading.Tasks;
using Draox.Client.Plugins;
using TMPro;
using UnityEngine;
using UnityEngine.UI;

namespace Draox.Demo
{
    /// <summary>
    /// Panel: exercise the MessagingPlugin — send, history, delete, typing.
    /// </summary>
    public class MessagingPanel : MonoBehaviour
    {
        [Header("Send")]
        [SerializeField] private TMP_InputField channelInput;
        [SerializeField] private TMP_InputField messageInput;
        [SerializeField] private Button         sendButton;
        [SerializeField] private Button         typingButton;

        [Header("History")]
        [SerializeField] private TMP_InputField historyLimitInput;
        [SerializeField] private Button         historyButton;
        [SerializeField] private TextMeshProUGUI historyText;

        [Header("Delete / Edit")]
        [SerializeField] private TMP_InputField messageIdInput;
        [SerializeField] private TMP_InputField editTextInput;
        [SerializeField] private Button         deleteButton;
        [SerializeField] private Button         editButton;

        [Header("React")]
        [SerializeField] private TMP_InputField reactMessageIdInput;
        [SerializeField] private TMP_InputField emojiInput;
        [SerializeField] private Button         reactButton;

        private MessagingPlugin _messaging;

        private void Start()
        {
            _messaging = DemoManager.Instance.Messaging;

            _messaging.OnMessage       += e => Log($"MSG from={e.Message?.SenderId}  \"{e.Message?.Text}\"", LogLevel.Event);
            _messaging.OnMessageDeleted += e => Log($"MSG deleted id={e.MessageId}", LogLevel.Event);
            _messaging.OnTyping        += e => Log($"TYPING {e.Username} is{(e.IsTyping ? "" : " not")} typing in {e.ChannelId}", LogLevel.Event);

            if (channelInput      != null) channelInput.text      = "general";
            if (historyLimitInput != null) historyLimitInput.text = "20";
            if (emojiInput        != null) emojiInput.text        = "👍";

            sendButton?.onClick.AddListener(() => SendAsync().Forget());
            typingButton?.onClick.AddListener(() => SendTypingAsync().Forget());
            historyButton?.onClick.AddListener(() => HistoryAsync().Forget());
            deleteButton?.onClick.AddListener(() => DeleteAsync().Forget());
            editButton?.onClick.AddListener(() => EditAsync().Forget());
            reactButton?.onClick.AddListener(() => ReactAsync().Forget());
        }

        private async UniTaskVoid SendAsync()
        {
            var channel = channelInput?.text?.Trim();
            var text    = messageInput?.text?.Trim();
            if (string.IsNullOrEmpty(channel) || string.IsNullOrEmpty(text)) return;

            Log($"→ Send msg to #{channel}: \"{text}\"");
            try
            {
                var res = await _messaging.SendMessageAsync(channel, text);
                Log($"Message sent  id={res.Message?.Id}", LogLevel.Success);
                if (messageIdInput != null) messageIdInput.text = res.Message?.Id ?? string.Empty;
                if (messageInput   != null) messageInput.text   = string.Empty;
            }
            catch (Exception ex) { Log($"Send error: {ex.Message}", LogLevel.Error); }
        }

        private async UniTaskVoid SendTypingAsync()
        {
            var channel = channelInput?.text?.Trim();
            if (string.IsNullOrEmpty(channel)) return;
            await _messaging.SendTypingAsync(channel);
            Log($"Typing indicator sent to #{channel}", LogLevel.Info);
        }

        private async UniTaskVoid HistoryAsync()
        {
            var channel = channelInput?.text?.Trim();
            if (string.IsNullOrEmpty(channel)) return;

            int limit = int.TryParse(historyLimitInput?.text, out var l) ? l : 20;

            Log($"Fetching {limit} messages from #{channel} …");
            try
            {
                var res = await _messaging.GetHistoryAsync(channel, limit);
                if (historyText != null)
                {
                    var sb = new System.Text.StringBuilder();
                    if (res.Messages != null)
                        foreach (var m in res.Messages)
                            sb.AppendLine($"[{m.SentAt}] {m.SenderId}: {m.Text}  id={m.Id}");
                    historyText.text = sb.ToString();
                }
                Log($"Got {res.Messages?.Length ?? 0} message(s). HasMore={res.HasMore}", LogLevel.Success);
            }
            catch (Exception ex) { Log($"History error: {ex.Message}", LogLevel.Error); }
        }

        private async UniTaskVoid DeleteAsync()
        {
            var id = messageIdInput?.text?.Trim();
            if (string.IsNullOrEmpty(id)) return;

            Log($"Deleting message {id} …");
            try
            {
                await _messaging.DeleteMessageAsync(id);
                Log($"Message {id} deleted.", LogLevel.Success);
            }
            catch (Exception ex) { Log($"Delete error: {ex.Message}", LogLevel.Error); }
        }

        private async UniTaskVoid EditAsync()
        {
            var id   = messageIdInput?.text?.Trim();
            var text = editTextInput?.text?.Trim();
            if (string.IsNullOrEmpty(id) || string.IsNullOrEmpty(text)) return;

            Log($"Editing message {id} …");
            try
            {
                var res = await _messaging.EditMessageAsync(id, text);
                Log($"Edited  id={res.Id}  editedAt={res.EditedAt}", LogLevel.Success);
            }
            catch (Exception ex) { Log($"Edit error: {ex.Message}", LogLevel.Error); }
        }

        private async UniTaskVoid ReactAsync()
        {
            var id    = reactMessageIdInput?.text?.Trim();
            var emoji = emojiInput?.text?.Trim();
            if (string.IsNullOrEmpty(id) || string.IsNullOrEmpty(emoji)) return;

            Log($"Reacting {emoji} to message {id} …");
            try
            {
                await _messaging.ReactAsync(id, emoji);
                Log("Reaction added.", LogLevel.Success);
            }
            catch (Exception ex) { Log($"React error: {ex.Message}", LogLevel.Error); }
        }

        private void Log(string msg, LogLevel level = LogLevel.Info) =>
            DemoManager.Instance.Log(msg, level);
    }
}
