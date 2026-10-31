using System.Text.Json;
using Cantrip.Server.Host;
using Cantrip.Server.InPerson;
using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Hosting;
using Microsoft.Extensions.Logging;
using Microsoft.Playwright;
using Xunit;

namespace Cantrip.Client.Web.Tests;

/// <summary>
/// Full screen at an in-person table (docs/in-person.md, "Full screen"): the Fullscreen button on a tablet and a phone,
/// entering and leaving full screen in Chromium with a finger's tap and following the browser when it is left some other
/// way; on an iPhone, which has no Fullscreen API, the explanation of Add to Home Screen and the re-entry code a
/// home-screen app comes back to its seat with; and none of it on the remote client.
/// </summary>
[Collection(WebClientCollection.Name)]
[Trait("Category", "WallClock")]
public sealed class FullScreenTests(WebClient web)
{
    private static readonly TimeSpan Patience = TimeSpan.FromSeconds(90);

    /// <summary>No game server: a phone's page that reaches no table still lays itself out and reaches its relay.</summary>
    private const string NoTable = "server=ws%3A%2F%2F127.0.0.1%3A9%2Fgame";

    /// <summary>What an iPhone's Safari has: no Fullscreen API on the page (only on a video), and an iPhone's name.</summary>
    private const string NoFullscreenApi = """
        Object.defineProperty(Document.prototype, 'fullscreenEnabled', { get: () => false });
        Object.defineProperty(Document.prototype, 'webkitFullscreenEnabled', { get: () => false });
        Element.prototype.requestFullscreen = undefined;
        """;

    private const string IPhoneAgent =
        "Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Mobile/15E148 Safari/604.1";

    /// <summary>What an app opened from an iPhone's Home Screen says of itself.</summary>
    private const string Standalone = "Object.defineProperty(Navigator.prototype, 'standalone', { get: () => true });";

    [Fact]
    public async Task A_tablet_enters_and_leaves_full_screen_with_its_button_and_follows_the_browser()
    {
        string pictures = WebClient.Pictures(nameof(FullScreenTests));
        await using TabletDriver tablet = await TabletDriver.OpenAsync(web, string.Empty, pictures, "tablet");
        await tablet.SettledAsync();
        await AssertInPersonHeadAsync(tablet.Page);

        JsonElement button = await FullScreenAsync(tablet.Page, "tablet", state => state.GetProperty("state").GetString() == "enter");
        Assert.Equal("Full screen", button.GetProperty("caption").GetString());
        await tablet.PictureAsync("full-screen-button");

        await tablet.TapAsync(X(button), Y(button));
        await tablet.Page.WaitForFunctionAsync("() => document.fullscreenElement === document.documentElement", null, Wait);
        button = await FullScreenAsync(tablet.Page, "tablet", state => state.GetProperty("state").GetString() == "exit");
        Assert.Equal("Exit full screen", button.GetProperty("caption").GetString());

        await tablet.TapAsync(X(button), Y(button));
        await tablet.Page.WaitForFunctionAsync("() => document.fullscreenElement === null", null, Wait);
        button = await FullScreenAsync(tablet.Page, "tablet", state => state.GetProperty("state").GetString() == "enter");

        // Left some other way - Escape, the system's own gesture - the button follows.
        await tablet.TapAsync(X(button), Y(button));
        await tablet.Page.WaitForFunctionAsync("() => document.fullscreenElement !== null", null, Wait);
        await FullScreenAsync(tablet.Page, "tablet", state => state.GetProperty("state").GetString() == "exit");
        await tablet.Page.EvaluateAsync("() => document.exitFullscreen()");
        await FullScreenAsync(tablet.Page, "tablet", state => state.GetProperty("state").GetString() == "enter");
        Assert.Empty(tablet.Errors);
    }

