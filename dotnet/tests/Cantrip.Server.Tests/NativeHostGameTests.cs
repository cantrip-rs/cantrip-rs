using System.Text.Json;
using Cantrip.Client;
using Cantrip.Client.Embedded;
using Cantrip.Core;
using Cantrip.Protocol;
using Cantrip.Server.Rules;
using Cantrip.TestKit;
using Xunit;
using Xunit.Abstractions;

namespace Cantrip.Server.Tests;

/// <summary>
/// A whole game through the real <c>cantrip-native-host</c>: the binary the crate builds, spawned by
/// <see cref="NativeHostProcess"/>, dealt two decks of real cards by name, and played to the end by two
/// <see cref="RandomPlay"/> seats over a <see cref="GameSession"/>'s mailboxes, as <c>HostAndPlay</c> and the
/// Playtest tool play it.
/// <para>
/// On the wall clock because it is a process: a sidecar that loads the native engine and answers over a pipe. It
/// skips, saying so, when the binary is not there, since it is not built by the solution
/// (<c>cargo build --profile cards -p cantrip-host</c> at the repository root).
/// </para>
/// </summary>
[Trait("Category", "WallClock")]
public sealed class NativeHostGameTests
{
    /// <summary>
    /// The codes the protocol allows a host to refuse an answer with (<c>cantrip-host</c>). Anything else in
    /// a <c>Rejected</c> reply, or no reply at all, is the host breaking its protocol rather than refusing a move.
    /// </summary>
    private static readonly HashSet<string> ExpectedRejections = ["IllegalAnswer", "StaleReply", "UnknownHandle", "NotYourTurnToAnswer"];

    /// <summary>Answers one game may take before it is called stuck; a long random game is a few thousand.</summary>
    private const int AnswerLimit = 50_000;

    /// <summary>Passes with nobody asked anything before the game is called stuck rather than merely quiet.</summary>
    private const int IdlePasses = 4;

    private readonly ITestOutputHelper _output;

    public NativeHostGameTests(ITestOutputHelper output)
    {
        _output = output;
    }

