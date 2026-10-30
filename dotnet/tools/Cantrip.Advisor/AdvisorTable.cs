using System.Globalization;
using System.Text.Json;
using System.Text.Json.Nodes;
using Cantrip.Art;
using Cantrip.Client;
using Cantrip.Client.Tactics;
using Cantrip.Protocol;
using Cantrip.Server;
using Cantrip.Server.Host;
using Cantrip.Server.Rules;
using MessagePack;

namespace Cantrip.Advisor;

/// <summary>How an advisor's table is set up.</summary>
public sealed class AdvisorOptions
{
    /// <summary>The repository root: content, decks and build output are found under it.</summary>
    public required string Root { get; init; }

    /// <summary>The person's decklist (a path), and the bot's.</summary>
    public required string MyDeck { get; init; }

    public required string TheirDeck { get; init; }

    /// <summary><c>neural</c> (the network's search, through the advisor's AI host) or <c>tactical</c>.</summary>
    public string Opponent { get; init; } = "neural";

    /// <summary>The model directory the advice and a neural opponent play with.</summary>
    public required string Model { get; init; }

    /// <summary>The inference server (<c>HOST:PORT</c>) the AI host evaluates on; none, it evaluates on the CPU (tract).</summary>
    public string? InferenceServer { get; init; }

    /// <summary>The search budget of each piece of advice, and of the neural opponent's answers, in milliseconds.</summary>
    public int BudgetMs { get; init; } = 3000;

    public int OpponentBudgetMs { get; init; } = 3000;

    /// <summary>Worlds a Decklist search deals.</summary>
    public int Worlds { get; init; } = 8;

    public ulong Seed { get; init; } = 1;

    /// <summary>Where sessions are recorded: a folder per date, one per game under it.</summary>
    public required string SessionsRoot { get; init; }

    /// <summary>The page's port (the client's files and the advisor's API); 0 picks one.</summary>
    public int WebPort { get; init; }

    /// <summary>The built web client to serve; none, the checkout's newest build.</summary>
    public string? WebRoot { get; init; }

    /// <summary>The metagame's decks, the AI host's prior.</summary>
    public string? Prior { get; init; }

    /// <summary>Who is answering: kept with every answer, so several people's answers are never merged.</summary>
    public string Rater { get; init; } = "user";

    public string PlayerName { get; init; } = "You";

    /// <summary>Card art: the cache it is rendered to. None serves no art.</summary>
    public string? ArtCache { get; init; }

    public Action<string> Log { get; init; } = Console.WriteLine;
}

/// <summary>
/// The advisor's table: a person in the AI's seat (seat 1) against a bot (seat 2), at a <see cref="GameServer"/> of its
/// own whose rules host is the native engine through <see cref="AdvisorTransport"/>, which tees every command to the
/// advisor's AI host. At every request of the person's seat the AI host is asked what it would do (<c>AiStart</c> with
/// <c>Report</c>, Decklist mode): the page shows it, and the play the person makes, through their own client, is what
/// happens. Every decision is recorded under <see cref="AdvisorOptions.SessionsRoot"/>.
/// <para>Nothing here is a real table's: it serves no other page and changes nothing any other table does.</para>
/// </summary>
public sealed class AdvisorTable : IAsyncDisposable
{
    public const int UserSeat = 1;
    public const int BotSeat = 2;

    private static readonly TimeSpan Patience = TimeSpan.FromSeconds(180);

    private readonly AdvisorOptions _options;
    private readonly object _lock = new();
    private readonly Dictionary<int, Decision> _decisions = [];
    private readonly List<Decision> _order = [];
    private readonly System.Collections.Concurrent.BlockingCollection<JsonNode> _opponentResults = [];
    private readonly CancellationTokenSource _stop = new();
    private readonly string _modelName;
    private AdvisorAiHost _ai = null!;
    private NativeHostProcess _native = null!;
    private GameServer _server = null!;
    private Microsoft.AspNetCore.Builder.WebApplication? _art;
    private Task _opponent = Task.CompletedTask;
    private TaskCompletionSource _changed = new(TaskCreationOptions.RunContinuationsAsynchronously);
    private int _version;
    private int _gameId = -1;
    private int _commands;
    private bool _over;
    private string _result = string.Empty;
    private JsonNode? _userView;
    private string _sessionDir = string.Empty;
    private string? _divergence;

    /// <summary>The settings the person changes on the page.</summary>
    private bool _hidden;
    private bool _autoTrivial = true;

