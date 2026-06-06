using System.Collections.Concurrent;

namespace DraoxDemo.Protocol;

public class RequestBroker
{
    private readonly ConcurrentDictionary<string, TaskCompletionSource<WireResponse>> _pending = new();
    private readonly TimeSpan _timeout;

    public RequestBroker(int timeoutMs = 10_000)
    {
        _timeout = TimeSpan.FromMilliseconds(timeoutMs);
    }

    public (string id, Task<WireResponse> task) CreatePending()
    {
        var id = $"req_{Guid.NewGuid():N}";
        var tcs = new TaskCompletionSource<WireResponse>(TaskCreationOptions.RunContinuationsAsynchronously);
        _pending[id] = tcs;

        var timeoutTask = Task.Delay(_timeout).ContinueWith(_ =>
        {
            if (_pending.TryRemove(id, out var t))
                t.TrySetException(new TimeoutException($"Request {id} timed out"));
        });

        _ = timeoutTask;
        return (id, tcs.Task);
    }

    // Returns true if this message was a response and consumed
    public bool TryComplete(WireResponse response)
    {
        if (string.IsNullOrEmpty(response.Id)) return false;
        if (_pending.TryRemove(response.Id, out var tcs))
        {
            tcs.TrySetResult(response);
            return true;
        }
        return false;
    }

    public void FailAll(Exception ex)
    {
        foreach (var (id, tcs) in _pending.ToArray())
        {
            if (_pending.TryRemove(id, out _))
                tcs.TrySetException(ex);
        }
    }
}
