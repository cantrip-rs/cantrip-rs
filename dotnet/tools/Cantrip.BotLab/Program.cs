using Cantrip.Client.Embedded;
using System.Collections.Concurrent;
using System.Diagnostics;
using System.Globalization;
using Cantrip.Client;
using Cantrip.Client.Tactics;
using Cantrip.Server;
using Cantrip.Server.Rules;

namespace Cantrip.BotLab;

/// <summary>
/// Sets one way of playing a seat against another, and fits the tactical seat's model to games it plays itself.
/// <para>
/// <c>duel</c> answers "is this seat better than that one": many games at the session layer, the seats swapped
/// every other game, and a tally with its margin of error. <c>train</c> is how <c>tactical-model.txt</c> is made:
/// the tactical seat plays itself, every position is marked with who went on to win, the model is fitted to that,
/// and the fitted model then has to beat the one it came from before it is kept.
/// </para>
/// </summary>
public static class Program
{
    /// <summary>
    /// The format the given decks are played in (<c>--format</c>, <see cref="HostAndPlay.FormatFrom"/>): Freeform unless the
    /// command line names one. A draft's decks are Limited by what they are.
    /// </summary>
    public static Cantrip.Decks.DeckFormat Format { get; private set; } = Cantrip.Decks.DeckFormat.Freeform;

    public static int Main(string[] args)
    {
        ArgumentNullException.ThrowIfNull(args);

        if (args.Length == 0 || args.Contains("--help") || args.Contains("-h"))
        {
            Help();
            return args.Length == 0 ? 2 : 0;
        }

        string root = FindRoot();
        Parallelism = Math.Clamp(Number(args, "--parallel", Parallelism), 1, 64);
        try
        {
            Format = HostAndPlay.FormatFrom(args);
        }
        catch (ArgumentException error)
        {
            Console.Error.WriteLine(error.Message);
            return 2;
        }
        EachShuffle = args.Contains("--each-shuffle");
        Match.NoteHeuristics = args.Contains("--note-heuristic");
        ExploreDuel = Real(args, "--explore", 0);
        PairingsFile = Text(args, "--pairings", string.Empty);

        // --hosts N: games dealt among N native hosts of their own rather than all sent to the one shared host. Not with
        // --ai-record, whose recording is the shared host's.
        int hosts = Number(args, "--hosts", 1);
        if (hosts > 1 && !args.Contains("--ai-record"))
        {
            Match.Hosts = [.. Enumerable.Range(0, hosts).Select(_ => (IRulesHostFactory)new NativeRulesHostFactory(NativeHostProcess.Start()))];
        }
        Mirrors = args.Contains("--mirrors");
        Neural = NeuralOptions.From(root, args);

        // --versus DIR: the --a seat always plays a deck of --decks and the --b seat one of these, each pair of games
        // the next of each: how one kind of deck is played against another, with the pilot of one side changed.
        string versus = Text(args, "--versus", string.Empty);
        if (versus.Length > 0 && args[0] is "duel" or "watch")
        {
            Versus = Decks(root, ["--decks", versus]);
        }

        try
        {
            return args[0] switch
            {
                "duel" => Duel(root, args),
                "train" => Train(root, args),
                "watch" => Watch(root, args),
                "tune" => Tune(root, args),
                "read" => ReadLab.Run(root, args),
                "champion" => Champion.Run(root, args),
                "soak" => Soak.Run(root, args, Parallelism),
                "draft" => args.ElementAtOrDefault(1) switch
                {
                    "watch" => DraftLab.Watch(root, args),
                    "calibrate" => DraftLab.Calibrate(root, args, Parallelism),
                    "fit" => DraftLab.Fit(root, args),
                    "shape" => DraftLab.Shape(root, args),
                    "paired" => DraftLab.Paired(args),
                    _ => DraftLab.Draft(root, args, Parallelism),
                },
                _ => Unknown(args[0]),
            };
        }
        catch (Exception error) when (error is FileNotFoundException or DirectoryNotFoundException or ArgumentException)
        {
            Console.Error.WriteLine(error.Message);
            return 2;
        }
    }

    /// <summary>The decks the --b seat plays against the --a seat's, for <c>--versus</c>; empty for mirrors.</summary>
    private static List<DeckList> Versus { get; set; } = [];

    /// <summary>
    /// <c>--pairings FILE</c>: a matchup a line, <c>first:second</c> by deck name among <c>--decks</c> (as the generation
    /// driver writes them, by metagame share); each pair of games the next line, its decks swapping seats with the
    /// players.
    /// </summary>
    private static string PairingsFile { get; set; } = string.Empty;

    private static List<(DeckList, DeckList)> Pairings { get; set; } = [];

    /// <summary><c>--explore P</c> in a duel: a tactical seat's share of decisions drawn at random, so its games differ.</summary>
    private static double ExploreDuel { get; set; }

    /// <summary>
    /// <c>--each-shuffle</c>: every game its own shuffle, rather than each pair of games (seats swapped) one. Two seats
    /// that play alike, the tactical bot against itself, play the same game twice from one shuffle, so a recorded
    /// self-duel (<c>--ai-record</c>) takes this on its own: the data would otherwise be every game twice.
    /// </summary>
    private static bool EachShuffle { get; set; }

    /// <summary><c>--mirrors</c>: with two decks, half the games the matchup and a quarter each deck's mirror.</summary>
    private static bool Mirrors { get; set; }

    /// <summary>How a <c>neural</c> seat is played: the model, its backend, its deadline (<see cref="NeuralOptions"/>).</summary>
    private static NeuralOptions Neural { get; set; } = null!;

    /// <summary>Every answer the neural seats of a run made, with what it cost.</summary>
    private static ConcurrentBag<NeuralAnswer> NeuralAnswers { get; } = [];

    /// <summary>The shuffle game number <paramref name="game"/> of a run is dealt from.</summary>
    internal static ulong Shuffle(ulong seed, int game) => seed + (ulong)(EachShuffle ? game : game / 2);

    /// <summary>Games played at once: one per core unless <c>--parallel</c> says fewer.</summary>
    private static int Parallelism { get; set; } = Math.Clamp(Environment.ProcessorCount, 1, 32);

    private static int Unknown(string command)
    {
        Console.Error.WriteLine($"No command '{command}'. See --help.");
        return 2;
    }

    // --- duel ----------------------------------------------------------------------------------------------------------

    private static int Duel(string root, string[] args)
    {
        string a = Text(args, "--a", "tactical");
        string b = Text(args, "--b", "random");
        int games = Number(args, "--games", 400);
        ulong seed = (ulong)Number(args, "--seed", Random.Shared.Next(1, 1_000_000_000));
        List<DeckList> decks = Decks(root, args);

        Console.WriteLine($"{a} against {b}, {games} games, {(Versus.Count > 0 ? $"{a} with {decks.Count} decks against {b} with {Versus.Count}" : Named(decks))}, shuffles from {seed}.");
        Console.WriteLine("Seats swapped every other game, so each plays first as often as the other and each plays each deck.");

        // --cards N: which cards each seat won with, the N it did worst and best by.
        int cards = Number(args, "--cards", 0);

        // --ai-record DIR: the host records every game that ends as AI training shards (docs/ai-player-plan.md, "Data").
        bool recording = Record(root, args, a, b);
        if (recording && a == b)
        {
            EachShuffle = true;
        }

        if (recording)
        {
            NativeHostProcess.SharedArguments = [.. NativeHostProcess.SharedArguments, "--ai-record-seeds", $"{seed}..{Shuffle(seed, games - 1) + 1}"];
            Console.WriteLine(EachShuffle ? "Every game its own shuffle." : "Each pair of games one shuffle.");
        }

        var clock = Stopwatch.StartNew();
        // --sprt: stop once the sequential test has its verdict, --games the most it may take (docs/ai-player-plan.md, "Serving and measuring").
        bool sprt = args.Contains("--sprt");
        Tally.SprtH1 = Real(args, "--sprt-h1", Tally.SprtH1);
        if (!(Tally.SprtH1 > Tally.SprtH0 && Tally.SprtH1 < 1))
        {
            Console.Error.WriteLine($"--sprt-h1 is A's share under H1, above {Tally.SprtH0} and below 1.");
            return 2;
        }

        Tally tally = Play(root, decks, games, seed, Maker(root, a), Maker(root, b, b: true), sample: false, out _, hands: cards > 0, sprt: sprt);
        tally.Write(a, b, clock.Elapsed);
        if (sprt)
        {
            Console.WriteLine(tally.SprtLine(a, b));
        }

        tally.WriteSplits(a, b, NeuralAnswers);

        if (recording)
        {
            // The host writes out the games still being recorded once its input ends: wait for it.
            NativeHostProcess.CloseShared(TimeSpan.FromMinutes(10));
            Console.WriteLine($"Recorded into {Text(args, "--ai-record", string.Empty)} in {clock.Elapsed:hh\\:mm\\:ss}.");
        }

        if (cards > 0)
        {
            tally.WriteCards(a, b, cards, Number(args, "--least", 60));
        }

        return tally.Failures.IsEmpty ? 0 : 1;
    }

