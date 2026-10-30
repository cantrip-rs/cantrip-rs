using System.Security.Cryptography;
using System.Text;
using System.Threading.Channels;
using Cantrip.Protocol;
using Cantrip.Protocol.InPerson;
using Cantrip.Server.Rules;

namespace Cantrip.Server.InPerson;

/// <summary>How a relay is set up: who sits where, how a phone's ticket is judged, and the time it keeps.</summary>
public sealed class InPersonRelayOptions
{
    /// <summary>The players' names, by seat from one.</summary>
    public required IReadOnlyList<string> PlayerNames { get; init; }

    /// <summary>
    /// The seat a phone's ticket takes, or zero for none: the event's own judgement (<c>DraftPod.SeatFor</c>), so the
    /// ticket that seats a phone at the table is the one that ties it to its seat here.
    /// </summary>
    public required Func<string, int> SeatForTicket { get; init; }

    public TimeProvider TimeProvider { get; init; } = TimeProvider.System;

    /// <summary>How long a phone has to say what it made of a tap before the tap goes to the next request.</summary>
    public TimeSpan TapTimeout { get; init; } = TimeSpan.FromSeconds(3);

    /// <summary>Called with anything the relay could not do, for the host's log; never with a credential.</summary>
    public Action<string>? Log { get; init; }
}

/// <summary>A device the relay expects, and whether it is connected: what the setup page shows.</summary>
public sealed record DeviceStatus(string Name, DeviceRole Role, int Seat, int Side, bool Connected);

/// <summary>
/// One connection to the relay, as the relay sees it: what is to be sent down it. The endpoint (<see cref="InPersonSite"/>)
/// writes <see cref="Outbox"/> to its socket and closes the socket once the outbox is complete.
/// </summary>
public sealed class RelayDevice
{
    private readonly Channel<DeviceServerMessage> _outbox = Channel.CreateUnbounded<DeviceServerMessage>(new UnboundedChannelOptions { SingleReader = true });

    internal RelayDevice()
    {
    }

    /// <summary>What the relay sends this device, in order; complete once the relay is done with it.</summary>
    public ChannelReader<DeviceServerMessage> Outbox => _outbox.Reader;

    /// <summary>Who the device was welcomed as, or null before its hello is accepted.</summary>
    internal Identity? As { get; set; }

    internal bool Closed { get; private set; }

    internal void Send(DeviceServerMessage message)
    {
        if (!Closed) _outbox.Writer.TryWrite(message);
    }

    /// <summary>Sends what is still to go, then ends the connection.</summary>
    internal void Close()
    {
        Closed = true;
        _outbox.Writer.TryComplete();
    }
}

/// <summary>A place at the table a device takes: a seat's phone, or a tablet by its token.</summary>
internal sealed class Identity(DeviceRole role, int seat, int side, string name)
{
    public DeviceRole Role { get; } = role;

    public int Seat { get; } = seat;

    public int Side { get; } = side;

    public string Name { get; } = name;

    /// <summary>The resume id and proof of the device's latest welcome; null before its first.</summary>
    public string? ResumeId { get; set; }

    public byte[] Proof { get; set; } = [];

    /// <summary>The device holding this place now, or null.</summary>
    public RelayDevice? Device { get; set; }
}

