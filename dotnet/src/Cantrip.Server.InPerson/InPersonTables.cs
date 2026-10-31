using System.Collections.Concurrent;
using System.Globalization;
using System.Reflection;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using Cantrip.Datasets;
using Cantrip.Decks;
using Cantrip.Protocol.InPerson;
using Cantrip.Server.Events;
using Cantrip.Server.Host;
using Cantrip.Server.Rules;
using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.Http.Features;
using Microsoft.AspNetCore.Routing;

namespace Cantrip.Server.InPerson;

/// <summary>What a site hosting in-person tables is given (<see cref="InPersonTables"/>).</summary>
public sealed class InPersonTablesOptions
{
    /// <summary>The folder holding the card datasets.</summary>
    public required string ContentRoot { get; init; }

    /// <summary>The data directory the events are kept in.</summary>
    public required EventStore Store { get; init; }

    /// <summary>Where the cube lists are: the only cubes a table may be drafted from.</summary>
    public required IReadOnlyList<string> CubeDirectories { get; init; }

    /// <summary>
    /// The ports a table may take - the host's pool for its tables, shared with its other hosts: a port another table holds
    /// refuses, and the next is tried.
    /// </summary>
    public required IReadOnlyList<int> Ports { get; init; }

    /// <summary>The site's address as the devices reach it: their links open the web client there.</summary>
    public required Uri Site { get; init; }

    /// <summary>Where card art is, which the tables tell their seats and the tablets' links say.</summary>
    public required Uri Art { get; init; }

    /// <summary>What the tables listen on.</summary>
    public string ListenAddress { get; init; } = "127.0.0.1";

    /// <summary>The host a table is reached at on its own port, when <see cref="TableAddress"/> says nothing.</summary>
    public string PublicHost { get; init; } = "localhost";

    /// <summary>A table's public address by its port - behind a proxy, through the site; null for its own port at <see cref="PublicHost"/>.</summary>
    public Func<int, Uri>? TableAddress { get; init; }

    /// <summary>How many tables may be played at once; another is refused as busy until one closes.</summary>
    public int MostGames { get; init; } = InPersonTables.DefaultMostGames;

    /// <summary>How long a table waits for its first device before it is closed as abandoned.</summary>
    public TimeSpan Unclaimed { get; init; } = TimeSpan.FromMinutes(15);

    /// <summary>How long a table every device has left is kept before it is closed.</summary>
    public TimeSpan Deserted { get; init; } = TimeSpan.FromMinutes(30);

    /// <summary>How long a table whose event is over is kept, for the last result to be read, before it is closed.</summary>
    public TimeSpan AfterEvent { get; init; } = TimeSpan.FromMinutes(10);

    /// <summary>What each game is dealt through: the native engine with a public view, unless a test says another.</summary>
    public IRulesHostFactory Rules { get; init; } = new NativeRulesHostFactory { PublicView = true };

    public TimeProvider TimeProvider { get; init; } = TimeProvider.System;

    public Action<string>? Log { get; init; }
}

/// <summary>Every table this site may host is being played: try again once one closes.</summary>
public sealed class InPersonBusyException(string message) : InvalidOperationException(message);

/// <summary>A table just made: its id, and the creator's key that alone shows its devices' codes.</summary>
public sealed record CreatedInPersonTable(string Id, string Key);

/// <summary>
/// In-person tables hosted on a public site - the Discord bot's (docs/in-person.md, "Hosted in person"): anyone may make
/// one, with no sign-in, so what is made is bounded. At most <see cref="InPersonTablesOptions.MostGames"/> at once; a table
/// no device comes to, or every device has left, is closed; the form's inputs are limited (<see cref="InPersonSetup.Validate"/>),
/// its cube only from the cube lists. A table's codes seat whoever scans them, so they are shown only to whoever holds the
/// table's creator key, which is made once, kept here only as its hash, and travels after a <c>#</c> and in request bodies,
/// never in an address a log keeps.
/// <para>
/// Each table is an ordinary <c>GameServer</c> on a port of the host's pool, its games dealt through an
/// <see cref="ObservedRulesFactory"/> to its own relay; every relay is served on one socket (<see cref="InPersonSite.SocketRoute"/>),
/// a connection going to the relay its hello names.
/// </para>
/// </summary>
public sealed class InPersonTables : IAsyncDisposable
{
    public const int DefaultMostGames = 2;

