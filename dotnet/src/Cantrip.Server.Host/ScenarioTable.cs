using System.Text.RegularExpressions;
using System.Threading.Channels;
using Cantrip.Server.Rules;

namespace Cantrip.Server.Host;

/// <summary>
/// A table playing a <c>.cantrip</c> scenario for something to drive (docs/scenarios-in-the-browser.md): a real server on
/// a loopback port, the script answering every seat but the driven ones, and each step's scenario part - what a driven
/// seat is meant to answer, the client checks, the verdict - queued for the driver in the order the table published it.
/// </summary>
public sealed partial class ScenarioTable : IAsyncDisposable
{
    private readonly Channel<ScenarioProgress> _progress = Channel.CreateUnbounded<ScenarioProgress>();

    private ScenarioTable(GameServer server, ScenarioSpec scenario)
    {
        Server = server;
        Scenario = scenario;
    }

    public GameServer Server { get; }

    public ScenarioSpec Scenario { get; }

    /// <summary>The address a client joins at.</summary>
    public Uri Address => new($"ws://127.0.0.1:{Server.Port}/game");

    /// <summary>
    /// Starts a table playing <paramref name="scenario"/>. It deals once the driven seats are seated and ready, each
    /// joining as a client would. Its clocks - every table's (docs/events.md, "Clocks") - run on <paramref name="time"/>,
    /// the system's unless a test gives one it moves itself. Its games are dealt by <paramref name="rules"/>, the native
    /// engine unless given another - in-person play's, which shows each step to its relay. <paramref name="decks"/> is the
    /// table's catalog, which a seat reserved with <see cref="GameServer.Reserve"/> names a deck from (the scenario deals
    /// its own decks whatever a seat names).
    /// </summary>
    public static async Task<ScenarioTable> StartAsync(
        ScenarioSpec scenario,
        string contentRoot,
        TimeProvider? time = null,
        IRulesHostFactory? rules = null,
        IReadOnlyList<DeckList>? decks = null)
    {
        ArgumentNullException.ThrowIfNull(scenario);
        GameServer server = await GameServer.StartAsync(new GameServerOptions
        {
            Port = 0,
            Players = Seats(scenario.Text),
            ContentRoot = contentRoot,
            AutoStart = true,
            Scenario = scenario,
            TimeProvider = time ?? TimeProvider.System,
            Rules = rules ?? RulesHostFactory.Native,
            Decks = decks ?? [],
        });
        var table = new ScenarioTable(server, scenario);
        server.ScenarioProgressed += progress => table._progress.Writer.TryWrite(progress);
        return table;
    }

    /// <summary>The next step's scenario part, in the order the table published them.</summary>
    public async Task<ScenarioProgress> NextAsync(TimeSpan patience, CancellationToken cancellation = default)
    {
        using var timeout = CancellationTokenSource.CreateLinkedTokenSource(cancellation);
        timeout.CancelAfter(patience);
        try
        {
            return await _progress.Reader.ReadAsync(timeout.Token);
        }
        catch (OperationCanceledException) when (!cancellation.IsCancellationRequested)
        {
            throw new TimeoutException($"{Scenario.Name}: the table published nothing for {patience.TotalSeconds:F0}s.");
        }
    }

    /// <summary>How many seats a scenario deals: one per <c>[BotN Deck]</c>.</summary>
    public static int Seats(string text) => Math.Max(2, DeckHeader().Matches(text).Count);

    [GeneratedRegex(@"^\s*\[Bot\d+ Deck\]", RegexOptions.Multiline)]
    private static partial Regex DeckHeader();

    public ValueTask DisposeAsync() => Server.DisposeAsync();
}
