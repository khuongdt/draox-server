namespace Draox.Client;

internal class Reconnector(ReconnectConfig config)
{
    public async Task<bool> AttemptAsync(Func<Task<bool>> tryConnect, CancellationToken ct)
    {
        var attempt = 0;
        while (!ct.IsCancellationRequested)
        {
            if (config.MaxAttempts > 0 && attempt >= config.MaxAttempts) return false;
            attempt++;

            var delay = Math.Min(
                config.BaseDelaySeconds * Math.Pow(2, attempt - 1),
                config.MaxDelaySeconds);

            try { await Task.Delay(TimeSpan.FromSeconds(delay), ct); }
            catch (OperationCanceledException) { return false; }

            try { if (await tryConnect()) return true; }
            catch { /* next attempt */ }
        }
        return false;
    }
}
