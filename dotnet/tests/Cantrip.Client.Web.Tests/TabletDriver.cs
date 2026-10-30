using System.Text.Json;
using Cantrip.Protocol.InPerson;
using MessagePack;
using Microsoft.Playwright;
using Xunit;

namespace Cantrip.Client.Web.Tests;

/// <summary>
/// One in-person tablet (<c>?inperson=tablet</c>, docs/in-person.md), driven as a person's fingers drive it: a landscape
/// tablet's page with a touch screen, read from what it publishes on <c>globalThis.cantripTest</c> (the client's
/// InPerson/TabletTestState.cs) and tapped with real touch events at those places, so a tap goes through the hit testing
/// a finger's does - the turned half's included. A tablet opened with no device token is fed by the test
/// (<see cref="FeedAsync"/>) and keeps the taps it would have sent the relay (<see cref="TabletState.Taps"/>).
/// </summary>
public sealed class TabletDriver : IAsyncDisposable
{
    /// <summary>A landscape tablet's page.</summary>
    public const int Width = 1280;
    public const int Height = 800;

    private static readonly JsonSerializerOptions Json = new() { PropertyNameCaseInsensitive = true };
    private static readonly TimeSpan Patience = TimeSpan.FromSeconds(90);

    private readonly IBrowserContext _context;
    private readonly List<string> _errors = [];
    private readonly string _pictures;
    private readonly string _name;
    private int _pictured;

    private TabletDriver(IBrowserContext context, IPage page, string pictures, string name)
    {
        _context = context;
        Page = page;
        _pictures = pictures;
        _name = name;
        page.PageError += (_, error) => _errors.Add(error);
    }

    public IPage Page { get; }

    /// <summary>Uncaught errors the page has thrown, which a test fails on.</summary>
    public IReadOnlyList<string> Errors => _errors;

    /// <summary>
    /// Opens a tablet with these query parameters - <c>side=N</c> for a player's own board, <c>art=...</c> for where its
    /// art is - and <c>inperson=tablet&amp;test</c>; a <c>#device=</c> fragment connects it to a relay instead of the test.
    /// <paramref name="name"/> prefixes its pictures under <paramref name="pictures"/>.
    /// </summary>
    public static async Task<TabletDriver> OpenAsync(WebClient web, string query, string pictures, string name)
    {
        IBrowserContext context = await web.Browser.NewContextAsync(new BrowserNewContextOptions
        {
            ViewportSize = new ViewportSize { Width = Width, Height = Height },
            HasTouch = true,
        });

        IPage page = await context.NewPageAsync();
        var tablet = new TabletDriver(context, page, pictures, name);
        int hash = query.IndexOf('#', StringComparison.Ordinal);
        string head = hash < 0 ? query : query[..hash];
        string fragment = hash < 0 ? string.Empty : query[hash..];
        await page.GotoAsync($"{web.Site}?inperson=tablet{(head.Length > 0 ? "&" + head : string.Empty)}&test{fragment}");

        // The page's scripts are loaded with the runtime, after the document itself.
        await page.WaitForFunctionAsync("() => typeof globalThis.cantripTabletFeed === 'function'", null, new PageWaitForFunctionOptions { Timeout = (float)Patience.TotalMilliseconds });
        return tablet;
    }

    /// <summary>Hands the page one message of the device's wire, as the relay would send it.</summary>
    public async Task FeedAsync(DeviceServerMessage message) =>
        await Page.EvaluateAsync("m => globalThis.cantripTabletFeed(m)", Convert.ToBase64String(MessagePackSerializer.Serialize(message)));

    /// <summary>The page's state as it last published it; null until the client has started.</summary>
    public async Task<TabletState?> StateAsync()
    {
        string? json = await Page.EvaluateAsync<string?>("() => globalThis.cantripTest ?? null");
        return json is null ? null : JsonSerializer.Deserialize<TabletState>(json, Json);
    }

    /// <summary>Waits until the page is up and has stopped moving for a few readings in a row, and what else is asked holds.</summary>
    public async Task<TabletState> SettledAsync(Func<TabletState, bool>? until = null)
    {
        DateTime giveUp = DateTime.UtcNow + Patience;
        int still = 0;
        TabletState? state = null;
        while (DateTime.UtcNow < giveUp)
        {
            state = await StateAsync();
            still = state is { Ready: true, Busy: false } && (until?.Invoke(state) ?? true) ? still + 1 : 0;
            if (still >= 3)
            {
                return state!;
            }

            await Task.Delay(100);
        }

        await PictureAsync("never-settled");
        Assert.Fail($"The tablet never settled in {Patience.TotalSeconds:0} s. Last state: screen '{state?.Screen}', busy {state?.Busy} ({state?.Doing}), {state?.Cards.Count} cards. Page errors: {string.Join(" | ", _errors)}");
        return null!;
    }

