using System.Runtime.InteropServices;
using Cantrip.Client;
using Cantrip.Client.Embedded;
using Cantrip.Client.Tactics;
using Cantrip.Protocol;
using Cantrip.Server;
using Cantrip.Server.Rules;

namespace Cantrip.BotLab;

/// <summary>One position a seat was in, as the model sees it, and whether that seat went on to win.</summary>
/// <param name="Game">Which game of the run it came from, so that a game's positions are kept together.</param>
public sealed record Sample(Dictionary<string, double> Features, bool Won, int Game = 0);

/// <summary>How one game between two seats came out.</summary>
/// <param name="Winner">The seat that won, zero for a draw, -1 for a game that did not finish.</param>
/// <param name="Starter">The seat that took the first turn.</param>
/// <param name="Failure">What went wrong, or empty.</param>
public sealed record MatchResult(int Winner, int Starter, int Turns, string Failure, IReadOnlyList<Sample> Samples)
{
    public bool Finished => Failure.Length == 0;

    /// <summary>The game as seat 1 was told it, a line per event, with the board at the start of each turn. Only when asked for.</summary>
    public IReadOnlyList<string> Story { get; init; } = [];

    /// <summary>The longest each seat took over one answer, in milliseconds, seat 1 first.</summary>
    public IReadOnlyList<double> Slowest { get; init; } = [];

    /// <summary>How many questions each seat answered, seat 1 first.</summary>
    public IReadOnlyList<int> Answers { get; init; } = [];

    /// <summary>How long each seat spent answering in all, in milliseconds, seat 1 first.</summary>
    public IReadOnlyList<double> Thinking { get; init; } = [];

    /// <summary>The names of the cards each seat held at some point in the game, seat 1 first. Only when asked for.</summary>
    public IReadOnlyList<IReadOnlySet<string>> Held { get; init; } = [];

    /// <summary>
    /// Each answer of each seat, seat 1 first: how long it took in all and how much of that was the native host's
    /// (the game's own work, which runs inside the send). Only for a game played on a <see cref="Lane"/>.
    /// </summary>
    public IReadOnlyList<IReadOnlyList<(double Act, double Host)>> Times { get; init; } = [];
}

/// <summary>
/// Plays one game between two seats at the session layer: the real session and the real protocol, a real client per
/// seat, in this process over a loopback transport, as <c>cantrip-playtest</c> does. Each seat is whatever
/// <see cref="IAutoPlayer"/> it is given, which is the difference: this is for setting one way of playing against
/// another, and for collecting the positions a seat was in along the way.
/// </summary>
internal static class Match
{
    /// <summary>
    /// <c>--note-heuristic</c>: before a tactical seat answers, the recording host is told what its model makes of the
    /// position (<see cref="TacticalModel.Evaluate"/>, log-odds of winning for the seat), kept with the game's samples.
    /// </summary>
    public static bool NoteHeuristics { get; set; }

    /// <summary>A factory that hands on what it makes, keeping the last: how a game's host, and its id, is found.</summary>
    private sealed class CatchingFactory(IRulesHostFactory inner) : IRulesHostFactory
    {
        public IRulesHost? Made { get; private set; }

        public IRulesHost Create(IReadOnlyList<RulesSeat> seats, ulong seed, RulesGameOptions options) => Made = inner.Create(seats, seed, options);

        public bool CompanionFulfilled(IReadOnlyList<string> deck, string companion, int minimumDeckSize) =>
            inner.CompanionFulfilled(deck, companion, minimumDeckSize);
    }

    /// <summary>
    /// The native hosts games are dealt among (<c>--hosts N</c>), a game to each by its shuffle; empty for the one shared host.
    /// A host answers one request at a time, so on a machine with cores to spare one host is what a run waits on.
    /// </summary>
    public static IReadOnlyList<IRulesHostFactory> Hosts { get; set; } = [];

    private const int AnswerLimit = 200_000;