    private AdvisorTable(AdvisorOptions options)
    {
        _options = options;
        _modelName = Path.GetFileName(Path.TrimEndingDirectorySeparator(options.Model));
        MyDeck = DeckList.Load(options.MyDeck);
        DeckList theirs = DeckList.Load(options.TheirDeck);
        // A table's decks are named apart; a mirror's second copy is the bot's.
        TheirDeck = theirs.Name == MyDeck.Name
            ? new DeckList { Name = theirs.Name + " (bot)", Source = theirs.Source, Entries = theirs.Entries, Sideboard = theirs.Sideboard, Companion = theirs.Companion }
            : theirs;
    }

    public DeckList MyDeck { get; }

    public DeckList TheirDeck { get; }

    /// <summary>The ticket that seats the person (seat 1); the page is opened with it.</summary>
    public string Ticket { get; private set; } = string.Empty;

    /// <summary>The table's WebSocket address.</summary>
    public Uri Table => new($"ws://127.0.0.1:{_server.Port}/game");

    /// <summary>Where this game is recorded.</summary>
    public string SessionDirectory => _sessionDir;

    /// <summary>Completes when the game is over and its records are written.</summary>
    public Task Finished => _finishedRecords.Task;

    private readonly TaskCompletionSource _finishedRecords = new(TaskCreationOptions.RunContinuationsAsynchronously);

    /// <summary>Deals the table and seats the bot; the person's seat waits for their page.</summary>
    public static async Task<AdvisorTable> StartAsync(AdvisorOptions options)
    {
        ArgumentNullException.ThrowIfNull(options);
        var table = new AdvisorTable(options);
        await table.StartCoreAsync().ConfigureAwait(false);
        return table;
    }

    private async Task StartCoreAsync()
    {
        List<string> arguments = ["--model", _options.Model, "--belief", _options.Model, "--workers", "2"];
        arguments.AddRange(_options.InferenceServer is { Length: > 0 } server ? ["--backend", "remote", "--server", server] : ["--backend", "tract"]);
        if (_options.Prior is { Length: > 0 } prior)
        {
            arguments.AddRange(["--prior", prior]);
        }

        _ai = AdvisorAiHost.Start(AdvisorAiHost.Locate(_options.Root), arguments, _options.Root, _options.Log);
        _ai.Result += OnResult;
        _native = NativeHostProcess.Start(log: _options.Log);
        var transport = new AdvisorTransport(_native, this);

        if (_options.ArtCache is { Length: > 0 } cache)
        {
            _art = await ArtServer.StartAsync(new ArtStore(new ArtStoreOptions { ContentRoot = Path.Combine(_options.Root, "content"), CacheDirectory = cache }), "127.0.0.1", 0).ConfigureAwait(false);
        }

        _server = await GameServer.StartAsync(new GameServerOptions
        {
            Port = 0,
            Players = 2,
            Seed = _options.Seed,
            ContentRoot = Path.Combine(_options.Root, "content"),
            Decks = [MyDeck, TheirDeck],
            Rules = new NativeRulesHostFactory(transport),
            PlayerClock = TimeSpan.FromHours(6),
            Art = _art is null ? null : new Uri(_art.Urls.First().TrimEnd('/') + "/"),
        }).ConfigureAwait(false);

        Ticket = _server.Reserve(UserSeat, _options.PlayerName, MyDeck.Name);
        string botTicket = _server.Reserve(BotSeat, _options.Opponent == "tactical" ? "Tactical bot" : $"Neural {_modelName}", TheirDeck.Name);
        _opponent = Task.Run(() => PlayOpponentAsync(botTicket));
    }

    // --- The bot's seat ----------------------------------------------------------------------------------------------

    private async Task PlayOpponentAsync(string ticket)
    {
        try
        {
            var client = new GameClient(await WebSocketTransport.ConnectAsync(Table, _stop.Token).ConfigureAwait(false));
            client.Join(_options.Opponent == "tactical" ? "Tactical bot" : $"Neural {_modelName}", BotSeat, ticket);
            IAutoPlayer player = _options.Opponent == "tactical" ? new TacticalPlay() : new NeuralOpponent(this);
            var seat = new AutoSeat(client, player);
            while (!client.IsOver && !_stop.IsCancellationRequested)
            {
                if (!seat.Step())
                {
                    await Task.Delay(15, _stop.Token).ConfigureAwait(false);
                }

                if (!client.IsConnected && !client.IsOver)
                {
                    client.Pump();
                    if (!client.IsOver)
                    {
                        _options.Log("The bot's seat lost its connection to the table.");
                        return;
                    }
                }
            }
        }
        catch (OperationCanceledException)
        {
        }
        catch (Exception error) when (error is InvalidOperationException or TimeoutException or IOException or System.Net.WebSockets.WebSocketException)
        {
            _options.Log($"The bot's seat stopped: {error.Message}");
        }
    }