    /// <summary>The setup page; its API and the relay's socket are under it.</summary>
    public const string Route = "/inperson";

    /// <summary>Marks an event as a hosted table's in its host data: one left live when the site stopped is called off when it starts again.</summary>
    internal const string HostedKey = "inperson.hosted";

    /// <summary>The largest form believed: two names and a cube's name.</summary>
    private const int MostBody = 4 * 1024;

    private static readonly Lazy<string> Page = new(() =>
    {
        using Stream stream = Assembly.GetExecutingAssembly().GetManifestResourceStream("Cantrip.Server.InPerson.hosted.html")
            ?? throw new InvalidOperationException("The hosted setup page is not built into the server.");
        using var reader = new StreamReader(stream);
        return reader.ReadToEnd();
    });

    private readonly InPersonTablesOptions _options;
    private readonly ContentLibrary _library;
    private readonly ConcurrentDictionary<string, Table> _tables = new(StringComparer.Ordinal);
    private readonly SemaphoreSlim _gate = new(1, 1);
    private readonly CancellationTokenSource _stop = new();
    private readonly Task _loop;

    public InPersonTables(InPersonTablesOptions options)
    {
        ArgumentNullException.ThrowIfNull(options);
        if (options.Ports.Count == 0) throw new ArgumentException("In-person tables need at least one port.", nameof(options));
        if (options.MostGames < 1) throw new ArgumentException("In-person tables need room for at least one.", nameof(options));
        _options = options;
        _library = ContentCache.Get(options.ContentRoot);
        _loop = Task.Run(RunAsync, CancellationToken.None);
    }

    /// <summary>How many tables are being played.</summary>
    public int Count => _tables.Count;

    public int MostGames => _options.MostGames;

    /// <summary>The cubes a table may be drafted from.</summary>
    public List<string> Cubes() => InPersonSetup.Cubes(_options.CubeDirectories);

    /// <summary>
    /// Whether a table this host made holds <paramref name="port"/>: one that opens on its ticket alone, with no sign-in.
    /// True only while the table is open, so a port it held and gave back is anyone's again.
    /// </summary>
    public bool Holds(int port) => _tables.Values.Any(table => table.Port == port);

    /// <summary>The relay of the table whose place <paramref name="hello"/> names, or null for none here.</summary>
    public InPersonRelay? RelayFor(DeviceHelloMessage hello)
    {
        ArgumentNullException.ThrowIfNull(hello);
        return _tables.Values.Select(table => table.Relay).FirstOrDefault(relay => relay.Recognizes(hello));
    }

    /// <summary>
    /// Makes the grid draft a form asks for and opens its table. Throws <see cref="ArgumentException"/> for a form outside its
    /// limits and <see cref="InPersonBusyException"/> when every table is being played or no port is free.
    /// </summary>
    public async Task<CreatedInPersonTable> CreateAsync(SetupRequest request)
    {
        InPersonSetup.Validate(request, Cubes());
        await _gate.WaitAsync().ConfigureAwait(false);
        try
        {
            if (_tables.Count >= _options.MostGames) throw Busy();

            // Players of their own, never a name's: a name is anybody's to type, and a record it named would be someone else's.
            string players = "inperson-" + Hex(6);
            string eventId = InPersonSetup.CreateEvent(_options.Store, _library, _options.CubeDirectories, request, (players + "-1", players + "-2"), _options.TimeProvider);
            DraftPod pod = DraftPod.Open(_options.Store, eventId, _library, _options.TimeProvider, InPersonHost.Timing(request.Clocks));
            pod.Keep(HostedKey, "site");
            pod.Keep(InPersonSetup.LayoutKey, request.Layout.ToString(CultureInfo.InvariantCulture));
            pod.Keep(InPersonSetup.ClocksKey, request.Clocks ? "on" : "off");
            InPersonSetup.Seating seating = InPersonSetup.Seat(pod, request.Layout, _options.TimeProvider, line => _options.Log?.Invoke($"In-person relay: {line}"));

            GameServer? server = null;
            try
            {
                server = await OpenAsync(pod, seating.Relay, request.Clocks).ConfigureAwait(false);
            }
            catch
            {
                pod.Cancel("Its table could not be opened.");
                throw;
            }

            if (server is null)
            {
                pod.Cancel("No table was free for it.");
                throw Busy();
            }

            Uri address = _options.TableAddress?.Invoke(server.Port) ?? new Uri($"ws://{_options.PublicHost}:{server.Port}/game");
            string id = Hex(8);
            string key = Hex(32);
            DateTimeOffset now = _options.TimeProvider.GetUtcNow();
            var table = new Table(id, Hash(key), pod, seating.Relay, server, server.Port, InPersonSetup.Links(seating, _options.Site, address, _options.Art), now);
            _tables[id] = table;
            _options.Log?.Invoke($"In person: table {id} (event {eventId}) opened on port {server.Port}; {_tables.Count} of {_options.MostGames} in play.");
            return new CreatedInPersonTable(id, key);
        }
        finally
        {
            _gate.Release();
        }
    }