    [Fact]
    public async Task A_phone_enters_and_leaves_full_screen_with_its_button()
    {
        await using IBrowserContext context = await PhoneContextAsync(iPhone: false, standalone: false);
        IPage phone = await OpenPhoneAsync(context, $"{web.Site}?inperson=phone&{NoTable}&test");
        await AssertInPersonHeadAsync(phone);

        JsonElement button = await FullScreenAsync(phone, "phone", state => state.GetProperty("state").GetString() == "enter");
        await Task.Delay(TimeSpan.FromSeconds(1.5));
        await phone.ScreenshotAsync(new PageScreenshotOptions { Path = Path.Combine(WebClient.Pictures(nameof(FullScreenTests)), "phone-full-screen-button.png") });
        await phone.Touchscreen.TapAsync(X(button), Y(button));
        await phone.WaitForFunctionAsync("() => document.fullscreenElement === document.documentElement", null, Wait);
        button = await FullScreenAsync(phone, "phone", state => state.GetProperty("state").GetString() == "exit");

        await phone.Touchscreen.TapAsync(X(button), Y(button));
        await phone.WaitForFunctionAsync("() => document.fullscreenElement === null", null, Wait);
        await FullScreenAsync(phone, "phone", state => state.GetProperty("state").GetString() == "enter");
    }

    /// <summary>
    /// An iPhone: the button explains Add to Home Screen and puts a re-entry code after the address's #. The app made of
    /// that address - storage of its own, nothing of the tab's - trades the code at the relay for a fresh seat ticket, keeps
    /// it and loads again with it, taking the phone's place from the tab; launched again from its icon, with the same spent
    /// code in its address, it comes back with the ticket it kept. The tab, its place taken, does not take it back.
    /// </summary>
    [Fact]
    public async Task An_iphone_is_told_to_add_the_page_to_its_home_screen_and_the_app_comes_back_to_the_seat()
    {
        List<string> presented = [];
        List<int> issued = [];
        var relay = new InPersonRelay(new InPersonRelayOptions
        {
            PlayerNames = ["Ann", "Bo"],
            SeatForTicket = ticket =>
            {
                lock (presented) presented.Add(ticket);
                return ticket is "first-ticket" or "app-ticket" ? 1 : 0;
            },
            IssueTicket = seat =>
            {
                lock (issued) issued.Add(seat);
                return "app-ticket";
            },
        });
        await using WebApplication site = await ServeAsync(relay);
        string pictures = WebClient.Pictures(nameof(FullScreenTests));

        // Safari's tab, seated by its QR code's ticket.
        await using IBrowserContext safari = await PhoneContextAsync(iPhone: true, standalone: false);
        IPage tab = await OpenPhoneAsync(safari, $"{site.Urls.First()}/?inperson=phone&{NoTable}&test#ticket=first-ticket");
        await Until(() => relay.Devices.Single(device => device.Seat == 1).Connected, "the tab's phone at the relay");
        await AssertInPersonHeadAsync(tab);

        JsonElement button = await FullScreenAsync(tab, "phone", state => state.GetProperty("state").GetString() == "homescreen");
        Assert.Equal("Full screen", button.GetProperty("caption").GetString());
        await tab.Touchscreen.TapAsync(X(button), Y(button));
        await FullScreenAsync(tab, "phone", state => state.GetProperty("explaining").GetBoolean() && state.GetProperty("reentry").GetBoolean());
        await tab.ScreenshotAsync(new PageScreenshotOptions { Path = Path.Combine(pictures, "iphone-add-to-home-screen.png") });

        // The address the icon is made of: the page's own, with the code after the #, the ticket long gone from it.
        var address = new Uri(tab.Url);
        Assert.Matches("^#reenter=[0-9A-F]{64}$", address.Fragment);
        Assert.DoesNotContain("first-ticket", tab.Url, StringComparison.Ordinal);

        // The home-screen app: opened at that address, with nothing in its storage.
        await using IBrowserContext app = await PhoneContextAsync(iPhone: true, standalone: true);
        IPage launched = await OpenPhoneAsync(app, address.AbsoluteUri);
        await launched.WaitForFunctionAsync("() => globalThis.cantripTicket === 'app-ticket' && typeof globalThis.cantripTest === 'string'", null, Wait);
        Assert.Equal("app-ticket", await launched.EvaluateAsync<string>("() => localStorage.getItem('cantrip-inperson-ticket:ws://127.0.0.1:9/game')"));
        await Until(() => relay.Devices.Single(device => device.Seat == 1).Connected, "the app at the relay");
        Assert.Equal([1], issued);

        // In an app the page has the whole screen already: no button.
        await launched.WaitForFunctionAsync("() => typeof globalThis.cantripTest === 'string' && JSON.parse(globalThis.cantripTest).inPerson?.fullScreen?.state === 'none'", null, Wait);
        Assert.False((await FullScreenAsync(launched, "phone")).GetProperty("shown").GetBoolean());

        // Launched again from the icon, its session gone: the code in its address is spent, and the ticket it kept is
        // what it brings.
        await launched.CloseAsync();
        await Until(() => !relay.Devices.Single(device => device.Seat == 1).Connected, "the app gone from the relay");
        IPage relaunched = await OpenPhoneAsync(app, address.AbsoluteUri);
        await Until(() =>
        {
            lock (presented) return presented.Contains("app-ticket");
        }, "the app back by its kept ticket");
        Assert.Equal("app-ticket", await relaunched.EvaluateAsync<string>("() => globalThis.cantripTicket"));
        await Until(() => relay.Devices.Single(device => device.Seat == 1).Connected, "the app at the relay again");
        Assert.Equal([1], issued);

        // The tab was told its place was taken, and has not opened its line again to take it back.
        await Task.Delay(TimeSpan.FromSeconds(4));
        lock (presented) Assert.Equal(1, presented.Count(each => each == "first-ticket"));
    }