/// <summary>
/// In-person play's relay (docs/in-person.md): between the table's game and the devices around it. It is shown every
/// step the game takes (<see cref="IStepObserver"/>, through an <see cref="ObservedRulesFactory"/>), keeps the public
/// projection and the requests outstanding, and:
/// <list type="bullet">
/// <item>sends each tablet the public feed alone - a <see cref="WelcomeMessage"/> for seat zero, each
/// <see cref="StateMessage"/> with the cards the public sees for the first time, each <see cref="EventsMessage"/>, the
/// <see cref="GameOverMessage"/> - never a prompt, a request id or a seat's handle;</item>
/// <item>routes a tablet's tap to the seat with the earliest outstanding request, in that seat's own handle space, and on
/// to the next when that seat's phone did not use it;</item>
/// <item>tells a seat's phone from a tablet by its credentials: the seat's ticket, a tablet token the relay minted, or
/// the resume id and proof of an earlier welcome, compared in constant time; a newer device takes an older one's place.</item>
/// </list>
/// Socket-free and thread-safe: the table thread steps it, the endpoint's connections feed it, and a test drives both
/// with a fake clock.
/// </summary>
public sealed class InPersonRelay : IStepObserver
{
    /// <summary>The most taps waiting on phones at once; an older one is let go to make room.</summary>
    private const int MostTaps = 32;

    private readonly object _gate = new();
    private readonly InPersonRelayOptions _options;
    private readonly List<Identity> _phones = [];
    private readonly List<(byte[] TokenHash, Identity Tablet)> _tablets = [];
    private readonly List<RelayDevice> _devices = [];
    private readonly List<PendingTap> _taps = [];
    private int _nextTap = 1;

    // The game being played, as the public sees it.
    private IRulesHost? _game;
    private PrintedCards _printed = new();
    private PlayerView? _view;
    private long _sequence;
    private GameOverMessage? _over;
    private IReadOnlyDictionary<int, IReadOnlyDictionary<ViewHandle, ViewHandle>> _handles = new Dictionary<int, IReadOnlyDictionary<ViewHandle, ViewHandle>>();
    private List<(int RequestId, int Seat)> _outstanding = [];

    // Where the event stands before its game.
    private TablePhase _phase = TablePhase.Setup;
    private string _line = string.Empty;

    public InPersonRelay(InPersonRelayOptions options)
    {
        ArgumentNullException.ThrowIfNull(options);
        ArgumentNullException.ThrowIfNull(options.SeatForTicket);
        if (options.PlayerNames.Count < 1)
            throw new ArgumentException("A table has at least one seat.", nameof(options));

        _options = options;
        for (int seat = 1; seat <= options.PlayerNames.Count; seat++)
        {
            _phones.Add(new Identity(DeviceRole.Phone, seat, 0, $"{options.PlayerNames[seat - 1]}'s phone"));
        }
    }

    /// <summary>The players' names, by seat from one.</summary>
    public IReadOnlyList<string> PlayerNames => _options.PlayerNames;

    /// <summary>
    /// Makes a tablet's token: what its QR code holds, which brings a tablet showing <paramref name="side"/> - zero for the
    /// shared board, or the seat whose half it is - to the table. A bearer credential, kept here only as its hash.
    /// </summary>
    public string MintTablet(int side)
    {
        if (side < 0 || side > _options.PlayerNames.Count)
            throw new ArgumentOutOfRangeException(nameof(side), "A tablet shows the shared board or one seat's half.");
        string token = Convert.ToHexString(RandomNumberGenerator.GetBytes(32));
        string name = side == 0 ? "The shared tablet" : $"{_options.PlayerNames[side - 1]}'s tablet";
        lock (_gate)
        {
            _tablets.Add((Hash(token), new Identity(DeviceRole.Tablet, 0, side, name)));
        }

        return token;
    }

    /// <summary>Every device the table expects - each seat's phone, each tablet minted - and whether it is connected.</summary>
    public IReadOnlyList<DeviceStatus> Devices
    {
        get
        {
            lock (_gate)
            {
                return [.. _phones.Concat(_tablets.Select(each => each.Tablet))
                    .Select(identity => new DeviceStatus(identity.Name, identity.Role, identity.Seat, identity.Side, identity.Device is not null))];
            }
        }
    }

    /// <summary>A new connection, which says who it is with its first message.</summary>
    public RelayDevice Attach()
    {
        var device = new RelayDevice();
        lock (_gate) _devices.Add(device);
        return device;
    }

