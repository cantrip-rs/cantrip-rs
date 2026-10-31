using Cantrip.Art;
using Cantrip.Protocol;
using Cantrip.Protocol.InPerson;
using Cantrip.Server.Host;
using Cantrip.Server.Rules;
using Xunit;

namespace Cantrip.Client.Web.Tests;

/// <summary>
/// In-person play's tablets (docs/in-person.md): the public board, shared between the players with seat 2's half turned
/// to face them, or one for each player; tapped with a finger, and sending the relay what was tapped and nothing else.
/// <para>
/// Fed here by the test rather than a relay: a real table plays the Bolt-the-Goyf scenario to Lightning Bolt on the stack,
/// and what it sent a seat - a state, events - is handed to the tablet as the public feed would carry it.
/// </para>
/// </summary>
[Collection(WebClientCollection.Name)]
[Trait("Category", "WallClock")]
public sealed class InPersonTests(WebClient web)
{
    private const string Scenario = "in-person-lightning-bolt-aimed-from-the-tablets-kills-tarmogoyf";

    private static readonly TimeSpan Patience = TimeSpan.FromSeconds(90);

    [Fact]
    public async Task A_tablet_draws_the_public_board_and_sends_only_what_is_tapped()
    {
        IReadOnlyList<ServerMessage> feed = await BoltOnTheStackAsync();
        await using Microsoft.AspNetCore.Builder.WebApplication art = await ArtServer.StartAsync(
            new ArtStore(new ArtStoreOptions { ContentRoot = WebClient.Content, CacheDirectory = WebClient.ArtCache }), "127.0.0.1", 0);
        string pictures = WebClient.Pictures(nameof(InPersonTests));
        string query = $"art={Uri.EscapeDataString(art.Urls.First())}";

        // The shared board: before the game, where the draft stands; then the game, seat 2's half turned at the top.
        await using (TabletDriver shared = await TabletDriver.OpenAsync(web, query, pictures, "shared"))
        {
            await shared.FeedAsync(new DeviceWelcomeMessage { Role = DeviceRole.Tablet, Side = 0, PlayerNames = ["Bot1", "Bot2"] });
            await shared.FeedAsync(new DevicePhaseMessage
            {
                Phase = TablePhase.Drafting,
                Line = "Grid 4 of 18 · Bot2 to pick",
                Connected = ["Bot1's phone", "Bot2's phone", "the table"],
            });
            await shared.SettledAsync(state => state.Screen == "phase" && state.PhaseLine.Contains("Grid 4", StringComparison.Ordinal));
            await shared.PictureAsync("phase");

            TabletState board = await FeedGameAsync(shared, feed);
            await shared.PictureAsync("board");
            Assert.Equal("shared", board.Layout);

            TabletCard goyf = board.Cards.Single(card => card.Name == "Tarmogoyf");
            TabletCard mountain = board.Cards.First(card => card.Name == "Mountain");
            Assert.Equal(("opponent-battlefield", 2), (goyf.Place, goyf.Seat));
            Assert.True(goyf.Y < TabletDriver.Height / 2f, $"Bot2's Tarmogoyf is on the turned half at the top, not at {goyf.Y}.");
            Assert.Equal(("battlefield", 1), (mountain.Place, mountain.Seat));
            Assert.True(mountain.Y > TabletDriver.Height / 2f, $"Bot1's Mountain is on the bottom half, not at {mountain.Y}.");
            Assert.Contains(board.Cards, card => card.Name == "Lightning Bolt" && card.Place == "stack");
            Assert.Equal([1, 2], board.Players.Select(player => player.Seat).Order());
            Assert.True(board.Players.Single(player => player.Seat == 2).Turned);

            // A tap on the turned half picks what is under the finger there, and the tablet sends it and says nothing.
            await shared.TapCardAsync(goyf);
            TabletState tapped = await shared.SettledAsync(state => state.Taps.Count == 1);
            Assert.Equal(new TabletTap(goyf.Handle, 0, tapped.Taps[0].Sequence), tapped.Taps[0]);

            await shared.TapCardAsync(mountain);
            tapped = await shared.SettledAsync(state => state.Taps.Count == 2);
            Assert.Equal(mountain.Handle, tapped.Taps[1].Object);

            await shared.TapPlayerAsync(2);
            tapped = await shared.SettledAsync(state => state.Taps.Count == 3);
            Assert.Equal((0, 2), (tapped.Taps[2].Object, tapped.Taps[2].Player));

            // The shared board's band shows the stack once facing each player; either Bolt is the one spell.
            TabletCard[] bolts = [.. tapped.Cards.Where(card => card.Name == "Lightning Bolt")];
            Assert.Equal(["opponent-stack", "stack"], bolts.Select(card => card.Place).Order());
            Assert.Single(bolts.Select(card => card.Handle).Distinct());
            TabletCard bolt = bolts.Single(card => card.Place == "opponent-stack");
            await shared.TapCardAsync(bolt);
            tapped = await shared.SettledAsync(state => state.Taps.Count == 4);
            Assert.Equal(bolt.Handle, tapped.Taps[3].Object);

            // A long press holds the card up to read, turned to face the player across from it, and taps nothing.
            await shared.PressAsync(goyf);
            await shared.SettledAsync(state => state.Held > 0);
            await shared.PictureAsync("held-up");
            await shared.LiftAsync();
            TabletState after = await shared.SettledAsync(state => state.Held == 0);
            Assert.Equal(4, after.Taps.Count);
            Assert.Empty(shared.Errors);
        }

        // A player's own board: that seat's half, the stack, and both life totals.
        foreach (int side in (int[])[1, 2])
        {
            await using TabletDriver own = await TabletDriver.OpenAsync(web, $"{query}&side={side}", pictures, $"side-{side}");
            await own.FeedAsync(new DeviceWelcomeMessage { Role = DeviceRole.Tablet, Side = side, PlayerNames = ["Bot1", "Bot2"] });
            TabletState board = await FeedGameAsync(own, feed);
            await own.PictureAsync("board");

            Assert.Equal($"side {side}", board.Layout);
            Assert.All(board.Cards.Where(card => card.Place != "stack"), card => Assert.Equal((side, "battlefield"), (card.Seat, card.Place)));
            Assert.Contains(board.Cards, card => card.Name == "Lightning Bolt" && card.Place == "stack");
            Assert.Equal([side], board.Players.Select(player => player.Seat));
            Assert.Contains("Bot1  20", board.Lives, StringComparison.Ordinal);
            Assert.Contains("Bot2  20", board.Lives, StringComparison.Ordinal);

            TabletCard mine = board.Cards.First(card => card.Name == (side == 1 ? "Mountain" : "Tarmogoyf"));
            await own.TapCardAsync(mine);
            TabletState tapped = await own.SettledAsync(state => state.Taps.Count == 1);
            Assert.Equal(mine.Handle, tapped.Taps[0].Object);
            Assert.Empty(own.Errors);
        }
    }

