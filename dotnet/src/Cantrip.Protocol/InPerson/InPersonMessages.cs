using MessagePack;

namespace Cantrip.Protocol.InPerson;

/// <summary>
/// The wire between in-person play's devices and the relay that serves them (docs/in-person.md): a phone for each
/// player's hidden information and prompts, a tablet or two for the public board. Its own unions and its own version, so
/// nothing here touches <see cref="ServerMessage"/>, <see cref="ClientMessage"/> or <see cref="PlayerView.ProtocolVersion"/>.
/// </summary>
public static class InPersonProtocol
{
    /// <summary>The in-person wire's version, sent in every <see cref="DeviceHelloMessage"/>; a mismatch is refused.</summary>
    public const int Version = 2;
}

/// <summary>What a device is at the table.</summary>
public enum DeviceRole
{
    /// <summary>One player's own device: their hand, their library searches, their private choices and every prompt.</summary>
    Phone = 0,

    /// <summary>A public board: what anyone at the table may see, and where its permanents and players are tapped.</summary>
    Tablet = 1,
}

/// <summary>
/// Anything a device sends the relay. A MessagePack union with explicit keys, as <see cref="ClientMessage"/> is.
/// </summary>
[Union(0, typeof(DeviceHelloMessage))]
[Union(1, typeof(TableTapMessage))]
[Union(2, typeof(TapResultMessage))]
[Union(8, typeof(ReentryRequestMessage))]
public abstract class DeviceClientMessage
{
    public abstract string Describe();
}

/// <summary>
/// A device's first message: who it is. A phone brings its seat's ticket, a tablet its device token; either brings the
/// resume id and proof it was welcomed with before, to take its place back after a reload.
/// </summary>
[MessagePackObject]
public sealed class DeviceHelloMessage : DeviceClientMessage
{
    [Key(0)]
    public int InPersonVersion { get; set; } = InPersonProtocol.Version;

    [Key(1)]
    public int ProtocolVersion { get; set; } = PlayerView.ProtocolVersion;

    /// <summary>A phone's seat ticket, a bearer credential: never logged or described. Null for a tablet or a resume.</summary>
    [Key(2)]
    public string? Ticket { get; set; }

    /// <summary>A tablet's device token, a bearer credential: never logged or described. Null for a phone or a resume.</summary>
    [Key(3)]
    public string? Device { get; set; }

    /// <summary>The <see cref="DeviceWelcomeMessage.ResumeId"/> this device was welcomed with before, or null.</summary>
    [Key(4)]
    public string? ResumeId { get; set; }

    /// <summary>The <see cref="DeviceWelcomeMessage.Proof"/> that goes with <see cref="ResumeId"/>, or null. Never logged.</summary>
    [Key(5)]
    public byte[]? Proof { get; set; }

    /// <summary>
    /// A phone's re-entry code (<see cref="ReentryCodeMessage"/>), brought by the home-screen app it was minted for on its
    /// first launch, to take the seat's phone over. A bearer credential: never logged or described. Null otherwise.
    /// </summary>
    [Key(6)]
    public string? Reentry { get; set; }

    public override string Describe() =>
        $"hello v{InPersonVersion}/{ProtocolVersion}{(Ticket is null ? "" : ", ticket")}{(Device is null ? "" : ", device")}{(ResumeId is null ? "" : ", resume")}{(Reentry is null ? "" : ", re-entry")}";
}

/// <summary>
/// A tablet's tap on its board: an object by its public handle, or a player. The relay hands it to the seat with the
/// earliest outstanding request, in that seat's own handle space (<see cref="SeatTapMessage"/>).
/// </summary>
[MessagePackObject]
public sealed class TableTapMessage : DeviceClientMessage
{
    /// <summary>How far through the public log the tablet had been brought when it was tapped.</summary>
    [Key(0)]
    public long Sequence { get; set; }

    /// <summary>The object tapped, in the public handle space; <see cref="ViewHandle.None"/> for a tapped player.</summary>
    [Key(1)]
    public ViewHandle Object { get; set; }

    /// <summary>The player tapped, by seat; zero for a tapped object.</summary>
    [Key(2)]
    public int Player { get; set; }

    public override string Describe() => Object.IsNone ? $"table tap, player {Player}" : $"table tap, {Object}";
}

/// <summary>A phone's report of what it made of a <see cref="SeatTapMessage"/>: whether its prompt took the tap.</summary>
[MessagePackObject]
public sealed class TapResultMessage : DeviceClientMessage
{
    [Key(0)]
    public int TapId { get; set; }

    /// <summary>Whether the tap answered or advanced the phone's prompt; false hands it to the next request.</summary>
    [Key(1)]
    public bool Used { get; set; }

    public override string Describe() => $"tap {TapId} {(Used ? "used" : "unused")}";
}

/// <summary>
/// A welcomed phone asks for a re-entry code (<see cref="ReentryCodeMessage"/>), for the home-screen app it is about to be
/// added as: an app with storage of its own, which knows nothing of the browser tab's ticket or welcome.
/// </summary>
[MessagePackObject]
public sealed class ReentryRequestMessage : DeviceClientMessage
{
    public override string Describe() => "re-entry asked for";
}

