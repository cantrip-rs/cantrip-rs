using Cantrip.Protocol;
using Cantrip.Server;
using Cantrip.Server.Events;
using Cantrip.Server.InPerson;
using Xunit;

namespace Cantrip.Client.Web.Tests;

/// <summary>
/// In-person play's grid draft (docs/in-person.md, "The grid draft"), drafted by touch on the tablets: a real in-person
/// server (<see cref="InPersonHost"/>) hosting a short grid draft, both players' phones at their seats showing "Pick on the
/// table" with their pools, and the tablets - one shared board, or one for each player - where every row and column is
/// picked with a finger: tapped once to choose it, and again (or Take) to take it. Each pick goes through the relay to the
/// picker's phone, which sends it to the table as its own, and the table's record of the draft is checked after each.
/// Then deck building, on the phones.
/// </summary>
[Collection(WebClientCollection.Name)]
[Trait("Category", "WallClock")]
public sealed class InPersonDraftTests(WebClient web)
{
    private const int Grids = 3;

    /// <summary>The shared tablet: seat 2's face of the draft turned at the top, seat 1's upright at the bottom.</summary>
    [Fact]
    public async Task A_grid_draft_is_drafted_by_touch_on_the_shared_tablet_and_the_deck_built_on_a_phone()
    {
        await DraftAsync(layout: 3, "shared");
    }

    /// <summary>A tablet for each player, each showing the grid, live only on that player's pick.</summary>
    [Fact]
    public async Task A_grid_draft_is_drafted_by_touch_on_each_players_own_tablet()
    {
        await DraftAsync(layout: 4, "own-tablets");
    }

    private async Task DraftAsync(int layout, string name)
    {
        string data = Path.Combine(Path.GetTempPath(), "cantrip-in-person-draft-" + Guid.NewGuid().ToString("N"));
        string pictures = WebClient.Pictures(Path.Combine(nameof(InPersonDraftTests), name));
        var store = new EventStore(data);
        CreatedEvent created = DraftPod.CreateGrid(store, ContentCache.Get(WebClient.Content), new GridDraftSetup
        {
            Name = "In person: rw-aggro",
            Cube = "rw-aggro",
            CubeCards = CubeList.Load(Path.Combine(WebClient.Root, "decks", "rw-aggro.txt")),
            Packs = Grids,
            MinimumDeck = 5,
            Seed = 7,
            Players = [("you", "Ann"), ("bo", "Bo")],
        });

        // Laid out for the devices asked, as the setup page keeps it with the event.
        DraftPod.Open(store, created.Record.Id, ContentCache.Get(WebClient.Content)).Keep("inperson.layout", layout.ToString(System.Globalization.CultureInfo.InvariantCulture));

        try
        {
            await using InPersonHost host = await InPersonHost.StartAsync(new InPersonOptions
            {
                ContentRoot = WebClient.Content,
                Store = store,
                ClientRoot = Path.Combine(WebClient.Root, "dotnet", "src", "Cantrip.Client.Web", "bin", "Debug", "net10.0-browserwasm", "wwwroot"),
                Port = 0,
                TablePort = 0,
                ListenAddress = "127.0.0.1",
                Address = "127.0.0.1",
                ArtCache = WebClient.ArtCache,
            });
            await host.HostEventAsync(created.Record.Id, "127.0.0.1");

            // The devices' links, phones then tablets, each opened as its QR code would open it.
            IReadOnlyList<string> links = host.DeviceLinks;
            Assert.Equal(layout == 4 ? 4 : 3, links.Count);
            List<TabletDriver> tablets = [];
            List<TablePage> phones = [];
            try
            {
                for (int i = 2; i < links.Count; i++)
                {
                    (Uri site, string query) = Split(links[i]);
                    query = query.Replace("inperson=tablet&", string.Empty, StringComparison.Ordinal);
                    TabletDriver tablet = await TabletDriver.OpenAsync(web, query, pictures, layout == 4 ? $"side-{i - 1}" : "shared", site);
                    tablets.Add(tablet);
                    await tablet.SettledAsync(state => state.Screen == "phase");
                }

                for (int seat = 1; seat <= 2; seat++)
                {
                    (Uri site, string query) = Split(links[seat - 1]);
                    string own = WebClient.Pictures(Path.Combine(nameof(InPersonDraftTests), name, $"phone-{seat}"));
                    phones.Add(await TablePage.OpenAsync(web.Browser, site, query, own, phone: true));
                }

                await PlayDraftAsync(store, created.Record.Id, host, tablets, phones);

                // Deck building, on the phones; the tablets say where the event stands.
                foreach (TabletDriver tablet in tablets)
                {
                    TabletState after = await tablet.SettledAsync(state => state.Screen == "phase");
                    Assert.False(after.Grid?.Shown ?? false);
                    await tablet.PictureAsync("deck-building");
                }

                await phones[1].SettledAsync(state => state.InPerson?.Deck is { Shown: true } && state.InPerson.Draft is { Shown: false });
                if (layout == 3)
                {
                    await DraftTests.BuildOnThePhoneAsync(phones[0], store, created.Record.Id, "you");
                }
                else
                {
                    TableState building = await phones[0].SettledAsync(state => state.InPerson?.Deck is { Shown: true, Rows.Count: > 0 });
                    await phones[0].PictureAsync("deck-building");
                    Assert.False(building.InPerson!.Draft!.Shown);
                }

                foreach (TablePage phone in phones)
                {
                    Assert.Empty(phone.Errors);
                }

                foreach (TabletDriver tablet in tablets)
                {
                    Assert.Empty(tablet.Errors);
                }
            }
            finally
            {
                foreach (TabletDriver tablet in tablets)
                {
                    await tablet.DisposeAsync();
                }

                foreach (TablePage phone in phones)
                {
                    await phone.DisposeAsync();
                }
            }
        }
        finally
        {
            Directory.Delete(data, recursive: true);
        }
    }

