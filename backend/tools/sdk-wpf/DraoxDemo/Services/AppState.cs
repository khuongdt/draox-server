namespace DraoxDemo.Services;

public enum DraoxProtocol { Tcp, WebSocket, Udp }

public class AppState
{
    public string Host { get; set; } = "localhost";
    public int TcpPort { get; set; } = 9000;
    public int UdpPort { get; set; } = 9001;
    public int WsPort { get; set; } = 9002;
    public int AdminPort { get; set; } = 9100;
    public bool UseTls { get; set; } = false;
    public DraoxProtocol Protocol { get; set; } = DraoxProtocol.Tcp;

    public string? Token { get; set; }
    public string? UserId { get; set; }
    public string? SessionId { get; set; }

    public bool IsAuthenticated => Token is not null && UserId is not null;

    public string AdminBaseUrl => UseTls
        ? $"https://{Host}:{AdminPort}"
        : $"http://{Host}:{AdminPort}";

    public int SocketPort => Protocol switch
    {
        DraoxProtocol.WebSocket => WsPort,
        DraoxProtocol.Udp => UdpPort,
        _ => TcpPort,
    };
}