    /// <summary>The neural opponent's answer to its request <paramref name="requestId"/>, or null to be asked again.</summary>
    internal AnswerPayload? AskOpponent(int requestId)
    {
        var start = new JsonObject
        {
            ["Cmd"] = "AiStart",
            ["GameId"] = _gameId,
            ["Seat"] = BotSeat,
            ["Model"] = _modelName,
            ["Mode"] = "Decklist",
            ["SearchMs"] = _options.OpponentBudgetMs,
            ["Worlds"] = _options.Worlds,
        };
        JsonNode? started = _ai.Call(start.ToJsonString(), Patience);
        if (started?["Ok"]?["Started"]?["RequestId"]?.GetValue<int>() is not int id || id != requestId)
        {
            // The mirror is not there yet, or the request has gone: the seat is asked again.
            Thread.Sleep(30);
            return null;
        }

        DateTime until = DateTime.UtcNow + Patience;
        while (DateTime.UtcNow < until)
        {
            if (!_opponentResults.TryTake(out JsonNode? result, TimeSpan.FromSeconds(1)))
            {
                continue;
            }

            if (result["RequestId"]?.GetValue<int>() == requestId && result["Answer"] is JsonNode answer)
            {
                return ProtocolJson.Deserialize<AnswerPayload>(answer.ToJsonString());
            }
        }

        throw new TimeoutException($"The AI host made no answer to the bot's request {requestId}.");
    }

    // --- What the table does, as the transport sees it -----------------------------------------------------------------

    /// <summary>A command and the native host's reply, on the table's pump. Mirrored, logged, and read for requests.</summary>
    internal void Exchanged(JsonNode request, string reply)
    {
        string? cmd = request["Cmd"]?.GetValue<string>();
        JsonNode? replyNode = JsonNode.Parse(reply);
        switch (cmd)
        {
            case "NewGame":
                if (replyNode?["Ok"]?["GameId"]?.GetValue<int>() is int id)
                {
                    _gameId = id;
                    request["GameId"] = id;
                    request["Echo"] = true;
                    BeginSession(request);
                    Mirror(request, null);
                }

                break;
            // The checks too: a check may mint the seat's handles, and the mirror's handles must stay the table's.
            case "Start" or "Answer" or "Concede" or "View" or "Close" or "Check" or "CheckBlocks" or "CheckAttacks":
                Mirror(request, replyNode);
                break;
            default:
                return;
        }

        if (replyNode?["Ok"] is not JsonObject ok)
        {
            return;
        }

        if (cmd == "Answer" && request["Seat"]?.GetValue<int>() == UserSeat && request["RequestId"]?.GetValue<int>() is int answered)
        {
            JsonNode payload = request["Answer"]!.DeepClone();
            _ai.Send(new JsonObject { ["Cmd"] = "LastAnswer", ["GameId"] = _gameId }.ToJsonString(), node =>
                Played(answered, payload, node?["Ok"]?["RequestId"]?.GetValue<int>() == answered ? node["Ok"]?["Label"]?.GetValue<string>() : null));
        }

        if (ok["Views"]?[UserSeat.ToString(CultureInfo.InvariantCulture)] is JsonNode view)
        {
            lock (_lock)
            {
                _userView = view.DeepClone();
            }
        }

        if (ok["Requests"] is JsonArray requests)
        {
            foreach (JsonNode? asked in requests)
            {
                if (asked?["Seat"]?.GetValue<int>() == UserSeat && asked["RequestId"]?.GetValue<int>() is int requestId)
                {
                    Asked(requestId, asked);
                }
            }
        }

        if (ok["Finished"]?.GetValue<bool>() == true)
        {
            GameOver(ok);
        }
    }

    /// <summary>
    /// Sends a command to the AI host's mirror, and checks the mirror's reply against the table's (<paramref name="table"/>;
    /// none for a NewGame, whose reply is the mirror's own): a mirror out of step would advise on another game, so the
    /// first difference is kept and shown, and no advice is trusted after it.
    /// </summary>
    private void Mirror(JsonNode request, JsonNode? table)
    {
        string line = request.ToJsonString();
        Action<JsonNode?> check = mine =>
        {
            if (table is null)
            {
                return;
            }

            bool mineOk = mine?["Ok"] is not null;
            bool tableOk = table["Ok"] is not null;
            if (mineOk != tableOk || (mineOk && !JsonNode.DeepEquals(mine!["Ok"], table["Ok"])))
            {
                lock (_lock)
                {
                    _divergence ??= $"after {request["Cmd"]} (command {_commands}): the AI's mirror of the game is not the table's";
                }

                _options.Log($"MIRROR DIVERGED: {Clip(line)} | table: {Clip(table.ToJsonString())} | mirror: {Clip(mine?.ToJsonString() ?? "none")}");
                Changed();
            }
        };
        lock (_lock)
        {
            _commands++;
            if (_sessionDir.Length > 0)
            {
                File.AppendAllText(Path.Combine(_sessionDir, "commands.jsonl"), line + "\n");
            }
        }

        _ai.Send(line, check);
    }