    /// <summary>
    /// Every pick of the draft made on a tablet: the face in front of the picker has its lines live, the other face's (or the
    /// other player's tablet) none; a tap on a dead line takes nothing. The first line with cards is tapped to choose it -
    /// lit on the tablet, and chosen through the picker's phone - then taken, with a second tap on it on even picks and
    /// with Take on odd ones. The table's draft must move on to the next pick, with the cards in the picker's pool.
    /// </summary>
    private static async Task PlayDraftAsync(EventStore store, string eventId, InPersonHost host, List<TabletDriver> tablets, List<TablePage> phones)
    {
        TabletDriver first = tablets[0];
        TabletState state = await first.SettledAsync(state => state.Screen == "grid" && state.Grid is { Shown: true });
        await first.PictureAsync("grid");
        foreach (TablePage phone in phones)
        {
            TableState shown = await phone.SettledAsync(state => state.InPerson?.Draft is { Shown: true });
            Assert.Equal("Pick on the table", shown.InPerson!.Draft!.Title);
        }

        int picks = 0;
        DateTime giveUp = DateTime.UtcNow + TimeSpan.FromMinutes(4);
        while (DateTime.UtcNow < giveUp)
        {
            GridDraft draft = Drafted(store, eventId);
            if (draft.IsOver)
            {
                break;
            }

            int active = draft.Active;
            int decision = draft.Decision;
            int poolBefore = draft.PoolOf(active).Count;

            // The tablet the picker picks on, and the face of it in front of them.
            (TabletDriver tablet, TabletState shown) = await PickerTabletAsync(tablets, active, decision);
            TabletFace face = shown.Grid!.Faces.Single(each => each.Seat == active);
            Assert.All(shown.Grid.Faces.Where(each => each.Seat != active).SelectMany(each => each.Buttons), button => Assert.False(button.Enabled, $"{button.Label} is live on the face of the player not picking."));
            foreach (TabletDriver other in tablets.Where(each => each != tablet))
            {
                TabletState theirs = await other.SettledAsync(state => state.Grid?.Decision == decision);
                Assert.All(theirs.Grid!.Faces.SelectMany(each => each.Buttons), button => Assert.False(button.Enabled, $"{button.Label} is live on the other player's tablet."));
            }

            // The other player's phone shows the pick is not theirs, the picker's that it is.
            TableState pickerPhone = await phones[active - 1].SettledAsync(state => state.InPerson?.Draft is { Shown: true, Mine: true });
            Assert.Equal(poolBefore, pickerPhone.InPerson!.Draft!.Pool);
            await phones[2 - active].SettledAsync(state => state.InPerson?.Draft is { Shown: true, Mine: false });

            // A tap on the dead face of a shared tablet, which must take nothing.
            if (shown.Grid.Faces.FirstOrDefault(each => each.Seat != active) is { } dead
                && dead.Buttons.FirstOrDefault(button => button.Line >= 0) is { } deadLine)
            {
                await tablet.TapAsync(deadLine.X, deadLine.Y);
            }

            TabletGridButton line = face.Buttons.First(button => button.Line >= 0 && button.Enabled);
            await tablet.TapAsync(line.X, line.Y);
            TabletState chosen = await tablet.SettledAsync(state => state.Grid!.Faces.Single(each => each.Seat == active).Buttons.Any(button => button.Line == line.Line && button.Chosen));
            Assert.Equal(decision, Drafted(store, eventId).Decision);
            if (picks == 0)
            {
                await tablet.PictureAsync("line-chosen");
            }

            await phones[active - 1].SettledAsync(state => state.InPerson!.Lines?.Any(each => each.Decision == decision && each.Line == line.Line && !each.Take) == true);

            TabletGridButton take = picks % 2 == 0
                ? chosen.Grid!.Faces.Single(each => each.Seat == active).Buttons.Single(button => button.Line == line.Line)
                : chosen.Grid!.Faces.Single(each => each.Seat == active).Buttons.Single(button => button.Line < 0);
            Assert.True(take.Enabled, $"{take.Label} is not live once {line.Label} is chosen.");
            await tablet.TapAsync(take.X, take.Y);
            if (picks == 0)
            {
                await tablet.PictureAsync("line-taken");
            }

            await DraftMovesOnAsync(store, eventId, decision);
            GridDraft after = Drafted(store, eventId);
            Assert.Equal(active, after.History[^1].Seat);
            Assert.Equal(line.Line, after.History[^1].Line);
            Assert.False(after.History[^1].Auto);
            Assert.True(after.PoolOf(active).Count > poolBefore);
            picks++;

            if (picks == 2)
            {
                foreach (TablePage phone in phones)
                {
                    await phone.SettledAsync(state => state.InPerson?.Draft is { Shown: true, Pool: > 0 });
                    await phone.PictureAsync("pick-on-the-table");
                }
            }
        }

        Assert.Equal(Grids * 2, picks);
        EventRecord record = store.LoadEvent(eventId)!;
        Assert.Equal(EventStage.DeckBuilding, record.Stage);
        GridDraft drafted = Drafted(store, eventId);
        Assert.Equal(drafted.PoolOf(1), record.Participant("you")!.Pool);
        Assert.NotNull(host.Relay);
    }

