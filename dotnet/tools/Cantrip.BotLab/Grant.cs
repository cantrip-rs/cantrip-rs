using System.Diagnostics;
using System.Globalization;
using System.Text;
using System.Text.Json;
using System.Text.Json.Nodes;
using Cantrip.Server;

namespace Cantrip.BotLab;

/// <summary>
/// A duel played as one host's part of a distributed one (docs/ai-training.md, "Distributed duels"): the games its grant
/// file grants, read again as it grows until it says <c>end</c>, as <c>cantrip-selfplay --grant</c> reads its own
/// (the same file format), every result closed into the run's output directory as it is played.
/// <para>
/// <c>duel --grant FILE --out DIR</c>: each finished game's result line (<see cref="Program.ResultRow"/>) goes to an open
/// <c>RUN-K.results.tsv.part</c>, renamed to <c>RUN-K.results.tsv</c> (closed) every <c>--chunk-games</c> results (25)
/// or <c>--chunk-seconds</c> (120) and at the end, so the driver takes them as they close; a file's first line is
/// <c># </c> and the run's header (<see cref="Header"/>), its second the columns. With <c>--ai-record NAME</c> the
/// results are held to the end and closed with the recordings, in one directory <c>RUN.replay</c> holding
/// <c>results.tsv</c> and the recordings under <c>NAME</c> (the native host writes them as games end; a result is only
/// ever handed over beside its recording). The run writes <c>duel-status.json</c> every few seconds and
/// <c>duel-report.json</c> at its end; <c>duel.stop</c> in the directory stops it (no new game starts, those in play
/// finish; saying <c>now</c>, it closes what it has and exits at once), and so does a lease (<c>--lease FILE
/// --lease-seconds S</c>) not touched for that long. It exits 0 once its grant is ended and every game granted is
/// played, 3 when it was stopped. <c>duel --recover --out DIR</c> closes what a run that died left open.
/// </para>
/// </summary>
internal sealed class GrantRun
{
    public const string StatusFile = "duel-status.json";
    public const string ReportFile = "duel-report.json";
    public const string StopFile = "duel.stop";
    public const string Results = ".results.tsv";
    public const string Replay = ".replay";
    public const string Columns = "game\tpair\tshuffle\ta_seat\tdeck1\tdeck2\twinner\ta_points\tstarter\tturns";
    public const int Stopped = 3;

    private static readonly TimeSpan GrantPoll = TimeSpan.FromMilliseconds(500);
    private static readonly TimeSpan StatusEvery = TimeSpan.FromSeconds(5);

    private readonly object gate = new();
    private readonly Queue<int> queue = new();
    private readonly HashSet<int> known = [];
    private readonly List<string> open = [];
    private string? openFile;
    private readonly Stopwatch clock = Stopwatch.StartNew();
    private readonly Stopwatch chunkClock = Stopwatch.StartNew();
    private readonly string run = $"r{Guid.NewGuid():N}"[..9];
    private DateTime readAt = DateTime.MinValue;
    private bool ended;
    private int chunks;
    private int granted;
    private int taken;
    private int played;
    private int failed;
    private int inPlay;
    private string? stopped;
    private Thread? watcher;

    private GrantRun(string grant, string output, int games, string header, string host, string lease, double leaseSeconds, int chunkGames, double chunkSeconds, string record)
    {
        Grant = grant;
        Output = output;
        Games = games;
        HeaderLine = header;
        Host = host;
        Lease = lease;
        LeaseSeconds = leaseSeconds;
        ChunkGames = Math.Max(1, chunkGames);
        ChunkSeconds = chunkSeconds;
        Record = record;
    }

    public string Grant { get; }

    public string Output { get; }

    public int Games { get; }

    public string HeaderLine { get; }

    public string Host { get; }

    public string Lease { get; }

    public double LeaseSeconds { get; }

    public int ChunkGames { get; }

    public double ChunkSeconds { get; }

    /// <summary>The recordings' directory name within the run's replay directory (<c>--ai-record</c>), or empty.</summary>
    public string Record { get; }

    /// <summary>The run's replay directory while it is open: where the native host records.</summary>
    public string ReplayPart => Path.Combine(Output, $"{run}{Replay}.part");

