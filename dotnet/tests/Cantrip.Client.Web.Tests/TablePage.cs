using System.Text.Json;
using Microsoft.Playwright;
using Xunit;

namespace Cantrip.Client.Web.Tests;

/// <summary>
/// One page of the web client, driven as a player drives it. The table is drawn on canvases, so there is no DOM to find
/// a card or a button in; the page is opened with <c>?test</c> and publishes where everything is, by name, on
/// <c>globalThis.cantripTest</c> (the client's MainPage.Test.cs). Everything here reads that and then acts with real
/// mouse events at those places, so a click goes through the same hit testing a person's does.
/// </summary>
public sealed class TablePage : IAsyncDisposable
{
    private static readonly JsonSerializerOptions Json = new() { PropertyNameCaseInsensitive = true };

    /// <summary>How long a page may take to load and settle: a Debug build is interpreted and loads slowly.</summary>
    private static readonly TimeSpan Patience = TimeSpan.FromSeconds(90);

    private readonly IBrowserContext _context;
    private readonly List<string> _errors = [];
    private readonly string _pictures;
    private int _pictured;

    private TablePage(IBrowserContext context, IPage page, string pictures, bool phone)
    {
        _context = context;
        Page = page;
        _pictures = pictures;
        Phone = phone;
        page.PageError += (_, error) => _errors.Add(error);
    }

    public IPage Page { get; }

    /// <summary>
    /// Whether the page is a phone's: opened upright at a phone's size, on a touch screen (<see cref="PhoneViewport"/>), and
    /// tapped rather than clicked.
    /// </summary>
    public bool Phone { get; }

    /// <summary>A phone held upright, as the in-person tests open one: a common handset's size in CSS pixels.</summary>
    public static readonly ViewportSize PhoneViewport = new() { Width = 390, Height = 844 };

    /// <summary>A click at a point, as a person makes it: a tap with a finger on a phone's page, the mouse's left button elsewhere.</summary>
    public Task TapAsync(float x, float y) => Phone ? Page.Touchscreen.TapAsync(x, y) : Page.Mouse.ClickAsync(x, y);

    /// <summary>Uncaught errors the page has thrown, which a test fails on.</summary>
    public IReadOnlyList<string> Errors => _errors;

    /// <summary>
    /// Opens the client with these query parameters and <c>test</c>, for a test that pictures what it does under
    /// <paramref name="pictures"/> (<see cref="WebClient.Pictures"/>). <paramref name="settings"/> are the player's
    /// settings as the page keeps them in localStorage, by name without the page's prefix ("auto-scale", "scale"), there
    /// before the page reads them; <paramref name="storage"/> is anything else in localStorage, by its full key
    /// ("cantrip-stops-mine").
    /// </summary>
    public static Task<TablePage> OpenAsync(
        WebClient web,
        string query,
        string pictures,
        IReadOnlyDictionary<string, string>? settings = null,
        IReadOnlyDictionary<string, string>? storage = null,
        bool phone = false) =>
        OpenAsync(web.Browser, web.Site, query, pictures, settings, storage, phone: phone);

    /// <summary>
    /// Opens the client served from another site than the suite's own - one with more behind it, as the deck editor's
    /// site has its decks - sending <paramref name="headers"/> with every request the page makes.
    /// </summary>
    public static async Task<TablePage> OpenAsync(
        IBrowser browser,
        Uri site,
        string query,
        string pictures,
        IReadOnlyDictionary<string, string>? settings = null,
        IReadOnlyDictionary<string, string>? storage = null,
        IEnumerable<KeyValuePair<string, string>>? headers = null,
        bool phone = false)
    {
        // A phone's page is a phone's size even headed, on a touch screen, as a phone's browser lays it out.
        IBrowserContext context = await browser.NewContextAsync(new BrowserNewContextOptions
        {
            ViewportSize = phone ? PhoneViewport
                : WebClient.Headed ? ViewportSize.NoViewport : new ViewportSize { Width = WebClient.Width, Height = WebClient.Height },
            HasTouch = phone,
            IsMobile = phone,
            DeviceScaleFactor = phone ? 2 : null,
            ExtraHTTPHeaders = headers,
        });

        foreach ((string name, string value) in settings ?? new Dictionary<string, string>())
        {
            await context.AddInitScriptAsync($"localStorage.setItem({JsonSerializer.Serialize("cantrip-setting-" + name)}, {JsonSerializer.Serialize(value)});");
        }

        foreach ((string key, string value) in storage ?? new Dictionary<string, string>())
        {
            await context.AddInitScriptAsync($"localStorage.setItem({JsonSerializer.Serialize(key)}, {JsonSerializer.Serialize(value)});");
        }

        IPage page = await context.NewPageAsync();
        var table = new TablePage(context, page, pictures, phone);
        // Into the query, before any fragment the caller gave (a ticket after the hash).
        int hash = query.IndexOf('#', StringComparison.Ordinal);
        string address = hash < 0 ? $"{site}?{query}&test" : $"{site}?{query[..hash]}&test{query[hash..]}";
        await page.GotoAsync(address);
        return table;
    }