    /// <summary>The tablet the player whose pick it is picks on, showing this pick, its lines live on their face.</summary>
    private static async Task<(TabletDriver Tablet, TabletState State)> PickerTabletAsync(List<TabletDriver> tablets, int active, int decision)
    {
        DateTime giveUp = DateTime.UtcNow + TimeSpan.FromSeconds(60);
        while (DateTime.UtcNow < giveUp)
        {
            foreach (TabletDriver tablet in tablets)
            {
                TabletState state = await tablet.SettledAsync(state => state.Grid is { Shown: true } grid && grid.Decision == decision);
                if (state.Grid!.Faces.FirstOrDefault(face => face.Seat == active) is { } face && face.Buttons.Any(button => button.Line >= 0 && button.Enabled))
                {
                    return (tablet, state);
                }
            }

            await Task.Delay(100);
        }

        throw new Xunit.Sdk.XunitException($"No tablet offers seat {active} pick {decision}.");
    }

    private static async Task DraftMovesOnAsync(EventStore store, string eventId, int decision)
    {
        DateTime giveUp = DateTime.UtcNow + TimeSpan.FromSeconds(20);
        while (Drafted(store, eventId).Decision == decision)
        {
            Assert.True(DateTime.UtcNow < giveUp, $"Pick {decision} was never taken at the table.");
            await Task.Delay(100);
        }
    }

    /// <summary>The draft as the table last saved it, read again should it be read while it is being written.</summary>
    private static GridDraft Drafted(EventStore store, string eventId)
    {
        for (int attempt = 0; ; attempt++)
        {
            try
            {
                return store.LoadDraft<GridDraft>(eventId) ?? throw new IOException("No draft yet.");
            }
            catch (Exception error) when (attempt < 20 && error is IOException or System.Text.Json.JsonException)
            {
                Thread.Sleep(50);
            }
        }
    }

    /// <summary>A device's link as its page and the query after it (with any fragment), as the page helpers open them.</summary>
    private static (Uri Site, string Query) Split(string link)
    {
        var uri = new Uri(link);
        return (new Uri(uri.GetLeftPart(UriPartial.Path)), uri.Query.TrimStart('?') + uri.Fragment);
    }
}