    /// <summary>
    /// What the game being watched was last doing: which seat was answering what, on which turn. For a game that
    /// never comes back at all, where there is no failure to read and this is the only thing that says where it is.
    /// </summary>
    public static volatile string Doing = string.Empty;

    /// <summary>The question that seat was answering, with <see cref="Doing"/>.</summary>
    public static volatile PromptPayload? Asked;
    private const int IdlePasses = 4;

    /// <summary>
    /// How much processor time one game may take before it is called stuck. A seat that has found a loop the rules
    /// allow - the deterministic one sacrificing Dauthi Voidwalker to cast the Corpse Dance that brings it back - never
    /// reaches the answer limit in any useful time, because each turn of the loop costs the game more than the last.
    /// Processor time, not time on the clock: a game is played on one thread, and a machine busy with other games
    /// makes its clock run long without the game doing any more (a slow card under load was once called a hang).
    /// </summary>
    private static readonly TimeSpan TimeLimit = TimeSpan.FromMinutes(3);

    [DllImport("kernel32.dll")]
    private static extern IntPtr GetCurrentThread();

    [DllImport("kernel32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool GetThreadTimes(IntPtr thread, out long creation, out long exit, out long kernel, out long user);

    /// <summary>The processor time this thread has used, where the system says (Windows); otherwise null.</summary>
    private static TimeSpan? ThreadTime() =>
        OperatingSystem.IsWindows() && GetThreadTimes(GetCurrentThread(), out _, out _, out long kernel, out long user)
            ? TimeSpan.FromTicks(kernel + user)
            : null;

    /// <param name="decks">The deck each seat plays, seat 1 first.</param>
    /// <param name="players">How each seat is played, seat 1 first.</param>
    /// <param name="sample">Whether to record each seat's position once a turn, for fitting the model to.</param>
    /// <param name="story">Whether to keep the game's log, for reading what a seat did.</param>
    /// <param name="limit">Answers the game may take before it is called stuck, or zero for the usual generous number.</param>
    /// <param name="hands">Whether to record which cards each seat held, for saying which of them a seat wins with.</param>
    /// <param name="lane">The worker's own native host and AI host, for a game with a neural seat; else the shared host.</param>
    public static MatchResult Play(
        string contentRoot, IReadOnlyList<DeckList> decks, Cantrip.Decks.DeckFormat format, ulong seed, IReadOnlyList<IAutoPlayer> players, bool sample, bool story = false, int limit = 0,
        bool hands = false, Lane? lane = null)
    {
        try
        {
            return Game(contentRoot, decks, format, seed, players, sample, story, limit > 0 ? limit : AnswerLimit, hands, lane);
        }
        catch (Exception error)
        {
            return new MatchResult(-1, 0, 0, $"{error.GetType().Name}: {error.Message.ReplaceLineEndings(" ")} (shuffle {seed})", []);
        }
    }

    private static MatchResult Game(
        string contentRoot, IReadOnlyList<DeckList> decks, Cantrip.Decks.DeckFormat format, ulong seed, IReadOnlyList<IAutoPlayer> players, bool sample, bool story, int limit, bool hands, Lane? lane)
    {
        foreach (NeuralPlay neural in players.OfType<NeuralPlay>())
        {
            neural.Lane = lane;
        }

        IRulesHostFactory rules = lane?.Rules ?? (Hosts.Count > 0 ? Hosts[(int)(seed % (ulong)Hosts.Count)] : RulesHostFactory.Native);
        // Noting the tactical model's worth of each position a tactical seat answers (--note-heuristic): the game's host
        // is caught as it is made, for its id.
        var noting = NoteHeuristics && lane is null ? new CatchingFactory(rules) : null;
        using HostedGame hosted = HostAndPlay.StartTable(noting ?? rules, decks, format, seed, [.. decks.Select((_, seat) => $"Bot {seat + 1}")]);

        List<(GameClient Client, IAutoPlayer Bot)> seats = [];
        foreach (int seat in hosted.Seats)
        {
            // A seat that plays through a client of its own (a champion) is shown its transport first.
            IGameTransport transport = hosted.TransportFor(seat);
            var client = new GameClient(players[seat - 1] is ISeatTransport own ? own.Wrap(transport) : transport);
            client.Join($"Bot {seat}");
            seats.Add((client, new LoopGuard(players[seat - 1])));
        }

        hosted.Start();

        // What each seat saw, a position a turn, to be marked won or lost once the game says which.
        List<(int Seat, Dictionary<string, double> Features)> seen = [];
        int[] sampled = new int[seats.Count + 1];
        Dictionary<int, string> boards = [];

        var clock = System.Diagnostics.Stopwatch.StartNew();
        TimeSpan? cpuAtStart = ThreadTime();

        // The slowest single answer, and what it answered: what a game called stuck was doing when it was.
        double slowestAnswer = 0;
        string slowestWhat = "none";
        string lastAnswered = "none";
        double[] slowest = new double[seats.Count];
        double[] thinking = new double[seats.Count];
        int[] answered = new int[seats.Count];
        List<(double Act, double Host)>[] times = [.. seats.Select(_ => new List<(double Act, double Host)>())];
        // Where the system does not say (Linux), the clock less a neural seat's own part of its answers (not the native
        // host's): that is worked out in the AI host's process, which this thread's processor time never counts, and a
        // searching seat spends minutes of it in a long game.
        bool[] elsewhere = [.. seats.Select(seat => seat.Bot is NeuralPlay)];
        TimeSpan Spent() => ThreadTime() is { } now && cpuAtStart is { } then ? now - then
            : clock.Elapsed - TimeSpan.FromMilliseconds(times.Where((_, i) => elsewhere[i]).SelectMany(t => t).Sum(t => t.Act - t.Host));
        var watch = new System.Diagnostics.Stopwatch();
        HashSet<string>[] held = [.. seats.Select(_ => new HashSet<string>(StringComparer.Ordinal))];

        int answers = 0;
        int[] refusals = new int[seats.Count];
        int idle = 0;
        int starter = 0;

        // A game that goes wrong is the one worth reading, so whatever stops it, what had happened is kept.
        MatchResult Stopped(string why) => Failed(seats, $"{why}; slowest answer {slowestAnswer / 1000:N1}s ({slowestWhat}); last answered {lastAnswered}; shuffle {seed}") with
        {
            Story = story ? [.. Told(seats[0].Client, boards), string.Empty, $"Last asked: {Doing}"] : [],
        };

        while (true)
        {
            // The game's clocks, as a table's (docs/events.md, "Clocks"): a seat out of time loses, a match out of time is a draw.
            hosted.Tick();
            try
            {
                foreach ((GameClient client, IAutoPlayer _) in seats)
                {
                    client.Pump();
                }
            }
            catch (Exception error) when (story)
            {
                return Stopped($"{error.GetType().Name}: {error.Message.ReplaceLineEndings(" ")}");
            }

            foreach ((GameClient client, IAutoPlayer _) in seats)
            {

                if (starter == 0 && client.View is { Turn: 1 } opening)
                {
                    starter = opening.ActivePlayer;
                }

                // Once a turn, at the first thing the seat is asked in a main phase: both hands are drawn and
                // nothing of the turn has happened yet, which is where the model is asked what a board is worth.
                if (sample && client.View is { } view && client.Seat > 0 && view.Turn > sampled[client.Seat]
                    && view.Step == TurnStepView.PrecombatMain)
                {
                    sampled[client.Seat] = view.Turn;
                    seen.Add((client.Seat, TacticalModel.Features(view)));
                }

                if (story && client.Seat == 1 && client.View is { Step: TurnStepView.PrecombatMain } now && !boards.ContainsKey(now.Turn))
                {
                    boards[now.Turn] = Board(client, now);
                }
            }

            if (seats.TrueForAll(seat => seat.Client.IsOver))
            {
                // A seat telling its game is told the end, which no question of its own followed.
                foreach ((GameClient client, IAutoPlayer bot) in seats)
                {
                    if ((bot as LoopGuard)?.Inner is TacticalPlay { Narrator: not null } narrator)
                    {
                        narrator.NarrateEnd(client);
                    }
                }

                break;
            }

            bool moved = false;

            for (int at = 0; at < seats.Count; at++)
            {
                (GameClient client, IAutoPlayer bot) = seats[at];

                // The session runs inside the send, so this is the decision and whatever the game then did with it:
                // a seat that decides nothing, as the random one, is the measure of the second part.
                if (story && client.Prompt is { } asking)
                {
                    Asked = asking.Prompt;
                    Doing = $"seat {client.Seat} ({bot.GetType().Name}) answering {asking.Prompt.GetType().Name} on turn {client.View?.Turn}, {client.View?.Step}, after {answers} answers"
                        + (asking.Prompt is PayCostsPromptMessage pay
                            ? $": paying for {client.Object(pay.Spell)?.Name}, total {pay.Total}, remaining {pay.Remaining}, X {pay.X} (has X: {pay.HasX}, announcing: {pay.AnnouncesXOrKicks}, kicker '{pay.Kicker}'), options {string.Join(" ", pay.Options.Select(option => option.Kind + (option.Produces.Count > 0 ? "/" + string.Join("", option.Produces) : string.Empty)))}"
                            : asking.Prompt is UnlessPaysPromptMessage unless
                            ? $": {unless.Verb} for {client.Object(unless.Source)?.Name} on {client.Object(unless.Subject)?.Name}, total {unless.Total}, remaining {unless.Remaining}, options {string.Join(" ", unless.Options.Select(option => option.Kind + (option.Produces.Count > 0 ? "/" + string.Join("", option.Produces) : string.Empty)))}"
                            : string.Empty);
                }

                // What a seat holds whenever it is asked something is every card it ever had the chance to play.
                if (hands && client.Prompt is not null)
                {
                    foreach (ObjectView card in client.MyHand())
                    {
                        held[at].Add(card.Name);
                    }
                }

                string answering = client.Prompt?.Prompt is { } prompt
                    ? $"seat {client.Seat} {prompt.GetType().Name.Replace("PromptMessage", string.Empty, StringComparison.Ordinal)} on turn {client.View?.Turn} {client.View?.Step}"
                    : string.Empty;
                double hostBefore = lane?.Tee.HostMs ?? 0;
                watch.Restart();
                bool acted;
                try
                {
                    if (noting?.Made is NativeRulesHost made && client.Prompt is { } noted && client.View is { } position
                        && (bot as LoopGuard)?.Inner is TacticalPlay)
                    {
                        // The recording host keeps it beside the sample of this request (cantrip_ai::record, Dealt::heuristics).
                        NativeHostProcess.Shared.Exchange(System.Text.Json.JsonSerializer.Serialize(new
                        {
                            Cmd = "Note",
                            made.GameId,
                            noted.RequestId,
                            Heuristic = TacticalModel.Default.Evaluate(position),
                        }));
                    }

                    acted = bot.Act(client);
                }
                catch (Exception error) when (story)
                {
                    return Stopped($"{error.GetType().Name} answering {client.Prompt?.Prompt?.GetType().Name} for seat {client.Seat}: {error.Message.ReplaceLineEndings(" ")}");
                }

                double took = watch.Elapsed.TotalMilliseconds;

                if (!acted)
                {
                    continue;
                }

                moved = true;
                lastAnswered = answering;
                if (took > slowestAnswer)
                {
                    (slowestAnswer, slowestWhat) = (took, answering);
                }

                answered[at]++;
                if (lane is not null)
                {
                    times[at].Add((took, lane.Tee.HostMs - hostBefore));
                }

                thinking[at] += took;
                slowest[at] = Math.Max(slowest[at], took);

                if (++answers > limit)
                {
                    return Stopped($"no end after {limit} answers");
                }

                if (Spent() > TimeLimit)
                {
                    return Stopped($"no end after {TimeLimit.TotalMinutes:N0} minutes of processor time ({clock.Elapsed.TotalMinutes:N1} on the clock) and {answers} answers");
                }

                // A refusal reaches a seat on its next pump, so each seat's own are counted and laid at its own door.
                if (client.Errors.Count > refusals[at])
                {
                    refusals[at] = client.Errors.Count;
                    if (!bot.Retrying)
                    {
                        ErrorMessage error = client.Errors[^1];
                        return Stopped($"seat {client.Seat} could not answer {client.Prompt?.Prompt?.GetType().Name}: {error.Code}: {error.Detail.ReplaceLineEndings(" ")}");
                    }
                }
            }

            idle = moved ? 0 : idle + 1;
            if (idle > IdlePasses)
            {
                return Stopped("every seat idle, game unfinished");
            }
        }

        GameClient first = seats[0].Client;
        GameOverMessage outcome = first.Outcome!;
        int winner = outcome.Outcome == GameOutcomeView.Draw
            ? 0
            : first.View?.Players.FirstOrDefault(player => player.Team == outcome.WinningTeam)?.Seat ?? 0;

        List<string> told = story ? Told(first, boards) : [];

        return new MatchResult(
            winner,
            starter,
            first.View?.Turn ?? 0,
            string.Empty,
            winner == 0 ? [] : [.. seen.Select(position => new Sample(position.Features, position.Seat == winner))])
        {
            Story = told,
            Slowest = slowest,
            Answers = answered,
            Thinking = thinking,
            Held = held,
            Times = times,
        };
    }

    /// <summary>The game as one seat's log tells it, with the board at the start of each turn.</summary>
    private static List<string> Told(GameClient client, Dictionary<int, string> boards)
    {
        List<string> told = [];
        int turn = 0;

        foreach (LogEntryView entry in client.Log)
        {
            if (entry.Turn != turn)
            {
                turn = entry.Turn;
                told.Add(string.Empty);
                told.Add($"--- Turn {turn}, seat {entry.ActivePlayer} --- {boards.GetValueOrDefault(turn, string.Empty)}");
            }

            told.Add($"  [{entry.Step}] {entry.Text}");
        }

        return told;
    }

    /// <summary>Both sides of the table in a line each, and seat 1's hand, for reading a game back.</summary>
    private static string Board(GameClient client, PlayerView view)
    {
        static string Card(ObjectView card) => card.Power is { } power
            ? $"{card.Name} {power}/{card.Toughness}{(card.Tapped ? " (T)" : string.Empty)}"
            : card.Name;

        IEnumerable<string> sides = view.Players.Select(player =>
            $"{Environment.NewLine}      seat {player.Seat}: {player.Life} life, {player.HandCount} in hand, " +
            $"{client.BattlefieldOf(player.Seat).Count(card => card.CardTypes.Contains(CardTypeView.Land))} lands | " +
            string.Join(", ", client.BattlefieldOf(player.Seat).Where(card => !card.CardTypes.Contains(CardTypeView.Land)).Select(Card)));

        return string.Concat(sides) + $"{Environment.NewLine}      seat 1 holds: " + string.Join(", ", client.MyHand().Select(card => card.Name));
    }

    private static MatchResult Failed(List<(GameClient Client, IAutoPlayer Bot)> seats, string why)
    {
        // Who is being asked what as the game is stopped; a seat with nothing outstanding is "not asked", which says
        // nothing about whether it was stuck: that is the check's timing, not the game's.
        string asked = string.Join(", ", seats.Select(seat => $"seat {seat.Client.Seat}: {seat.Client.Prompt?.Prompt?.GetType().Name ?? "not asked"}"));
        return new MatchResult(-1, 0, seats[0].Client.View?.Turn ?? 0, $"{why} (turn {seats[0].Client.View?.Turn}, {asked})", []);
    }
}
