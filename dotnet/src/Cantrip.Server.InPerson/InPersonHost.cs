using System.Net;
using System.Reflection;
using Cantrip.Art;
using Cantrip.Datasets;
using Cantrip.Decks;
using Cantrip.Protocol;
using Cantrip.Server.Events;
using Cantrip.Server.Host;
using Cantrip.Server.Rules;
using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Hosting;
using Microsoft.AspNetCore.Http;
using Microsoft.Extensions.Logging;
using QRCoder;

namespace Cantrip.Server.InPerson;

/// <summary>What an in-person server is given: where things are, and the ports it serves on.</summary>
public sealed class InPersonOptions
{
    /// <summary>The folder holding the card datasets.</summary>
    public required string ContentRoot { get; init; }

    /// <summary>The data directory the event is kept in (<see cref="EventStore"/>).</summary>
    public required EventStore Store { get; init; }

    /// <summary>The built web client's folder (its index.html), served at the site's root; null serves none.</summary>
    public string? ClientRoot { get; init; }

    /// <summary>Where the cube lists are, for the setup page's choice of cube.</summary>
    public IReadOnlyList<string> CubeDirectories { get; init; } = [];

    /// <summary>The site's port: the client, the relay's socket and the setup page. Zero for any free port.</summary>
    public int Port { get; init; } = 5200;

    /// <summary>The table's port (<c>GameServer</c>); card art is served on the next one up. Zero for any free ports.</summary>
    public int TablePort { get; init; } = 7790;

    /// <summary>The address the devices are to reach this computer at; null takes the best of <see cref="LanAddress.Candidates"/>.</summary>
    public string? Address { get; init; }

    /// <summary>What the site listens on: every interface, since the devices are other machines on the network.</summary>
    public string ListenAddress { get; init; } = "0.0.0.0";

    /// <summary>Where card art is cached once resized.</summary>
    public string? ArtCache { get; init; }

    /// <summary>What each game is dealt through: the native engine with a public view, unless a test says another.</summary>
    public IRulesHostFactory Rules { get; init; } = new NativeRulesHostFactory { PublicView = true };

    public TimeProvider TimeProvider { get; init; } = TimeProvider.System;
}

/// <summary>What the setup page's form asks for (docs/in-person.md, "Setup").</summary>
public sealed record SetupRequest(IReadOnlyList<string> Names, string Cube, int Packs, int Layout, bool Clocks, string? Address);

/// <summary>
/// In-person play's server (docs/in-person.md): one site, on every interface, serving the web client at its root, the
/// relay's socket at <c>/inperson</c> and - to this computer alone - the setup page at <c>/setup</c>. The page starts a
/// grid draft for two and shows a QR code for each device; the draft, deck building and the games are then played at an
/// ordinary <c>GameServer</c> on the network, its every game dealt through an <see cref="ObservedRulesFactory"/> so the
/// relay sees each step.
/// </summary>
public sealed class InPersonHost : IAsyncDisposable
{
    public const string SetupRoute = "/setup";

    private const string LayoutKey = InPersonSetup.LayoutKey;

    private const string ClocksKey = InPersonSetup.ClocksKey;

    private static readonly Lazy<string> Page = new(() =>
    {
        using Stream stream = Assembly.GetExecutingAssembly().GetManifestResourceStream("Cantrip.Server.InPerson.setup.html")
            ?? throw new InvalidOperationException("The setup page is not built into the server.");
        using var reader = new StreamReader(stream);
        return reader.ReadToEnd();
    });

    private readonly InPersonOptions _options;
    private readonly WebApplication _site;
    private readonly SemaphoreSlim _starting = new(1, 1);
    private readonly CancellationTokenSource _stop = new();
    private readonly ContentLibrary _library;
    private Task _loop = Task.CompletedTask;
    private volatile Table? _table;

    private InPersonHost(InPersonOptions options, WebApplication site)
    {
        _options = options;
        _site = site;
        _library = ContentCache.Get(options.ContentRoot);
    }

    /// <summary>The site's address on this computer.</summary>
    public Uri Site { get; private set; } = new("http://localhost/");

    /// <summary>The relay of the table being played, or null before one is started.</summary>
    public InPersonRelay? Relay => _table?.Relay;

    /// <summary>The table being played, or null before one is started.</summary>
    public GameServer? Server => _table?.Server;

    /// <summary>The event being played, or null before one is started.</summary>
    public DraftPod? Pod => _table?.Pod;

    /// <summary>The devices' links, in the relay's order (<see cref="InPersonRelay.Devices"/>), or none before a table is started.</summary>
    public IReadOnlyList<string> DeviceLinks => _table?.Devices.Select(device => device.Url).ToList() ?? [];

