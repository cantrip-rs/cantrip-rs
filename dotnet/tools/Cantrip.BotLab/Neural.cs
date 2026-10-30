using System.Collections.Concurrent;
using System.Diagnostics;
using System.Text;
using System.Text.Json;
using System.Text.Json.Nodes;
using Cantrip.Client;
using Cantrip.Protocol;
using Cantrip.Server.Rules;

namespace Cantrip.BotLab;

/// <summary>
/// The <c>cantrip-ai-host</c> sidecar (docs/ai-host.md): the AI player beside a table, holding a mirror of each game
/// the table deals. Lines go to it without waiting for their replies, so mirroring a game costs the table nothing;
/// a reader takes the replies in order, checks each mirrored one against the table's, and hands <c>AiResult</c> lines
/// to whoever asked.
/// </summary>
internal sealed class AiHostProcess : IDisposable
{
    private const string BinaryName = "cantrip-ai-host";

    /// <summary>The environment variable naming the binary, when it is not where cargo builds it.</summary>
    public const string PathVariable = "CANTRIP_AI_HOST";

    private readonly Process _process;
    private readonly StreamWriter _input;
    private readonly ConcurrentQueue<Pending> _pending = new();
    private readonly BlockingCollection<string> _results = [];
    private readonly Thread _reader;
    private readonly object _gate = new();
    private string? _divergence;
    private bool _disposed;

    private sealed class Pending
    {
        public required string Command { get; init; }

        /// <summary>The table's reply to the same command, for a mirrored one; null for one of the AI host's own.</summary>
        public string? Table { get; init; }

        public TaskCompletionSource<string>? Reply { get; init; }
    }

    private AiHostProcess(Process process, bool echo)
    {
        _process = process;
        Echo = echo;
        _input = process.StandardInput;
        process.ErrorDataReceived += (_, line) =>
        {
            if (line.Data is not null && !line.Data.StartsWith("cantrip-ai-host: model ", StringComparison.Ordinal))
            {
                Console.Error.WriteLine($"[{BinaryName}] {line.Data}");
            }
        };
        process.BeginErrorReadLine();
        _reader = new Thread(Read) { IsBackground = true, Name = BinaryName };
        _reader.Start();
    }

    /// <summary>Whether each mirrored reply is the table's whole reply, compared with it; else only Ok against Ok.</summary>
    public bool Echo { get; }

    /// <summary>Which lane the host serves, for its failures (set by <see cref="Lane"/>).</summary>
    public int LaneNumber { get; set; }

    /// <summary>
    /// Whether the host once gave no answer in time: it may still be busy with that request, and whatever is asked of it
    /// next waits behind it, so a lane whose host stalled is not used again (<see cref="Lane.Usable"/>).
    /// </summary>
    public bool Stalled { get; private set; }

    /// <summary>The first mirrored reply that was not the table's, or null: a mirror out of step is a bug.</summary>
    public string? Divergence => _divergence;

    public static AiHostProcess Start(string root, IReadOnlyList<string> arguments, bool echo)
    {
        string binary = Locate(root);
        var info = new ProcessStartInfo(binary)
        {
            UseShellExecute = false,
            RedirectStandardInput = true,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            StandardInputEncoding = new UTF8Encoding(encoderShouldEmitUTF8Identifier: false),
            StandardOutputEncoding = Encoding.UTF8,
            StandardErrorEncoding = Encoding.UTF8,
            CreateNoWindow = true,
            WorkingDirectory = root,
        };
        foreach (string argument in arguments)
        {
            info.ArgumentList.Add(argument);
        }

        Process process = Process.Start(info) ?? throw new InvalidOperationException($"{binary} could not be started.");
        return new AiHostProcess(process, echo);
    }

    /// <summary><see cref="PathVariable"/>, else the freshest of the workspace's build output, else the bare name.</summary>
    public static string Locate(string root)
    {
        if (Environment.GetEnvironmentVariable(PathVariable) is { Length: > 0 } configured)
        {
            return configured;
        }

        string file = OperatingSystem.IsWindows() ? BinaryName + ".exe" : BinaryName;
        string? best = null;
        foreach (string profile in (string[])["cards", "release", "debug"])
        {
            string candidate = Path.Combine(root, "target", profile, file);
            if (File.Exists(candidate) && (best is null || File.GetLastWriteTimeUtc(candidate) > File.GetLastWriteTimeUtc(best)))
            {
                best = candidate;
            }
        }

        return best ?? BinaryName;
    }