    [Fact]
    public void Two_random_seats_play_a_whole_game_through_the_real_host()
    {
        string binary = NativeHostProcess.Locate();
        if (!File.Exists(binary))
        {
            _output.WriteLine(
                $"Skipped: no cantrip-native-host at {binary}. Build it with `cargo build --profile cards -p cantrip-host` in " +
                "the repository root, or point CANTRIP_NATIVE_HOST at one.");
            return;
        }


        List<string> stderr = [];
        using var process = NativeHostProcess.Start(binary, log: line =>
        {
            lock (stderr)
            {
                stderr.Add(line);
            }

            _output.WriteLine($"[cantrip-native-host] {line}");
        });

        var transport = new RecordingTransport(process);

        // Real cards the engine has, by name: a burn deck with elves to block with, against engineers.
        RulesSeat[] seats =
        [
            new("Alice", [.. Copies("Mountain", 20), .. Copies("Lightning Bolt", 10), .. Copies("Llanowar Elves", 10)]),
            new("Bob", [.. Copies("Forest", 20), .. Copies("Goblin Engineer", 20)]),
        ];

        using var session = new GameSession(NativeRulesHost.NewGame(transport, seats, seed: 20261009, new RulesGameOptions { MinimumDeckSize = 60 }));

        List<(GameClient Client, RandomPlay Bot)> players = [];
        foreach (PlayerId seat in session.Seats)
        {
            var client = new GameClient(new LoopbackTransport(session, seat));
            client.Join(seats[seat.Value - 1].Name);
            players.Add((client, new RandomPlay(seed: 100 + seat.Value)));
        }

        session.Start();

        int answers = 0;
        int idle = 0;
        int events = 0;
        string? stuck = null;

        while (true)
        {
            // Every seat drains its mailbox before anyone is asked whether it is over: an answer runs the session
            // inside the send, so the message that ends the game lands in one seat's mailbox while another has not
            // looked yet.
            foreach ((GameClient client, RandomPlay _) in players)
            {
                client.Pump();
                events += client.RecentEvents.Count;
            }

            if (players.TrueForAll(player => player.Client.IsOver))
            {
                break;
            }

            bool moved = false;
            foreach ((GameClient client, RandomPlay bot) in players)
            {
                if (!bot.Act(client))
                {
                    continue;
                }

                moved = true;
                if (++answers > AnswerLimit)
                {
                    stuck = $"no end after {AnswerLimit} answers";
                    break;
                }
            }

            if (stuck is not null)
            {
                break;
            }

            // Nobody asked anything of anybody and the game is not over: one pass of that is ordinary, several in a
            // row means the game waits on something that will never arrive.
            idle = moved ? 0 : idle + 1;
            if (idle > IdlePasses)
            {
                stuck = $"every seat idle for {IdlePasses} passes";
                break;
            }
        }

        string asked = string.Join(", ", players.Select(player =>
            $"seat {player.Client.Seat}: {player.Client.Prompt?.Prompt?.GetType().Name ?? "idle"}"));
        string refusals = transport.Rejections.Count == 0
            ? "no answer was refused"
            : $"{transport.Rejections.Count} refusal(s):\n{string.Join("\n", transport.Rejections.Select(pair => $"  request: {pair.Request}\n  reply:   {pair.Reply}"))}";

        _output.WriteLine($"{answers} answers, {events} events, turn {players[0].Client.View?.Turn}, {refusals}");

        // The host kept its protocol: every refusal was one of the codes the protocol names, with the question
        // still outstanding, and it never crashed or answered with something that is neither Ok nor Rejected.
        foreach ((string request, string reply, string code) in transport.Rejections)
        {
            Assert.True(ExpectedRejections.Contains(code),
                $"The host refused an answer with {code}, which the protocol does not allow.\n  request: {request}\n  reply:   {reply}");
        }

        Assert.True(stuck is null, $"The game did not end: {stuck} (turn {players[0].Client.View?.Turn}, {asked}).\n{refusals}");

        // These decks carry no rule a prompt cannot state — no creature that must block, no attack restriction, no
        // cost the prompt does not list — so every answer a random seat built from what it was offered is legal, and
        // a refusal of one is the translation refusing a legal move: a bug on the Rust side, never worked around
        // here. The exact exchange is in the message, which is what the fix is written from.
        Assert.True(transport.Rejections.Count == 0, $"The host refused answers a random seat legitimately sent:\n{refusals}");

        foreach ((GameClient client, RandomPlay _) in players)
        {
            GameOverMessage over = Assert.IsType<GameOverMessage>(client.Outcome);
            Assert.True(over.Outcome is GameOutcomeView.TeamWon or GameOutcomeView.Draw, $"Seat {client.Seat} was told the game ended {over.Outcome}.");

            // The log and the events arrived: lines written for this seat, and events numbered by the host.
            Assert.NotEmpty(client.Log);
            Assert.True(client.EventSequence > 0, $"Seat {client.Seat} was brought to event {client.EventSequence}.");
        }

        Assert.True(events > 0, "No event reached any seat.");
        Assert.True(answers > 0, "No seat was ever asked anything.");
        Assert.Contains(stderr, line => line.Contains("cards loaded", StringComparison.Ordinal));
        Assert.DoesNotContain(stderr, line => line.Contains("panicked", StringComparison.Ordinal));
    }

    /// <summary>
    /// The decks the tools and the web client's tests deal by default are made of cards the native engine has, so a table
    /// is never refused at the deal. A card ported out of one of them is a deck to fix, not a test to skip.
    /// </summary>
    [Fact]
    public void The_demo_and_spike_decks_deal_in_the_real_host()
    {
        string binary = NativeHostProcess.Locate();
        if (!File.Exists(binary))
        {
            _output.WriteLine($"Skipped: no cantrip-native-host at {binary}.");
            return;
        }

        using var process = NativeHostProcess.Start(binary);

        IEnumerable<string> paths = Directory.GetFiles(RepoRoot.DemoDecks, "*.txt")
            .Where(path => !path.EndsWith("card-test.txt", StringComparison.Ordinal))
            .Concat(Directory.GetFiles(Path.Combine(RepoRoot.Path, "decks", "spike"), "*.txt"));

        foreach (string path in paths)
        {
            DeckList deck = DeckList.Load(path);
            List<string> cards = [.. deck.Entries.SelectMany(entry => Enumerable.Repeat(entry.Card, entry.Copies))];

            using IRulesHost game = NativeRulesHost.NewGame(process, [new RulesSeat("A", cards), new RulesSeat("B", cards)], 1, new RulesGameOptions { MinimumDeckSize = 60 });
            Assert.Equal(2, game.Seats.Count);
        }
    }