    public static async Task<InPersonHost> StartAsync(InPersonOptions options)
    {
        ArgumentNullException.ThrowIfNull(options);
        WebApplicationBuilder builder = WebApplication.CreateBuilder();
        builder.Logging.SetMinimumLevel(LogLevel.Information);
        builder.Logging.AddFilter("Microsoft.AspNetCore", LogLevel.Warning);
        builder.WebHost.UseUrls($"http://{options.ListenAddress}:{options.Port}");
        WebApplication site = builder.Build();
        var host = new InPersonHost(options, site);

        site.UseWebSockets();
        site.MapInPerson(() => host.Relay);
        site.MapGet(SetupRoute, (HttpContext context) =>
        {
            if (!InPersonSite.IsLoopback(context)) return Results.NotFound();
            context.Response.Headers.CacheControl = "no-cache";
            return Results.Content(Page.Value, "text/html; charset=utf-8");
        });
        site.MapGet(SetupRoute + "/api/state", (HttpContext context) =>
            InPersonSite.IsLoopback(context) ? Fresh(context, host.State()) : Results.NotFound());
        site.MapPost(SetupRoute + "/api/start", async (HttpContext context, SetupRequest request) =>
        {
            if (!InPersonSite.IsLoopback(context)) return Results.NotFound();
            try
            {
                await host.StartTableAsync(request).ConfigureAwait(false);
                return Fresh(context, host.State());
            }
            catch (Exception error) when (error is ArgumentException or InvalidOperationException or IOException)
            {
                return Results.Json(new { error = error.Message }, statusCode: StatusCodes.Status400BadRequest);
            }
        });
        if (options.ClientRoot is { } root && File.Exists(Path.Combine(root, "index.html")))
        {
            site.UseWebClient(root);
        }
        else
        {
            site.Logger.LogWarning("No built web client to serve: the devices will find nothing at the site's root.");
        }

        await site.StartAsync().ConfigureAwait(false);
        host.Site = new Uri($"http://localhost:{site.Urls.Select(address => new Uri(address).Port).First(port => port != 0)}/");
        host._loop = Task.Run(host.RunAsync, CancellationToken.None);
        return host;
    }

    /// <summary>
    /// Hosts an event already made - a table stopped and started again - with fresh tickets and tablet codes, laid out as
    /// it was set up.
    /// </summary>
    public async Task HostEventAsync(string eventId, string? address = null)
    {
        await _starting.WaitAsync().ConfigureAwait(false);
        try
        {
            if (_table is not null) throw new InvalidOperationException("A table is already being played here.");
            _table = await OpenAsync(eventId, address ?? _options.Address, layout: null, clocks: null).ConfigureAwait(false);
        }
        finally
        {
            _starting.Release();
        }
    }

    /// <summary>Makes the grid draft the setup page asked for and hosts it.</summary>
    public async Task StartTableAsync(SetupRequest request)
    {
        InPersonSetup.Validate(request, Cubes());

        await _starting.WaitAsync().ConfigureAwait(false);
        try
        {
            if (_table is not null) throw new InvalidOperationException("A table is already being played here.");
            string first = EventStore.ToId(request.Names[0]);
            string second = EventStore.ToId(request.Names[1]);
            if (second == first) second += "-2";
            string created = InPersonSetup.CreateEvent(_options.Store, _library, _options.CubeDirectories, request, (first, second), _options.TimeProvider);
            _table = await OpenAsync(created, request.Address ?? _options.Address, request.Layout, request.Clocks).ConfigureAwait(false);
        }
        finally
        {
            _starting.Release();
        }
    }

    /// <summary>
    /// How long everything is given. Off, the default in person since the players sit at one table: no pick clock and no
    /// chess clock to speak of, and the absences that forfeit a remote player - a phone asleep in a pocket - forfeit
    /// nobody. On, an event's usual clocks.
    /// </summary>
    internal static DraftTiming Timing(bool clocks) => clocks
        ? new DraftTiming { Seating = TimeSpan.FromHours(1) }
        : new DraftTiming
        {
            Seating = TimeSpan.FromHours(1),
            GridPick = TimeSpan.Zero,
            MatchClock = TimeSpan.Zero,
            DeckBuilding = TimeSpan.FromHours(4),
            NoShow = TimeSpan.FromHours(12),
            Away = TimeSpan.FromHours(12),
        };

    /// <summary>
    /// Each player's clock in the games with clocks off. A table always has one (<c>GameServerOptions.PlayerClock</c>), so
    /// off is one no game at a table runs out: a week.
    /// </summary>
    internal static readonly TimeSpan NoClock = TimeSpan.FromDays(7);