    private static string Clip(string text) => text.Length > 400 ? text[..400] + "..." : text;

    private void BeginSession(JsonNode newGame)
    {
        string date = DateTime.Now.ToString("yyyy-MM-dd", CultureInfo.InvariantCulture);
        string name = $"{DateTime.Now.ToString("HHmmss", CultureInfo.InvariantCulture)}-{Slug(MyDeck.Name)}-vs-{Slug(TheirDeck.Name)}-{_options.Opponent}";
        string dir = Path.Combine(_options.SessionsRoot, date, name);
        Directory.CreateDirectory(Path.Combine(dir, "decisions"));
        lock (_lock)
        {
            _sessionDir = dir;
        }

        WriteGame(newGame["Seed"]?.ToString() ?? string.Empty);
        _options.Log($"Recording to {dir}");
    }

    private static string Slug(string text) =>
        new([.. text.ToLowerInvariant().Select(c => char.IsLetterOrDigit(c) ? c : '-')]);

    private void WriteGame(string seed)
    {
        var game = new JsonObject
        {
            ["Session"] = Path.GetFileName(_sessionDir),
            ["Started"] = DateTime.Now.ToString("o", CultureInfo.InvariantCulture),
            ["Seed"] = seed.Length > 0 ? seed : _options.Seed.ToString(CultureInfo.InvariantCulture),
            ["GameId"] = _gameId,
            ["Rater"] = _options.Rater,
            ["UserSeat"] = UserSeat,
            ["Mode"] = "Decklist",
            ["Opponent"] = _options.Opponent,
            ["Model"] = _options.Model,
            ["BudgetMs"] = _options.BudgetMs,
            ["OpponentBudgetMs"] = _options.Opponent == "neural" ? _options.OpponentBudgetMs : null,
            ["Worlds"] = _options.Worlds,
            ["Decks"] = new JsonObject
            {
                ["1"] = DeckJson(MyDeck),
                ["2"] = DeckJson(TheirDeck),
            },
            ["Over"] = _over,
            ["Result"] = _result,
            ["MirrorDivergence"] = _divergence,
            ["Values"] = new JsonArray([.. _order.Where(d => d.Value is not null).Select(d => (JsonNode)new JsonArray(d.Index, d.Turn, d.Value))]),
            ["Files"] = new JsonObject
            {
                ["commands.jsonl"] = "the native host's commands in order (NewGame, Start, Answer, Concede, View, Close): the game again, exactly, through the native host or an AI host's NewGame Replay",
                ["trace.json"] = "the game as a cantrip_ai::trace (written at the end)",
                ["decisions"] = "one record per decision of seat 1",
            },
        };
        File.WriteAllText(Path.Combine(_sessionDir, "game.json"), game.ToJsonString(Indented));
    }

    private static JsonObject DeckJson(DeckList deck) => new()
    {
        ["Name"] = deck.Name,
        ["Source"] = deck.Source,
        ["Cards"] = new JsonArray([.. deck.Entries.Select(e => (JsonNode)new JsonObject { ["Count"] = e.Copies, ["Name"] = e.Card })]),
    };

    private static readonly JsonSerializerOptions Indented = new() { WriteIndented = true };

    // --- Decisions ------------------------------------------------------------------------------------------------------