    /// <summary>
    /// <c>--ai-record DIR</c> (with <c>--ai-record-mode full|decklist|hidden</c>, <c>--ai-record-threads N</c>,
    /// <c>--ai-record-games N</c>, <c>--ai-record-mb N</c>): starts the shared host recording the games that end as
    /// training shards, its header naming the seats that played and the shuffles. False when not asked for.
    /// </summary>
    private static bool Record(string root, string[] args, string a, string b)
    {
        string directory = Text(args, "--ai-record", string.Empty);
        if (directory.Length == 0)
        {
            return false;
        }

        string model = Path.Combine(root, "dotnet", "src", "Cantrip.Client", "Tactics", "tactical-model.txt");
        string Producer(string seat) => seat == "tactical" && File.Exists(model) ? $"tactical:{Fnv1a(File.ReadAllBytes(model)):x16}" : seat;
        string producer = a == b ? Producer(a) : $"{Producer(a)} v {Producer(b)}";

        List<string> host = ["--ai-record", Path.GetFullPath(directory), "--ai-record-mode", Text(args, "--ai-record-mode", "full"),
            "--ai-record-producer", producer];
        foreach (string option in (string[])["--ai-record-threads", "--ai-record-games", "--ai-record-mb", "--ai-record-also", "--ai-record-distill",
            "--ai-record-distill-server", "--ai-record-distill-worlds"])
        {
            if (Text(args, option, string.Empty) is { Length: > 0 } value)
            {
                host.AddRange([option, value]);
            }
        }

        NativeHostProcess.SharedArguments = host;
        Console.WriteLine($"Recording games into {Path.GetFullPath(directory)} ({Text(args, "--ai-record-mode", "full")} information), as {producer}.");
        return true;
    }

    private static ulong Fnv1a(byte[] bytes)
    {
        ulong hash = 0xcbf29ce484222325;
        foreach (byte b in bytes)
        {
            hash = (hash ^ b) * 0x100000001b3;
        }

        return hash;
    }

    // --- tune ----------------------------------------------------------------------------------------------------------

    /// <summary>
    /// Moves every weight towards winning, by playing: simultaneous-perturbation stochastic approximation, as chess
    /// engines are tuned.
    /// <para>
    /// Each step nudges every weight at once, half of them up and half down, plays the model nudged one way against
    /// the model nudged the other, and moves the weights a little towards whichever won more. No one step says much —
    /// a few hundred games cannot tell a one-percent edge from luck — but the luck averages out over hundreds of
    /// steps and the edge does not. Unlike the fit, this needs no notion of what a position is worth: it asks only
    /// which of two seats wins, so it reaches the policy weights the fit cannot, and it corrects a weight that
    /// predicts well and plays badly.
    /// </para>
    /// </summary>
    private static int Tune(string root, string[] args)
    {
        int steps = Number(args, "--steps", 300);
        int games = Number(args, "--games", 200);
        int check = Number(args, "--check", 2000);
        double reach = Real(args, "--reach", 0.3);
        double rate = Real(args, "--rate", 0.15);
        ulong seed = (ulong)Number(args, "--seed", Random.Shared.Next(1, 1_000_000_000));
        string output = Text(args, "--out", Path.Combine(root, "dotnet", "src", "Cantrip.Client", "Tactics", "tactical-model.txt"));
        string from = Text(args, "--from", "shipped");
        List<DeckList> decks = Decks(root, args);

        TacticalModel start = Named(root, from);
        TacticalModel model = start;

        // A weight's own size is the scale it is nudged on, with a floor so that one at zero can still move.
        // --no-cards leaves each card's own weight where it is: over a pool of drafted decks any one card is in too
        // few games for a nudge to it to be told from luck, and moving it would only be noise.
        bool cards = !args.Contains("--no-cards");

        // --hold A,B: weights left where they are. A behaviour kept on by judgement is one a mirror cannot punish the
        // absence of (both seats share the blind spot), so a tuning by mirrors would only drift it.
        HashSet<string> held = [.. Text(args, "--hold", string.Empty)
            .Split(',', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries)];
        List<string> names = [.. start.Weights.Keys
            .Where(name => cards || !name.StartsWith(TacticalModel.CardPrefix, StringComparison.Ordinal))
            .Where(name => !held.Contains(name))
            .Order(StringComparer.Ordinal)];
        Dictionary<string, double> scale = names.ToDictionary(
            name => name,
            name => Math.Max(Math.Abs(start.Weight(name)), name.StartsWith(TacticalModel.CardPrefix, StringComparison.Ordinal) ? 0.1 : 0.05),
            StringComparer.Ordinal);

        Console.WriteLine($"Tuning {names.Count} weights by play: {steps} steps of {games} games, {Named(decks)}, from {from}.");
        Console.WriteLine($"Shuffles from {seed}.{(held.Count > 0 ? $" Held: {string.Join(", ", held.Order(StringComparer.Ordinal))}." : string.Empty)}");

        // Out of a pool bigger than one step plays, each step draws its own decks: a pool is played a deck per pair of
        // games from its start, so without this every step of 200 games would play the same first 100 decks.
        int perStep = Math.Max(1, games / 2);
        string progress = Path.ChangeExtension(output, ".tuning.txt");

        var random = new Random(unchecked((int)seed));
        var clock = Stopwatch.StartNew();

        for (int step = 1; step <= steps; step++)
        {
            // Both shrink as it goes: wide early steps find the slope, narrow late ones settle on it.
            double nudge = reach / Math.Pow(step, 0.1);
            double move = rate / Math.Pow(1 + (step / 50.0), 0.6);

            Dictionary<string, double> signs = names.ToDictionary(name => name, _ => random.Next(2) == 0 ? -1.0 : 1.0, StringComparer.Ordinal);
            TacticalModel current = model;
            TacticalModel up = current.With(names.Select(name => KeyValuePair.Create(name, current.Weight(name) + (nudge * scale[name] * signs[name]))));
            TacticalModel down = current.With(names.Select(name => KeyValuePair.Create(name, current.Weight(name) - (nudge * scale[name] * signs[name]))));

            List<DeckList> drawn = decks.Count > perStep
                ? [.. decks.OrderBy(_ => random.Next()).Take(perStep)]
                : decks;

            Tally tally = Play(
                root, drawn, games, seed + (ulong)(step * 100_003),
                (_, _) => new TacticalPlay(up), (_, _) => new TacticalPlay(down), sample: false, out _);

            int decided = tally.WinsA + tally.WinsB;
            double won = decided == 0 ? 0 : (tally.WinsA - tally.WinsB) / (double)decided;

            model = current.With(names.Select(name => KeyValuePair.Create(name, current.Weight(name) + (move * won * scale[name] * signs[name]))));

            if (step % 25 == 0 || step == steps)
            {
                Console.WriteLine($"  step {step}: {clock.Elapsed.TotalMinutes:N1} min");
                File.WriteAllText(progress, model.Write($"Tuning, after {step} steps."));
            }
        }

        // What the tuned weights have to beat: what they started from, or with --against a seat by name, which is how
        // a tuning on today's code is checked against a frozen champion rather than against itself.
        string rival = Text(args, "--against", string.Empty);
        TacticalModel tuned = model;

        // The check, too, is spread over the whole pool rather than its first check/2 decks.
        List<DeckList> checking = [.. decks.OrderBy(_ => random.Next())];
        Tally against = Play(
            root, checking, check, seed + 999_983,
            (_, _) => new TacticalPlay(tuned), rival.Length > 0 ? Maker(root, rival) : (_, _) => new TacticalPlay(start), sample: false, out _);

        Console.WriteLine();
        Console.WriteLine($"Tuned against {(rival.Length > 0 ? rival : "what it came from")}: {against.Line()}");

        if (!against.Ahead)
        {
            Console.WriteLine("Not kept: it did not beat what it came from by more than chance would.");
            return 1;
        }

        string heading =
            "What the tactical seat thinks a position is worth (TacticalModel): log-odds of winning, per feature,\n" +
            "and the policy.* weights it plays by.\n" +
            $"Fitted to the seat's own games and then tuned by play (tools\\bot-train.cmd) with {Named(decks)}:\n" +
            $"{steps} steps of {games} games, after which it beat what it started from {against.Line()}.";

        File.WriteAllText(output, tuned.Write(heading));
        Console.WriteLine($"Kept, and written to {output}.");
        return 0;
    }