    /// <summary>A connection is gone. The place it held waits for the next device to take it.</summary>
    public void Detach(RelayDevice device)
    {
        ArgumentNullException.ThrowIfNull(device);
        lock (_gate)
        {
            _devices.Remove(device);
            device.Close();
            if (device.As is { } identity && identity.Device == device)
            {
                identity.Device = null;
                PhaseChanged();
            }

            device.As = null;
        }
    }

    /// <summary>Something a connection sent.</summary>
    public void Receive(RelayDevice device, DeviceClientMessage message)
    {
        ArgumentNullException.ThrowIfNull(device);
        ArgumentNullException.ThrowIfNull(message);
        lock (_gate)
        {
            if (device.Closed) return;
            switch (message)
            {
                case DeviceHelloMessage hello:
                    Hello(device, hello);
                    break;
                case TableTapMessage tap when device.As is { Role: DeviceRole.Tablet }:
                    TableTap(tap);
                    break;
                case TapResultMessage result when device.As is { Role: DeviceRole.Phone } phone:
                    TapResult(phone.Seat, result);
                    break;
                case { } when device.As is null:
                    Refuse(device, ErrorCode.MembershipRequired, "A device says who it is first.");
                    break;
                default:
                    device.Send(new DeviceErrorMessage { Code = ErrorCode.IllegalAnswer, Detail = $"A {device.As!.Role.ToString().ToLowerInvariant()} does not send that." });
                    break;
            }
        }
    }

    /// <summary>
    /// Where the event stands before its game, for the tablets to show until the first public state: told to them when it
    /// changes. Safe from any thread.
    /// </summary>
    public void Phase(TablePhase phase, string line)
    {
        lock (_gate)
        {
            if (phase == _phase && string.Equals(line, _line, StringComparison.Ordinal)) return;
            _phase = phase;
            _line = line ?? string.Empty;
            PhaseChanged();
        }
    }

    /// <summary>Lets a tap go on to the next request once its phone has had too long to say what it made of it.</summary>
    public void Tick()
    {
        lock (_gate)
        {
            DateTimeOffset now = _options.TimeProvider.GetUtcNow();
            foreach (PendingTap tap in _taps.Where(tap => tap.Deadline <= now).ToList())
            {
                Route(tap);
            }
        }
    }

    public void Stepped(IRulesHost game, RulesStep step, bool started)
    {
        ArgumentNullException.ThrowIfNull(game);
        ArgumentNullException.ThrowIfNull(step);
        lock (_gate)
        {
            if (started)
            {
                _game = game;
                _printed = new PrintedCards();
                _view = null;
                _sequence = 0;
                _over = null;
                _taps.Clear();
            }
            else if (!ReferenceEquals(game, _game))
            {
                return;
            }

            _outstanding = [.. step.Requests.OrderBy(request => request.RequestId).Select(request => (request.RequestId, request.Seat))];
            if (step.Public is not { } projection)
            {
                _options.Log?.Invoke("A step came without a public view: the game was not dealt with one, so the tablets show nothing of it.");
                return;
            }

            // Stamped with each object's fingerprint already, as the host stamps every view it parses.
            PlayerView view = projection.View;
            List<ObjectView> fresh;
            try
            {
                fresh = _printed.NewlySeen(view, game.Oracle);
            }
            catch (Exception error) when (error is RulesRejectedException or InvalidOperationException or IOException)
            {
                // The rules could not say what a card is: the tablets go without it rather than the table stopping.
                _options.Log?.Invoke($"The public view's cards could not be read: {error.Message}");
                fresh = [];
            }

            _view = view;
            _handles = projection.Handles;
            _sequence = step.Sequence;
            List<ServerMessage> feed = [];
            if (started) feed.Add(PublicWelcome());
            feed.Add(new StateMessage { View = view, Printed = fresh });
            if (projection.Events.Count > 0)
            {
                feed.Add(new EventsMessage
                {
                    Events = [.. projection.Events.Select(each => each.View)],
                    Sequences = [.. projection.Events.Select(each => each.Sequence)],
                    Sequence = step.Sequence,
                });
            }

            if (step.Finished && _over is null)
            {
                _over = new GameOverMessage { Outcome = game.Outcome, WinningTeam = game.WinningTeam };
                feed.Add(_over);
            }

            foreach (RelayDevice tablet in Tablets())
            {
                foreach (ServerMessage message in feed)
                {
                    tablet.Send(new PublicFeedMessage { Message = message });
                }
            }
        }
    }