    private void Asked(int requestId, JsonNode asked)
    {
        lock (_lock)
        {
            if (_decisions.ContainsKey(requestId))
            {
                return;
            }

            var decision = new Decision
            {
                Index = _order.Count + 1,
                RequestId = requestId,
                CommandIndex = _commands,
                Prompt = asked.DeepClone(),
                View = _userView?.DeepClone(),
                AskedAt = DateTime.UtcNow,
            };
            _decisions[requestId] = decision;
            _order.Add(decision);
        }

        var start = new JsonObject
        {
            ["Cmd"] = "AiStart",
            ["GameId"] = _gameId,
            ["Seat"] = UserSeat,
            ["Model"] = _modelName,
            ["Mode"] = "Decklist",
            ["SearchMs"] = _options.BudgetMs,
            ["Worlds"] = _options.Worlds,
            ["Report"] = true,
        };
        _ai.Send(start.ToJsonString(), node =>
        {
            if (node?["Ok"]?["Started"]?["RequestId"]?.GetValue<int>() is int started && started == requestId)
            {
                return;
            }

            lock (_lock)
            {
                if (_decisions.TryGetValue(requestId, out Decision? d) && d.Analysis is null)
                {
                    d.Error = node?.ToJsonString() ?? "the AI host closed";
                }
            }

            Changed();
        });
        Changed();
    }

    private void OnResult(JsonNode result)
    {
        int seat = result["Seat"]?.GetValue<int>() ?? 0;
        if (seat == BotSeat)
        {
            _opponentResults.Add(result.DeepClone());
            return;
        }

        if (seat != UserSeat || result["RequestId"]?.GetValue<int>() is not int requestId)
        {
            return;
        }

        lock (_lock)
        {
            if (!_decisions.TryGetValue(requestId, out Decision? d))
            {
                return;
            }

            d.AnalysedAt = DateTime.UtcNow;
            d.Analysis = result["Analysis"]?.DeepClone() ?? new JsonObject();
            d.BotPayload = result["Answer"]?.DeepClone();
            if (result["Error"] is JsonNode error)
            {
                d.Error = error.ToString();
            }

            d.Value = d.Analysis["Value"]?.GetValue<double>();
            d.Turn = d.Analysis["Turn"]?.GetValue<int>() ?? 0;
            Settle(d);
        }

        Changed();
    }

    private void Played(int requestId, JsonNode payload, string? label)
    {
        lock (_lock)
        {
            if (!_decisions.TryGetValue(requestId, out Decision? d))
            {
                return;
            }

            d.UserPayload = payload;
            d.UserLabel = label;
            d.AnsweredAt = DateTime.UtcNow;
            d.HiddenAtAnswer = _hidden;
            Settle(d);
        }

        Changed();
    }

    /// <summary>Writes a decision's record once both the AI's analysis and the person's play are in, and after.</summary>
    private void Settle(Decision d)
    {
        if (d.Analysis is null || d.UserPayload is null || _sessionDir.Length == 0)
        {
            return;
        }

        JsonObject record = Record(d);
        string file = Path.Combine(_sessionDir, "decisions", $"{d.Index:D4}-r{d.RequestId}.json");
        File.WriteAllText(file, record.ToJsonString(Indented));
        d.Written = true;
    }

    /// <summary>Whether the person's play is the AI's: the same payload, or the same answer in words.</summary>
    private static bool Same(JsonNode? a, string? aLabel, JsonNode? b, string? bLabel) =>
        (a is not null && b is not null && JsonNode.DeepEquals(a, b)) || (aLabel is not null && aLabel == bLabel);

