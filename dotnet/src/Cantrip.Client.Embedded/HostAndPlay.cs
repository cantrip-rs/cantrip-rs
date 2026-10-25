using Cantrip.Core;
using Cantrip.Protocol;
using Cantrip.Server;
using Cantrip.Server.Rules;

namespace Cantrip.Client.Embedded;

/// <summary>
/// Starts a game inside this process and hands back a transport per seat.
/// <para>
/// The only thing that distinguishes this from joining a remote server is which <see cref="IGameTransport"/> the
/// client ends up holding. Everything above that interface — the view, the prompts, the answers, the rendering — is
/// identical, so single-player is not a second implementation of the game to keep in step with the networked one.
/// </para>
/// </summary>
public static class HostAndPlay
{
    /// <summary>
    /// The format a tool's games are played in, from its command line: <c>--format KEY</c> - a constructed format's key, or
    /// <c>limited</c> with <c>--min-deck N</c> (forty by default) - and Freeform, with no deck-construction restriction at
    /// all, when it names none (<see cref="Cantrip.Decks.DeckFormat.Freeform"/>). Its minimum deck size is what the engine is told.
    /// </summary>
    /// <exception cref="ArgumentException">A format no deck is built for.</exception>
    public static Cantrip.Decks.DeckFormat FormatFrom(IReadOnlyList<string> args)
    {
        ArgumentNullException.ThrowIfNull(args);
        string? key = null;
        int minimum = 40;
        for (int i = 0; i + 1 < args.Count; i++)
        {
            if (args[i] == "--format") key = args[i + 1];
            if (args[i] == "--min-deck" && int.TryParse(args[i + 1], out int n)) minimum = n;
        }

        if (key is null) return Cantrip.Decks.DeckFormat.Freeform;
        if (key == Cantrip.Decks.DeckFormat.LimitedKey) return Cantrip.Decks.DeckFormat.Limited(minimum);
        return Cantrip.Decks.DeckFormat.Find(key) ?? throw new ArgumentException($"There is no format '{key}'.", nameof(args));
    }


    /// <summary>
    /// Starts a game from decks named one per seat on the rules engine <paramref name="rules"/> deals: the decks' card
    /// names as written, dealt round again where there are more seats than decks, as a server's table deals them
    /// (<c>GameServer.BuildTable</c>).
    /// <para>
    /// For playing many games in one process: every seat's deck is stated, so a matchup is the matchup asked for, and
    /// the loopback wiring is the same one a single-player client gets — the seats answer real protocol prompts from
    /// the real session, which is what makes a bulk run a test of the server and not of a shortcut around it.
    /// </para>
    /// <para>
    /// The cards are the native engine's own: nothing under <c>content/</c> is loaded, and a card the engine has not
    /// ported is the host's refusal to deal (<see cref="RulesRejectedException"/>).
    /// </para>
    /// <para>
    /// The game has the chess clocks every table has (docs/events.md, "Clocks"), on <paramref name="time"/> - the system's
    /// unless a test gives one it moves itself: each player's <paramref name="playerClock"/>, an hour by default, running
    /// while the game waits on them. Whoever plays the game charges them as it goes (<see cref="HostedGame.Tick"/>).
    /// </para>
    /// </summary>
    public static HostedGame StartTable(
        IRulesHostFactory rules,
        IReadOnlyList<DeckList> decks,
        Cantrip.Decks.DeckFormat format,
        ulong seed,
        IReadOnlyList<string> playerNames,
        TimeProvider? time = null,
        TimeSpan? playerClock = null)
    {
        ArgumentNullException.ThrowIfNull(rules);
        ArgumentNullException.ThrowIfNull(decks);
        ArgumentNullException.ThrowIfNull(format);
        ArgumentNullException.ThrowIfNull(playerNames);

        if (decks.Count == 0)
        {
            throw new ArgumentException("A table needs at least one deck.", nameof(decks));
        }

        static List<string> Named(IReadOnlyList<DeckListEntry> entries) =>
            [.. entries.SelectMany(entry => Enumerable.Repeat(entry.Card, entry.Copies))];

        List<RulesSeat> seats = [.. playerNames.Select((name, index) =>
            new RulesSeat(name, Named(decks[index % decks.Count].Entries), Named(decks[index % decks.Count].Sideboard), decks[index % decks.Count].Companion)
            {
                DeckName = decks[index % decks.Count].Name,
            })];

        // The engine is told the format's minimum deck size, as a server's table tells it (CR 100.2a-b).
        IRulesHost game = rules.Create(seats, seed, new RulesGameOptions { MinimumDeckSize = format.MinimumDeckSize });

        var clocks = new HostedClocks(time ?? TimeProvider.System, playerClock ?? MatchClocks.DefaultPlayerClock);
        return Host(new GameSession(game), Listed(decks), clocks);
    }