    /// <summary>
    /// The page's state as it last published it; null until the client has started - and while the page is loading
    /// again, as it does when its seat is moved to another table.
    /// </summary>
    public async Task<TableState?> StateAsync()
    {
        string? json;
        try
        {
            json = await Page.EvaluateAsync<string?>("() => globalThis.cantripTest ?? null");
        }
        catch (PlaywrightException navigating) when (navigating.Message.Contains("context was destroyed", StringComparison.Ordinal)
            || navigating.Message.Contains("navigating", StringComparison.OrdinalIgnoreCase))
        {
            return null;
        }

        return json is null ? null : JsonSerializer.Deserialize<TableState>(json, Json);
    }

    /// <summary>
    /// Waits until the table is up and has stopped moving - no face being made, no art on its way, no card in the air -
    /// for a few readings in a row, and returns it.
    /// </summary>
    public async Task<TableState> SettledAsync(Func<TableState, bool>? until = null)
    {
        DateTime giveUp = DateTime.UtcNow + Patience;
        int still = 0;
        TableState? state = null;

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

        Assert.Fail($"The table never settled in {Patience.TotalSeconds:0} s. Last state: {Describe(state)}. Page errors: {string.Join(" | ", _errors)}");
        return null!;
    }

    /// <summary>Waits for the game to end, however long a whole game takes.</summary>
    public async Task<TableState> OverAsync(TimeSpan within)
    {
        DateTime giveUp = DateTime.UtcNow + within;
        TableState? state = null;
        while (DateTime.UtcNow < giveUp)
        {
            state = await StateAsync();
            if (state?.Over == true)
            {
                return state;
            }

            await Task.Delay(250);
        }

        await PictureAsync("not-over");
        Assert.Fail($"The game was not over in {within.TotalSeconds:0} s. Last state: {Describe(state)}. Page errors: {string.Join(" | ", _errors)}");
        return null!;
    }

    /// <summary>The first card in view with this name.</summary>
    public async Task<CardState> CardAsync(string name)
    {
        TableState state = await SettledAsync();
        return state.Cards.FirstOrDefault(card => card.Visible && card.Name == name)
            ?? throw new Xunit.Sdk.XunitException($"No card named '{name}' in view. In view: {string.Join(", ", state.Cards.Where(c => c.Visible).Select(c => c.Name).Distinct())}");
    }

    /// <summary>Turns the mouse wheel over a point, a notch down for a positive <paramref name="notches"/>.</summary>
    public async Task WheelAsync(float x, float y, int notches)
    {
        await Page.Mouse.MoveAsync(x, y);
        await Page.Mouse.WheelAsync(0, notches * 100);
    }

    /// <summary>Rests the pointer on a point, long enough for a tooltip to show.</summary>
    public async Task RestAsync(float x, float y)
    {
        await Page.Mouse.MoveAsync(x - 3, y - 3);
        await Page.Mouse.MoveAsync(x, y);
        await Task.Delay(900);
    }

    /// <summary>Clicks a card in view, as a person clicks it.</summary>
    public async Task ClickCardAsync(string name)
    {
        CardState card = await CardAsync(name);
        await Page.Mouse.ClickAsync(card.X, card.Y);
    }

    /// <summary>Presses the step up, or down, beside the share of a division a card wears: combat damage, divided damage or counters.</summary>
    public async Task StepShareAsync(string name, bool up)
    {
        CardState card = await CardAsync(name);
        Spot step = (up ? card.Raise : card.Lower) ?? throw new Xunit.Sdk.XunitException($"'{name}' has no share to step.");
        await Page.Mouse.ClickAsync(step.X, step.Y);
    }