    /// <summary>Sends a mirrored command, whose reply is checked against <paramref name="table"/>'s when it comes.</summary>
    public void Mirror(string command, string table) => Write(new Pending { Command = command, Table = table });

    /// <summary>Sends one of the AI host's own commands and waits for its reply.</summary>
    public string Call(string command, TimeSpan wait)
    {
        var reply = new TaskCompletionSource<string>(TaskCreationOptions.RunContinuationsAsynchronously);
        Write(new Pending { Command = command, Reply = reply });
        if (reply.Task.Wait(wait))
        {
            return reply.Task.Result;
        }

        Stalled = true;
        throw new TimeoutException($"{BinaryName} did not reply to {command} in {wait.TotalSeconds:N0}s ({Where()}).");
    }

    /// <summary>The next <c>AiResult</c> line.</summary>
    public string Result(TimeSpan wait)
    {
        if (_results.TryTake(out string? line, wait))
        {
            return line;
        }

        Stalled = true;
        throw new TimeoutException($"{BinaryName} made no answer in {wait.TotalSeconds:N0}s ({Where()}).");
    }

    /// <summary>The host as a failure names it: its lane, its process, the time, and what it held then.</summary>
    private string Where()
    {
        string at = $"lane {LaneNumber}, pid {_process.Id}, at {DateTime.Now:yyyy-MM-dd HH:mm:ss}";
        try
        {
            if (_process.HasExited)
            {
                return $"{at}; it exited with {_process.ExitCode}";
            }

            _process.Refresh();
            return $"{at}; private {_process.PrivateMemorySize64 >> 20:N0} MB, working set {_process.WorkingSet64 >> 20:N0} MB, "
                + $"CPU {_process.TotalProcessorTime.TotalSeconds:N0} s";
        }
        catch (Exception error) when (error is InvalidOperationException or System.ComponentModel.Win32Exception)
        {
            return at;
        }
    }

    private void Write(Pending pending)
    {
        lock (_gate)
        {
            ObjectDisposedException.ThrowIf(_disposed, this);
            if (_process.HasExited)
            {
                throw new InvalidOperationException($"{BinaryName} has exited with code {_process.ExitCode}.");
            }

            // Queued before it is written, so the reader never meets a reply it has no request for.
            _pending.Enqueue(pending);
            _input.Write(pending.Command);
            _input.Write('\n');
            _input.Flush();
        }
    }

    private void Read()
    {
        StreamReader output = _process.StandardOutput;
        while (output.ReadLine() is { } line)
        {
            if (line.StartsWith("{\"AiResult\"", StringComparison.Ordinal))
            {
                _results.Add(line);
                continue;
            }

            if (!_pending.TryDequeue(out Pending? pending))
            {
                _divergence ??= $"a reply with no request: {Clip(line)}";
                continue;
            }

            if (pending.Reply is not null)
            {
                pending.Reply.TrySetResult(line);
            }
            else if (pending.Table is not null && Diverges(pending, line) is { } why)
            {
                _divergence ??= why;
            }
        }

        _results.CompleteAdding();
        while (_pending.TryDequeue(out Pending? left))
        {
            left.Reply?.TrySetException(new InvalidOperationException($"{BinaryName} closed its output."));
        }
    }

    private string? Diverges(Pending pending, string line)
    {
        JsonNode? mine = JsonNode.Parse(line);
        JsonNode? table = JsonNode.Parse(pending.Table!);
        bool mineOk = mine?["Ok"] is not null;
        bool tableOk = table?["Ok"] is not null;
        if (mineOk != tableOk)
        {
            return $"{Clip(pending.Command)}: the table said {Clip(pending.Table!)}, the mirror {Clip(line)}";
        }

        // A NewGame's reply is the mirror's own (no decklists); the rest are the table's whole reply when echoed.
        if (Echo && mineOk && !pending.Command.Contains("\"NewGame\"", StringComparison.Ordinal) && !JsonNode.DeepEquals(mine!["Ok"], table!["Ok"]))
        {
            return $"{Clip(pending.Command)}: the mirror's reply is not the table's";
        }

        return null;
    }