    /// <summary>
    /// The seed a seat's own draws come from, the same every run: <c>HashCode</c> is salted per process, and a run that
    /// cannot be repeated cannot be looked at again.
    /// </summary>
    private static int Draws(int game, int seat, int round) => unchecked((game * 7919) + (seat * 104_729) + (round * 1_299_709) + 17);

    /// <summary>
    /// A model by name: <c>prior</c>, <c>shipped</c>, or a path to one — and after any of them, <c>+weight=value</c> as
    /// often as wanted, which is that model with those weights set: <c>tactical+policy.holdcounter=0</c> is the shipped
    /// seat that never holds mana back. That is how one build plays itself with a behaviour on and off.
    /// </summary>
    private static TacticalModel Named(string root, string name)
    {
        string[] parts = name.Split('+');
        List<KeyValuePair<string, double>> set = [];

        // From the end, so that a path with a plus sign in it is still a path.
        int last = parts.Length;
        while (last > 1 && parts[last - 1].Split('=') is [var weight, var value]
            && double.TryParse(value, NumberStyles.Float, CultureInfo.InvariantCulture, out double number))
        {
            set.Add(KeyValuePair.Create(weight.Trim(), number));
            last--;
        }

        string from = string.Join('+', parts[..last]);
        TacticalModel model = from switch
        {
            "prior" => TacticalModel.Prior(),
            "shipped" or "tactical" => TacticalModel.Default,
            _ => TacticalModel.Parse(File.ReadAllText(File.Exists(from) ? from : Path.Combine(root, from))),
        };

        return set.Count == 0 ? model : model.With(set);
    }

    // --- watch ---------------------------------------------------------------------------------------------------------

    /// <summary>One game, told turn by turn as seat 1 saw it: how a seat's play is read rather than counted.</summary>
    private static int Watch(string root, string[] args)
    {
        string a = Text(args, "--a", "tactical");
        string b = Text(args, "--b", "tactical");
        ulong seed = (ulong)Number(args, "--seed", Random.Shared.Next(1, 1_000_000_000));
        List<DeckList> decks = Decks(root, args);

        // --lost looks through shuffles for the first game seat 1 loses: the games worth reading are those.
        bool lost = args.Contains("--lost");
        MatchResult result;
        int tries = 0;

        // --hang SECONDS gives up on a game that has not come back by then and says what it was doing: an endless
        // loop inside one answer never reaches the answer limit.
        int patience = Number(args, "--hang", 0);

        // --game N plays game N of a duel again, given the duel's own --seed, seats and decks: the same seating, the
        // same shuffle and the same draws for a seat that draws. What a failure line of a run names is this number.
        int game = Number(args, "--game", -1);

        do
        {
            ulong shuffle = game >= 0 ? Shuffle(seed, game) : seed;
            (List<DeckList> seated, List<IAutoPlayer> players) = game >= 0
                ? Seating(decks, game, Maker(root, a), Maker(root, b))
                : ([decks[0], decks[^1]], [Maker(root, a)(0, 1), Maker(root, b)(0, 2)]);

            // --narrate FILE: the --a seat, if it is the tactical one, writes there what it was asked and why it
            // answered as it did (TacticalPlay.Narrator), a decision at a time.
            string narrate = Text(args, "--narrate", string.Empty);
            StreamWriter? narration = null;
            if (narrate.Length > 0 && players[game >= 0 && game % 2 == 1 ? 1 : 0] is TacticalPlay narrator)
            {
                narration = new StreamWriter(narrate, append: false);
                narrator.Narrator = narration;
            }

            if (game >= 0)
            {
                Console.WriteLine($"Game {game}: seat 1 is {(game % 2 == 1 ? b : a)} with {seated[0].Name}, seat 2 is {(game % 2 == 1 ? a : b)} with {seated[1].Name}.");
            }

            // A neural seat plays at a table with an AI host beside it, as in a duel.
            Lane? lane = players.Any(p => p is NeuralPlay) ? new Lane(root, Neural.Arguments, Neural.Echo) : null;
            Task<MatchResult> playing = Task.Run(() => Match.Play(
                Path.Combine(root, "content"), seated, Format, shuffle, players,
                sample: false, story: true, limit: Number(args, "--limit", 0), lane: lane));

            if (patience > 0 && !playing.Wait(TimeSpan.FromSeconds(patience)))
            {
                Console.WriteLine($"Shuffle {seed} had not finished after {patience}s. Last seen: {Match.Doing}");

                // Whether the answer is what never came back, or what the game did with it: the deterministic seat's
                // payment is a function of the prompt, so it can be asked again here.
                if (Match.Asked is Cantrip.Protocol.PayCostsPromptMessage pay)
                {
                    Task<Cantrip.Protocol.TakeActionAnswer> again = Task.Run(() => AutoPlay.Pay(pay));
                    Console.WriteLine(again.Wait(TimeSpan.FromSeconds(3))
                        ? $"AutoPlay.Pay answers that at once ({again.Result.Kind}), so it is the game that has not come back from it."
                        : "AutoPlay.Pay does not come back from that prompt either: the loop is in the seat.");
                }

                Environment.Exit(3);
            }

            result = playing.GetAwaiter().GetResult();
            narration?.Dispose();
            lane?.Dispose();
        }
        while (lost && result.Winner == 1 && ++tries < 500 && ++seed > 0);

        if (game < 0)
        {
            Console.WriteLine($"Seat 1 is {a} with {decks[0].Name}, seat 2 is {b} with {decks[^1].Name}; shuffle {seed}.");
        }

        // A game that did not finish is told from its end, which is where whatever stopped it is.
        int tail = Number(args, "--tail", result.Finished ? int.MaxValue : 80);
        foreach (string line in result.Story.Skip(Math.Max(result.Story.Count - tail, 0)))
        {
            Console.WriteLine(line);
        }

        Console.WriteLine();
        Console.WriteLine(result.Finished ? $"Seat {result.Winner} won on turn {result.Turns}." : result.Failure);
        return result.Finished ? 0 : 1;
    }

    // --- train ---------------------------------------------------------------------------------------------------------