    /// <summary>
    /// A game dealt with a public view (<see cref="NativeRulesHostFactory.PublicView"/>) through the real host, played to the
    /// end by seats that play a land when they may and otherwise give the simplest answer: every step carries the public part,
    /// a view that is no seat's and shows no hand or library, each object stamped; its events are numbered as the seats' are;
    /// and each pair of its handle map names, for that seat, the same card in the same zone.
    /// </summary>
    [Fact]
    public void A_public_view_comes_with_every_step_of_a_whole_game_through_the_real_host()
    {
        string binary = NativeHostProcess.Locate();
        if (!File.Exists(binary))
        {
            _output.WriteLine($"Skipped: no cantrip-native-host at {binary}.");
            return;
        }

        using var process = NativeHostProcess.Start(binary);
        RulesSeat[] seats =
        [
            new("Alice", [.. Copies("Mountain", 20), .. Copies("Lightning Bolt", 10), .. Copies("Llanowar Elves", 10)]),
            new("Bob", [.. Copies("Forest", 20), .. Copies("Goblin Engineer", 20)]),
        ];
        using IRulesHost game = new NativeRulesHostFactory(process) { PublicView = true }
            .Create(seats, 20261030, new RulesGameOptions { MinimumDeckSize = 40, FirstPlayer = 1 });

        RulesStep step = game.Start();
        int steps = 0;
        int paired = 0;
        while (true)
        {
            PublicProjection shown = Assert.IsType<PublicProjection>(step.Public);
            Assert.Equal(0, shown.View.Viewer);
            Assert.All(shown.View.Zones.Where(zone => zone.Kind is ZoneKindView.Hand or ZoneKindView.Library), zone => Assert.Empty(zone.Contents));
            Assert.All(shown.View.Objects, item => Assert.NotEqual(0, item.Fingerprint));
            HashSet<long> told = [.. step.Events.Values.SelectMany(events => events.Select(e => e.Sequence))];
            Assert.All(shown.Events, e => Assert.Contains(e.Sequence, told));
            foreach ((int seat, IReadOnlyDictionary<ViewHandle, ViewHandle> map) in shown.Handles)
            {
                foreach ((ViewHandle mine, ViewHandle theirs) in map)
                {
                    if (shown.View.Objects.FirstOrDefault(o => o.Handle == mine) is { } a
                        && step.Views[seat].Objects.FirstOrDefault(o => o.Handle == theirs) is { } b)
                    {
                        Assert.Equal((a.Name, a.Zone, a.Controller), (b.Name, b.Zone, b.Controller));
                        paired++;
                    }
                }
            }

            if (step.Finished)
            {
                break;
            }

            Assert.True(++steps < 20_000, "The game did not end.");
            RulesRequest request = step.Requests[0];
            AnswerPayload answer = request.Prompt is PriorityPromptMessage priority
                && priority.Options.FirstOrDefault(option => option.Kind == ActionKindView.PlayLand) is { } land
                ? TakeActionAnswer.Take(land)
                : request.Prompt.Default();
            step = game.Answer(request.RequestId, request.Seat, answer);
        }

        Assert.True(paired > 0, "No public object was ever paired with a seat's.");
    }

    private static IEnumerable<string> Copies(string card, int count) => Enumerable.Repeat(card, count);

    /// <summary>
    /// The real sidecar, with every refusal it sends kept beside the request that drew it, and a host that stops
    /// replying named with the request it stopped on.
    /// </summary>
    private sealed class RecordingTransport : IHostTransport
    {
        private readonly IHostTransport _inner;

        public RecordingTransport(IHostTransport inner)
        {
            _inner = inner;
        }

        public List<(string Request, string Reply, string Code)> Rejections { get; } = [];

        public int Exchanges { get; private set; }

        public string Exchange(string request)
        {
            Exchanges++;
            string reply;
            try
            {
                reply = _inner.Exchange(request);
            }
            catch (InvalidOperationException error)
            {
                throw new InvalidOperationException($"{error.Message} The request was: {request}", error);
            }

            using JsonDocument document = JsonDocument.Parse(reply);
            if (document.RootElement.ValueKind == JsonValueKind.Object
                && document.RootElement.TryGetProperty("Rejected", out JsonElement rejected))
            {
                string code = rejected.TryGetProperty("Code", out JsonElement element) ? element.GetString() ?? string.Empty : string.Empty;
                Rejections.Add((request, reply, code));
            }

            return reply;
        }
    }
}