/// <summary>
/// Anything the relay sends a device. A MessagePack union with explicit keys, as <see cref="ServerMessage"/> is.
/// </summary>
[Union(0, typeof(DeviceWelcomeMessage))]
[Union(1, typeof(PublicFeedMessage))]
[Union(2, typeof(SeatTapMessage))]
[Union(3, typeof(DevicePhaseMessage))]
[Union(4, typeof(DeviceErrorMessage))]
[Union(8, typeof(ReentryCodeMessage))]
public abstract class DeviceServerMessage
{
    public abstract string Describe();
}

/// <summary>Sent once a device's hello is accepted: what it is, and how it takes its place back after a reload.</summary>
[MessagePackObject]
public sealed class DeviceWelcomeMessage : DeviceServerMessage
{
    [Key(0)]
    public DeviceRole Role { get; set; }

    /// <summary>A phone's seat, from one; zero for a tablet.</summary>
    [Key(1)]
    public int Seat { get; set; }

    /// <summary>Which board a tablet shows: zero for the shared board of both halves, or the seat whose half it is.</summary>
    [Key(2)]
    public int Side { get; set; }

    /// <summary>What the device sends back in a later hello (<see cref="DeviceHelloMessage.ResumeId"/>).</summary>
    [Key(3)]
    public string ResumeId { get; set; } = string.Empty;

    /// <summary>The secret that goes with <see cref="ResumeId"/>, kept for the tab alone. Never logged or described.</summary>
    [Key(4)]
    public byte[] Proof { get; set; } = [];

    /// <summary>The players' names, by seat from one.</summary>
    [Key(5)]
    public List<string> PlayerNames { get; set; } = [];

    /// <summary>
    /// For a phone welcomed by its re-entry code, a fresh seat ticket for the table's own connection, which the home-screen
    /// app keeps in its storage; null for every other welcome. A bearer credential: never logged or described.
    /// </summary>
    [Key(6)]
    public string? Ticket { get; set; }

    public override string Describe() => $"device welcome, {Role}, seat {Seat}, side {Side}";
}

/// <summary>
/// One message of the public feed, for a tablet: only a <see cref="WelcomeMessage"/> for seat zero, a
/// <see cref="StateMessage"/>, an <see cref="EventsMessage"/> or a <see cref="GameOverMessage"/>, all in the public
/// handle space. Never a prompt, a request id or a seat's handle.
/// </summary>
[MessagePackObject]
public sealed class PublicFeedMessage : DeviceServerMessage
{
    [Key(0)]
    public ServerMessage Message { get; set; } = new WelcomeMessage();

    public override string Describe() => $"public feed: {Message.Describe()}";
}

/// <summary>
/// A tap made on a tablet, for the phone whose request it was routed to, in that seat's own handle space. The phone acts
/// on it only while <see cref="RequestId"/> is its prompt, and answers with a <see cref="TapResultMessage"/>.
/// </summary>
[MessagePackObject]
public sealed class SeatTapMessage : DeviceServerMessage
{
    [Key(0)]
    public int TapId { get; set; }

    /// <summary>The outstanding request the tap was routed to.</summary>
    [Key(1)]
    public int RequestId { get; set; }

    /// <summary>The object tapped, in the seat's handle space; <see cref="ViewHandle.None"/> for a tapped player.</summary>
    [Key(2)]
    public ViewHandle Object { get; set; }

    /// <summary>The player tapped, by seat; zero for a tapped object.</summary>
    [Key(3)]
    public int Player { get; set; }

    public override string Describe() =>
        Object.IsNone ? $"seat tap {TapId} for {RequestId}, player {Player}" : $"seat tap {TapId} for {RequestId}, {Object}";
}

/// <summary>
/// Where the event stands before its game - drafting, building decks - for a tablet to show until the first public state.
/// </summary>
[MessagePackObject]
public sealed class DevicePhaseMessage : DeviceServerMessage
{
    [Key(0)]
    public TablePhase Phase { get; set; }

    /// <summary>One line saying where the event stands: the grid being drafted, whose pick it is, who has submitted.</summary>
    [Key(1)]
    public string Line { get; set; } = string.Empty;

    /// <summary>The devices connected now, by name.</summary>
    [Key(2)]
    public List<string> Connected { get; set; } = [];

    public override string Describe() => $"phase {Phase}: {Line}";
}

/// <summary>Something a device sent could not be accepted: a hello refused, or a message out of place.</summary>
[MessagePackObject]
public sealed class DeviceErrorMessage : DeviceServerMessage
{
    [Key(0)]
    public ErrorCode Code { get; set; }

    [Key(1)]
    public string Detail { get; set; } = string.Empty;

    public override string Describe() => $"device error {Code}: {Detail}";
}

/// <summary>
/// A phone's re-entry code, for the home-screen app it is being added as (docs/in-person.md, "Full screen"): good once,
/// for the seat's phone, until it is used, a newer one is asked for, the seat's ticket takes the phone's place anew or the
/// table closes. A bearer credential: never logged or described.
/// </summary>
[MessagePackObject]
public sealed class ReentryCodeMessage : DeviceServerMessage
{
    [Key(0)]
    public string Code { get; set; } = string.Empty;

    public override string Describe() => "re-entry code";
}