    private static int Train(string root, string[] args)
    {
        int rounds = Number(args, "--rounds", 3);
        int games = Number(args, "--games", 4000);
        int check = Number(args, "--check", 1000);
        double explore = Real(args, "--explore", 0.03);
        double general = Real(args, "--general", 200);
        double card = Real(args, "--card", 400);
        ulong seed = (ulong)Number(args, "--seed", Random.Shared.Next(1, 1_000_000_000));
        string output = Text(args, "--out", Path.Combine(root, "dotnet", "src", "Cantrip.Client", "Tactics", "tactical-model.txt"));
        string from = Text(args, "--from", "prior");
        List<DeckList> decks = Decks(root, args);
        List<string> without = [.. Text(args, "--without", string.Empty)
            .Split(',', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries)];
        List<string> only = [.. Text(args, "--only", string.Empty)
            .Split(',', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries)];

        TacticalModel model = Named(root, from);

        Console.WriteLine($"Fitting the tactical model to its own games: {rounds} round(s) of {games}, {Named(decks)}, from {from}.");
        Console.WriteLine($"Shuffles from {seed}; {explore:P0} of decisions drawn at random so the games differ.");

        int kept = 0;
        int played = 0;

        for (int round = 1; round <= rounds; round++)
        {
            TacticalModel playing = model;
            int thisRound = round;
            var clock = Stopwatch.StartNew();

            Tally selfPlay = Play(
                root, decks, games, seed + (ulong)(round * 1_000_003),
                (game, seat) => new TacticalPlay(playing, Draws(game, seat, thisRound)) { Explore = explore },
                (game, seat) => new TacticalPlay(playing, Draws(game, seat, thisRound + 1000)) { Explore = explore },
                sample: true,
                out List<Sample> samples);

            played += selfPlay.Finished;

            Console.WriteLine();
            Console.WriteLine($"Round {round}: {selfPlay.Finished} games in {clock.Elapsed.TotalSeconds:N0}s, {samples.Count} positions.");
            if (!selfPlay.Failures.IsEmpty)
            {
                Console.WriteLine($"  {selfPlay.Failures.Count} did not finish: {selfPlay.Failures.GroupBy(failure => failure).OrderByDescending(group => group.Count()).First().Key}");
            }

            (TacticalModel fitted, FitReport report) = Fit.Run(samples, model, general, card, only: only);
            Console.WriteLine(
                $"  Fitted {report.Features} weights. Log loss on positions kept back: {report.Before:N4} before, {report.After:N4} after; " +
                $"the favourite won {report.Accuracy:P1} of the time.");

            // --without: the same games fitted again with those features left out and weighed at nothing, written beside the
            // output, so that what a new feature is worth is a duel between two fits of one set of games.
            if (without.Count > 0)
            {
                (TacticalModel control, FitReport controlReport) = Fit.Run(samples, model, general, card, without: without, only: only);
                string controlPath = Path.ChangeExtension(output, null) + $".without-round{round}.txt";
                File.WriteAllText(controlPath, control.Write($"Round {round}'s fit without {string.Join(", ", without)}."));
                Console.WriteLine($"  Without {string.Join(", ", without)}: log loss {controlReport.After:N4}, written to {controlPath}.");
            }

            // A model that predicts better is not thereby a model that plays better, so it is made to play.
            Tally against = Play(
                root, decks, check, seed + (ulong)(round * 7_000_003),
                (_, _) => new TacticalPlay(fitted), (_, _) => new TacticalPlay(playing), sample: false, out _);

            Console.WriteLine($"  Fitted against what it came from: {against.Line()}");

            // Kept or not, what the fit came to is worth reading: it is written where temporary files go.
            string candidate = Path.Combine(Path.GetTempPath(), "cantrip-tactical-model.candidate.txt");
            File.WriteAllText(candidate, fitted.Write($"Round {round}'s fit, {against.Line()} against what it came from."));
            Console.WriteLine($"  The fit itself is in {candidate}.");

            if (against.Ahead)
            {
                model = fitted;
                kept++;

                string heading =
                    "What the tactical seat thinks a position is worth (TacticalModel): log-odds of winning, per feature.\n" +
                    $"Fitted by tools\\bot-train.cmd to {played} games the seat played against itself with {Named(decks)}.\n" +
                    "The policy.* weights are set by hand; everything else is the fit, held towards TacticalModel.Prior.";

                File.WriteAllText(output, model.Write(heading));
                Console.WriteLine($"  Kept, and written to {output}.");
            }
            else
            {
                Console.WriteLine("  Not kept: it did not beat what it came from.");
            }
        }

        return kept > 0 ? 0 : 1;
    }

    // --- Playing -------------------------------------------------------------------------------------------------------

    /// <summary>Who won a run of games between two ways of playing, A and B.</summary>
    private sealed class Tally
    {
        public int WinsA;
        public int WinsB;
        public int Draws;
        public int PlayWins;
        public int Decided;
        public long Turns;
        public long AnswersA;
        public long AnswersB;
        public double ThinkingA;
        public double ThinkingB;
        public double SlowestA;
        public double SlowestB;
        public object Gate { get; } = new();

        /// <summary>A's wins and games by the seat it sat in, seat 1 first.</summary>
        public int[] WinsBySeat { get; } = new int[2];

        public int[] GamesBySeat { get; } = new int[2];

        /// <summary>A's wins and games on the play (it took the first turn) and on the draw.</summary>
        public int[] WinsByTurn { get; } = new int[2];

        public int[] GamesByTurn { get; } = new int[2];

        /// <summary>A's wins and decided games by its deck against B's.</summary>
        public SortedDictionary<string, int[]> ByPair { get; } = new(StringComparer.Ordinal);

        /// <summary>Each answer of A's and of B's: its whole time and the native host's part, for a game on a lane.</summary>
        public List<(double Act, double Host)> TimesA { get; } = [];

        public List<(double Act, double Host)> TimesB { get; } = [];

        /// <summary>
        /// Per card, how the seats whose deck had it did: games won and lost holding it at some point, and won and lost
        /// never having held it, first for A and then for B.
        /// </summary>
        public Dictionary<string, int[]> Cards { get; } = new(StringComparer.Ordinal);

        public ConcurrentQueue<string> Failures { get; } = [];

        public int Finished => WinsA + WinsB + Draws;

        /// <summary>
        /// The sequential probability ratio test's log-likelihood ratio of A's games so far, H1 (A wins 55%) against H0
        /// (50%), a draw half a win and half a loss: each game adds <c>x ln(p1/p0) + (1-x) ln((1-p1)/(1-p0))</c> for its
        /// score x. With alpha = beta = 0.05 it accepts H1 at ln(19) and H0 at -ln(19) (Wald).
        /// </summary>
        public double Llr
        {
            get
            {
                double wins = WinsA + (Draws / 2.0);
                double losses = WinsB + (Draws / 2.0);
                return (wins * Math.Log(SprtH1 / SprtH0)) + (losses * Math.Log((1 - SprtH1) / (1 - SprtH0)));
            }
        }

        public const double SprtH0 = 0.5;
        /// <summary>A's share under H1: 55% unless --sprt-h1 says otherwise.</summary>
        public static double SprtH1 = 0.55;

        /// <summary>Wald's bound for alpha = beta = 0.05: ln((1 - beta) / alpha) = ln 19.</summary>
        public static readonly double SprtBound = Math.Log(19);

        /// <summary>The SPRT's verdict so far: +1 H1 (A is the better by 55%), -1 H0 (it is not), 0 not yet.</summary>
        public int Verdict => Llr >= SprtBound ? 1 : Llr <= -SprtBound ? -1 : 0;

        /// <summary>The games that had finished when the SPRT first reached a verdict, and the LLR then; zero before.</summary>
        public int StoppedAt;

        public double StoppedLlr;

        public string SprtLine(string a, string b) =>
            $"SPRT (H0 {SprtH0:P0}, H1 {SprtH1:P0}, alpha = beta = 0.05, bounds +/-{SprtBound:N3}): LLR {Llr:N3} after {Finished} games; " +
            (StoppedAt > 0
                ? $"{(StoppedLlr > 0 ? $"H1 accepted, {a} is the better" : $"H0 accepted, {a} is not the better by {SprtH1:P0}")} at game {StoppedAt} (LLR {StoppedLlr:N3}; games already in play finished after)."
                : Verdict != 0 ? $"{(Verdict > 0 ? "H1" : "H0")} by the end." : "no verdict within the games played.");

        /// <summary>
        /// Whether A won by more than luck would give it: its share of the decided games is above a half by more than
        /// the margin of error. A model is only kept on this, since 2005 games to 1995 is a coin that landed heads.
        /// </summary>
        public bool Ahead
        {
            get
            {
                int decided = WinsA + WinsB;
                double share = decided == 0 ? 0 : WinsA / (double)decided;
                return decided > 0 && share - (1.96 * Math.Sqrt(share * (1 - share) / decided)) > 0.5;
            }
        }

        public string Line()
        {
            int decided = WinsA + WinsB;
            double share = decided == 0 ? 0 : WinsA / (double)decided;

            // The normal approximation's 95% interval: enough to say whether a difference is one.
            double margin = decided == 0 ? 0 : 1.96 * Math.Sqrt(share * (1 - share) / decided);
            return $"{WinsA} to {WinsB}{(Draws > 0 ? $", {Draws} drawn" : string.Empty)} ({share:P1} +/- {margin:P1})";
        }

        /// <summary>A share of games with its 95% interval, the normal approximation.</summary>
        public static string Share(int won, int of)
        {
            double share = of == 0 ? 0 : won / (double)of;
            double margin = of == 0 ? 0 : 1.96 * Math.Sqrt(share * (1 - share) / of);
            return $"{won} of {of} ({share:P1} +/- {margin:P1})";
        }