    /// <summary>The run's parts from its command line, or null without <c>--grant</c>.</summary>
    public static GrantRun? From(IReadOnlyList<string> args, int games, string header)
    {
        string grant = Program.Text(args, "--grant", string.Empty);
        if (grant.Length == 0)
        {
            return null;
        }

        string output = Program.Text(args, "--out", string.Empty);
        if (output.Length == 0)
        {
            throw new ArgumentException("--grant needs --out DIR, where its results go.");
        }

        Directory.CreateDirectory(output);
        return new GrantRun(Path.GetFullPath(grant), Path.GetFullPath(output), games, header, Program.Text(args, "--host", Environment.MachineName.ToLowerInvariant()),
            Program.Text(args, "--lease", string.Empty), double.Parse(Program.Text(args, "--lease-seconds", "900"), CultureInfo.InvariantCulture),
            Program.Number(args, "--chunk-games", 25), double.Parse(Program.Text(args, "--chunk-seconds", "120"), CultureInfo.InvariantCulture),
            Program.Text(args, "--ai-record", string.Empty));
    }

    /// <summary>
    /// The header a run's results carry: what the duel is, the same on every host (no path), as JSON with its keys in a
    /// fixed order. The driver asks for it before any game (<c>--print-header</c>) and holds every results file to it.
    /// </summary>
    public static string Header(string a, string b, int games, ulong seed, IReadOnlyList<DeckList> decks, string pairings, bool eachShuffle, bool mirrors,
        IReadOnlyList<string> models)
    {
        var deckText = new StringBuilder();
        foreach (DeckList deck in decks)
        {
            deckText.Append(deck.Name).Append('\n');
            foreach ((string part, IReadOnlyList<DeckListEntry> entries) in (ReadOnlySpan<(string, IReadOnlyList<DeckListEntry>)>)
                     [("deck", deck.Entries), ("commanders", deck.Commanders), ("sideboard", deck.Sideboard)])
            {
                deckText.Append(part).Append('\n');
                foreach (DeckListEntry entry in entries)
                {
                    deckText.Append(entry.Copies.ToString(CultureInfo.InvariantCulture)).Append(' ').Append(entry.Card).Append('\n');
                }
            }

            deckText.Append("companion ").Append(deck.Companion ?? string.Empty).Append('\n');
        }

        var header = new JsonObject
        {
            ["format"] = "duel-results/1",
            ["seeds"] = $"{seed}..{seed + (ulong)games}",
            ["a"] = a,
            ["b"] = b,
            ["decks"] = $"{decks.Count}:{Fnv(deckText.ToString()):x16}",
            ["pairings"] = pairings.Length > 0 ? $"{Fnv(File.ReadAllText(pairings).ReplaceLineEndings("\n")):x16}" : string.Empty,
            ["each_shuffle"] = eachShuffle,
            ["mirrors"] = mirrors,
            ["models"] = new JsonArray([.. models.Select(m => (JsonNode)JsonValue.Create(Model(m))!)]),
        };
        return header.ToJsonString();
    }

    /// <summary>A model as a header names it: its directory's name and its manifest's hash.</summary>
    private static string Model(string directory)
    {
        string manifest = Path.Combine(directory, "manifest.json");
        string name = Path.GetFileName(Path.TrimEndingDirectorySeparator(directory));
        return File.Exists(manifest) ? $"{name}:{Fnv(File.ReadAllText(manifest).ReplaceLineEndings("\n")):x16}" : name;
    }

    private static ulong Fnv(string text)
    {
        ulong hash = 0xcbf29ce484222325;
        foreach (byte b in Encoding.UTF8.GetBytes(text))
        {
            hash = (hash ^ b) * 0x100000001b3;
        }

        return hash;
    }

    /// <summary>
    /// A grant file's games, in its order, and whether it is closed: <c>A..B</c> the games A to B - 1, <c>A</c> one game,
    /// <c>end</c> no more to come, <c>#</c> a comment; only lines that end in a newline count (a file read as it is
    /// written grants nothing it does not mean to). <c>cantrip_ai_host::selfplay::parse_grant</c>'s format.
    /// </summary>
    public static (List<int> Games, bool Ended) ParseGrant(string text, int games)
    {
        List<int> granted = [];
        bool ended = false;
        int at = 0;
        while (text.IndexOf('\n', at) is var newline and >= 0)
        {
            string line = text[at..newline].Trim();
            at = newline + 1;
            if (line.Length == 0 || line.StartsWith('#'))
            {
                continue;
            }

            if (line == "end")
            {
                ended = true;
                continue;
            }

            int Number(string word) => int.TryParse(word.Trim(), NumberStyles.None, CultureInfo.InvariantCulture, out int n)
                ? n
                : throw new FormatException($"a grant line is A..B, A or end, not {line}");
            int dots = line.IndexOf("..", StringComparison.Ordinal);
            (int from, int to) = dots >= 0 ? (Number(line[..dots]), Number(line[(dots + 2)..])) : (Number(line), Number(line) + 1);
            if (to > games)
            {
                throw new FormatException($"the grant {line} passes the run's {games} games");
            }

            for (int game = from; game < to; game++)
            {
                granted.Add(game);
            }
        }

        return (granted, ended);
    }