    private static string Clip(string text) => text.Length > 300 ? text[..300] + "..." : text;

    public void Dispose()
    {
        lock (_gate)
        {
            if (_disposed)
            {
                return;
            }

            _disposed = true;
            try
            {
                _input.Dispose();
                if (!_process.WaitForExit(TimeSpan.FromSeconds(5)))
                {
                    _process.Kill(entireProcessTree: true);
                }
            }
            catch (Exception error) when (error is InvalidOperationException or IOException or System.ComponentModel.Win32Exception)
            {
                // Already gone.
            }
        }

        _reader.Join(TimeSpan.FromSeconds(5));
        _process.Dispose();
    }
}

/// <summary>
/// The transport a table deals through when a seat is the network's: every exchange goes to the native host, which is
/// the table, and every command that moves a game is sent on to the AI host too, so that its mirror stays the game.
/// How long the native host took is kept, which is what splits an answer's time into the seat's and the game's.
/// </summary>
internal sealed class TeeTransport : IHostTransport
{
    private readonly IHostTransport _native;
    private readonly AiHostProcess _ai;
    private readonly Stopwatch _watch = new();

    public TeeTransport(IHostTransport native, AiHostProcess ai)
    {
        _native = native;
        _ai = ai;
    }

    /// <summary>The native host's id for the game dealt last, which is its mirror's too.</summary>
    public int GameId { get; private set; } = -1;

    /// <summary>Milliseconds spent in the native host in all.</summary>
    public double HostMs { get; private set; }

    public string Exchange(string request)
    {
        _watch.Restart();
        string reply = _native.Exchange(request);
        HostMs += _watch.Elapsed.TotalMilliseconds;

        JsonNode? node = JsonNode.Parse(request);
        switch (node?["Cmd"]?.GetValue<string>())
        {
            case "NewGame":
                if (JsonNode.Parse(reply)?["Ok"]?["GameId"]?.GetValue<int>() is int id)
                {
                    GameId = id;
                    node["GameId"] = id;
                    node["Echo"] = _ai.Echo;
                    _ai.Mirror(node.ToJsonString(), reply);
                }

                break;
            case "Start" or "Answer" or "Concede" or "View" or "Close":
                _ai.Mirror(request, reply);
                break;
        }

        return reply;
    }
}

/// <summary>One worker's pair of hosts: a native host and an AI host beside it, and the transport that joins them.</summary>
internal sealed class Lane : IDisposable
{
    private static int _made;

    public Lane(string root, IReadOnlyList<string> aiArguments, bool echo)
    {
        Native = NativeHostProcess.Start(arguments: NativeHostProcess.SharedArguments);
        Ai = AiHostProcess.Start(root, aiArguments, echo);
        Ai.LaneNumber = Interlocked.Increment(ref _made);
        Tee = new TeeTransport(Native, Ai);
        Rules = new NativeRulesHostFactory(Tee);
    }

    public NativeHostProcess Native { get; }

    public AiHostProcess Ai { get; }

    public TeeTransport Tee { get; }

    public IRulesHostFactory Rules { get; }

    /// <summary>
    /// Whether the lane can play another game: not once its AI host has stalled (<see cref="AiHostProcess.Stalled"/>),
    /// which would leave the next game asking a host still busy with the last one's request.
    /// </summary>
    public bool Usable => !Ai.Stalled;

    public void Dispose()
    {
        Ai.Dispose();
        Native.Dispose();
    }
}

