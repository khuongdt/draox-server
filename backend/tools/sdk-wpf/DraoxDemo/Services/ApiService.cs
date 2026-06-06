using DraoxDemo.Models;
using System.Net.Http;
using System.Net.Http.Json;
using System.Text.Json;

namespace DraoxDemo.Services;

public class ApiService
{
    private readonly AppState _state;
    private readonly HttpClient _http;

    private static readonly JsonSerializerOptions _json = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower,
        PropertyNameCaseInsensitive = true,
    };

    public ApiService(AppState state)
    {
        _state = state;
        _http = new HttpClient(new HttpClientHandler
        {
            // Accept self-signed certs in dev
            ServerCertificateCustomValidationCallback = (_, _, _, _) => true
        });
    }

    private HttpClient Http
    {
        get
        {
            _http.BaseAddress = new Uri(_state.AdminBaseUrl);
            _http.DefaultRequestHeaders.Clear();
            if (_state.Token is not null)
                _http.DefaultRequestHeaders.Authorization =
                    new System.Net.Http.Headers.AuthenticationHeaderValue("Bearer", _state.Token);
            return _http;
        }
    }

    // --- Auth ---
    public async Task<LoginResponse?> LoginAsync(string username, string password)
    {
        var req = new LoginRequest { Username = username, Password = password };
        var res = await Http.PostAsJsonAsync("/api/auth/login", req, _json);
        res.EnsureSuccessStatusCode();
        var wrap = await res.Content.ReadFromJsonAsync<ApiResponse<LoginResponse>>(_json);
        return wrap?.Data;
    }

    // --- Channels ---
    public async Task<List<ChannelDto>> GetChannelsAsync()
    {
        var res = await Http.GetFromJsonAsync<ApiResponse<List<ChannelDto>>>("/api/channels", _json);
        return res?.Data ?? [];
    }

    public async Task<ChannelDto?> CreateChannelAsync(string name, string? description = null)
    {
        var req = new CreateChannelRequest { Name = name, Description = description };
        var res = await Http.PostAsJsonAsync("/api/channels", req, _json);
        res.EnsureSuccessStatusCode();
        var wrap = await res.Content.ReadFromJsonAsync<ApiResponse<ChannelDto>>(_json);
        return wrap?.Data;
    }

    public async Task DeleteChannelAsync(string channelId)
    {
        var res = await Http.DeleteAsync($"/api/channels/{channelId}");
        res.EnsureSuccessStatusCode();
    }

    public async Task<MessageHistoryResponse?> GetMessagesAsync(string channelId, int limit = 50, string? before = null)
    {
        var url = $"/api/channels/{channelId}/messages?limit={limit}";
        if (before is not null) url += $"&before={before}";
        return await Http.GetFromJsonAsync<MessageHistoryResponse>(url, _json);
    }

    public async Task SubscribeChannelAsync(string channelId)
    {
        var res = await Http.PostAsync($"/api/channels/{channelId}/subscribe", null);
        res.EnsureSuccessStatusCode();
    }

    public async Task UnsubscribeChannelAsync(string channelId)
    {
        var res = await Http.PostAsync($"/api/channels/{channelId}/unsubscribe", null);
        res.EnsureSuccessStatusCode();
    }

    // --- Messages ---
    public async Task<MessageDto?> SendMessageAsync(string channelId, string text, string? replyToId = null)
    {
        var req = new SendMessageRequest { ChannelId = channelId, Text = text, ReplyToId = replyToId };
        var res = await Http.PostAsJsonAsync("/api/messages/send", req, _json);
        res.EnsureSuccessStatusCode();
        var wrap = await res.Content.ReadFromJsonAsync<ApiResponse<MessageDto>>(_json);
        return wrap?.Data;
    }

    public async Task DeleteMessageAsync(string messageId)
    {
        var res = await Http.DeleteAsync($"/api/messages/{messageId}");
        res.EnsureSuccessStatusCode();
    }

    public async Task<MessageDto?> EditMessageAsync(string messageId, string newText)
    {
        var req = new EditMessageRequest { Text = newText };
        var res = await Http.PatchAsJsonAsync($"/api/messages/{messageId}", req, _json);
        res.EnsureSuccessStatusCode();
        var wrap = await res.Content.ReadFromJsonAsync<ApiResponse<MessageDto>>(_json);
        return wrap?.Data;
    }

    public async Task ReactAsync(string messageId, string emoji)
    {
        var req = new ReactRequest { Emoji = emoji };
        var res = await Http.PostAsJsonAsync($"/api/messages/{messageId}/react", req, _json);
        res.EnsureSuccessStatusCode();
    }

    // --- Clans ---
    public async Task<List<ClanDto>> GetClansAsync()
    {
        var res = await Http.GetFromJsonAsync<ApiResponse<List<ClanDto>>>("/api/clans", _json);
        return res?.Data ?? [];
    }

    public async Task<ClanDto?> CreateClanAsync(string name, string tag)
    {
        var req = new CreateClanRequest { Name = name, Tag = tag };
        var res = await Http.PostAsJsonAsync("/api/clans", req, _json);
        res.EnsureSuccessStatusCode();
        var wrap = await res.Content.ReadFromJsonAsync<ApiResponse<ClanDto>>(_json);
        return wrap?.Data;
    }

    public async Task DeleteClanAsync(string clanId)
    {
        var res = await Http.DeleteAsync($"/api/clans/{clanId}");
        res.EnsureSuccessStatusCode();
    }

    public async Task JoinClanAsync(string clanId)
    {
        var res = await Http.PostAsync($"/api/clans/{clanId}/join", null);
        res.EnsureSuccessStatusCode();
    }

    public async Task LeaveClanAsync(string clanId)
    {
        var res = await Http.PostAsync($"/api/clans/{clanId}/leave", null);
        res.EnsureSuccessStatusCode();
    }

    public async Task<List<ClanMemberDto>> GetClanMembersAsync(string clanId)
    {
        var res = await Http.GetFromJsonAsync<ApiResponse<List<ClanMemberDto>>>($"/api/clans/{clanId}/members", _json);
        return res?.Data ?? [];
    }

    // --- Server Info ---
    public async Task<ServerInfoDto?> GetServerInfoAsync()
    {
        var res = await Http.GetFromJsonAsync<ApiResponse<ServerInfoDto>>("/api/info", _json);
        return res?.Data;
    }

    public async Task<HealthDto?> GetHealthAsync()
    {
        var res = await Http.GetFromJsonAsync<ApiResponse<HealthDto>>("/api/health", _json);
        return res?.Data;
    }
}