    /// <summary>Starts the run's watch: its status file, its stop file and its lease.</summary>
    public void Start()
    {
        WriteStatus(finished: false);
        // Ctrl-C (the driver's console, which a part played here shares) stops it as its stop file does: the first
        // gracefully, the second at once.
        Console.CancelKeyPress += (_, e) =>
        {
            e.Cancel = true;
            lock (gate)
            {
                interrupts++;
            }
        };
        watcher = new Thread(Watch) { IsBackground = true, Name = "grant watch" };
        watcher.Start();
    }

    private int interrupts;

    /// <summary>
    /// The next game to play: the next granted one, waiting while the grant may grow; null once the grant is ended and
    /// handed out, or the run is stopping.
    /// </summary>
    public int? Next()
    {
        while (true)
        {
            lock (gate)
            {
                if (stopped is not null)
                {
                    return null;
                }

                if (queue.TryDequeue(out int game))
                {
                    taken++;
                    inPlay++;
                    return game;
                }

                if (ended)
                {
                    return null;
                }

                if (DateTime.UtcNow - readAt >= GrantPoll)
                {
                    readAt = DateTime.UtcNow;
                    ReadGrant();
                    if (queue.Count > 0 || ended)
                    {
                        continue;
                    }
                }
            }

            Thread.Sleep(100);
        }
    }

    private void ReadGrant()
    {
        string text;
        try
        {
            text = File.ReadAllText(Grant);
        }
        catch (IOException)
        {
            return;
        }
        catch (UnauthorizedAccessException)
        {
            return;
        }

        (List<int> games, bool closed) = ParseGrant(text, Games);
        foreach (int game in games)
        {
            if (known.Add(game))
            {
                granted++;
                queue.Enqueue(game);
            }
        }

        ended = closed;
    }

    /// <summary>A game played (or not finished): its result line kept, and the open results closed when they are due.</summary>
    public void Played(string row, bool finished)
    {
        lock (gate)
        {
            inPlay--;
            if (finished)
            {
                played++;
            }
            else
            {
                failed++;
            }

            open.Add(row);
            if (Record.Length > 0)
            {
                return;
            }

            // On disk as it is played, so a run that dies keeps it (`Recover`).
            if (openFile is null)
            {
                openFile = Path.Combine(Output, $"{run}-{chunks++:D4}{Results}.part");
                File.WriteAllText(openFile, Text(HeaderLine, []));
            }

            File.AppendAllText(openFile, row + "\n");
            if (open.Count >= ChunkGames || chunkClock.Elapsed.TotalSeconds >= ChunkSeconds)
            {
                CloseChunk();
            }
        }
    }

    /// <summary>The open results file closed (under the gate): renamed to its closed name.</summary>
    private void CloseChunk()
    {
        chunkClock.Restart();
        if (openFile is null)
        {
            return;
        }

        File.Move(openFile, openFile[..^".part".Length], overwrite: true);
        openFile = null;
        open.Clear();
    }

    /// <summary>A results file's text: its header line, its columns, its rows.</summary>
    public static string Text(string header, IEnumerable<string> rows) =>
        $"# {header}\n{Columns}\n" + string.Concat(rows.Select(row => row + "\n"));

    private static void WriteWhole(string path, string text)
    {
        File.WriteAllText(path + ".tmp", text);
        File.Move(path + ".tmp", path, overwrite: true);
    }

    /// <summary>
    /// The run's end, after its games (and with a recording, after the native host closed it): the results closed
    /// (a replay's beside its recordings), the report written. Returns the exit code: 0 when the grant was ended and
    /// all of it played, 3 when the run was stopped.
    /// </summary>
    public int Finish()
    {
        lock (gate)
        {
            if (Record.Length > 0)
            {
                if (open.Count > 0)
                {
                    Directory.CreateDirectory(ReplayPart);
                    WriteWhole(Path.Combine(ReplayPart, "results.tsv"), Text(HeaderLine, open));
                    open.Clear();
                    Directory.Move(ReplayPart, ReplayPart[..^".part".Length]);
                }
            }
            else
            {
                CloseChunk();
            }

            stopped ??= ended && queue.Count == 0 ? null : "stopped";
            WriteReport();
            WriteStatus(finished: true);
            return stopped is null ? 0 : Stopped;
        }
    }