    private async Task<Table> OpenAsync(string eventId, string? address, int? layout, bool? clocks)
    {
        DraftPod pod = DraftPod.Open(_options.Store, eventId, _library, _options.TimeProvider,
            Timing(clocks ?? _options.Store.LoadEvent(eventId)?.HostData.GetValueOrDefault(ClocksKey) == "on"));
        if (pod.Seats != 2) throw new InvalidOperationException($"Event {eventId} is not for two players.");
        if (pod.Stage is EventStage.Complete or EventStage.Cancelled) throw new InvalidOperationException($"Event {eventId} is over.");
        if (layout is { } laid) pod.Keep(LayoutKey, laid.ToString(System.Globalization.CultureInfo.InvariantCulture));
        if (clocks is { } kept) pod.Keep(ClocksKey, kept ? "on" : "off");
        int devices = pod.Kept(LayoutKey) == "4" ? 4 : 3;
        bool clocked = pod.Kept(ClocksKey) == "on";

        string lan = address is { Length: > 0 } given && IPAddress.TryParse(given, out _) ? given
            : LanAddress.Candidates().FirstOrDefault()?.Address.ToString() ?? "127.0.0.1";

        InPersonSetup.Seating seating = InPersonSetup.Seat(pod, devices, _options.TimeProvider, line => _site.Logger.LogWarning("In-person relay: {Line}", line));
        InPersonRelay relay = seating.Relay;

        int artPort = _options.TablePort == 0 ? 0 : _options.TablePort + 1;
        string artCache = _options.ArtCache ?? Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "Cantrip", "art-cache");
        WebApplication art = await ArtServer.StartAsync(new ArtStore(new ArtStoreOptions { ContentRoot = _options.ContentRoot, CacheDirectory = artCache }),
            _options.ListenAddress, artPort).ConfigureAwait(false);
        GameServer server;
        try
        {
            int servedArt = art.Urls.Select(each => new Uri(each).Port).First(port => port != 0);

            // The tablets are shown the grid draft as both players see it, from the event on the table's thread.
            var observed = new ObservedTableEvent(pod, relay);
            observed.Publish();
            server = await GameServer.StartAsync(new GameServerOptions
            {
                Format = DeckFormat.Limited(),
                Port = _options.TablePort,
                Players = pod.Seats,
                ContentRoot = _options.ContentRoot,
                Event = observed,
                ListenAddress = _options.ListenAddress,
                Rules = new ObservedRulesFactory(_options.Rules, relay),
                Art = new Uri($"http://{lan}:{servedArt}/"),
                PlayerClock = clocked ? Cantrip.Server.MatchClocks.DefaultPlayerClock : NoClock,
                TimeProvider = _options.TimeProvider,
            }).ConfigureAwait(false);
        }
        catch
        {
            await art.DisposeAsync().ConfigureAwait(false);
            throw;
        }