/// <summary>What one of the network's answers cost, as it measured it and as the seat did.</summary>
/// <param name="Decide">From asking the AI host to holding its answer, on the seat's clock (ms).</param>
/// <param name="Total">From the AI host taking the request to its answer, on its clock (ms).</param>
/// <param name="Encode">Of <paramref name="Total"/>, encoding observations (ms).</param>
/// <param name="Evaluate">Of <paramref name="Total"/>, the network (ms).</param>
/// <param name="Steps">The factored steps the answer took (each a pick from the options of one).</param>
/// <param name="Evaluations">The network's evaluations for it.</param>
/// <param name="Seat">Which seat of the run made it: its name on the command line (<c>neural@64</c>).</param>
/// <param name="Simulations">The search's simulations, zero for an answer of the network alone (docs/ai-host.md, "Search").</param>
/// <param name="TreeBytes">What the search's tree held at its peak, in bytes.</param>
/// <param name="Nodes">The search's tree nodes.</param>
/// <param name="LineDepth">The decisions along the search's most visited line, the root's counted.</param>
/// <param name="DeepestDepth">The decisions along its deepest visited line.</param>
internal sealed record NeuralAnswer(
    double Decide, double Total, double Encode, double Evaluate, int Steps, int Evaluations, bool Fallback, bool Late, string Error,
    string Seat = "", int Simulations = 0, long TreeBytes = 0, int Nodes = 0, bool Searched = false, bool Shortcut = false, int Pruned = 0,
    int LineDepth = 0, int DeepestDepth = 0);

/// <summary>
/// A seat played by the network: each question is asked of the AI host beside the table, whose mirror of the game
/// answers it (<c>AiStart</c>), and the answer it makes is sent as the seat's own. With a budget
/// (<see cref="Simulations"/> or <see cref="SearchMs"/>) the AI host searches; without, the network answers alone.
/// </summary>
internal sealed class NeuralPlay : IAutoPlayer
{
    private static readonly TimeSpan Patience = TimeSpan.FromSeconds(120);

    private static long _stale;

    /// <summary>Results passed over for being an earlier request's, across the run.</summary>
    public static long StaleResults => Interlocked.Read(ref _stale);

    /// <summary>Whether <paramref name="r"/> answers another game's request, or an earlier request of this game's.</summary>
    private static bool Stale(JsonElement r, long id, int game) =>
        (r.TryGetProperty("GameId", out JsonElement g) && g.GetInt32() != game)
        || (r.TryGetProperty("RequestId", out JsonElement q) && q.GetInt64() < id);

    /// <summary><c>CANTRIP_AI_TRACE=FILE</c>: every question asked and the answer the network made, a line each.</summary>
    private static readonly TextWriter? Trace =
        Environment.GetEnvironmentVariable("CANTRIP_AI_TRACE") is { Length: > 0 } path ? new StreamWriter(path, append: false) : null;

    public NeuralPlay(string model, string mode, int deadline, ConcurrentBag<NeuralAnswer> answers, NeuralSeat? seat = null)
    {
        Model = model;
        Mode = mode;
        Deadline = deadline;
        Answers = answers;
        Seat = seat ?? new NeuralSeat("neural", mode, 0, 0);
    }

    /// <summary>The seat as the command line named it, with its search budget.</summary>
    public NeuralSeat Seat { get; }

    /// <summary>Search settings every searching seat of the run sends (<c>--ai-worlds</c>, <c>--ai-batch</c>, ...).</summary>
    public static JsonObject SearchSettings { get; set; } = [];

    public string Model { get; }

    public string Mode { get; }

    /// <summary>Milliseconds after which the network is asked nothing more for an answer; zero, no limit.</summary>
    public int Deadline { get; }

    /// <summary>Where each answer's cost is put.</summary>
    public ConcurrentBag<NeuralAnswer> Answers { get; }

    /// <summary>The worker's hosts, given by the game before it starts.</summary>
    public Lane? Lane { get; set; }

    /// <summary>A refused answer is a bug in the mapping, not something to try again: the game stops on it.</summary>
    public bool Retrying => false;