    private JsonObject Record(Decision d)
    {
        JsonNode analysis = d.Analysis!;
        string? botLabel = analysis["Chosen"]?["Label"]?.GetValue<string>();
        bool agreed = Same(d.UserPayload, d.UserLabel, d.BotPayload, botLabel);
        int? candidate = null;
        if (analysis["Candidates"] is JsonArray candidates)
        {
            for (int i = 0; i < candidates.Count; i++)
            {
                if (Same(d.UserPayload, d.UserLabel, candidates[i]?["Payload"], candidates[i]?["Label"]?.GetValue<string>()))
                {
                    candidate = i;
                    break;
                }
            }
        }

        bool disagreement = analysis["Stats"]?["WorldBestDisagree"]?.GetValue<bool>() == true
            || (analysis["WorldValues"]?["Sd"]?.GetValue<double>() ?? 0) > 0.15;
        bool full = !agreed || d.Flagged || disagreement;

        var observation = new JsonObject
        {
            ["Prompt"] = d.Prompt.DeepClone(),
            ["CommandIndex"] = d.CommandIndex,
            ["Decklists"] = "game.json Decks: both seats' lists (Decklist mode)",
        };
        if (full)
        {
            observation["View"] = d.View?.DeepClone();
        }

        var bot = new JsonObject
        {
            ["Choice"] = botLabel,
            ["Payload"] = d.BotPayload?.DeepClone(),
            ["Value"] = d.Value,
            ["WinPct"] = d.Value is double v ? Math.Round((v + 1) * 50, 1) : null,
            ["Trivial"] = analysis["Trivial"]?.DeepClone(),
            ["TrivialWhy"] = analysis["TrivialWhy"]?.DeepClone(),
            ["Search"] = analysis["Search"]?.DeepClone(),
            ["Error"] = d.Error,
        };

        JsonArray candidatesOut = [];
        if (analysis["Candidates"] is JsonArray all)
        {
            foreach (JsonNode? c in all)
            {
                if (c is null)
                {
                    continue;
                }

                JsonNode copy = c.DeepClone();
                if (!full)
                {
                    copy.AsObject().Remove("PerWorld");
                }

                candidatesOut.Add(copy);
            }
        }

        var record = new JsonObject
        {
            ["Schema"] = 1,
            ["Session"] = Path.GetFileName(_sessionDir),
            ["GameId"] = _gameId,
            ["Index"] = d.Index,
            ["RequestId"] = d.RequestId,
            ["Turn"] = d.Turn,
            ["Kind"] = analysis["Kind"]?.DeepClone(),
            ["Full"] = full,
            ["Observation"] = observation,
            ["Bot"] = bot,
            ["Candidates"] = candidatesOut,
            ["WorldDisagreement"] = new JsonObject
            {
                ["Fusion"] = analysis["Fusion"]?.DeepClone(),
                ["WorldValues"] = analysis["WorldValues"]?.DeepClone(),
            },
            ["Stats"] = analysis["Stats"]?.DeepClone(),
            ["Belief"] = analysis["Belief"]?.DeepClone(),
            ["ValueSoFar"] = new JsonArray([.. _order.Where(o => o.Index <= d.Index && o.Value is not null).Select(o => (JsonNode)new JsonArray(o.Index, o.Turn, o.Value))]),
            ["Answers"] = new JsonArray(new JsonObject
            {
                ["Rater"] = _options.Rater,
                ["Source"] = "advisor",
                ["Payload"] = d.UserPayload?.DeepClone(),
                ["Label"] = d.UserLabel,
                ["Agreed"] = agreed,
                ["Candidate"] = candidate,
                ["Via"] = d.Via,
                ["SuggestionHidden"] = d.HiddenAtAnswer,
                ["BeforeAnalysis"] = d.AnalysedAt is null || d.AnsweredAt < d.AnalysedAt,
                ["AskedAt"] = d.AskedAt.ToString("o", CultureInfo.InvariantCulture),
                ["AnsweredAt"] = d.AnsweredAt?.ToString("o", CultureInfo.InvariantCulture),
                ["Reason"] = d.Reason,
                ["Flagged"] = d.Flagged,
            }),
            ["Truth"] = analysis["Truth"]?.DeepClone(),
        };
        if (full)
        {
            record["Worlds"] = analysis["Worlds"]?.DeepClone();
        }

        return record;
    }

    private void GameOver(JsonObject ok)
    {
        lock (_lock)
        {
            if (_over)
            {
                return;
            }

            _over = true;
            int team = ok["WinningTeam"]?.GetValue<int>() ?? 0;
            _result = $"{ok["Outcome"]}{(team > 0 ? $", seat {team} won" : string.Empty)}";
        }

        _ai.Send(new JsonObject { ["Cmd"] = "Trace", ["GameId"] = _gameId }.ToJsonString(), node =>
        {
            lock (_lock)
            {
                if (_sessionDir.Length > 0)
                {
                    File.WriteAllText(Path.Combine(_sessionDir, "trace.json"), (node?["Ok"]?["Trace"] ?? node ?? new JsonObject()).ToJsonString(Indented));
                    WriteGame(string.Empty);
                }
            }

            _finishedRecords.TrySetResult();
            Changed();
        });
        Changed();
    }

    // --- The page's side ------------------------------------------------------------------------------------------------

    private void Changed()
    {
        TaskCompletionSource done;
        lock (_lock)
        {
            _version++;
            done = _changed;
            _changed = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        }

        done.TrySetResult();
    }

    /// <summary>The page's state once it is newer than <paramref name="since"/>, or after <paramref name="wait"/>.</summary>
    public async Task<JsonObject> StateAsync(int since, TimeSpan wait, CancellationToken cancel)
    {
        Task changed;
        lock (_lock)
        {
            if (_version > since)
            {
                return State();
            }

            changed = _changed.Task;
        }

        try
        {
            await changed.WaitAsync(wait, cancel).ConfigureAwait(false);
        }
        catch (TimeoutException)
        {
        }

        lock (_lock)
        {
            return State();
        }
    }