        int sitePort = Site.Port;
        // A tablet is sent no TableMessage, which tells a seat's page where art is: its link says.
        List<DeviceLink> links = InPersonSetup.Links(seating, new Uri($"http://{lan}:{sitePort}/"), new Uri($"ws://{lan}:{server.Port}/game"),
            new Uri($"http://{lan}:{art.Urls.Select(each => new Uri(each).Port).First(port => port != 0)}/"));
        _site.Logger.LogInformation("In person: event {Event} at ws://{Address}:{Port}/game; devices at http://{Address}:{Site}/.", pod.EventId, lan, server.Port, lan, sitePort);
        return new Table(pod, relay, server, art, links, lan);
    }

    /// <summary>What the setup page shows: the form's choices before a table is started, its devices after.</summary>
    internal object State()
    {
        Table? table = _table;
        if (table is null)
        {
            IReadOnlyList<LanAddress> addresses = LanAddress.Candidates();
            List<string> cubes = Cubes();
            return new
            {
                started = false,
                cubes,
                cube = cubes.Contains("LSVCube") ? "LSVCube" : cubes.FirstOrDefault(),
                addresses = (object[])[
                    .. _options.Address is { } given && addresses.All(each => each.Address.ToString() != given) ? [new { address = given, name = "given" }] : Array.Empty<object>(),
                    .. addresses.Select(each => new { address = each.Address.ToString(), name = each.Name }),
                    new { address = "127.0.0.1", name = "this computer only" }],
                address = _options.Address ?? addresses.FirstOrDefault()?.Address.ToString() ?? "127.0.0.1",
            };
        }

        IReadOnlyList<DeviceStatus> statuses = table.Relay.Devices;
        StatusEventDetail? detail = new EventStatusBoard(_options.Store).Detail(table.Pod.EventId);
        return new
        {
            started = true,
            @event = new { id = table.Pod.EventId, name = table.Pod.Name, stage = detail?.Event.Stage ?? string.Empty, detail = PhaseLine(detail) },
            address = table.Address,
            devices = table.Devices.Select((device, i) => new
            {
                name = device.Name,
                url = device.Url,
                qr = device.Qr,
                connected = i < statuses.Count && statuses[i].Connected,
            }),
        };
    }

    private List<string> Cubes() => InPersonSetup.Cubes(_options.CubeDirectories);

    /// <summary>Keeps the relay's taps moving and tells the tablets where the event stands, until the host is stopped.</summary>
    private async Task RunAsync()
    {
        var board = new EventStatusBoard(_options.Store);
        DateTimeOffset nextPhase = DateTimeOffset.MinValue;
        while (!_stop.IsCancellationRequested)
        {
            try
            {
                if (_table is { } table)
                {
                    table.Relay.Tick();
                    DateTimeOffset now = DateTimeOffset.UtcNow;
                    if (now >= nextPhase)
                    {
                        nextPhase = now + TimeSpan.FromSeconds(1);
                        StatusEventDetail? detail = board.Detail(table.Pod.EventId);
                        table.Relay.Phase(PhaseOf(detail), PhaseLine(detail));
                    }
                }
            }
            catch (Exception error) when (error is IOException or InvalidOperationException or UnauthorizedAccessException)
            {
                // The event's file was being written as it was read: it is read again next time.
                _site.Logger.LogDebug(error, "In person: the event's status could not be read.");
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

    /// <summary>The table's phase as the event's stage says.</summary>
    internal static TablePhase PhaseOf(StatusEventDetail? detail) => detail?.Event.Stage switch
    {
        nameof(EventStage.Drafting) => TablePhase.Drafting,
        nameof(EventStage.DeckBuilding) => TablePhase.DeckBuilding,
        nameof(EventStage.Playing) => TablePhase.Playing,
        nameof(EventStage.Complete) or nameof(EventStage.Cancelled) => TablePhase.Finished,
        _ => TablePhase.Setup,
    };

    /// <summary>One line saying where the event stands, for a tablet's phase screen and the setup page.</summary>
    internal static string PhaseLine(StatusEventDetail? detail)
    {
        if (detail is null) return string.Empty;
        StatusEvent summary = detail.Event;
        switch (summary.Stage)
        {
            case nameof(EventStage.Seating):
                List<string> waiting = [.. summary.Players.Where(player => !player.Seated).Select(player => player.Name)];
                return waiting.Count == 0 ? "Everyone is seated." : $"Waiting for {string.Join(" and ", waiting)} to scan their phone's code.";
            case nameof(EventStage.Drafting) when detail.Draft is { } draft:
                string picking = draft.Seats.FirstOrDefault(seat => seat.Active)?.Name ?? "nobody";
                return $"Grid {Math.Max(1, draft.Pack)} of {draft.Packs}: {picking} picks.";
            case nameof(EventStage.DeckBuilding):
                List<string> submitted = [.. summary.Players.Where(player => player.Submitted).Select(player => player.Name)];
                return submitted.Count == 0 ? "Building decks." : $"Building decks: {string.Join(" and ", submitted)} {(submitted.Count == 1 ? "has" : "have")} submitted.";
            default:
                return summary.Detail.Length > 0 ? summary.Detail : summary.Stage;
        }
    }

    private static IResult Fresh(HttpContext context, object value)
    {
        context.Response.Headers.CacheControl = "no-store";
        return Results.Json(value);
    }

    public async ValueTask DisposeAsync()
    {
        await _stop.CancelAsync().ConfigureAwait(false);
        await _loop.ConfigureAwait(false);
        if (_table is { } table)
        {
            await table.Server.DisposeAsync().ConfigureAwait(false);
            await table.Art.DisposeAsync().ConfigureAwait(false);
        }

        await _site.DisposeAsync().ConfigureAwait(false);
        _stop.Dispose();
        _starting.Dispose();
    }

    /// <summary>
    /// A device's QR code: its link, which holds its credential, drawn on this side as SVG. Sized by its view box alone, so
    /// the page's box scales the whole code into it; fixed width and height attributes would crop a long link's code.
    /// </summary>
    internal static string QrSvg(string text)
    {
        using var generator = new QRCodeGenerator();
        using QRCodeData data = generator.CreateQrCode(text, QRCodeGenerator.ECCLevel.M);
        return new SvgQRCode(data).GetGraphic(4, "#000000", "#ffffff", drawQuietZones: true, sizingMode: SvgQRCode.SizingMode.ViewBoxAttribute);
    }

    private sealed record Table(DraftPod Pod, InPersonRelay Relay, GameServer Server, WebApplication Art, IReadOnlyList<DeviceLink> Devices, string Address);
}