    public bool Act(GameClient client)
    {
        ArgumentNullException.ThrowIfNull(client);
        if (client.IsOver || client.Prompt is not { } asked)
        {
            return false;
        }

        Lane lane = Lane ?? throw new InvalidOperationException("A neural seat plays only at a table with an AI host beside it.");
        if (lane.Ai.Divergence is { } divergence)
        {
            throw new InvalidOperationException($"The AI host's mirror left the game: {divergence}");
        }

        var watch = Stopwatch.StartNew();
        var start = new JsonObject
        {
            ["Cmd"] = "AiStart",
            ["GameId"] = lane.Tee.GameId,
            ["Seat"] = client.Seat,
            ["Model"] = Model,
            ["Mode"] = Mode,
        };
        if (Deadline > 0)
        {
            start["Deadline"] = Deadline;
        }

        if (Seat.Simulations > 0)
        {
            start["Simulations"] = Seat.Simulations;
        }

        if (Seat.SearchMs > 0)
        {
            start["SearchMs"] = Seat.SearchMs;
        }

        if (Seat.Searches)
        {
            foreach ((string key, JsonNode? value) in SearchSettings)
            {
                start[key] = value?.DeepClone();
            }

            // The seat's own settings (neural...+cut), over the run's: two seats of one run searching differently.
            foreach ((string key, JsonNode? value) in Seat.Settings)
            {
                start[key] = value?.DeepClone();
            }
        }

        JsonNode? started = JsonNode.Parse(lane.Ai.Call(start.ToJsonString(), Patience));
        if (started?["Ok"]?["Started"]?["RequestId"]?.GetValue<long>() is not long id)
        {
            throw new InvalidOperationException($"The AI host would not answer request {asked.RequestId}: {started?.ToJsonString()}");
        }

        if (id != asked.RequestId)
        {
            throw new InvalidOperationException($"The AI host's mirror is at request {id}, the seat was asked {asked.RequestId}.");
        }

        // A result for an earlier request (one whose game stopped before it was read) is passed over, so one game's
        // failure is not read into every later game of the lane.
        JsonDocument result = JsonDocument.Parse(lane.Ai.Result(Patience));
        while (Stale(result.RootElement.GetProperty("AiResult"), id, lane.Tee.GameId))
        {
            Interlocked.Increment(ref _stale);
            result.Dispose();
            result = JsonDocument.Parse(lane.Ai.Result(Patience));
        }

        using JsonDocument owned = result;
        double decide = watch.Elapsed.TotalMilliseconds;
        JsonElement r = result.RootElement.GetProperty("AiResult");
        if (r.GetProperty("RequestId").GetInt64() != id || !r.TryGetProperty("Answer", out JsonElement answer))
        {
            throw new InvalidOperationException($"The AI host made no answer to request {id}: {r}");
        }

        static double Ms(JsonElement e, string name) => e.TryGetProperty(name, out JsonElement v) ? v.GetInt64() / 1000.0 : 0;
        static int Count(JsonElement e, string name) => e.TryGetProperty(name, out JsonElement v) ? v.GetInt32() : 0;
        static bool Flag(JsonElement e, string name) => e.TryGetProperty(name, out JsonElement v) && v.GetBoolean();

        Answers.Add(new NeuralAnswer(
            decide,
            Ms(r, "TotalUs"),
            Ms(r, "EncodeUs"),
            Ms(r, "EvaluateUs"),
            Count(r, "Steps"),
            Count(r, "Evaluations"),
            Flag(r, "Fallback"),
            Flag(r, "Late"),
            r.TryGetProperty("Error", out JsonElement error) ? error.GetString() ?? string.Empty : string.Empty,
            Seat.Name,
            r.TryGetProperty("Search", out JsonElement search) ? Count(search, "Simulations") : 0,
            r.TryGetProperty("Search", out JsonElement held) && held.TryGetProperty("TreeBytes", out JsonElement bytes) ? bytes.GetInt64() : 0,
            r.TryGetProperty("Search", out JsonElement tree) ? Count(tree, "Nodes") : 0,
            r.TryGetProperty("Search", out _),
            r.TryGetProperty("Search", out JsonElement cut) && Flag(cut, "Shortcut"),
            r.TryGetProperty("Search", out JsonElement trimmed) ? Count(trimmed, "Pruned") : 0,
            r.TryGetProperty("Search", out JsonElement line) ? Count(line, "LineDecisions") : 0,
            r.TryGetProperty("Search", out JsonElement deepest) ? Count(deepest, "DeepestDecisions") : 0));

        if (Trace is { } trace)
        {
            lock (trace)
            {
                trace.WriteLine($"{lane.Tee.GameId} seat {client.Seat} request {id} {asked.Prompt.GetType().Name}: {answer.GetRawText()} [{(r.TryGetProperty("Factored", out JsonElement f) ? f.GetString() : string.Empty)}]{(Flag(r, "Fallback") ? " FALLBACK" : string.Empty)}");
                trace.Flush();
            }
        }

        client.Answer(ProtocolJson.Deserialize<AnswerPayload>(answer)
            ?? throw new InvalidOperationException($"The AI host's answer to request {id} is empty."));
        return true;
    }
}