    /// <summary>
    /// Each seat's decklist as names and counts, seat one first.
    /// <para>
    /// Kept because a seat answered by something that reasons about the game wants it: what is left in the library
    /// is most of what a decision turns on, and it is the one thing about a game that no projection of the position
    /// can say. Nothing else reads it, and a seat that ignores it is none the worse.
    /// </para>
    /// </summary>
    private static Dictionary<int, string> Listed(IReadOnlyList<DeckList> decks)
    {
        Dictionary<int, string> listed = [];

        for (int seat = 1; seat <= decks.Count; seat++)
        {
            listed[seat] = string.Join(
                '\n',
                decks[seat - 1].Entries.Select(entry => $"{entry.Copies} {entry.Card}"));
        }

        return listed;
    }

    private static HostedGame Host(GameSession session, Dictionary<int, string> decks, HostedClocks clocks)
    {
        Dictionary<int, IGameTransport> transports = [];
        foreach (PlayerId seat in session.Seats)
        {
            transports[seat.Value] = new LoopbackTransport(session, seat);
        }

        return new HostedGame(session, transports, decks, clocks);
    }
}

/// <summary>What a game in one process is clocked by: its time, and each player's clock.</summary>
internal sealed record HostedClocks(TimeProvider Time, TimeSpan PlayerClock);

/// <summary>A game running in this process, with a transport per seat.</summary>
public sealed class HostedGame : IDisposable
{
    private readonly Dictionary<int, IGameTransport> _transports;

    internal HostedGame(
        GameSession session,
        Dictionary<int, IGameTransport> transports,
        IReadOnlyDictionary<int, string> decks,
        HostedClocks clocking)
    {
        _session = session;
        _transports = transports;
        Decks = decks;
        _clocking = clocking;
    }

    private readonly HostedClocks _clocking;

    /// <summary>The game's clocks once it has started (docs/events.md, "Clocks").</summary>
    public MatchClocks? Clocks { get; private set; }

    /// <summary>What each seat was dealt, as a decklist of names and counts.</summary>
    public IReadOnlyDictionary<int, string> Decks { get; }

    private readonly GameSession _session;

    /// <summary>Start the hosted authority without exposing its implementation to presentation, and its clocks.</summary>
    public void Start()
    {
        _session.Start();
        Clocks = new MatchClocks(_session.Seats.ToDictionary(seat => seat.Value, _ => _clocking.PlayerClock), _clocking.Time.GetUtcNow());
    }

    /// <summary>
    /// Charges the game's clocks for the time since the last look, as a table does: a player out of time loses and leaves
    /// the game through its rules (CR 104.3a, 800.4a), the others playing on. Called by whatever plays the game, as often as
    /// it looks at it.
    /// </summary>
    public void Tick()
    {
        if (Clocks is null || _session.IsOver)
        {
            return;
        }

        ClockTick tick = Clocks.Tick(_clocking.Time.GetUtcNow(), _session.WaitingOn, _session.Courtesies);
        if (tick.RanOut.Count > 0)
        {
            _session.TimeOut(tick.RanOut);
        }
    }

    public IGameTransport TransportFor(int seat) => _transports[seat];

    public IReadOnlyCollection<int> Seats => _transports.Keys;

    public void Dispose()
    {
        foreach (IGameTransport transport in _transports.Values)
        {
            transport.Dispose();
        }

        _session.Dispose();
    }

}