    public void Closed(IRulesHost game)
    {
        lock (_gate)
        {
            if (!ReferenceEquals(game, _game)) return;

            // The board stays on the tablets; there is nothing left to tap for.
            _outstanding = [];
            _taps.Clear();
        }
    }

    private void Hello(RelayDevice device, DeviceHelloMessage hello)
    {
        if (device.As is not null)
        {
            device.Send(new DeviceErrorMessage { Code = ErrorCode.IllegalAnswer, Detail = "This device has already said who it is." });
            return;
        }

        if (hello.InPersonVersion != InPersonProtocol.Version || hello.ProtocolVersion != PlayerView.ProtocolVersion)
        {
            Refuse(device, ErrorCode.ProtocolMismatch,
                $"Expected in-person protocol {InPersonProtocol.Version} and protocol {PlayerView.ProtocolVersion}: reload the page.");
            return;
        }

        Identity? identity = null;
        bool fresh = true;
        if (hello.ResumeId is { Length: > 0 } resumeId && hello.Proof is { } proof)
        {
            identity = _phones.Concat(_tablets.Select(each => each.Tablet))
                .FirstOrDefault(each => each.ResumeId is { } id && string.Equals(id, resumeId, StringComparison.Ordinal)
                    && CryptographicOperations.FixedTimeEquals(each.Proof, proof));
            fresh = identity is null;
        }

        if (identity is null && hello.Ticket is { Length: > 0 } ticket)
        {
            int seat;
            try
            {
                seat = _options.SeatForTicket(ticket);
            }
            catch (FormatException)
            {
                seat = 0;
            }

            identity = seat >= 1 && seat <= _phones.Count ? _phones[seat - 1] : null;
        }

        if (identity is null && hello.Device is { Length: > 0 } token)
        {
            byte[] presented = Hash(token);
            foreach ((byte[] hash, Identity tablet) in _tablets)
            {
                // Every token compared, so how long the look takes says nothing of which one matched.
                if (CryptographicOperations.FixedTimeEquals(hash, presented)) identity = tablet;
            }
        }

        if (identity is null)
        {
            Refuse(device, ErrorCode.MembershipRefused, "That code is not for this table, or no longer works: scan the setup page's again.");
            return;
        }

        // A newer device takes the place over: the one there is told so and let go.
        if (identity.Device is { } older && older != device)
        {
            Refuse(older, ErrorCode.MembershipRefused, "Another device has taken this one's place at the table.");
            older.As = null;
        }

        if (fresh)
        {
            identity.ResumeId = Convert.ToHexString(RandomNumberGenerator.GetBytes(16));
            identity.Proof = RandomNumberGenerator.GetBytes(32);
        }

        identity.Device = device;
        device.As = identity;
        device.Send(new DeviceWelcomeMessage
        {
            Role = identity.Role,
            Seat = identity.Seat,
            Side = identity.Side,
            ResumeId = identity.ResumeId!,
            Proof = identity.Proof,
            PlayerNames = [.. _options.PlayerNames],
        });

        if (identity.Role == DeviceRole.Tablet)
        {
            CatchUp(device);
        }

        PhaseChanged();
    }