    /// <summary>What the creator's page shows of table <paramref name="id"/> - its event and its devices' codes - or null for no such table or the wrong key.</summary>
    public object? State(string? id, string? key)
    {
        if (id is null || key is null || !_tables.TryGetValue(id, out Table? table)) return null;
        if (!CryptographicOperations.FixedTimeEquals(table.KeyHash, Hash(key))) return null;
        IReadOnlyList<DeviceStatus> statuses = table.Relay.Devices;
        StatusEventDetail? detail = table.Detail;
        return new
        {
            @event = new { name = table.Pod.Name, stage = detail?.Event.Stage ?? string.Empty, detail = InPersonHost.PhaseLine(detail) },
            welcomed = table.Relay.Welcomed,
            unclaimedMinutes = (int)_options.Unclaimed.TotalMinutes,
            devices = table.Devices.Select((device, i) => new
            {
                name = device.Name,
                url = device.Url,
                qr = device.Qr,
                connected = i < statuses.Count && statuses[i].Connected,
            }),
        };
    }

    /// <summary>
    /// Calls off every live event an earlier run of the site left hosted: its table went with that run, and its codes with
    /// the creator's page. Returns how many.
    /// </summary>
    public int CancelLeftOver()
    {
        int cancelled = 0;
        foreach (EventRecord record in _options.Store.LiveEvents().Where(record => record.HostData.GetValueOrDefault(HostedKey) == "site").ToList())
        {
            if (_tables.Values.Any(table => table.Pod.EventId == record.Id)) continue;
            try
            {
                DraftPod.Open(_options.Store, record.Id, _library, _options.TimeProvider).Cancel("The site stopped while it was being played.");
                cancelled++;
            }
            catch (Exception error) when (error is IOException or InvalidOperationException or JsonException)
            {
                _options.Log?.Invoke($"In person: event {record.Id} could not be called off: {error.Message}");
            }
        }

        return cancelled;
    }

    /// <summary>
    /// Closes every table abandoned - no device came within <see cref="InPersonTablesOptions.Unclaimed"/>, or every device
    /// has been gone <see cref="InPersonTablesOptions.Deserted"/> - and every one whose event has been over
    /// <see cref="InPersonTablesOptions.AfterEvent"/>, by the host's clock.
    /// </summary>
    internal async Task SweepAsync()
    {
        await _gate.WaitAsync().ConfigureAwait(false);
        try
        {
            DateTimeOffset now = _options.TimeProvider.GetUtcNow();
            foreach (Table table in _tables.Values.ToList())
            {
                if (table.Relay.AnyConnected) table.LastHere = now;
                if (table.Server.Finished.IsCompleted) table.EndedAt ??= now;

                string? why = !table.Relay.Welcomed && now - table.Opened >= _options.Unclaimed ? "Nobody came to the table."
                    : table.Relay.Welcomed && !table.Relay.AnyConnected && now - table.LastHere >= _options.Deserted ? "Everyone left the table."
                    : table.EndedAt is { } ended && now - ended >= _options.AfterEvent ? "The event is over."
                    : null;
                if (why is not null) await CloseAsync(table, why).ConfigureAwait(false);
            }
        }
        finally
        {
            _gate.Release();
        }
    }