    /// <summary>What the page is shown: the seat's own view of things, and the true hidden cards only once the game is over.</summary>
    private JsonObject State()
    {
        Decision? current = _order.LastOrDefault();
        var state = new JsonObject
        {
            ["Version"] = _version,
            ["Game"] = new JsonObject
            {
                ["Seat"] = UserSeat,
                ["Opponent"] = _options.Opponent == "tactical" ? "the tactical bot" : $"the neural bot ({_modelName}, {_options.OpponentBudgetMs} ms)",
                ["Model"] = _modelName,
                ["BudgetMs"] = _options.BudgetMs,
                ["MyDeck"] = MyDeck.Name,
                ["TheirDeck"] = TheirDeck.Name,
                ["Over"] = _over,
                ["Result"] = _result,
                ["Session"] = _sessionDir,
                ["Divergence"] = _divergence,
            },
            ["Settings"] = new JsonObject { ["Hidden"] = _hidden, ["AutoTrivial"] = _autoTrivial },
            ["Current"] = current is null ? null : PageDecision(current),
            ["History"] = new JsonArray([.. _order.TakeLast(40).Reverse().Select(d => (JsonNode)new JsonObject
            {
                ["Index"] = d.Index,
                ["RequestId"] = d.RequestId,
                ["Turn"] = d.Turn,
                ["Kind"] = d.Analysis?["Kind"]?.DeepClone(),
                ["Bot"] = d.Analysis?["Chosen"]?["Label"]?.DeepClone(),
                ["You"] = d.UserLabel,
                ["Agreed"] = d.Analysis is null || d.UserPayload is null ? null : Same(d.UserPayload, d.UserLabel, d.BotPayload, d.Analysis["Chosen"]?["Label"]?.GetValue<string>()),
                ["Trivial"] = d.Analysis?["Trivial"]?.DeepClone(),
                ["WinPct"] = d.Value is double v ? Math.Round((v + 1) * 50, 1) : null,
                ["Reason"] = d.Reason,
                ["Flagged"] = d.Flagged,
                ["TheirHand"] = _over ? TheirHand(d) : null,
            })]),
            ["Values"] = new JsonArray([.. _order.Where(d => d.Value is not null).Select(d => (JsonNode)new JsonArray(d.Index, d.Turn, Math.Round((d.Value!.Value + 1) * 50, 1)))]),
        };
        if (_over)
        {
            state["Truth"] = _order.LastOrDefault(d => d.Analysis?["Truth"] is not null)?.Analysis?["Truth"]?.DeepClone();
        }

        return state;
    }

    private static JsonNode? TheirHand(Decision d) =>
        d.Analysis?["Truth"]?["Seats"] is JsonArray seats
            ? seats.FirstOrDefault(s => s?["Seat"]?.GetValue<int>() == BotSeat)?["Hand"]?.DeepClone()
            : null;

    /// <summary>One decision as the page shows it: the advice with each answer's payload ready to send, and no truth.</summary>
    private static JsonObject PageDecision(Decision d)
    {
        var page = new JsonObject
        {
            ["Index"] = d.Index,
            ["RequestId"] = d.RequestId,
            ["Status"] = d.Analysis is not null ? "ready" : d.Error is not null ? "error" : "thinking",
            ["Answered"] = d.UserPayload is not null,
            ["You"] = d.UserLabel,
            ["Reason"] = d.Reason,
            ["Flagged"] = d.Flagged,
            ["Error"] = d.Error,
        };
        if (d.Analysis is not JsonNode a)
        {
            return page;
        }

        page["Turn"] = d.Turn;
        page["Kind"] = a["Kind"]?.DeepClone();
        page["Trivial"] = a["Trivial"]?.DeepClone();
        page["TrivialWhy"] = a["TrivialWhy"]?.DeepClone();
        page["WinPct"] = d.Value is double v ? Math.Round((v + 1) * 50, 1) : null;
        page["Chosen"] = new JsonObject
        {
            ["Label"] = a["Chosen"]?["Label"]?.DeepClone(),
            ["Payload"] = Wire(d.BotPayload),
        };
        page["Search"] = a["Search"]?.DeepClone();
        page["Fusion"] = a["Fusion"]?.DeepClone();
        page["WorldValues"] = a["WorldValues"]?.DeepClone();
        page["Stats"] = a["Stats"]?.DeepClone();
        page["Belief"] = a["Belief"]?.DeepClone();
        if (a["Candidates"] is JsonArray candidates)
        {
            page["Candidates"] = new JsonArray([.. candidates.Where(c => c is not null).Select(c => (JsonNode)new JsonObject
            {
                ["Label"] = c!["Label"]?.DeepClone(),
                ["Prior"] = c["Prior"]?.DeepClone(),
                ["Visits"] = c["Visits"]?.DeepClone(),
                ["VisitShare"] = c["VisitShare"]?.DeepClone(),
                ["WinPct"] = c["Value"]?.GetValue<double?>() is double cv ? Math.Round((cv + 1) * 50, 1) : null,
                ["Chosen"] = c["Chosen"]?.DeepClone(),
                ["PrunedWorlds"] = c["PrunedWorlds"]?.DeepClone(),
                ["Refused"] = c["Refused"]?.DeepClone(),
                ["Payload"] = Wire(c["Payload"]),
            })]);
        }

        return page;
    }