        /// <summary>A's results by seat, by turn order and by decks; and, for games on a lane, what an answer cost each.</summary>
        public void WriteSplits(string a, string b, IReadOnlyCollection<NeuralAnswer> neural)
        {
            Console.WriteLine();
            Console.WriteLine($"{a} in seat 1: {Share(WinsBySeat[0], GamesBySeat[0])}; in seat 2: {Share(WinsBySeat[1], GamesBySeat[1])}.");
            Console.WriteLine($"{a} on the play: {Share(WinsByTurn[0], GamesByTurn[0])}; on the draw: {Share(WinsByTurn[1], GamesByTurn[1])}.");
            foreach ((string pair, int[] counts) in ByPair)
            {
                Console.WriteLine($"  {pair}: {Share(counts[0], counts[1])}");
            }

            if (TimesA.Count == 0)
            {
                return;
            }

            Console.WriteLine();
            Console.WriteLine("An answer's time (ms), p50 / p95 / max: the whole of it, the native host's part (the game's own work), the seat's own part (the rest).");
            void Seat(string name, List<(double Act, double Host)> times) => Console.WriteLine(
                $"  {name,-10} {times.Count,8} answers   whole {Spread(times.Select(t => t.Act))}   host {Spread(times.Select(t => t.Host))}   seat {Spread(times.Select(t => t.Act - t.Host))}");
            Seat(a, TimesA);
            Seat(b, TimesB);

            foreach (IGrouping<string, NeuralAnswer> group in neural.GroupBy(n => n.Seat).OrderBy(g => g.Key, StringComparer.Ordinal))
            {
                WriteNeural(group.Key, [.. group]);
            }
        }

        /// <summary>What one neural seat's answers cost, and for a searching seat what its searches were.</summary>
        private static void WriteNeural(string seat, IReadOnlyCollection<NeuralAnswer> neural)
        {
            {
                Console.WriteLine();
                Console.WriteLine($"{seat}: the network's {neural.Count} answers (ms), p50 / p95 / max:");
                Console.WriteLine($"  asked to answered, on the seat's clock   {Spread(neural.Select(n => n.Decide))}");
                Console.WriteLine($"  in the AI host                           {Spread(neural.Select(n => n.Total))}");
                Console.WriteLine($"    encoding                               {Spread(neural.Select(n => n.Encode))}");
                Console.WriteLine($"    the network                            {Spread(neural.Select(n => n.Evaluate))}");
                Console.WriteLine($"    the rest (mirror, factoring, mapping)  {Spread(neural.Select(n => n.Total - n.Encode - n.Evaluate))}");
                Console.WriteLine($"  the process round trip (pipe, JSON, the mirror catching up)  {Spread(neural.Select(n => n.Decide - n.Total))}");
                Console.WriteLine($"  steps an answer {Spread(neural.Select(n => (double)n.Steps), "N1")}, evaluations an answer {Spread(neural.Select(n => (double)n.Evaluations), "N1")}");
                double evaluating = neural.Sum(n => n.Evaluate) / 1000;
                long evaluations = neural.Sum(n => (long)n.Evaluations);
                Console.WriteLine(
                    $"  {evaluations} evaluations, {evaluations / Math.Max(evaluating, 1e-9):N0} a second of the network's time; " +
                    $"{neural.Count(n => n.Fallback)} fallbacks, {neural.Count(n => n.Late)} late, {neural.Count(n => n.Error.Length > 0)} with an error" +
                    (neural.FirstOrDefault(n => n.Error.Length > 0) is { } first ? $" (first: {first.Error})" : string.Empty));

                List<NeuralAnswer> searched = [.. neural.Where(n => n.Searched && !n.Shortcut)];
                if (searched.Count > 0)
                {
                    double seconds = searched.Sum(n => n.Total) / 1000;
                    Console.WriteLine(
                        $"  searched {searched.Count} of {neural.Count} answers ({neural.Count(n => n.Shortcut)} passed by the priority shortcut, " +
                        $"the rest forced or a payment): " +
                        $"simulations {Spread(searched.Select(n => (double)n.Simulations), "N0")}, nodes {Spread(searched.Select(n => (double)n.Nodes), "N0")}, " +
                        $"pruned {Spread(searched.Select(n => (double)n.Pruned), "N0")}, tree KB {Spread(searched.Select(n => n.TreeBytes / 1024.0), "N0")}");
                    Console.WriteLine(
                        $"  depth in decisions (the root's counted): the most visited line {Spread(searched.Select(n => (double)n.LineDepth), "N0")}, " +
                        $"the deepest {Spread(searched.Select(n => (double)n.DeepestDepth), "N0")}");
                    Console.WriteLine(
                        $"  {searched.Count / Math.Max(seconds, 1e-9):N1} searched decisions a second of a seat's time, " +
                        $"{searched.Sum(n => (long)n.Evaluations) / Math.Max(seconds, 1e-9):N0} evaluations a second; " +
                        $"{neural.Count / Math.Max(neural.Sum(n => n.Total) / 1000, 1e-9):N1} decisions a second in all");
                }
            }
        }

        private static string Spread(IEnumerable<double> values, string format = "N3")
        {
            double[] sorted = [.. values.Order()];
            if (sorted.Length == 0)
            {
                return "-";
            }

            double At(double q) => sorted[Math.Min(sorted.Length - 1, (int)Math.Floor(q * sorted.Length))];
            return $"{At(0.5).ToString(format, CultureInfo.InvariantCulture)} / {At(0.95).ToString(format, CultureInfo.InvariantCulture)} / {sorted[^1].ToString(format, CultureInfo.InvariantCulture)}";
        }

        public void Write(string a, string b, TimeSpan elapsed)
        {
            Console.WriteLine();
            Console.WriteLine($"{a} beat {b} {Line()}.");
            Console.WriteLine(
                $"{Finished} games in {elapsed.TotalSeconds:N1}s ({Finished / Math.Max(elapsed.TotalSeconds, 0.001):N0} games/s), " +
                $"{Turns / (double)Math.Max(Finished, 1):N1} turns a game; whoever played first won {PlayWins / (double)Math.Max(Decided, 1):P1}.");

            // What an answer costs, with the game's own work in the same call: the session runs inside the send.
            Console.WriteLine(
                $"An answer took {a} {ThinkingA / Math.Max(AnswersA, 1):N3} ms on average and {SlowestA:N1} ms at most; " +
                $"{b} {ThinkingB / Math.Max(AnswersB, 1):N3} ms and {SlowestB:N1} ms.");

            if (!Failures.IsEmpty)
            {
                Console.WriteLine();
                Console.WriteLine($"{Failures.Count} game(s) did not finish; the first: {(Failures.TryPeek(out string? first) ? first : string.Empty)}");
                foreach (IGrouping<string, string> group in Failures.GroupBy(failure => failure).OrderByDescending(group => group.Count()).Take(8))
                {
                    Console.WriteLine($"  {group.Count()}x {group.Key}");
                }
            }

            if (NeuralPlay.StaleResults > 0)
            {
                Console.WriteLine($"{NeuralPlay.StaleResults} AI result(s) of an earlier request passed over.");
            }
        }

        /// <summary>
        /// Which cards a seat wins with: for each card, how often the seat won the games it held the card in against
        /// the games with the same decks in which it never did.
        /// <para>
        /// In a mirror both seats have the card in their deck, so the deck is no part of the difference: what is left
        /// is whether drawing the card helped whoever drew it. A strong card that the seat holding it does not win
        /// more with is a card it does not know how to play, which is what this is for finding. It is not a card's
        /// worth: a card held is also a game that went long enough to draw it.
        /// </para>
        /// </summary>
        public void WriteCards(string a, string b, int count, int least)
        {
            bool same = string.Equals(a, b, StringComparison.Ordinal);

            foreach (int side in same ? new[] { -1 } : [0, 1])
            {
                // With the same seat on both sides there is one table, of both.
                List<(string Card, int Held, double WonHeld, int Not, double WonNot, double Difference, double Margin)> rows = [];

                foreach ((string card, int[] counts) in Cards)
                {
                    int wonHeld = side < 0 ? counts[0] + counts[4] : counts[side * 4];
                    int lostHeld = side < 0 ? counts[1] + counts[5] : counts[(side * 4) + 1];
                    int wonNot = side < 0 ? counts[2] + counts[6] : counts[(side * 4) + 2];
                    int lostNot = side < 0 ? counts[3] + counts[7] : counts[(side * 4) + 3];
                    int held = wonHeld + lostHeld;
                    int not = wonNot + lostNot;

                    if (held < least || not < least)
                    {
                        continue;
                    }

                    double shareHeld = wonHeld / (double)held;
                    double shareNot = wonNot / (double)not;
                    double margin = 1.96 * Math.Sqrt((shareHeld * (1 - shareHeld) / held) + (shareNot * (1 - shareNot) / not));
                    rows.Add((card, held, shareHeld, not, shareNot, shareHeld - shareNot, margin));
                }

                rows = [.. rows.OrderBy(row => row.Difference)];

                Console.WriteLine();
                Console.WriteLine(side < 0
                    ? $"What a seat won with, of {rows.Count} cards held and not held in at least {least} games each:"
                    : $"What {(side == 0 ? a : b)} won with, of {rows.Count} cards held and not held in at least {least} games each:");
                Console.WriteLine("  won holding it    won without    difference          card");

                void Row((string Card, int Held, double WonHeld, int Not, double WonNot, double Difference, double Margin) row) =>
                    Console.WriteLine(
                        $"  {row.WonHeld,6:P1} of {row.Held,-5} {row.WonNot,6:P1} of {row.Not,-5} {row.Difference,7:+0.0%;-0.0%} +/- {row.Margin,5:P1}   {row.Card}");

                foreach (var row in rows.Take(count))
                {
                    Row(row);
                }

                if (rows.Count > count)
                {
                    Console.WriteLine("  ...");
                    foreach (var row in rows.Skip(Math.Max(rows.Count - count, count)))
                    {
                        Row(row);
                    }
                }
            }
        }
    }