    /// <summary>The remote client gains nothing of in-person play's full screen: no manifest, no meta tag, no viewport-fit, no button.</summary>
    [Fact]
    public async Task The_remote_client_has_no_manifest_no_home_screen_tags_and_no_full_screen_button()
    {
        await using IBrowserContext context = await web.Browser.NewContextAsync(new BrowserNewContextOptions
        {
            ViewportSize = new ViewportSize { Width = WebClient.Width, Height = WebClient.Height },
        });
        IPage page = await context.NewPageAsync();
        await page.GotoAsync($"{web.Site}?{NoTable}&test");
        await page.WaitForFunctionAsync("() => typeof globalThis.cantripFullScreenState === 'function' && typeof globalThis.cantripTest === 'string'", null, Wait);

        Assert.Equal(0, await page.EvaluateAsync<int>("() => document.querySelectorAll('link[rel=manifest], link[rel=apple-touch-icon]').length"));
        Assert.Equal(0, await page.EvaluateAsync<int>("() => document.querySelectorAll('meta[name=apple-mobile-web-app-capable], meta[name=mobile-web-app-capable], meta[name=apple-mobile-web-app-title]').length"));
        Assert.DoesNotContain("viewport-fit", await page.EvaluateAsync<string>("() => document.querySelector('meta[name=viewport]').content"), StringComparison.Ordinal);

        // The one apple tag there is Uno's own, in the page every client is served, as it always was.
        Assert.Equal(["apple-mobile-web-app-status-bar-style"], await page.EvaluateAsync<string[]>("() => [...document.querySelectorAll('meta')].map(m => m.name).filter(n => n.startsWith('apple-'))"));
        Assert.False((await page.EvaluateAsync<string>("() => globalThis.cantripTest")).Contains("fullScreen", StringComparison.Ordinal));
    }

    private static PageWaitForFunctionOptions Wait => new() { Timeout = (float)Patience.TotalMilliseconds };

    private static float X(JsonElement box) => box.GetProperty("x").GetSingle();

    private static float Y(JsonElement box) => box.GetProperty("y").GetSingle();

