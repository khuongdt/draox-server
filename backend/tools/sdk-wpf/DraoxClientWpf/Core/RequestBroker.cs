using System.Collections.Concurrent;

namespace Draox.Client;

internal class RequestBroker
{
    private readonly ConcurrentDictionary<string, TaskCompletionSource<DraoxResponse>> _pending = new();

    public async Task<DraoxResponse> SendAsync(
        IConnection connection, string json, string id, int timeoutMs, CancellationToken ct)
    {
        var tcs = new TaskCompletionSource<DraoxResponse>(TaskCreationOptions.RunContinuationsAsynchronously);
        _pending[id] = tcs;

        using var linked = CancellationTokenSource.CreateLinkedTokenSource(ct);
        linked.CancelAfter(timeoutMs);

        await using (linked.Token.Register(() =>
        {
            if (_pending.TryRemove(id, out var t))
                t.TrySetException(new DraoxTimeoutException(id));
        }))
        {
            await connection.SendTextAsync(json, ct);
            return await tcs.Task;
        }
    }

    public void Complete(string id, DraoxResponse response)
    {
        if (_pending.TryRemove(id, out var tcs))
            tcs.TrySetResult(response);
    }

    public void FailAll(Exception ex)
    {
        foreach (var key in _pending.Keys)
            if (_pending.TryRemove(key, out var tcs))
                tcs.TrySetException(ex);
    }
}