    private static Tally Play(
        string root,
        IReadOnlyList<DeckList> decks,
        int games,
        ulong seed,
        Func<int, int, IAutoPlayer> a,
        Func<int, int, IAutoPlayer> b,
        bool sample,
        out List<Sample> samples,
        bool hands = false,
        bool sprt = false)
    {
        var tally = new Tally();
        var collected = new ConcurrentBag<Sample>();
        string content = Path.Combine(root, "content");
        int done = 0;

        // A neural seat plays at a table with an AI host beside it: each worker its own pair of hosts, reused game
        // after game, and every seat's answer timed against the native host's part of it.
        bool neural = a(0, 1) is NeuralPlay || b(0, 2) is NeuralPlay;
        var lanes = new ConcurrentBag<Lane>();
        var made = new ConcurrentBag<Lane>();
        NeuralAnswers.Clear();

        Parallel.For(0, games, new ParallelOptions { MaxDegreeOfParallelism = Parallelism }, (game, loop) =>
        {
            // Once the SPRT has its verdict no new game starts; those in play finish and are counted.
            if (sprt && loop.ShouldExitCurrentIteration)
            {
                return;
            }

            // A sits in seat 1 on even games and seat 2 on odd ones, each pair dealt from the same shuffle; the decks
            // swap every second pair, so each way of playing has each deck in each seat.
            bool swapped = game % 2 == 1;
            (List<DeckList> seated, List<IAutoPlayer> players) = Seating(decks, game, a, b);

            Lane? lane = null;
            if (neural && !lanes.TryTake(out lane))
            {
                lane = new Lane(root, Neural.Arguments, Neural.Echo);
                made.Add(lane);
            }

            MatchResult result;
            try
            {
                result = Match.Play(content, seated, Format, Shuffle(seed, game), players, sample, hands: hands, lane: lane);
            }
            finally
            {
                // A lane whose AI host stalled is let go (its hosts stopped) rather than handed to the next game, which
                // would wait behind the request it is still busy with; the next game makes a fresh lane.
                if (lane is not null && lane.Usable)
                {
                    lanes.Add(lane);
                }
                else
                {
                    lane?.Dispose();
                }
            }

            if (!result.Finished)
            {
                // The game's number in the run, with the run's own seed, is what plays it again: watch --game.
                tally.Failures.Enqueue($"{result.Failure} [game {game}: {seated[0].Name} / {seated[1].Name}]");
                return;
            }

            foreach (Sample position in result.Samples)
            {
                collected.Add(position with { Game = game });
            }

            int seatOfA = swapped ? 2 : 1;
            if (result.Winner == 0)
            {
                Interlocked.Increment(ref tally.Draws);
            }
            else
            {
                Interlocked.Increment(ref result.Winner == seatOfA ? ref tally.WinsA : ref tally.WinsB);
                Interlocked.Increment(ref tally.Decided);
                if (result.Winner == result.Starter)
                {
                    Interlocked.Increment(ref tally.PlayWins);
                }
            }

            Interlocked.Add(ref tally.Turns, result.Turns);

            lock (tally.Gate)
            {
                int seatA = seatOfA - 1;
                int seatB = 1 - seatA;
                if (result.Winner != 0)
                {
                    bool won = result.Winner == seatOfA;
                    int turn = result.Starter == seatOfA ? 0 : 1;
                    tally.GamesBySeat[seatA]++;
                    tally.WinsBySeat[seatA] += won ? 1 : 0;
                    tally.GamesByTurn[turn]++;
                    tally.WinsByTurn[turn] += won ? 1 : 0;
                    string pair = $"{seated[seatA].Name} vs {seated[seatB].Name}";
                    if (!tally.ByPair.TryGetValue(pair, out int[]? counts))
                    {
                        tally.ByPair[pair] = counts = new int[2];
                    }

                    counts[0] += won ? 1 : 0;
                    counts[1]++;
                }

                if (result.Times.Count == 2)
                {
                    tally.TimesA.AddRange(result.Times[seatA]);
                    tally.TimesB.AddRange(result.Times[seatB]);
                }

                tally.AnswersA += result.Answers[seatA];
                tally.AnswersB += result.Answers[seatB];
                tally.ThinkingA += result.Thinking[seatA];
                tally.ThinkingB += result.Thinking[seatB];
                tally.SlowestA = Math.Max(tally.SlowestA, result.Slowest[seatA]);
                tally.SlowestB = Math.Max(tally.SlowestB, result.Slowest[seatB]);

                if (hands && result.Winner != 0)
                {
                    for (int seat = 0; seat < result.Held.Count && seat < seated.Count; seat++)
                    {
                        int side = seat == seatA ? 0 : 4;
                        bool won = result.Winner == seat + 1;

                        foreach (string card in seated[seat].Entries.Select(entry => entry.Card).Distinct(StringComparer.Ordinal))
                        {
                            if (!tally.Cards.TryGetValue(card, out int[]? counts))
                            {
                                tally.Cards[card] = counts = new int[8];
                            }

                            counts[side + (result.Held[seat].Contains(card) ? 0 : 2) + (won ? 0 : 1)]++;
                        }
                    }
                }
            }

            if (sprt)
            {
                lock (tally.Gate)
                {
                    if (tally.StoppedAt == 0 && tally.Verdict != 0)
                    {
                        tally.StoppedAt = tally.Finished;
                        tally.StoppedLlr = tally.Llr;
                        Console.WriteLine($"SPRT verdict after {tally.Finished} games: LLR {tally.Llr:N3} ({tally.WinsA} to {tally.WinsB}, {tally.Draws} drawn).");
                        loop.Stop();
                    }
                    else if (tally.Finished % 20 == 0)
                    {
                        Console.WriteLine($"  {tally.Finished} games: {tally.WinsA} to {tally.WinsB}, {tally.Draws} drawn, LLR {tally.Llr:N3}");
                    }
                }
            }

            if (Interlocked.Increment(ref done) % 100 == 0 && !Console.IsOutputRedirected)
            {
                Console.Write($"\r{done} of {games}...    ");
            }
        });

        if (!Console.IsOutputRedirected)
        {
            Console.Write($"\r{new string(' ', 32)}\r");
        }

        foreach (Lane lane in made)
        {
            if (lane.Ai.Divergence is { } divergence)
            {
                tally.Failures.Enqueue($"an AI host's mirror left its game: {divergence}");
            }

            lane.Dispose();
        }

        samples = [.. collected];
        return tally;
    }