    /// <summary>Holds a card up to read, as the right button held down does; <see cref="PutDownAsync"/> lets it go.</summary>
    public async Task HoldUpAsync(string name) => await HoldUpAsync(await CardAsync(name));

    /// <summary>Holds up this card, where there are several of one name.</summary>
    public async Task HoldUpAsync(CardState card)
    {
        await Page.Mouse.MoveAsync(card.X, card.Y);
        await Page.Mouse.DownAsync(new MouseDownOptions { Button = MouseButton.Right });
    }

    /// <summary>Holds a card up from its name in a line of text, as the right button held down on the name does.</summary>
    public async Task HoldUpAsync(MentionState name)
    {
        await Page.Mouse.MoveAsync(name.X, name.Y);
        await Page.Mouse.DownAsync(new MouseDownOptions { Button = MouseButton.Right });
    }

    public Task PutDownAsync() => Page.Mouse.UpAsync(new MouseUpOptions { Button = MouseButton.Right });

    /// <summary>Clicks a control - a button, a toggle, a drop-down, a row of an open menu - by what it says.</summary>
    public async Task ClickAsync(string label, string? kind = null)
    {
        TableState state = await SettledAsync();
        ControlState control = state.Controls.FirstOrDefault(c => (kind is null || c.Kind == kind) && c.Label == label)
            ?? state.Controls.FirstOrDefault(c => (kind is null || c.Kind == kind) && c.Label.Contains(label, StringComparison.Ordinal))
            ?? throw new Xunit.Sdk.XunitException($"No control saying '{label}'. There are: {string.Join(", ", state.Controls.Select(c => $"{c.Kind} '{c.Label}'"))}");
        await TapAsync(control.X, control.Y);
    }

    /// <summary>Clicks into a box by its placeholder and types into it, key by key, as a person does.</summary>
    public async Task TypeAsync(string placeholder, string text)
    {
        await ClickAsync(placeholder, "textbox");
        await Page.Keyboard.TypeAsync(text, new KeyboardTypeOptions { Delay = 50 });
    }

    /// <summary>Picks an item from a drop-down by its caption, as Sort: Name.</summary>
    public async Task ChooseAsync(string caption, string item)
    {
        await ClickAsync(caption, "dropdown");
        await ClickAsync(item, "item");
    }

    /// <summary>Picks a card up and carries it to a point, letting go there, as a person drags it with the mouse.</summary>
    public async Task DragAsync(CardState card, float x, float y)
    {
        await Page.Mouse.MoveAsync(card.X, card.Y);
        await Page.Mouse.DownAsync();
        await Page.Mouse.MoveAsync(card.X + 12, card.Y + 12, new MouseMoveOptions { Steps = 4 });
        await Page.Mouse.MoveAsync(x, y, new MouseMoveOptions { Steps = 12 });
        await Page.Mouse.UpAsync();
    }

    /// <summary>Clicks a card twice in quick succession.</summary>
    public async Task DoubleClickCardAsync(CardState card) =>
        await Page.Mouse.DblClickAsync(card.X, card.Y);

    /// <summary>Takes a picture of the page, numbered in the order taken, and returns where it went.</summary>
    public async Task<string> PictureAsync(string name)
    {
        string path = Path.Combine(_pictures, $"{++_pictured:00}-{name}.png");
        await Page.ScreenshotAsync(new PageScreenshotOptions { Path = path });
        return path;
    }

    private static string Describe(TableState? state) => state is null
        ? "nothing published (the client never started)"
        : $"ready {state.Ready}, busy {state.Busy} ({state.Doing}), over {state.Over}, prompt '{state.Prompt}' (request {state.RequestId}), {state.Cards.Count} cards";

    public async ValueTask DisposeAsync() => await _context.DisposeAsync();
}