/// <summary>
/// A neural seat by name: <c>neural[:full|:decklist|:hidden][@N|@Nms][+SETTING...]</c>, the information mode it plays in
/// (Full by default) and its search budget, <c>N</c> simulations an answer or <c>N</c> milliseconds of searching; with no
/// budget the network answers alone. <c>neural@64</c> against <c>neural</c> is search against the same network alone.
/// Each <c>+SETTING</c> is a search setting of this seat's alone, over the run's (<c>--ai-...</c>): <c>+cut</c> (a
/// trimmed edge is cut for good, the search before 2026-10-27) or <c>+reversible</c>, or any <c>AiStart</c> field as
/// <c>+Key=value</c> (a number, true or false).
/// </summary>
internal sealed record NeuralSeat(string Name, string Mode, int Simulations, int SearchMs)
{
    public bool Searches => Simulations > 0 || SearchMs > 0;

    /// <summary>The seat's own search settings, sent with each of its <c>AiStart</c>s.</summary>
    public JsonObject Settings { get; init; } = [];

    public static NeuralSeat? Parse(string name)
    {
        if (!name.StartsWith("neural", StringComparison.Ordinal))
        {
            return null;
        }

        string[] plus = name.Split('+');
        if (plus.Length > 1)
        {
            NeuralSeat seat = Parse(plus[0]) ?? throw new ArgumentException($"No neural seat '{name}'.");
            var settings = new JsonObject();
            foreach (string setting in plus.Skip(1))
            {
                switch (setting)
                {
                    case "cut":
                        settings["PruneReversible"] = false;
                        break;
                    case "reversible":
                        settings["PruneReversible"] = true;
                        break;
                    default:
                        string[] pair = setting.Split('=', 2);
                        if (pair.Length != 2 || pair[0].Length == 0)
                        {
                            throw new ArgumentException($"No search setting '{setting}' in '{name}': +cut, +reversible or +Key=value.");
                        }

                        settings[pair[0]] = bool.TryParse(pair[1], out bool flag) ? flag
                            : double.TryParse(pair[1], System.Globalization.NumberStyles.Float, System.Globalization.CultureInfo.InvariantCulture, out double number) ? number
                            : pair[1];
                        break;
                }
            }

            return seat with { Name = name, Settings = settings };
        }

        string[] parts = name.Split('@', 2);
        string mode = parts[0] switch
        {
            "neural" or "neural:full" => "Full",
            "neural:decklist" => "Decklist",
            "neural:hidden" => "Hidden",
            _ => throw new ArgumentException($"No neural seat '{name}': neural[:full|:decklist|:hidden][@N|@Nms]."),
        };
        if (parts.Length == 1)
        {
            return new NeuralSeat(name, mode, 0, 0);
        }

        string budget = parts[1];
        bool ms = budget.EndsWith("ms", StringComparison.Ordinal);
        if (!int.TryParse(ms ? budget[..^2] : budget, System.Globalization.NumberStyles.None, System.Globalization.CultureInfo.InvariantCulture, out int n) || n <= 0)
        {
            throw new ArgumentException($"No search budget '{budget}' in '{name}': @N simulations or @Nms.");
        }

        return ms ? new NeuralSeat(name, mode, 0, n) : new NeuralSeat(name, mode, n, 0);
    }
}

/// <summary>
/// How a run's <c>neural</c> seats are played: <c>--model DIR</c>, <c>--ai-backend</c>, <c>--ai-deadline MS</c>,
/// <c>--ai-echo</c>, and for searching seats <c>--ai-worlds N</c>, <c>--ai-batch N</c>, <c>--ai-cpuct X</c>,
/// <c>--ai-prior-temperature X</c>, <c>--ai-prune true|false</c>, <c>--ai-prune-bad X</c>, <c>--ai-prune-margin X</c>,
/// <c>--ai-prune-after N</c> and <c>--ai-prune-after-replies N</c>; <c>--model-b DIR</c> plays the b seat's neural seats with a second model (a
/// generation's gate: the candidate against the champion, docs/ai-player-plan.md "Phase 4"). The AI host is given
/// <c>--ai-prior DIR</c> (the metagame's decks, <c>decks/</c> by default) as its prior, which a Hidden search needs.
/// <c>--ai-server HOST:PORT</c> evaluates on the inference server there (<c>python -m cantrip_ai.serve</c>, serving
/// every model the run names) instead of in each AI host: the AI host is given <c>--backend remote --server</c>.
/// </summary>
internal sealed record NeuralOptions(string Model, string Backend, int Deadline, bool Echo, string Prior, string ModelB = "", string Server = "", string ModelC = "")
{
    /// <summary>The model's name, as the AI host knows it: its directory's.</summary>
    public string Name => Path.GetFileName(Path.TrimEndingDirectorySeparator(Model));

