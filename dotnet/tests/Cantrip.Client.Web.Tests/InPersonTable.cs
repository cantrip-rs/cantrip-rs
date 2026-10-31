using Cantrip.Client.InPerson;
using Cantrip.Protocol;
using Cantrip.Server;
using Cantrip.Server.Host;
using Cantrip.Server.InPerson;
using Cantrip.Server.Rules;
using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Hosting;
using Microsoft.Extensions.Logging;
using Xunit;

namespace Cantrip.Client.Web.Tests;

/// <summary>
/// A real in-person table (docs/in-person.md) for a scenario played with Bot1's seat a phone (<see cref="ScenarioPlayer"/>
/// with <c>atTable</c>): the table's games dealt with the public view and shown step by step to an
/// <see cref="InPersonRelay"/>, the relay served at its socket by a site that serves the web client too, as
/// <c>cantrip-in-person</c> hosts it, and real tablets at that site - one shared board, or one for each player - joined
/// with the tokens the relay minted. Every tap meant for the battlefield is made with a finger on a tablet; the relay routes
/// it to the phone, which must use it, and the table judges each answer against the script.
/// </summary>
public sealed class InPersonTable : IAsyncDisposable
{
    private static readonly TimeSpan Patience = TimeSpan.FromSeconds(90);

    private readonly WebClient _web;
    private readonly int[] _sides;
    private readonly Observer _observer = new();
    private readonly List<TabletDriver> _tablets = [];
    private readonly CancellationTokenSource _stop = new();
    private WebApplication? _site;
    private Task? _ticking;
    private string? _ticket;

    /// <param name="sides">The tablets: zero for the shared board, or the seat whose own board it is.</param>
    public InPersonTable(WebClient web, string name, params int[] sides)
    {
        _web = web;
        _sides = sides;
        Name = name;
        Relay = new InPersonRelay(new InPersonRelayOptions
        {
            PlayerNames = ["Bot1", "Bot2"],
            SeatForTicket = ticket => _ticket is not null && string.Equals(ticket, _ticket, StringComparison.Ordinal) ? 1 : 0,
        });
        _observer.Relay = Relay;
        Rules = new ObservedRulesFactory(new NativeRulesHostFactory { PublicView = true }, _observer);
    }

    /// <summary>What the pictures of the phone and the tablets are named for.</summary>
    public string Name { get; }

    public InPersonRelay Relay { get; }

    /// <summary>What the table deals its games with: the native engine with the public view, each step shown to the relay.</summary>
    public IRulesHostFactory Rules { get; }

    /// <summary>The table's catalog, for the deck the phone's reserved seat names; the scenario deals its own.</summary>
    public IReadOnlyList<DeckList> Decks { get; } = [DeckList.Load(Path.Combine(WebClient.Root, "decks", "rw-aggro.txt"))];

    public IReadOnlyList<TabletDriver> Tablets => _tablets;

    /// <summary>
    /// Once the table is up: Bot1's seat reserved with a ticket, the site and its relay started, and each tablet opened at
    /// the site with its token and waiting on its phase screen. Returns where the phone's page is served and what puts it
    /// in its seat - the ticket after the hash, as the phone's QR code holds it.
    /// </summary>
    public async Task<(Uri Site, string Seat)> SeatAsync(ScenarioTable table, string art)
    {
        _ticket = table.Server.Reserve(1, "Bot1", Decks[0].Name);
        _site = await ServeAsync();
        var site = new Uri(_site.Urls.First());

        // The relay is reached where a device's page finds it, from the page's own address.
        using (DeviceConnection probe = await DeviceConnection.ConnectAsync(DeviceConnection.AddressFor(site)))
        {
            Assert.True(probe.IsConnected, $"No relay at {DeviceConnection.AddressFor(site)}.");
        }

        _ticking = TickAsync(_stop.Token);
        string pictures = WebClient.Pictures(Path.Combine(nameof(InPersonTests), Name));
        foreach (int side in _sides)
        {
            string token = Relay.MintTablet(side);
            TabletDriver tablet = await TabletDriver.OpenAsync(
                _web,
                $"art={Uri.EscapeDataString(art)}{(side == 0 ? string.Empty : $"&side={side}")}#device={token}",
                pictures,
                side == 0 ? "shared" : $"side-{side}",
                site);
            _tablets.Add(tablet);
        }

        foreach (TabletDriver tablet in _tablets)
        {
            await tablet.SettledAsync();
        }

        return (site, $"#ticket={_ticket}");
    }