    /// <summary>
    /// Serves the setup page at <see cref="Route"/>, its API under it and every table's relay at
    /// <see cref="InPersonSite.SocketRoute"/>, open to anyone. <paramref name="page"/> serves the page's template in the
    /// site's own look; null serves it as it is. The app must already use WebSockets and routing.
    /// </summary>
    public void Map(IEndpointRouteBuilder app, Func<HttpContext, string, IResult>? page = null)
    {
        ArgumentNullException.ThrowIfNull(app);
        app.MapGet(Route, (HttpContext context) =>
        {
            if (page is not null) return page(context, Page.Value);
            context.Response.Headers.CacheControl = "no-cache";
            return Results.Content(Page.Value, "text/html; charset=utf-8");
        });

        app.MapGet(Route + "/api/choices", (HttpContext context) =>
        {
            List<string> cubes = Cubes();
            return Fresh(context, new
            {
                cubes,
                cube = cubes.Contains("LSVCube") ? "LSVCube" : cubes.FirstOrDefault(),
                mostPacks = InPersonSetup.MostPacks,
                longestName = InPersonSetup.LongestName,
                busy = _tables.Count >= _options.MostGames,
                mostGames = _options.MostGames,
                unclaimedMinutes = (int)_options.Unclaimed.TotalMinutes,
            });
        });

        app.MapPost(Route + "/api/start", async (HttpContext context) =>
        {
            if (await BodyAsync<SetupRequest>(context).ConfigureAwait(false) is not { } request)
                return Results.Json(new { error = "That is not a table's form." }, statusCode: StatusCodes.Status400BadRequest);
            try
            {
                CreatedInPersonTable created = await CreateAsync(request).ConfigureAwait(false);
                return Fresh(context, new { id = created.Id, key = created.Key });
            }
            catch (InPersonBusyException busy)
            {
                return Results.Json(new { error = busy.Message, busy = true }, statusCode: StatusCodes.Status503ServiceUnavailable);
            }
            catch (Exception error) when (error is ArgumentException or InvalidOperationException or IOException)
            {
                return Results.Json(new { error = error.Message }, statusCode: StatusCodes.Status400BadRequest);
            }
        });

        app.MapPost(Route + "/api/table", async (HttpContext context) =>
        {
            TableKey? asked = await BodyAsync<TableKey>(context).ConfigureAwait(false);
            return State(asked?.Id, asked?.Key) is { } state
                ? Fresh(context, state)
                : Results.Json(new { error = "There is no such table here, or it has closed." }, statusCode: StatusCodes.Status404NotFound);
        });

        app.MapInPerson(RelayFor, mostConnections: Math.Max(16, _options.MostGames * 12));
    }

    public async ValueTask DisposeAsync()
    {
        await _stop.CancelAsync().ConfigureAwait(false);
        await _loop.ConfigureAwait(false);
        await _gate.WaitAsync().ConfigureAwait(false);
        try
        {
            foreach (Table table in _tables.Values.ToList())
            {
                // Left live: the next start calls it off (CancelLeftOver).
                _tables.TryRemove(table.Id, out _);
                table.Relay.CloseAll("The site is stopping.");
                await table.Server.DisposeAsync().ConfigureAwait(false);
            }
        }
        finally
        {
            _gate.Release();
        }

        _stop.Dispose();
    }

    /// <summary>Closes a table: no longer anyone's to reach, its devices told, its server stopped and its event called off.</summary>
    private async Task CloseAsync(Table table, string why)
    {
        // Out of the list first, so its port opens on a ticket no more and no device is sent to its relay.
        _tables.TryRemove(table.Id, out _);
        table.Relay.CloseAll($"This table has closed. {why}");
        try
        {
            await table.Server.DisposeAsync().ConfigureAwait(false);
        }
        finally
        {
            table.Pod.Cancel(why);
            _options.Log?.Invoke($"In person: table {table.Id} (event {table.Pod.EventId}) closed. {why}");
        }
    }

    /// <summary>Opens the event's table on the first port of the pool that will have it, or null when every one is held.</summary>
    private async Task<GameServer?> OpenAsync(DraftPod pod, InPersonRelay relay, bool clocks)
    {
        foreach (int port in _options.Ports)
        {
            try
            {
                return await GameServer.StartAsync(new GameServerOptions
                {
                    Format = DeckFormat.Limited(),
                    Port = port,
                    Players = pod.Seats,
                    ContentRoot = _options.ContentRoot,
                    Event = pod,
                    ListenAddress = _options.ListenAddress,
                    Rules = new ObservedRulesFactory(_options.Rules, relay),
                    Art = _options.Art,
                    PlayerClock = clocks ? MatchClocks.DefaultPlayerClock : InPersonHost.NoClock,
                    TimeProvider = _options.TimeProvider,
                }).ConfigureAwait(false);
            }
            catch (IOException)
            {
                // Another table holds it: the next is tried.
            }
        }

        return null;
    }

