using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

namespace Draox.Client
{
    internal static class Serializer
    {
        private static readonly JsonSerializerSettings _settings = new JsonSerializerSettings
        {
            NullValueHandling = NullValueHandling.Ignore,
        };

        public static string Serialize(object obj) =>
            JsonConvert.SerializeObject(obj, _settings);

        public static T Deserialize<T>(string json)
        {
            if (string.IsNullOrEmpty(json)) return default;
            return JsonConvert.DeserializeObject<T>(json);
        }

        // Parses an incoming server message into a discriminated ParsedMessage.
        public static ParsedMessage Parse(string json)
        {
            if (string.IsNullOrEmpty(json)) return null;

            JObject token;
            try { token = JObject.Parse(json); }
            catch { return null; }

            var type = token["type"]?.Value<string>();

            return type switch
            {
                "response" => new ParsedMessage
                {
                    Type    = "response",
                    Id      = token["id"]?.Value<string>(),
                    Success = token["success"]?.Value<bool>() ?? false,
                    RawData = token["data"]?.ToString(Formatting.None),
                    Error   = token["error"]?.Value<string>(),
                },
                "event" => new ParsedMessage
                {
                    Type      = "event",
                    Category  = token["category"]?.Value<string>(),
                    Name      = token["name"]?.Value<string>(),
                    RawData   = token["data"]?.ToString(Formatting.None),
                    Timestamp = token["timestamp"]?.Value<string>(),
                },
                "pong" => new ParsedMessage { Type = "pong" },
                _      => new ParsedMessage { Type = type ?? "unknown" },
            };
        }
    }

    internal class ParsedMessage
    {
        public string Type;
        // response
        public string Id;
        public bool   Success;
        public string RawData;
        public string Error;
        // event
        public string Category;
        public string Name;
        public string Timestamp;
    }
}