    /// <summary>What an in-person page adds to its head, for the Home Screen and the notch.</summary>
    private static async Task AssertInPersonHeadAsync(IPage page)
    {
        Assert.Equal("inperson.webmanifest", await page.EvaluateAsync<string>("() => document.querySelector('link[rel=manifest]').getAttribute('href')"));
        Assert.Equal("yes", await page.EvaluateAsync<string>("() => document.querySelector('meta[name=apple-mobile-web-app-capable]').content"));
        Assert.Equal("black-translucent", await page.EvaluateAsync<string>("() => document.querySelector('meta[name=apple-mobile-web-app-status-bar-style]').content"));
        Assert.Contains("viewport-fit=cover", await page.EvaluateAsync<string>("() => document.querySelector('meta[name=viewport]').content"), StringComparison.Ordinal);

        // The manifest is the hand-written one, served as a manifest, full screen, with its icons; it names no start_url,
        // so the page's own address is where the icon opens.
        IAPIResponse manifest = await page.APIRequest.GetAsync(new Uri(new Uri(page.Url), "inperson.webmanifest").AbsoluteUri);
        Assert.True(manifest.Ok);
        using JsonDocument json = JsonDocument.Parse(await manifest.TextAsync());
        Assert.Equal("fullscreen", json.RootElement.GetProperty("display").GetString());
        Assert.False(json.RootElement.TryGetProperty("start_url", out _));
        foreach (JsonElement icon in json.RootElement.GetProperty("icons").EnumerateArray())
        {
            Assert.True((await page.APIRequest.GetAsync(new Uri(new Uri(page.Url), icon.GetProperty("src").GetString()).AbsoluteUri)).Ok);
        }
    }

    /// <summary>The page's Fullscreen button as it last published it, once it is as asked.</summary>
    private static async Task<JsonElement> FullScreenAsync(IPage page, string role, Func<JsonElement, bool>? until = null)
    {
        DateTime giveUp = DateTime.UtcNow + Patience;
        JsonElement? last = null;
        while (DateTime.UtcNow < giveUp)
        {
            string? state = await page.EvaluateAsync<string?>("() => globalThis.cantripTest ?? null");
            if (state is not null)
            {
                using JsonDocument json = JsonDocument.Parse(state);
                JsonElement root = role == "phone" && json.RootElement.TryGetProperty("inPerson", out JsonElement inPerson) ? inPerson : json.RootElement;
                if (root.TryGetProperty("fullScreen", out JsonElement button))
                {
                    last = button.Clone();
                    if ((until?.Invoke(button) ?? true) && (until is null || !button.TryGetProperty("x", out _) || button.GetProperty("w").GetSingle() > 0))
                    {
                        return last.Value;
                    }
                }
            }

            await Task.Delay(100);
        }

        Assert.Fail($"The {role}'s Fullscreen button never got there: {last}");
        return default;
    }

    private async Task<IBrowserContext> PhoneContextAsync(bool iPhone, bool standalone)
    {
        IBrowserContext context = await web.Browser.NewContextAsync(new BrowserNewContextOptions
        {
            ViewportSize = new ViewportSize { Width = 390, Height = 844 },
            HasTouch = true,
            IsMobile = true,
            UserAgent = iPhone ? IPhoneAgent : null,
        });
        if (iPhone) await context.AddInitScriptAsync(NoFullscreenApi);
        if (standalone) await context.AddInitScriptAsync(Standalone);
        return context;
    }

    private static async Task<IPage> OpenPhoneAsync(IBrowserContext context, string address)
    {
        IPage page = await context.NewPageAsync();
        await page.GotoAsync(address);
        await page.WaitForFunctionAsync("() => typeof globalThis.cantripTest === 'string'", null, Wait);
        return page;
    }

    private static async Task Until(Func<bool> condition, string what)
    {
        DateTime giveUp = DateTime.UtcNow + Patience;
        while (!condition())
        {
            Assert.True(DateTime.UtcNow < giveUp, $"Never: {what}.");
            await Task.Delay(100);
        }
    }

    /// <summary>The built client and the relay's socket on one loopback site, as <c>cantrip-in-person</c> serves them.</summary>
    private static async Task<WebApplication> ServeAsync(InPersonRelay relay)
    {
        WebApplicationBuilder builder = WebApplication.CreateBuilder();
        builder.WebHost.UseUrls("http://127.0.0.1:0");
        builder.Logging.ClearProviders();
        WebApplication app = builder.Build();
        app.UseWebSockets();
        app.MapInPerson(relay);
        app.UseWebClient(Path.Combine(WebClient.Root, "dotnet", "src", "Cantrip.Client.Web", "bin", "Debug", "net10.0-browserwasm", "wwwroot"));
        await app.StartAsync();
        return app;
    }
}
