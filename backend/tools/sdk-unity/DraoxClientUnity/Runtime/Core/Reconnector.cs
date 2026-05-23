using System;
using System.Threading;
using Cysharp.Threading.Tasks;
using UnityEngine;

namespace Draox.Client
{
    // Implements exponential backoff reconnect with jitter.
    internal class Reconnector
    {
        private readonly ReconnectConfig _config;
        private readonly Random          _rng = new Random();
        private int _attempts;

        public event Action OnReconnecting;
        public event Action OnReconnected;
        public event Action OnFailed;

        public Reconnector(ReconnectConfig config) => _config = config;

        public void Reset() => _attempts = 0;

        // Calls tryConnect() repeatedly with exponential backoff until success or max attempts.
        // Returns true if reconnected, false if all attempts exhausted.
        public async UniTask<bool> AttemptAsync(Func<UniTask<bool>> tryConnect, CancellationToken ct = default)
        {
            while (_attempts < _config.MaxAttempts)
            {
                _attempts++;

                float delay = Mathf.Min(
                    _config.BaseDelaySeconds * Mathf.Pow(2f, _attempts - 1),
                    _config.MaxDelaySeconds);
                delay += (float)(_rng.NextDouble() * 0.5); // ±0.5s jitter

                OnReconnecting?.Invoke();
                Debug.Log($"[Draox] Reconnect attempt {_attempts}/{_config.MaxAttempts} in {delay:F1}s");

                await UniTask.Delay(TimeSpan.FromSeconds(delay), cancellationToken: ct);

                if (ct.IsCancellationRequested) break;

                if (await tryConnect())
                {
                    Reset();
                    OnReconnected?.Invoke();
                    return true;
                }
            }

            OnFailed?.Invoke();
            return false;
        }
    }
}