    /// <summary>The first card in view with this name, on a row whose place is this, if one is named.</summary>
    public async Task<TabletCard> CardAsync(string name, string? place = null)
    {
        TabletState state = await SettledAsync();
        return state.Cards.FirstOrDefault(card => card.Visible && card.Name == name && (place is null || card.Place == place))
            ?? throw new Xunit.Sdk.XunitException($"No card named '{name}' in view. In view: {string.Join(", ", state.Cards.Where(c => c.Visible).Select(c => $"{c.Name} ({c.Place})").Distinct())}");
    }

    /// <summary>Taps a point with a finger.</summary>
    public Task TapAsync(float x, float y) => Page.Touchscreen.TapAsync(x, y);

    /// <summary>Taps a card in view, as a finger taps it.</summary>
    public async Task TapCardAsync(TabletCard card) => await TapAsync(card.X, card.Y);

    /// <summary>Taps a player's box.</summary>
    public async Task TapPlayerAsync(int seat)
    {
        TabletState state = await SettledAsync();
        TabletPlayer player = state.Players.FirstOrDefault(each => each.Seat == seat)
            ?? throw new Xunit.Sdk.XunitException($"No box for player {seat} on this tablet.");
        await TapAsync(player.X, player.Y);
    }

    /// <summary>
    /// Presses a card and keeps the finger on it, as a long press does, until <see cref="LiftAsync"/>. Sent as the touch
    /// events themselves, since Playwright's touch screen only taps.
    /// </summary>
    public async Task PressAsync(TabletCard card)
    {
        _cdp ??= await _context.NewCDPSessionAsync(Page);
        _pressed = (card.X, card.Y);
        await _cdp.SendAsync("Input.dispatchTouchEvent", new Dictionary<string, object>
        {
            ["type"] = "touchStart",
            ["touchPoints"] = new[] { new Dictionary<string, object> { ["x"] = card.X, ["y"] = card.Y } },
        });
    }

    /// <summary>Lifts the finger a long press put down.</summary>
    public async Task LiftAsync()
    {
        if (_cdp is null || _pressed is null)
        {
            return;
        }

        await _cdp.SendAsync("Input.dispatchTouchEvent", new Dictionary<string, object>
        {
            ["type"] = "touchEnd",
            ["touchPoints"] = Array.Empty<object>(),
        });
        _pressed = null;
    }

    private ICDPSession? _cdp;
    private (float X, float Y)? _pressed;

    /// <summary>Takes a picture of the page, numbered in the order taken, and returns where it went.</summary>
    public async Task<string> PictureAsync(string name)
    {
        string path = Path.Combine(_pictures, $"{_name}-{++_pictured:00}-{name}.png");

        // The splash fades away over a page already running, and goes once it has.
        await Page.WaitForFunctionAsync("() => !document.querySelector('.uno-loader')", null, new PageWaitForFunctionOptions { Timeout = (float)Patience.TotalMilliseconds });
        await Page.ScreenshotAsync(new PageScreenshotOptions { Path = path });
        return path;
    }

    public async ValueTask DisposeAsync() => await _context.DisposeAsync();
}

/// <summary>What a tablet publishes under <c>?test</c> (InPerson/TabletTestState.cs).</summary>
public sealed record TabletState(
    bool Ready,
    bool Busy,
    string Doing,
    bool Over,
    string Layout,
    string Screen,
    string Phase,
    string PhaseLine,
    int Seat,
    int Held,
    string Tooltip,
    int Flown,
    string Lives,
    IReadOnlyList<TabletCard> Cards,
    IReadOnlyList<TabletPlayer> Players,
    IReadOnlyList<TabletTap> Taps);

/// <summary>
/// A card on a tablet: its name (empty for one nobody at the table may see), the middle and size of its box on the page,
/// and where it is - <c>battlefield</c> on the half at the tablet's bottom edge, <c>opponent-battlefield</c> on the turned
/// one, <c>stack</c> - and whose half that is.
/// </summary>
public sealed record TabletCard(string Name, bool Visible, string Place, int Seat, int Handle, bool Tapped, float X, float Y, float W, float H);

/// <summary>A player's box on a tablet, which a tap picks them with.</summary>
public sealed record TabletPlayer(int Seat, string Name, int Life, bool Turned, float X, float Y, float W, float H);

/// <summary>A tap the tablet sent: an object by its public handle (zero for a player), or a player by seat.</summary>
public sealed record TabletTap(int Object, int Player, long Sequence);