    private void Watch()
    {
        DateTime statusAt = DateTime.UtcNow;
        while (true)
        {
            Thread.Sleep(500);
            string? word = null;
            try
            {
                string stop = Path.Combine(Output, StopFile);
                if (File.Exists(stop))
                {
                    word = File.ReadAllText(stop).Trim();
                }
            }
            catch (IOException)
            {
                word = string.Empty;
            }

            string? why = word is null ? null : word == "now" ? "now" : "asked";
            lock (gate)
            {
                why = interrupts >= 2 ? "now" : interrupts == 1 && why is null ? "interrupted" : why;
            }

            if (why is null && Lease.Length > 0)
            {
                DateTime touched = File.Exists(Lease) ? File.GetLastWriteTimeUtc(Lease) : DateTime.MinValue;
                if ((DateTime.UtcNow - touched).TotalSeconds > LeaseSeconds)
                {
                    why = "its lease ran out";
                }
            }

            lock (gate)
            {
                if (why is not null && stopped is null)
                {
                    stopped = why;
                    Console.WriteLine($"Stopping: {why}.");
                }

                if (why == "now" && Record.Length == 0)
                {
                    // Its games in flight are given up: what is played is closed, and the run ends now.
                    CloseChunk();
                    WriteReport();
                    WriteStatus(finished: true);
                    Console.Out.Flush();
                    Environment.Exit(Stopped);
                }

                if (DateTime.UtcNow - statusAt >= StatusEvery)
                {
                    statusAt = DateTime.UtcNow;
                    WriteStatus(finished: false);
                }
            }
        }
    }

    private double Hours => Math.Max(clock.Elapsed.TotalHours, 1e-9);

    private void WriteStatus(bool finished)
    {
        var status = new JsonObject
        {
            ["Pid"] = Environment.ProcessId,
            ["Host"] = Host,
            ["Granted"] = granted,
            ["Taken"] = taken,
            ["GamesThisRun"] = played,
            ["Dropped"] = failed,
            ["Seconds"] = Math.Round(clock.Elapsed.TotalSeconds, 1),
            ["GamesPerHour"] = Math.Round(played / Hours, 1),
            ["Ended"] = ended,
            ["Finished"] = finished,
            ["Governor"] = new JsonObject
            {
                ["LanesAllowed"] = Program.Lanes,
                ["LanesCeiling"] = Program.Lanes,
                ["Running"] = inPlay,
            },
        };
        try
        {
            WriteWhole(Path.Combine(Output, StatusFile), status.ToJsonString());
        }
        catch (IOException)
        {
            // Read this moment: written again in a few seconds.
        }
    }

    private void WriteReport()
    {
        var report = new JsonObject
        {
            ["Host"] = Host,
            ["Games"] = played,
            ["GamesThisRun"] = played,
            ["Dropped"] = failed,
            ["Seconds"] = Math.Round(clock.Elapsed.TotalSeconds, 1),
            ["GamesPerHour"] = Math.Round(played / Hours, 1),
            ["Stopped"] = stopped is null ? null : new JsonObject { ["Reason"] = stopped },
        };
        WriteWhole(Path.Combine(Output, ReportFile), report.ToJsonString(new JsonSerializerOptions { WriteIndented = true }));
    }

    /// <summary>
    /// What a run that died left open in <paramref name="output"/>, closed: each open results file's whole lines into a
    /// closed one beside it (<c>NAME.recovered.results.tsv</c>), then the open one removed. A replay's open directory is
    /// removed: its recordings may be cut, so its games are played again. Returns how many results were kept.
    /// </summary>
    public static int Recover(string output)
    {
        int kept = 0;
        if (!Directory.Exists(output))
        {
            return 0;
        }

        foreach (string part in Directory.GetFiles(output, "*" + Results + ".part"))
        {
            string text = File.ReadAllText(part);
            string[] lines = text[..(text.LastIndexOf('\n') + 1)].Split('\n', StringSplitOptions.RemoveEmptyEntries);
            string[] rows = [.. lines.Where(line => !line.StartsWith('#') && line != Columns)];
            string? header = lines.FirstOrDefault(line => line.StartsWith("# ", StringComparison.Ordinal));
            if (rows.Length > 0 && header is not null)
            {
                string name = Path.GetFileName(part)[..^(Results.Length + ".part".Length)];
                WriteWhole(Path.Combine(output, $"{name}.recovered{Results}"), Text(header[2..], rows));
                kept += rows.Length;
            }

            File.Delete(part);
        }

        foreach (string part in Directory.GetDirectories(output, "*" + Replay + ".part"))
        {
            Directory.Delete(part, recursive: true);
        }

        return kept;
    }
}