    /// <summary>The b seat's model's name (<c>--model-b</c>), or the one model's.</summary>
    public string NameB => ModelB.Length > 0 ? Path.GetFileName(Path.TrimEndingDirectorySeparator(ModelB)) : Name;

    /// <summary>The model a seat plays with, and the seat's name in the splits: with two models, the model's name added.</summary>
    public (string Model, string Seat) For(string seat, bool b) =>
        ModelB.Length == 0 ? (Name, seat) : b ? (NameB, $"{seat}/{NameB}") : (Name, $"{seat}/{Name}");

    /// <summary>The AI host's command line: the model (and <c>--model-b</c>'s), one worker, one inference thread (a game a core).</summary>
    public IReadOnlyList<string> Arguments =>
        ["--model", Model, .. ModelB.Length > 0 ? (string[])["--model", ModelB] : [], .. ModelC.Length > 0 ? (string[])["--model", ModelC] : [],
            .. Server.Length > 0 ? (string[])["--backend", "remote", "--server", Server] : Backend.Length > 0 ? (string[])["--backend", Backend] : [],
            "--threads", "1", "--workers", "1",
            .. Prior.Length > 0 ? (string[])["--prior", Prior] : []];

    public static NeuralOptions From(string root, IReadOnlyList<string> args)
    {
        var settings = new JsonObject();
        if (Program.Number(args, "--ai-worlds", 0) is > 0 and int worlds)
        {
            settings["Worlds"] = worlds;
        }

        if (Program.Number(args, "--ai-batch", 0) is > 0 and int batch)
        {
            settings["Batch"] = batch;
        }

        if (Program.Text(args, "--ai-prune", string.Empty) is { Length: > 0 } pruneText
            && bool.TryParse(pruneText, out bool prune))
        {
            settings["Prune"] = prune;
        }

        foreach ((string flag, string key) in (ReadOnlySpan<(string, string)>)[("--ai-prune-after", "PruneAfter"), ("--ai-prune-after-replies", "PruneAfterReplies")])
        {
            if (Program.Number(args, flag, -1) is >= 0 and int count)
            {
                settings[key] = count;
            }
        }

        foreach ((string flag, string key) in (ReadOnlySpan<(string, string)>)[
                     ("--ai-cpuct", "CPuct"),
                     ("--ai-prior-temperature", "PriorTemperature"),
                     ("--ai-prune-bad", "PruneBad"),
                     ("--ai-prune-margin", "PruneMargin")])
        {
            if (Program.Text(args, flag, string.Empty) is { Length: > 0 } text)
            {
                settings[key] = double.Parse(text, System.Globalization.CultureInfo.InvariantCulture);
            }
        }

        NeuralPlay.SearchSettings = settings;
        string modelB = Program.Text(args, "--model-b", string.Empty);
        // --model-c DIR: a third model loaded beside them, named by a seat's own setting (+Model=, +ValueModel=).
        string modelC = Program.Text(args, "--model-c", string.Empty);
        return new(
            Path.GetFullPath(Program.Text(args, "--model", Path.Combine(root, "..", "ai-data", "models", "gen0"))),
            Program.Text(args, "--ai-backend", string.Empty),
            Program.Number(args, "--ai-deadline", 0),
            args.Contains("--ai-echo"),
            Path.GetFullPath(Program.Text(args, "--ai-prior", Path.Combine(root, "decks"))),
            modelB.Length > 0 ? Path.GetFullPath(modelB) : string.Empty,
            Program.Text(args, "--ai-server", string.Empty),
            modelC.Length > 0 ? Path.GetFullPath(modelC) : string.Empty);
    }
}