    /// <summary>
    /// Who sits where with what in one game of a run, which is all of a run that is not its seed.
    /// </summary>
    internal static (List<DeckList> Decks, List<IAutoPlayer> Players) Seating(
        IReadOnlyList<DeckList> decks, int game, Func<int, int, IAutoPlayer> a, Func<int, int, IAutoPlayer> b)
    {
        bool swapped = game % 2 == 1;

        if (PairingsFile.Length > 0 && Pairings.Count == 0)
        {
            lock (decks)
            {
                if (Pairings.Count == 0)
                {
                    DeckList Named(string name) => decks.FirstOrDefault(d => d.Name == name)
                        ?? throw new ArgumentException($"--pairings names {name}, which --decks does not hold.");
                    Pairings = [.. File.ReadAllLines(PairingsFile).Where(l => l.Contains(':', StringComparison.Ordinal))
                        .Select(l => l.Trim().Split(':', 2)).Select(p => (Named(p[0]), Named(p[1])))];
                }
            }
        }

        if (Pairings.Count > 0)
        {
            (DeckList first, DeckList second) = Pairings[game / 2 % Pairings.Count];
            return (swapped ? [second, first] : [first, second], swapped ? [b(game, 1), a(game, 2)] : [a(game, 1), b(game, 2)]);
        }

        if (Versus.Count > 0 && decks.Count > 0)
        {
            DeckList mine = decks[game / 2 % decks.Count];
            DeckList theirs = Versus[game / 2 % Versus.Count];
            return (swapped ? [theirs, mine] : [mine, theirs], swapped ? [b(game, 1), a(game, 2)] : [a(game, 1), b(game, 2)]);
        }

        bool decksSwapped = decks.Count == 2 && game / 2 % 2 == 1;

        // More than two decks is a pool of mirrors: each pair of games takes the next deck for both seats, so what
        // differs between the seats is only how they are played, over as many kinds of deck as the pool holds.
        // Two decks with --mirrors: each four pairs of games the matchup both ways round, then each deck's mirror.
        List<DeckList> seated = decks.Count > 2 ? [decks[game / 2 % decks.Count], decks[game / 2 % decks.Count]]
            : Mirrors && decks.Count == 2 && game / 2 % 4 >= 2 ? [decks[game / 2 % 4 - 2], decks[game / 2 % 4 - 2]]
            : decksSwapped ? [decks[1], decks[0]]
            : [decks[0], decks[^1]];

        return (seated, swapped ? [b(game, 1), a(game, 2)] : [a(game, 1), b(game, 2)]);
    }

    /// <summary>
    /// A way of playing, by name: <c>random</c>, <c>auto</c> (the one deterministic line), <c>tactical</c> (the
    /// shipped model), <c>prior</c> (the tactical seat on its hand-written weights), a path to a model file, or a
    /// frozen build of an earlier tactical seat (<c>champion</c>, <c>referee</c>, <c>champion:NAME</c>; see
    /// <see cref="Champion"/>). Each call of what it returns is one seat of one game.
    /// </summary>
    internal static Func<int, int, IAutoPlayer> Maker(string root, string name, bool b = false)
    {
        switch (name)
        {
            case "random":
                return (game, seat) => new RandomPlay(Draws(game, seat, 0));
            case "auto":
                return (_, _) => new AutoPlayer();
            case var _ when NeuralSeat.Parse(name) is { } neural:
                (string seatModel, string seatName) = Neural.For(neural.Name, b);
                return (_, _) => new NeuralPlay(seatModel, neural.Mode, Neural.Deadline, NeuralAnswers, neural with { Name = seatName });
            default:
                // A frozen build of an earlier seat: champion, referee, champion:NAME.
                if (Champion.Named(root, name) is { } champion)
                {
                    return (_, _) => champion.Seat();
                }

                TacticalModel model = Named(root, name);
                return ExploreDuel > 0
                    ? (game, seat) => new TacticalPlay(model, Draws(game, seat, 0)) { Explore = ExploreDuel }
                    : (_, _) => new TacticalPlay(model);
        }
    }

    // --- Arguments -----------------------------------------------------------------------------------------------------

    /// <summary>The decks: <c>--deck</c> once for a mirror, twice for a matchup. RW aggro when none is named.</summary>
    internal static List<DeckList> Decks(string root, IReadOnlyList<string> args)
    {
        string decksRoot = Path.Combine(root, "decks");
        List<string> named = [];

        for (int i = 0; i < args.Count - 1; i++)
        {
            if (string.Equals(args[i], "--deck", StringComparison.Ordinal))
            {
                named.Add(args[i + 1]);
            }
        }

        // --decks DIR: every decklist in a folder, each played as a mirror. --match PATTERN keeps those whose file name
        // has it, so one drafter's decks can be told from another's.
        // --decks as often as wanted: the pools one after another, so one run can be spread over aggro, control, combo
        // and ramp decks rather than over one archetype the seat happens to be good at.
        List<string> folders = [];
        for (int i = 0; i < args.Count - 1; i++)
        {
            if (string.Equals(args[i], "--decks", StringComparison.Ordinal))
            {
                folders.Add(args[i + 1]);
            }
        }

        if (folders.Count > 0)
        {
            string match = Text(args, "--match", string.Empty);

            // --has TEXT, as often as wanted: only the decks with a card named so, every one of them; "A|B" is either.
            // A behaviour that is about one kind of card is measured on the decks that have it.
            List<string> has = [];
            for (int i = 0; i < args.Count - 1; i++)
            {
                if (string.Equals(args[i], "--has", StringComparison.Ordinal))
                {
                    has.Add(args[i + 1]);
                }
            }

            List<DeckList> pool = [.. folders.SelectMany(folder => Directory.GetFiles(folder, "*.txt")
                .Where(path => Path.GetFileName(path).Contains(match, StringComparison.OrdinalIgnoreCase))
                .Order(StringComparer.Ordinal))
                .Select(path => DeckList.Parse(Path.GetFileNameWithoutExtension(path), File.ReadAllText(path), path))

                // A pool's run.txt and the like are no decks: a decklist has at least a deck's worth of cards.
                .Where(deck => deck.Count >= 30)
                .Where(deck => has.TrueForAll(text => text.Split('|').Any(name =>
                    deck.Entries.Any(entry => entry.Card.Contains(name, StringComparison.OrdinalIgnoreCase)))))];

            return pool.Count > 0 ? pool : throw new FileNotFoundException($"No decklists in {string.Join(", ", folders)}.");
        }

        if (named.Count == 0)
        {
            named.Add("rw-aggro");
        }

        // More than two named is a pool, each a mirror, as --decks makes.
        return [.. named.Take(named.Count > 2 ? named.Count : 2).Select(deck =>
        {
            string path = DeckList.Find(decksRoot, deck);
            return DeckList.Parse(deck, File.ReadAllText(path), path);
        })];
    }

    /// <summary>The decks of a run in a few words: their names, or how many mirrors there are.</summary>
    private static string Named(IReadOnlyList<DeckList> decks) =>
        decks.Count > 2 ? $"{decks.Count} decks, each a mirror" : string.Join(" vs ", decks.Select(deck => deck.Name));

    private static string FindRoot()
    {
        DirectoryInfo? here = new(AppContext.BaseDirectory);

        while (here is not null)
        {
            if (Directory.Exists(Path.Combine(here.FullName, "content", "mtg")) && Directory.Exists(Path.Combine(here.FullName, "decks")))
            {
                return here.FullName;
            }

            here = here.Parent;
        }

        throw new DirectoryNotFoundException("No content\\mtg above this executable; run it from inside the repository.");
    }

    internal static string Text(IReadOnlyList<string> args, string name, string fallback)
    {
        for (int i = 0; i < args.Count - 1; i++)
        {
            if (string.Equals(args[i], name, StringComparison.Ordinal))
            {
                return args[i + 1];
            }
        }

        return fallback;
    }

    internal static int Number(IReadOnlyList<string> args, string name, int fallback) =>
        int.TryParse(Text(args, name, string.Empty), NumberStyles.Integer, CultureInfo.InvariantCulture, out int value) ? value : fallback;

    private static double Real(IReadOnlyList<string> args, string name, double fallback) =>
        double.TryParse(Text(args, name, string.Empty), NumberStyles.Float, CultureInfo.InvariantCulture, out double value) ? value : fallback;