/// <summary>
/// What the page publishes under <c>?test</c>. <see cref="Phase"/> is the table's: Setup, Drafting, DeckBuilding, Playing...
/// <see cref="Art"/> is where it is asking for card art, as a URL template. <see cref="Answered"/> is, in the prompt
/// gallery, the last answer the dock sent - its kind, and the indexes of any modes or options chosen.
/// <see cref="Combat"/> is each combat group as its ground is captioned, with its attackers: "Attacking Bot2: Ornithopter".
/// <see cref="Selected"/> is how many creatures are picked up to attack or block, not yet sent anywhere.
/// <see cref="AnsweredRequest"/> is the request the page last answered, by a click or by itself.
/// </summary>
public sealed record TableState(
    bool Ready,
    bool Busy,
    string Doing,
    bool Over,
    string Prompt,
    string Fixture,
    string Outcome,
    int Flown,
    int Held,
    LogState Log,
    string Tooltip,
    IReadOnlyList<CardState> Cards,
    IReadOnlyList<ControlState> Controls,
    string Phase,
    PairingsState? Pairings = null,
    IReadOnlyList<MentionState>? Mentions = null,
    string Art = "",
    string Answered = "",
    int Selected = 0,
    int RequestId = 0,
    int AnsweredRequest = 0,
    int Seat = 0,
    IReadOnlyList<PlayerState>? Players = null,
    IReadOnlyList<EffectState>? Effects = null,
    string Title = "",
    string Detail = "",
    IReadOnlyList<string>? Modes = null,
    IReadOnlyList<string>? LogText = null,
    bool GraveyardsOpen = false,
    IReadOnlyList<DockButtonState>? Dock = null,
    IReadOnlyList<string>? Combat = null,
    DeckEditorState? DeckEditor = null,
    IReadOnlyList<string>? TargetLabels = null,
    IReadOnlyList<CompanionWindowState>? Companions = null,
    InPersonState? InPerson = null);

/// <summary>
/// A phone at an in-person table (MainPage.InPerson.cs): whether its Board is shown, how many taps from the tablets wait,
/// each tap it settled, and the line saying a tap did nothing while it is up. Null for any other page.
/// </summary>
public sealed record InPersonState(
    string Role,
    bool Board,
    int Waiting,
    IReadOnlyList<TableTapState> Taps,
    string Refusal,
    PhoneDeckState? Deck = null,
    PhoneDraftState? Draft = null,
    IReadOnlyList<PhoneLineState>? Lines = null);

/// <summary>
/// A phone's screen during the grid draft (InPerson/PhonePickScreen.cs): whether it is up, whether the pick is this seat's,
/// what it says, how many cards the pool has, whether a card is shown whole, and the pool's rows with their groups.
/// </summary>
public sealed record PhoneDraftState(bool Shown, bool Mine, string Title, string Grid, string Status, int Pool, bool Preview, IReadOnlyList<PhoneDraftRow> Rows);

/// <summary>A card of the pool on the pick screen: its name, the group it is under, whether it is in view, and its box.</summary>
public sealed record PhoneDraftRow(string Name, string Group, bool InView, float X, float Y, float W, float H);

/// <summary>A line of the grid picked on a tablet that the phone acted on: chosen, or taken.</summary>
public sealed record PhoneLineState(int Decision, int Line, bool Take);

/// <summary>
/// A phone's deck building (InPerson/PhoneDeckBuilder.cs): whether it is up, the list shown (<c>Deck</c>, <c>Sideboard</c>,
/// <c>Lands</c>), the deck's size against the least, whether it is submitted or a card is shown whole, the rows and buttons
/// in view, and the basic lands.
/// </summary>
public sealed record PhoneDeckState(
    bool Shown,
    string Tab,
    int Size,
    int Minimum,
    bool Submitted,
    bool Preview,
    IReadOnlyList<PhoneDeckRow> Rows,
    IReadOnlyList<PhoneDeckButton> Buttons,
    IReadOnlyList<PhoneDeckBasic> Basics);

/// <summary>A card's row: its name, its place in the pool, whether it is in the deck, and its box on the page.</summary>
public sealed record PhoneDeckRow(string Name, int Place, bool InDeck, float X, float Y, float W, float H);

/// <summary>A button of the phone's deck building: "Submit deck", "Tab Lands", "Add Mountain", "Colour red", "By type", "Show Name".</summary>
public sealed record PhoneDeckButton(string Label, bool Enabled, float X, float Y, float W, float H);

/// <summary>A basic land and how many are in the deck.</summary>
public sealed record PhoneDeckBasic(string Name, int Count);

/// <summary>One tap from a tablet a phone settled: the seat's object tapped or the player, the request it was for, whether it was used and had to wait for its prompt.</summary>
public sealed record TableTapState(int TapId, int RequestId, int Handle, int Player, bool Used, bool Held);

/// <summary>A companion window over a half of the table: whose it is ("mine" or "opponent") and which corner it stands in.</summary>
public sealed record CompanionWindowState(string Whose, string Corner);