    /// <summary>
    /// A tap with a finger on whichever tablet shows what Bot1's phone names - <c>o12</c> the object its seat knows as 12,
    /// <c>p2</c> player 2 - found there by the public handle the relay's map pairs with the seat's. A card in a graveyard or
    /// in exile is tapped in the list its pile opens, which is shut again after.
    /// </summary>
    public async Task TapAsync(string what, TableState phone)
    {
        if (what[0] == 'p')
        {
            int seat = int.Parse(what[1..], System.Globalization.CultureInfo.InvariantCulture);
            foreach (TabletDriver tablet in _tablets)
            {
                if ((await tablet.SettledAsync()).Players.Any(player => player.Seat == seat))
                {
                    await tablet.TapPlayerAsync(seat);
                    await tablet.PictureAsync($"tapped-player-{seat}");
                    return;
                }
            }

            throw new Xunit.Sdk.XunitException($"No tablet shows player {seat}.");
        }

        int seatHandle = int.Parse(what[1..], System.Globalization.CultureInfo.InvariantCulture);
        int handle = _observer.PublicHandleOf(1, seatHandle)
            ?? throw new Xunit.Sdk.XunitException($"The public sees nothing Bot1 knows as {seatHandle}.");

        foreach (TabletDriver tablet in _tablets)
        {
            TabletState state = await tablet.SettledAsync();
            if (state.Cards.FirstOrDefault(card => card.Visible && card.Handle == handle) is { } card)
            {
                await tablet.TapCardAsync(card);
                await tablet.PictureAsync($"tapped-{Describe(card)}");
                return;
            }
        }

        // Not on any board: in a pile, which a person opens to find it.
        foreach (TabletDriver tablet in _tablets)
        {
            foreach (TabletPlayer player in (await tablet.SettledAsync()).Players)
            {
                foreach (string kind in (player.Piles ?? []).Select(pile => pile.Kind).ToList())
                {
                    await tablet.TapPileAsync(player.Seat, kind);
                    TabletState opened = await tablet.SettledAsync(state => state.Players.Any(each => each.Seat == player.Seat && each.Piles!.Any(pile => pile.Kind == kind && pile.Open)));
                    TabletCard? card = opened.Cards.FirstOrDefault(each => each.Visible && each.Handle == handle && each.Place.EndsWith(kind, StringComparison.Ordinal));
                    if (card is not null)
                    {
                        await tablet.PictureAsync($"{kind}-opened");
                        await tablet.TapCardAsync(card);
                        await tablet.PictureAsync($"tapped-{Describe(card)}");
                    }

                    await tablet.TapPileAsync(player.Seat, kind);
                    await tablet.SettledAsync(state => state.Players.Any(each => each.Seat == player.Seat && each.Piles!.All(pile => !pile.Open)));
                    if (card is not null)
                    {
                        return;
                    }
                }
            }
        }

        throw new Xunit.Sdk.XunitException($"No tablet shows public handle {handle} ({what} to Bot1), on a board or in a pile.");

        static string Describe(TabletCard card) => $"{card.Name.ToLowerInvariant().Replace(' ', '-')}-{card.Place}";
    }

    /// <summary>
    /// The tablets show nothing of Bot1's hand: no card on any of them is named as a card in the phone's hand is, but for the
    /// names the public also sees elsewhere (a Mountain on the battlefield and another in the hand).
    /// </summary>
    public async Task CheckAsync(TableState phone)
    {
        HashSet<string> seen = _observer.PublicNames();
        string[] hidden = [.. phone.Cards.Where(card => card.Place == "hand" && card.Name.Length > 0).Select(card => card.Name).Where(name => !seen.Contains(name)).Distinct()];
        foreach (TabletDriver tablet in _tablets)
        {
            TabletState state = await tablet.SettledAsync();
            string[] named = [.. state.Cards.Select(card => card.Name).Where(name => hidden.Contains(name)).Distinct()];
            Assert.True(named.Length == 0, $"A tablet names Bot1's hand: {string.Join(", ", named)}.");
            Assert.Empty(tablet.Errors);
        }
    }

    /// <summary>The built client and the relay's socket on one loopback site, as <c>cantrip-in-person</c> serves them.</summary>
    private async Task<WebApplication> ServeAsync()
    {
        WebApplicationBuilder builder = WebApplication.CreateBuilder();
        builder.WebHost.UseUrls("http://127.0.0.1:0");
        builder.Logging.ClearProviders();
        WebApplication app = builder.Build();
        app.UseWebSockets();
        app.MapInPerson(Relay);
        app.UseWebClient(Path.Combine(WebClient.Root, "dotnet", "src", "Cantrip.Client.Web", "bin", "Debug", "net10.0-browserwasm", "wwwroot"));
        await app.StartAsync();
        return app;
    }

    /// <summary>The relay's clock, as its host keeps it: a tap a phone never answered goes on after its timeout.</summary>
    private async Task TickAsync(CancellationToken stop)
    {
        try
        {
            while (!stop.IsCancellationRequested)
            {
                Relay.Tick();
                await Task.Delay(250, stop);
            }
        }
        catch (OperationCanceledException)
        {
        }
    }

    public async ValueTask DisposeAsync()
    {
        await _stop.CancelAsync();
        if (_ticking is not null)
        {
            await _ticking;
        }

        foreach (TabletDriver tablet in _tablets)
        {
            await tablet.DisposeAsync();
        }

        if (_site is not null)
        {
            await _site.DisposeAsync();
        }

        _stop.Dispose();
    }

    /// <summary>Hands every step to the relay, and keeps the last public projection to find a seat's object on a tablet by.</summary>
    private sealed class Observer : IStepObserver
    {
        private readonly object _gate = new();
        private PublicProjection? _last;

        public InPersonRelay? Relay { get; set; }

        public void Stepped(IRulesHost game, RulesStep step, bool started)
        {
            lock (_gate)
            {
                _last = step.Public ?? _last;
            }

            Relay?.Stepped(game, step, started);
        }

        public void Closed(IRulesHost game) => Relay?.Closed(game);

        /// <summary>The public handle of what <paramref name="seat"/> knows as <paramref name="seatHandle"/>, if the public sees it.</summary>
        public int? PublicHandleOf(int seat, int seatHandle)
        {
            lock (_gate)
            {
                if (_last?.Handles.GetValueOrDefault(seat) is not { } map)
                {
                    return null;
                }

                foreach ((ViewHandle publicHandle, ViewHandle seats) in map)
                {
                    if (seats.Value == seatHandle)
                    {
                        return publicHandle.Value;
                    }
                }

                return null;
            }
        }

        /// <summary>The names of every card the public view shows face up.</summary>
        public HashSet<string> PublicNames()
        {
            lock (_gate)
            {
                return [.. (_last?.View.Objects ?? []).Where(card => card.Known && card.Name.Length > 0).Select(card => card.Name)];
            }
        }
    }
}