    private static void Help() => Console.WriteLine(
        """
        cantrip-botlab: sets one way of playing a seat against another, and fits the tactical seat's model.

          duel    plays two seats against each other and says which won more
            --a NAME --b NAME   random, auto, tactical, prior, neural (neural:full, neural:decklist, neural:hidden), or a
                                path to a model file (default: tactical, random);
                                NAME+weight=value sets a weight of it: tactical+policy.holdcounter=0;
                                champion, referee or champion:NAME is a frozen build of an earlier tactical seat
            --deck NAME         once for a mirror, twice for a matchup, more for a pool of mirrors (default: rw-aggro);
                                --mirrors with two: half the games the matchup, a quarter each deck's mirror
            --format KEY        the format the decks are played in: a constructed format's key, or limited
                                with --min-deck N (default: freeform, no deck-construction restriction)
            --decks DIR         every decklist in a folder, each played as a mirror (more than once: the folders together);
                                --match TEXT keeps the files named so,
                                --has TEXT the decks with a card named so (as often as wanted: every one of them; A|B for either)
            --versus DIR        not mirrors: the --a seat plays the --decks decks and the --b seat these, a pair of each per
                                two games (so --a X --b tactical, against --a tactical --b tactical, is X's play of one kind of deck)
            --games N           how many (default 400)
            --seed N            the first shuffle (default: a new one each run)
            --each-shuffle      every game its own shuffle, not each pair of games (on by itself for a recorded self-duel)
            --hosts N           deal the games among N native hosts (default 1, the shared one): one host answers
                                one request at a time, so this is what lets --parallel use the cores; not with --ai-record
            --cards N           which cards a seat won with: the N it did worst and best by, holding one against not
                                (--least N: only cards held, and not held, in that many games; default 60)
            --ai-record DIR     the native host records every game that ends as AI training shards there
                                (docs/ai-player-plan.md, "Data"); --ai-record-mode full|decklist|hidden (default full),
                                --ai-record-threads N, --ai-record-games N and --ai-record-mb N are passed on to it
            --sprt              stop once the sequential test says: H0 A wins 50%, H1 55%, alpha = beta = 0.05, a draw
                                half a game each way; --games is then the most it may take; --sprt-h1 P another H1
                                (0.52: about 3,700 games to a verdict at 50% or 52%, more between)
            a neural seat       from cantrip-ai-host beside each worker's own native host (docs/ai-host.md):
                                neural[:full|:decklist|:hidden][@N|@Nms] - the mode (Full by default), and a search of N
                                simulations or N ms an answer (none: the network alone), so --a neural@64 --b neural is
                                search against the same network alone; --model DIR (default ../ai-data/models/gen0),
                                --model-b DIR (the b seat's neural seats play this model instead: a generation's gate),
                                --ai-backend ort|tract (default: the build's), --ai-server HOST:PORT (evaluate on
                                the inference server there, python -m cantrip_ai.serve), --ai-deadline MS (default none),
                                --ai-echo (check every mirrored reply whole); a searching seat's --ai-worlds N,
                                --ai-batch N, --ai-cpuct X, --ai-prior-temperature X, --ai-prune true|false,
                                --ai-prune-bad X, --ai-prune-margin X, --ai-prune-after N,
                                --ai-prune-after-replies N (default: the host's)

          watch   plays one game and tells it turn by turn, as seat 1 saw it
            --a NAME --b NAME   as for duel (default: tactical against itself)
            --deck NAME --seed N
            --lost              the first game from that shuffle on that seat 1 loses
            --game N            game N of a duel again: give the duel's --seed, --a, --b, decks and --each-shuffle with it
            --limit N           call the game stuck after N answers; --tail N tells only its last N lines
            --narrate FILE      the --a seat (a tactical one) writes there each decision it made and what it weighed

          train   plays the tactical seat against itself and fits its model to who won
            --rounds N          rounds of play, fit and check (default 3)
            --games N           games played per round (default 4000)
            --check N           games the fitted model then plays against what it came from (default 1000)
            --explore P         the share of decisions drawn at random while playing (default 0.03)
            --from WHICH        prior, shipped, or a path (default prior)
            --out PATH          where the model is written (default: the one the client ships)
            --deck NAME         as for duel
            --only A,B          fit only the weights whose names start so, every other weight kept as it came: a new
                                feature weighed against the weights that ship, not against a refit of all of them
            --without A,B       also fit the same games with the features whose names start so left out and weighed
                                at nothing, written beside --out: what to duel a new feature's fit against

          tune    moves every weight, policy included, towards winning, by playing the model against itself nudged
            --steps N           how many nudges (default 300)
            --games N           games played per step (default 200)
            --check N           games the tuned model then plays against what it started from (default 2000)
            --from WHICH        prior, shipped, or a path (default shipped)
            --no-cards          tune the general and policy weights only, not each card's own
            --hold A,B          leave these weights where they are (the behaviours on by judgement)
            --against SEAT      what the tuned model then has to beat, as a duel names a seat (default: what it started from)
            --out PATH          where the model is written if it won (default: the one the client ships)
            --deck NAME         as for duel; --decks DIR (more than once) a pool, each step drawing its own decks

          read    lists the instants and sorceries of a dataset the tactical seat cannot read (--all: every one, as read)

          champion  keeps frozen builds of the tactical seat to play today's against, in .cantrip-lab\champions
            freeze NAME         copies this build's client there under that name (--from DIR: a build in another
                                folder; --commit ID and --note TEXT say what it is). Seat it as champion:NAME
            use NAME            makes it the seat named 'champion': the last pilot landed, which a change has to beat
            referee NAME        makes it the seat named 'referee': the pilot the draft lab's numbers are taken with
            list                says which there are

          soak    plays games to find the ones that do not finish, and says nothing else: fuzzing, not measurement
            --games N           how many (default 2000), shared between the decks below
            --deck NAME         a deck to play as a mirror (default demo/card-test, which has every card)
            --decks DIR         a pool of decks as well, each a mirror
            --a NAME --b NAME   as for duel (default: tactical in both seats)
            --seed N            the first shuffle (default: a new one each run)

          draft   drafts a cube with two ways of drafting in alternate seats, and says what their decks are
            --a NAME --b NAME   new, plain (the pick order and nothing else), new-picks, new-deck, or
                                new:Sureness=1.5,Signals=0 to set the new drafter's numbers (default: new, plain).
                                A number of the deck's is drafted towards and built by; Build.Creatures=15 only
                                builds by it (the same picks), Draft.Creatures=15 only drafts towards it, and
                                Varied=1 draws each seat's deck numbers afresh, for draft shape to read
            --format WHICH      booster (default), grid (two seats, eighteen grids of nine) or winston (two
                                seats, ninety cards in three piles, as the server deals them)
            --apart             each way of drafting has the table to itself: the same packs drafted by a table
                                of one and by a table of the other, and the decks of one play the other's
            --pods N            how many tables of eight (default 50)
            --games N           games each deck of one plays each deck of the other in its pod, the tactical seat
                                playing both (default 0: no games)
            --pilot WHICH       how the tactical seat plays them: referee, champion, champion:NAME (a frozen
                                pilot), tactical (as built), prior, or a path to a model (default referee;
                                with --out the games' pilot is written to pilot.txt)
            --cards N           with games: the N cards whose decks did worst and best, for finding what the
                                seat playing them cannot use
            --cube NAME         a cube on docs\cube-lists (default LSVCube)
            --no-order          the server sends no pick order: every card is worth what it reads as
            --out DIR           writes every deck there as a decklist, with a decks.csv of what each deck is, and
                                with --games how it did, and a games.csv of who beat whom
            --keep WHICH        writes only the decks that are name:least..most,... by decks.csv's columns from
                                lands on, and share with games: counterspells:2..  or  control:1..  or
                                creatures:..9,lean:..0  or  score:95..,share:..0.25. A deck is one of combo,
                                control, ramp, aggro, midrange (each a column, 1 or 0)
            --limit N           writes no more than N decks
            --seed N            the first pod's packs

          draft watch       one draft told pick by pick as one seat thought it, as a bot seat narrates, and its deck
                            (--seat N, --seed N, --cube NAME, --format booster|grid|winston, --others NAME,
                            --tuning ...); --event ID [--data DIR] replays a stored booster draft's seat instead
          draft calibrate   measures what a pack usually still holds, for the drafter's tables  (--pods N)
          draft fit         fits what a card is worth by its text to the pick orders, and prints the weights
          draft shape       what about a deck wins games, fitted to the decks.csv and games.csv of earlier runs
                            (--from DIR, more than once; --features cast,creatures,... as decks.csv spells
                            them, and side where the two ways of drafting differ; --draws N for the margins;
                            --test DIR for games to try the fit on; --cards N for the same fit card by card, the
                            N the pilot gets least and most from)
          draft paired      two runs of the same packs and shuffles (the same --pods and --seed) set side by
                            side pod by pod: --base DIR --variant DIR, each an earlier run's --out. The tight
                            test of a change that leaves the other side as it was (a builder-only number)

          --parallel N          games at once, for any of them (default: one per core)
        """);
}