/// <summary>
/// The deck editor (<c>?decks</c>) as the page publishes it: the deck's id, name and format, whether it has changes not
/// saved, the question asked over it and what it says, its status line and problems, how many cards the search finds and how many of them
/// are laid out (only the rows near the view are), each zone's
/// cards ("2 Mountain"), the player's saved decks ("standard: Red Test (2)"), where the command zone is, and what it says of the
/// deck's companions once the site has judged them.
/// </summary>
public sealed record DeckEditorState(
    string Id,
    string Name,
    string Format,
    bool Unsaved,
    bool SignedIn,
    bool Asking,
    string Question,
    string Status,
    int Results,
    IReadOnlyList<string> Problems,
    IReadOnlyList<string> Commanders,
    IReadOnlyList<string> Main,
    IReadOnlyList<string> Side,
    IReadOnlyList<string> Decks,
    Spot? CommandZone = null,
    int LaidOut = 0,
    IReadOnlyList<string>? Picked = null,
    Spot? Marquee = null,
    string AskDetail = "",
    bool Checked = false,
    string Companion = "");

/// <summary>A button of the dock, in the order the dock offers them, and whether it can be pressed.</summary>
public sealed record DockButtonState(string Label, bool Enabled);

/// <summary>
/// A player's box on the page, which a click on targets them, and the mana in their pool; the facts beside their life
/// total, the buttons into exile and a sideboard, and their graveyard pile: whether it is lit, its note and where to click.
/// </summary>
public sealed record PlayerState(
    int Seat,
    float X,
    float Y,
    float W,
    float H,
    IReadOnlyList<ManaChipState> Mana,
    IReadOnlyList<string>? Facts = null,
    IReadOnlyList<ZoneButtonState>? Zones = null,
    bool GraveyardLit = false,
    string GraveyardNote = "",
    float GraveyardX = 0,
    float GraveyardY = 0,
    string Name = "");

/// <summary>A button over a player's piles (Exile, Companion): what it says and its tooltip, whether it is lit, and where it is.</summary>
public sealed record ZoneButtonState(string Text, string Tooltip, bool Lit, bool Open, float X, float Y, float W, float H);

/// <summary>
/// A kind of mana in a pool, how much, whether a click spends one, and where it is; its caption says what restricts it
/// ("land abilities") and how much of it is spoken for.
/// </summary>
public sealed record ManaChipState(string Type, int Count, bool Clickable, float X, float Y, float W, float H, string Caption = "");

/// <summary>An ability shown resolving: the way it cut, and each arrow's target and way.</summary>
public sealed record EffectState(string Feeling, bool Lit, IReadOnlyList<ArrowState> Arrows);

public sealed record ArrowState(string To, string Feeling);

/// <summary>An event's pairings as the page shows them: the round, whether final, how long before it begins, each match in words.</summary>
public sealed record PairingsState(int Round, bool Final, int StartsIn, IReadOnlyList<string> Matches);

/// <summary>How many lines the game log has, and how many of them the page has made as controls.</summary>
public sealed record LogState(int Lines, int Shown);

/// <summary>
/// A card on the page: its name (empty when this seat may not see it), the middle and size of its box, and which of an
/// event's rows it is in - <c>pack</c>, <c>pool</c> or <c>sideboard</c> - if any.
/// </summary>
public sealed record CardState(
    string Name,
    bool Visible,
    float X,
    float Y,
    float W,
    float H,
    Spot? Lower,
    Spot? Raise,
    string Row = "",
    Spot? FlewFrom = null,
    string Place = "",
    int Handle = 0,
    bool Tapped = false,
    string Doors = "",
    string Actions = "",
    string Halves = "",
    string TypeLine = "",
    string Rules = "",
    string Status = "",
    string Annotation = "",
    string Counters = "",
    string Damage = "",
    string FaceDownName = "",
    bool Picked = false);

/// <summary>A card named in a line of text - the game log, a draft's picks - where its name is, and whether it is in view.</summary>
public sealed record MentionState(string Name, bool Visible, float X, float Y, float W, float H);

/// <summary>A control on the page: <c>button</c>, <c>toggle</c>, <c>dropdown</c> (by its caption), <c>textbox</c> (by its placeholder) or <c>item</c> (a menu row).</summary>
public sealed record ControlState(string Kind, string Label, float X, float Y, float W, float H, int Ability = -1, int OptionId = 0);

public sealed record Spot(float X, float Y);