    /// <summary>
    /// The whole table, end to end: Bot1's phone and one shared tablet at a relay, beside a table playing the Bolt-the-Goyf
    /// scenario. Bolt is tapped in the phone's hand, the Mountain and the Goyf with a finger on the tablet - the Goyf on
    /// seat 2's turned half - and each tap goes through the relay to the phone, which answers the table; the table judges
    /// every answer against the script. No tablet ever names a card in the phone's hand.
    /// </summary>
    [Fact]
    public async Task Bolt_is_aimed_from_a_phone_and_a_shared_tablet_through_the_relay()
    {
        await using var table = new InPersonTable(web, "shared", 0);
        string? failure = await ScenarioPlayer.PlayAsync(web, ScenarioPath(Scenario), atTable: table);
        Assert.True(failure is null, $"{Scenario}:\n{failure}");
        await table.Tablets[0].PictureAsync("over");
    }

    /// <summary>
    /// The same with a tablet for each player: the Mountain tapped on Bot1's own board, the Goyf on Bot2's.
    /// </summary>
    [Fact]
    public async Task Bolt_is_aimed_from_a_phone_and_each_players_own_tablet_through_the_relay()
    {
        await using var table = new InPersonTable(web, "own-boards", 1, 2);
        string? failure = await ScenarioPlayer.PlayAsync(web, ScenarioPath(Scenario), atTable: table);
        Assert.True(failure is null, $"{Scenario}:\n{failure}");
        await table.Tablets[0].PictureAsync("over");
        await table.Tablets[1].PictureAsync("over");
    }

