using Cantrip.Protocol;

namespace Cantrip.Client.Web.InPerson;

/// <summary>
/// The public feed a tablet is sent (<see cref="Cantrip.Protocol.InPerson.PublicFeedMessage"/>), as a transport a plain
/// <see cref="GameClient"/> reads, so the tablet keeps its view, its recent events and the printed cards as any seat's
/// page does. Only the four messages a feed may carry are let through - the welcome, a state, events and the game's end -
/// so nothing of a prompt could reach the page even were one sent. A tablet sends the game nothing: its taps go to the
/// relay on the device's own wire.
/// </summary>
public sealed class FeedTransport : IGameTransport
{
    private readonly List<ServerMessage> _pending = [];

    /// <summary>Whether the device's connection is up; the page sets it from the connection it reads the feed from.</summary>
    public bool IsConnected { get; set; } = true;

    /// <summary>Takes one message of the feed, if it is one a feed may carry.</summary>
    public void Feed(ServerMessage message)
    {
        if (message is WelcomeMessage or StateMessage or EventsMessage or GameOverMessage)
        {
            _pending.Add(message);
        }
    }

    /// <summary>A tablet answers nothing and asks for nothing.</summary>
    public void Send(ClientMessage message)
    {
    }

    public IReadOnlyList<ServerMessage> Receive()
    {
        if (_pending.Count == 0)
        {
            return [];
        }

        List<ServerMessage> taken = [.. _pending];
        _pending.Clear();
        return taken;
    }

    public void Dispose()
    {
    }
}