    /// <summary>Keeps the relays' taps moving, tells the tablets where each event stands, and closes what is abandoned.</summary>
    private async Task RunAsync()
    {
        var board = new EventStatusBoard(_options.Store);
        DateTimeOffset nextLook = DateTimeOffset.MinValue;
        while (!_stop.IsCancellationRequested)
        {
            try
            {
                foreach (Table table in _tables.Values) table.Relay.Tick();
                DateTimeOffset now = DateTimeOffset.UtcNow;
                if (now >= nextLook)
                {
                    nextLook = now + TimeSpan.FromSeconds(1);
                    foreach (Table table in _tables.Values)
                    {
                        StatusEventDetail? detail = board.Detail(table.Pod.EventId);
                        table.Detail = detail;
                        table.Relay.Phase(InPersonHost.PhaseOf(detail), InPersonHost.PhaseLine(detail));
                    }

                    await SweepAsync().ConfigureAwait(false);
                }
            }
            catch (Exception error) when (error is IOException or InvalidOperationException or UnauthorizedAccessException or JsonException)
            {
                // An event's file was being written as it was read: it is read again next time.
                _options.Log?.Invoke($"In person: {error.Message}");
            }

            try
            {
                await Task.Delay(TimeSpan.FromMilliseconds(250), _stop.Token).ConfigureAwait(false);
            }
            catch (OperationCanceledException)
            {
                break;
            }
        }
    }

    private InPersonBusyException Busy() =>
        new($"All in-person tables are busy: {_options.MostGames} are being played. Try again when one finishes.");

    /// <summary>A small JSON body, or null for one too big or not the shape asked for.</summary>
    private static async Task<T?> BodyAsync<T>(HttpContext context) where T : class
    {
        if (context.Request.ContentLength is > MostBody) return null;
        if (context.Features.Get<IHttpMaxRequestBodySizeFeature>() is { IsReadOnly: false } limit) limit.MaxRequestBodySize = MostBody;
        try
        {
            using var body = new MemoryStream();
            var buffer = new byte[1024];
            int read;
            while ((read = await context.Request.Body.ReadAsync(buffer, context.RequestAborted).ConfigureAwait(false)) > 0)
            {
                if (body.Length + read > MostBody) return null;
                body.Write(buffer, 0, read);
            }

            return JsonSerializer.Deserialize<T>(body.ToArray(), JsonOptions);
        }
        catch (Exception error) when (error is JsonException or IOException or BadHttpRequestException or NotSupportedException)
        {
            return null;
        }
    }

    private static readonly JsonSerializerOptions JsonOptions = new(JsonSerializerDefaults.Web);

    private static IResult Fresh(HttpContext context, object value)
    {
        context.Response.Headers.CacheControl = "no-store";
        return Results.Json(value);
    }

    private static string Hex(int bytes) => Convert.ToHexString(RandomNumberGenerator.GetBytes(bytes)).ToLowerInvariant();

    private static byte[] Hash(string key) => SHA256.HashData(Encoding.UTF8.GetBytes(key));

    /// <summary>What the creator's page asks with: the table and its key, in the request's body.</summary>
    private sealed record TableKey(string? Id, string? Key);

    private sealed class Table(string id, byte[] keyHash, DraftPod pod, InPersonRelay relay, GameServer server, int port, IReadOnlyList<DeviceLink> devices, DateTimeOffset opened)
    {
        public string Id { get; } = id;

        public byte[] KeyHash { get; } = keyHash;

        public DraftPod Pod { get; } = pod;

        public InPersonRelay Relay { get; } = relay;

        public GameServer Server { get; } = server;

        public int Port { get; } = port;

        public IReadOnlyList<DeviceLink> Devices { get; } = devices;

        public DateTimeOffset Opened { get; } = opened;

        public DateTimeOffset LastHere { get; set; } = opened;

        public DateTimeOffset? EndedAt { get; set; }

        /// <summary>Where the event stood when last looked at.</summary>
        public StatusEventDetail? Detail { get; set; }
    }
}