    /// <summary>
    /// A target in a graveyard, tapped on the shared tablet: Reanimate cast from the phone at the Goyf just Bolted, Bot2's
    /// graveyard pile tapped on the turned half to open its list, and the Goyf tapped in it.
    /// </summary>
    [Fact]
    public async Task A_graveyard_target_is_tapped_in_the_list_its_pile_opens_on_the_tablet()
    {
        const string reanimate = "in-person-reanimate-aimed-from-a-tablets-graveyard-pile-takes-the-bolted-tarmogoyf";
        await using var table = new InPersonTable(web, "graveyard", 0);
        string? failure = await ScenarioPlayer.PlayAsync(web, ScenarioPath(reanimate), atTable: table);
        Assert.True(failure is null, $"{reanimate}:\n{failure}");
        await table.Tablets[0].PictureAsync("over");
    }

    private static string ScenarioPath(string name) => Path.Combine(WebClient.Root, "scenarios", name + ".cantrip");

    /// <summary>Hands a tablet the game as the public feed would, and waits for Lightning Bolt on its stack.</summary>
    private static async Task<TabletState> FeedGameAsync(TabletDriver tablet, IReadOnlyList<ServerMessage> feed)
    {
        foreach (ServerMessage message in feed)
        {
            await tablet.FeedAsync(new PublicFeedMessage { Message = message });
        }

        return await tablet.SettledAsync(state => state.Screen == "board" && state.Cards.Any(card => card.Name == "Lightning Bolt" && card.Place == "stack"));
    }

    /// <summary>
    /// Plays the scenario at a real table, seat 2 answered as the script means, to Lightning Bolt on the stack aimed at
    /// Tarmogoyf; and returns what seat 2 was sent, as the public feed carries it: the welcome for seat zero, with no
    /// decklist, then the states and events. Seat 2's hand is in those views, but a tablet draws no hand.
    /// </summary>
    private static async Task<IReadOnlyList<ServerMessage>> BoltOnTheStackAsync()
    {
        ScenarioSpec scenario = ScenarioSpec.Load(Path.Combine(WebClient.Root, "scenarios", Scenario + ".cantrip"), [2]);
        await using ScenarioTable table = await ScenarioTable.StartAsync(scenario, WebClient.Content);
        var recording = new RecordingTransport(await WebSocketTransport.ConnectAsync(table.Address));
        var bot2 = new GameClient(recording);
        bot2.Join("Bot2", 2);

        try
        {
            while (true)
            {
                ScenarioProgress progress = await table.NextAsync(Patience);
                Assert.True(progress.Verdict is null, "The scenario ended before Lightning Bolt was on the stack.");
                if (progress.Expected is not { Seat: 2 } asked)
                {
                    continue;
                }

                DateTime giveUp = DateTime.UtcNow + Patience;
                while (bot2.Prompt?.RequestId != asked.RequestId)
                {
                    Assert.True(DateTime.UtcNow < giveUp, $"Bot2 was never sent request {asked.RequestId}.");
                    if (bot2.Pump() == 0)
                    {
                        await Task.Delay(5);
                    }
                }

                if (bot2.Zone(ZoneKindView.Stack, 0).Any(card => card.Name == "Lightning Bolt"))
                {
                    break;
                }

                bot2.Answer(asked.Answer ?? throw new Xunit.Sdk.XunitException($"Bot2 is not meant to undo ({asked.Line})."));
            }
        }
        finally
        {
            bot2.Disconnect();
        }

        return [.. recording.Received.Select(message => message is WelcomeMessage welcome
                ? new WelcomeMessage { Seat = 0, PlayerNames = welcome.PlayerNames }
                : message)
            .Where(message => message is WelcomeMessage or StateMessage or EventsMessage or GameOverMessage)];
    }

    /// <summary>A transport that keeps everything it receives.</summary>
    private sealed class RecordingTransport(IGameTransport inner) : IGameTransport
    {
        private readonly List<ServerMessage> _received = [];

        public IReadOnlyList<ServerMessage> Received => _received;

        public bool IsConnected => inner.IsConnected;

        public void Send(ClientMessage message) => inner.Send(message);

        public IReadOnlyList<ServerMessage> Receive()
        {
            IReadOnlyList<ServerMessage> received = inner.Receive();
            _received.AddRange(received);
            return received;
        }

        public void Dispose() => inner.Dispose();
    }
}
