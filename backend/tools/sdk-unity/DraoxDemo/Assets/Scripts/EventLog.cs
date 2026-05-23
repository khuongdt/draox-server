using System;
using System.Collections.Generic;
using TMPro;
using UnityEngine;
using UnityEngine.UI;

namespace Draox.Demo
{
    /// <summary>
    /// Singleton scrollable log panel.
    /// Call EventLog.Instance.Append(...) from any script.
    /// </summary>
    public class EventLog : MonoBehaviour
    {
        public static EventLog Instance { get; private set; }

        [SerializeField] private ScrollRect scrollRect;
        [SerializeField] private TextMeshProUGUI logText;
        [SerializeField] private int maxLines = 200;

        private readonly Queue<string> _lines = new Queue<string>();

        private void Awake()
        {
            if (Instance != null) { Destroy(gameObject); return; }
            Instance = this;
        }

        public void Append(string message, LogLevel level = LogLevel.Info)
        {
            var time  = DateTime.Now.ToString("HH:mm:ss");
            var color = level switch
            {
                LogLevel.Success => "#55dd55",
                LogLevel.Warning => "#ffcc55",
                LogLevel.Error   => "#ff5555",
                LogLevel.Event   => "#88ccff",
                _                => "#dddddd",
            };
            var label = level switch
            {
                LogLevel.Success => "OK",
                LogLevel.Warning => "WARN",
                LogLevel.Error   => "ERR",
                LogLevel.Event   => "EVT",
                _                => "LOG",
            };

            var line = $"<color=#888888>[{time}]</color> <color={color}>[{label}]</color> {message}";
            _lines.Enqueue(line);

            while (_lines.Count > maxLines)
                _lines.Dequeue();

            logText.text = string.Join("\n", _lines);

            // Scroll to bottom on next frame.
            Canvas.ForceUpdateCanvases();
            scrollRect.verticalNormalizedPosition = 0f;
        }

        public void Clear()
        {
            _lines.Clear();
            logText.text = string.Empty;
        }
    }

    public enum LogLevel { Info, Success, Warning, Error, Event }
}