    /// <summary>
    /// A payload as the native host writes it, as the page's client sends one: the protocol's MessagePack, in base64.
    /// </summary>
    private static string? Wire(JsonNode? payload)
    {
        if (payload is null || ProtocolJson.Deserialize<AnswerPayload>(payload.ToJsonString()) is not { } answer)
        {
            return null;
        }

        return Convert.ToBase64String(MessagePackSerializer.Serialize(answer));
    }

    /// <summary>The page's settings: hide the advice until the person has played; play trivial decisions by themselves.</summary>
    public void Settings(bool? hidden, bool? autoTrivial)
    {
        lock (_lock)
        {
            _hidden = hidden ?? _hidden;
            _autoTrivial = autoTrivial ?? _autoTrivial;
        }

        Changed();
    }

    /// <summary>A note on a decision: the person's one-line reason, a flag, and how the play was made.</summary>
    public void Note(int requestId, string? reason, bool? flagged, string? via)
    {
        lock (_lock)
        {
            if (!_decisions.TryGetValue(requestId, out Decision? d))
            {
                return;
            }

            if (reason is not null)
            {
                d.Reason = reason.Length > 500 ? reason[..500] : reason;
            }

            d.Flagged = flagged ?? d.Flagged;
            d.Via = via ?? d.Via;
            Settle(d);
        }

        Changed();
    }

    public async ValueTask DisposeAsync()
    {
        await _stop.CancelAsync().ConfigureAwait(false);
        try
        {
            await _opponent.WaitAsync(TimeSpan.FromSeconds(5)).ConfigureAwait(false);
        }
        catch (TimeoutException)
        {
        }

        await _server.DisposeAsync().ConfigureAwait(false);
        if (_art is not null)
        {
            await _art.DisposeAsync().ConfigureAwait(false);
        }

        _ai.Dispose();
        _native.Dispose();
        _stop.Dispose();
        _opponentResults.Dispose();
    }

    private sealed class Decision
    {
        public int Index { get; init; }

        public int RequestId { get; init; }

        /// <summary>The commands mirrored before the request was seen: replaying that many makes the position again.</summary>
        public int CommandIndex { get; init; }

        public required JsonNode Prompt { get; init; }

        public JsonNode? View { get; init; }

        public DateTime AskedAt { get; init; }

        public int Turn { get; set; }

        public JsonNode? Analysis { get; set; }

        public JsonNode? BotPayload { get; set; }

        public double? Value { get; set; }

        public DateTime? AnalysedAt { get; set; }

        public string? Error { get; set; }

        public JsonNode? UserPayload { get; set; }

        public string? UserLabel { get; set; }

        public DateTime? AnsweredAt { get; set; }

        public bool? HiddenAtAnswer { get; set; }

        public string Reason { get; set; } = string.Empty;

        public bool Flagged { get; set; }

        public string? Via { get; set; }

        public bool Written { get; set; }
    }
}

/// <summary>
/// The advisor's table's transport to the native engine: every exchange goes to the native host, which is the table,
/// and each one is handed to the <see cref="AdvisorTable"/>, which mirrors it to the AI host and reads it.
/// </summary>
internal sealed class AdvisorTransport(IHostTransport native, AdvisorTable table) : IHostTransport
{
    public string Exchange(string request)
    {
        string reply = native.Exchange(request);
        if (JsonNode.Parse(request) is JsonNode node)
        {
            table.Exchanged(node, reply);
        }

        return reply;
    }
}

/// <summary>The neural bot's seat: each question asked of the advisor's AI host, its answer sent as the seat's own.</summary>
internal sealed class NeuralOpponent(AdvisorTable table) : IAutoPlayer
{
    public bool Retrying => false;

    public bool Act(GameClient client)
    {
        if (client.IsOver || client.Prompt is not { } asked)
        {
            return false;
        }

        if (table.AskOpponent(asked.RequestId) is not { } answer)
        {
            return false;
        }

        client.Answer(answer);
        return true;
    }
}
