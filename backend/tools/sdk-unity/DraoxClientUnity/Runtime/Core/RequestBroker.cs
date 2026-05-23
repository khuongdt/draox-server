using System;
using System.Collections.Concurrent;
using System.Threading;
using Cysharp.Threading.Tasks;

namespace Draox.Client
{
    // Tracks in-flight requests and correlates them with incoming responses by ID.
    internal class RequestBroker
    {
        private readonly ConcurrentDictionary<string, Entry> _pending = new ConcurrentDictionary<string, Entry>();

        private class Entry
        {
            public UniTaskCompletionSource<DraoxResponse> Tcs;
            public CancellationTokenSource                Cts;
        }

        public async UniTask<DraoxResponse> SendAsync(
            IConnection connection,
            string json,
            string requestId,
            int timeoutMs,
            CancellationToken ct)
        {
            var tcs  = new UniTaskCompletionSource<DraoxResponse>();
            var cts  = CancellationTokenSource.CreateLinkedTokenSource(ct);

            _pending[requestId] = new Entry { Tcs = tcs, Cts = cts };

            cts.Token.Register(() =>
            {
                if (_pending.TryRemove(requestId, out _))
                    tcs.TrySetException(new DraoxTimeoutException(requestId));
            });
            cts.CancelAfter(timeoutMs);

            await connection.SendTextAsync(json, ct);
            return await tcs.Task;
        }

        // Called by message dispatcher when a "response" message arrives.
        public void Complete(string id, DraoxResponse response)
        {
            if (_pending.TryRemove(id, out var entry))
            {
                entry.Cts.Dispose();
                entry.Tcs.TrySetResult(response);
            }
        }

        // Fails all pending requests — called on disconnect.
        public void FailAll(Exception ex)
        {
            foreach (var kv in _pending)
            {
                if (_pending.TryRemove(kv.Key, out var entry))
                {
                    entry.Cts.Dispose();
                    entry.Tcs.TrySetException(ex);
                }
            }
        }
    }
}
