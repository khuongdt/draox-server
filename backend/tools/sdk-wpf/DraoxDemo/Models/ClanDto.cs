using System.Text.Json.Serialization;

namespace DraoxDemo.Models;

public class ClanDto
{
    [JsonPropertyName("id")]
    public string Id { get; set; } = string.Empty;

    [JsonPropertyName("name")]
    public string Name { get; set; } = string.Empty;

    [JsonPropertyName("tag")]
    public string Tag { get; set; } = string.Empty;

    [JsonPropertyName("description")]
    public string? Description { get; set; }

    [JsonPropertyName("owner_id")]
    public string OwnerId { get; set; } = string.Empty;

    [JsonPropertyName("member_count")]
    public int MemberCount { get; set; }

    [JsonPropertyName("max_members")]
    public int MaxMembers { get; set; } = 50;

    [JsonPropertyName("created_at")]
    public string CreatedAt { get; set; } = string.Empty;

    [JsonPropertyName("is_system")]
    public bool IsSystem { get; set; }

    [JsonPropertyName("frozen")]
    public bool Frozen { get; set; }

    // Client-side state, not from server
    [JsonIgnore]
    public bool IsMember { get; set; }
}

public class ClanMemberDto
{
    [JsonPropertyName("user_id")]
    public string UserId { get; set; } = string.Empty;

    [JsonPropertyName("username")]
    public string Username { get; set; } = string.Empty;

    [JsonPropertyName("role")]
    public string Role { get; set; } = "member";

    [JsonPropertyName("joined_at")]
    public string JoinedAt { get; set; } = string.Empty;

    [JsonIgnore]
    public string RoleIcon => Role switch
    {
        "owner" => "👑",
        "officer" => "⭐",
        _ => "👤"
    };

    [JsonIgnore]
    public string RoleLabel => Role switch
    {
        "owner" => "Owner",
        "officer" => "Officer",
        _ => "Member"
    };
}

public class CreateClanRequest
{
    [JsonPropertyName("name")]
    public string Name { get; set; } = string.Empty;

    [JsonPropertyName("tag")]
    public string Tag { get; set; } = string.Empty;
}