    /// <summary>A tablet just welcomed is brought up to date: the game as it stands, or where the event stands before it.</summary>
    private void CatchUp(RelayDevice tablet)
    {
        if (_view is null)
        {
            tablet.Send(PhaseMessage());
            return;
        }

        tablet.Send(new PublicFeedMessage { Message = PublicWelcome() });
        tablet.Send(new PublicFeedMessage { Message = new StateMessage { View = _view, Printed = [.. _printed.Cards] } });
        tablet.Send(new PublicFeedMessage { Message = new EventsMessage { Sequence = _sequence } });
        if (_over is not null) tablet.Send(new PublicFeedMessage { Message = _over });
    }

    private void TableTap(TableTapMessage tap)
    {
        if (_view is null || _over is not null) return;
        bool player = tap.Object.IsNone;
        if (player ? tap.Player < 1 || tap.Player > _phones.Count : tap.Player != 0) return;

        if (_taps.Count >= MostTaps) _taps.RemoveAt(0);
        var pending = new PendingTap(_nextTap++, tap.Object, tap.Player);
        _taps.Add(pending);
        Route(pending);
    }

    private void TapResult(int seat, TapResultMessage result)
    {
        if (_taps.FirstOrDefault(tap => tap.TapId == result.TapId && tap.Seat == seat) is not { } tap) return;
        if (result.Used)
        {
            _taps.Remove(tap);
            return;
        }

        Route(tap);
    }

    /// <summary>
    /// Gives a tap to the earliest outstanding request it has not been offered to whose seat names the object tapped and
    /// has a phone at the table, or lets it go when there is none.
    /// </summary>
    private void Route(PendingTap tap)
    {
        foreach ((int requestId, int seat) in _outstanding)
        {
            if (!tap.Tried.Add(requestId)) continue;
            if (_phones[seat - 1].Device is not { } phone) continue;

            ViewHandle seatHandle = ViewHandle.None;
            if (!tap.Object.IsNone && (!_handles.TryGetValue(seat, out IReadOnlyDictionary<ViewHandle, ViewHandle>? map)
                || !map.TryGetValue(tap.Object, out seatHandle)))
            {
                continue;
            }

            tap.RequestId = requestId;
            tap.Seat = seat;
            tap.Deadline = _options.TimeProvider.GetUtcNow() + _options.TapTimeout;
            phone.Send(new SeatTapMessage { TapId = tap.TapId, RequestId = requestId, Object = seatHandle, Player = tap.Player });
            return;
        }

        _taps.Remove(tap);
    }

    private WelcomeMessage PublicWelcome() => new() { Seat = 0, PlayerNames = [.. _options.PlayerNames] };

    private DevicePhaseMessage PhaseMessage() => new()
    {
        Phase = _phase,
        Line = _line,
        Connected = [.. _phones.Concat(_tablets.Select(each => each.Tablet)).Where(each => each.Device is not null).Select(each => each.Name)],
    };

    private void PhaseChanged()
    {
        DevicePhaseMessage message = PhaseMessage();
        foreach (RelayDevice tablet in Tablets())
        {
            tablet.Send(message);
        }
    }

    private IEnumerable<RelayDevice> Tablets() =>
        _tablets.Select(each => each.Tablet.Device).OfType<RelayDevice>().ToList();

    private static void Refuse(RelayDevice device, ErrorCode code, string detail)
    {
        device.Send(new DeviceErrorMessage { Code = code, Detail = detail });
        device.Close();
    }

    private static byte[] Hash(string token) => SHA256.HashData(Encoding.UTF8.GetBytes(token));

    private sealed class PendingTap(int tapId, ViewHandle @object, int player)
    {
        public int TapId { get; } = tapId;

        public ViewHandle Object { get; } = @object;

        public int Player { get; } = player;

        /// <summary>The requests it has been offered to.</summary>
        public HashSet<int> Tried { get; } = [];

        public int RequestId { get; set; }

        public int Seat { get; set; }

        public DateTimeOffset Deadline { get; set; } = DateTimeOffset.MaxValue;
    }
}
