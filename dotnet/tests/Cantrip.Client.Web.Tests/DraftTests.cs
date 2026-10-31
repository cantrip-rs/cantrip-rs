using Cantrip.Protocol;
using Cantrip.Server;
using Cantrip.Server.Events;
using Cantrip.Server.Host;
using Xunit;

namespace Cantrip.Client.Web.Tests;

/// <summary>
/// A Winston draft played in the page against a bot, from the ticket that seats the player to the game it deals: the
/// draft, taking piles with the page's own buttons; deck building, moving a card to the sideboard and adding basic
/// lands; and submitting, after which the decks just built are dealt. Pictured at each step, under TestResults\web.
/// <para>
/// The event is made in a data directory of the test's own, as a host makes one, so what the page does is saved as it
/// would be for a player coming back to it.
/// </para>
/// </summary>
[Collection(WebClientCollection.Name)]
public sealed class DraftTests(WebClient web)
{
    [Fact]
    public async Task A_winston_draft_is_drafted_built_and_dealt_in_the_page()
    {
        string data = Path.Combine(Path.GetTempPath(), "cantrip-draft-test-" + Guid.NewGuid().ToString("N"));
        try
        {
            await Draft(data);
        }
        finally
        {
            Directory.Delete(data, recursive: true);
        }
    }

    [Fact]
    public async Task A_grid_draft_is_drafted_a_row_or_a_column_at_a_time_in_the_page()
    {
        string data = Path.Combine(Path.GetTempPath(), "cantrip-grid-test-" + Guid.NewGuid().ToString("N"));
        try
        {
            await Grid(data);
        }
        finally
        {
            Directory.Delete(data, recursive: true);
        }
    }

    /// <summary>
    /// The same grid draft on a player's phone at an in-person table (<c>?inperson=phone</c>, PhoneTests): the draft and deck
    /// building are the page's own, at a phone's size and tapped with a finger.
    /// </summary>
    [Fact]
    public async Task A_grid_draft_is_drafted_on_an_in_person_phone()
    {
        string data = Path.Combine(Path.GetTempPath(), "cantrip-grid-phone-test-" + Guid.NewGuid().ToString("N"));
        try
        {
            await Grid(data, phone: true);
        }
        finally
        {
            Directory.Delete(data, recursive: true);
        }
    }

    /// <summary>
    /// A grid draft of three grids against a bot: on each of this seat's picks a line with cards left in it is chosen
    /// by its button, lighting it, and taken with Take; the cards fly into the pool, and the draft ends in deck building.
    /// </summary>
    private async Task Grid(string data, bool phone = false)
    {
        var store = new EventStore(data);
        CreatedEvent created = DraftPod.CreateGrid(store, ContentCache.Get(WebClient.Content), new GridDraftSetup
        {
            Name = "Grid test",
            Cube = "rw-aggro",
            CubeCards = CubeList.Load(Path.Combine(WebClient.Root, "decks", "rw-aggro.txt")),
            Packs = 3,
            MinimumDeck = 5,
            Seed = 7,
            Players = [("you", "You"), ("bot", "Bot")],
        });

        DraftPod pod = DraftPod.Open(store, created.Record.Id, ContentCache.Get(WebClient.Content));
        await using GameServer server = await GameServer.StartAsync(new GameServerOptions
        {
            Format = Cantrip.Decks.DeckFormat.Limited(),
            Port = 0,
            Players = 2,
            ContentRoot = WebClient.Content,
            Event = pod,
        });

        var address = new Uri($"ws://localhost:{server.Port}/game");
        var bot = new GameClient(await WebSocketTransport.ConnectAsync(address));
        bot.Join("Bot", 0, created.Tickets["bot"]);
        var seat = new AutoSeat(bot) { DraftThink = TimeSpan.FromMilliseconds(600) };
        using var playing = new CancellationTokenSource();
        Task botPlays = Task.Run(async () =>
        {
            while (!playing.IsCancellationRequested)
            {
                if (!seat.Step())
                {
                    await Task.Delay(20);
                }
            }
        });

        await using TablePage page = await TablePage.OpenAsync(
            web,
            $"server={Uri.EscapeDataString(address.AbsoluteUri)}{(phone ? "&inperson=phone" : string.Empty)}#ticket={created.Tickets["you"]}",
            WebClient.Pictures(nameof(DraftTests) + (phone ? "-phone" : string.Empty)),
            phone: phone);

        TableState state = await page.SettledAsync(state => state.Phase == "Drafting");
        Assert.Equal($"Cantrip - Grid Draft ({created.Record.Id})", await page.Page.TitleAsync());
        int flownBefore = state.Flown;
        int picks = 0;
        int pickedFor = -1;
        bool botPictured = false;
        DateTime giveUp = DateTime.UtcNow + TimeSpan.FromMinutes(2);
        while (state.Phase == "Drafting" && DateTime.UtcNow < giveUp)
        {
            // The server's grid says whose pick it is and which lines have cards left; the page is clicked as a player would.
            if (pod.ViewFor(1) is DraftMessage { Active: 1 } draft && draft.Decision != pickedFor)
            {
                // The bot's pick just before this one is shown being made: its line lit on the grid it took it from.
                if (!botPictured && draft.Recent.LastOrDefault()?.StartsWith("Bot took", StringComparison.Ordinal) == true)
                {
                    await Task.Delay(400);
                    await page.PictureAsync("grid-bot-takes");
                    botPictured = true;
                }

                int line = Enumerable.Range(0, 6).First(line => GridLine(line).Any(place => draft.Pile[place].Known));
                string name = line < 3 ? $"Row {line + 1}" : $"Column {line - 2}";
                state = await page.SettledAsync();

                // Pressed until the page has the pick too: its line buttons do nothing while it is the other seat's.
                string taking = $"Take {name.ToLowerInvariant()}";
                for (int tries = 0; tries < 20 && !state.Controls.Any(control => control.Label.StartsWith(taking, StringComparison.Ordinal)); tries++)
                {
                    await page.ClickAsync(name, "button");
                    await Task.Delay(500);
                    state = await page.StateAsync() ?? state;
                }

                state = await page.SettledAsync(state => state.Controls.Any(control => control.Label.StartsWith(taking, StringComparison.Ordinal)));
                if (picks == 0)
                {
                    await page.PictureAsync("grid-line-chosen");
                    Assert.Equal(draft.Pile.Count(card => card.Known), state.Cards.Count(card => card.Visible && card.Row == "grid"));
                }

                // The cards of the line, where they are drawn: a column's under its button, a row's beside it.
                ControlState button = state.Controls.First(control => control.Kind == "button" && control.Label == name);
                List<CardState> lineCards = [.. state.Cards.Where(card => card.Visible && card.Row == "grid"
                    && (line < 3 ? MathF.Abs(card.Y - button.Y) < card.H / 2 : MathF.Abs(card.X - button.X) < card.W / 2))];
                int pooled = state.Cards.Count(card => card.Row is "pool" or "sideboard");

                ControlState take = state.Controls.First(control => control.Label.StartsWith("Take ", StringComparison.Ordinal));
                await page.TapAsync(take.X, take.Y);
                pickedFor = draft.Decision;
                picks++;

                // Each card taken flies into the pool from where that very card was in the grid, even with another copy of
                // it left there.
                state = await page.SettledAsync(state => state.Phase != "Drafting" || state.Cards.Count(card => card.Row is "pool" or "sideboard") > pooled);
                foreach (CardState taken in lineCards)
                {
                    Assert.Contains(state.Cards, card => card.Row is "pool" or "sideboard" && card.Name == taken.Name
                        && card.FlewFrom is { } from && MathF.Abs(from.X - taken.X) < 3 && MathF.Abs(from.Y - taken.Y) < 3);
                }
            }
            else
            {
                await Task.Delay(100);
            }

            state = await page.StateAsync() ?? state;

            // A card never moves along the grid: a copy of a card taken, or a card dealt again in the next grid, is put
            // in its place rather than flown there from where the other copy was.
            Assert.DoesNotContain(state.Cards, card => card.Row == "grid" && card.FlewFrom is not null);
        }

        Assert.Equal(3, picks);
        state = await page.SettledAsync(state => state.Phase == "DeckBuilding");
        Assert.Equal($"Cantrip - Deck Building ({created.Record.Id})", await page.Page.TitleAsync());
        Assert.True(state.Flown > flownBefore, "Nothing taken from the grid flew into the pool.");
        await page.PictureAsync("grid-deck-building");

        EventRecord record = store.LoadEvent(created.Record.Id)!;
        Assert.Equal(EventStage.DeckBuilding, record.Stage);
        GridDraft drafted = store.LoadDraft<GridDraft>(created.Record.Id)!;
        Assert.Equal(27, drafted.PoolOf(1).Count + drafted.PoolOf(2).Count + drafted.Discarded.Count);
        Assert.Equal(drafted.PoolOf(1), record.Participant("you")!.Pool);

        if (phone)
        {
            await BuildOnThePhoneAsync(page, store, created.Record.Id);
        }

        await playing.CancelAsync();
        await botPlays;
        bot.Disconnect();
        Assert.Empty(page.Errors);

        static IEnumerable<int> GridLine(int line) => line < 3
            ? Enumerable.Range(0, 3).Select(column => (line * 3) + column)
            : Enumerable.Range(0, 3).Select(row => (row * 3) + (line - 3));
    }

    /// <summary>
    /// The deck built on the phone (InPerson/PhoneDeckBuilder.cs), tapped with a finger at a phone's size: a card tapped out
    /// of the deck and another after it, the first tapped back from the sideboard, the list narrowed to a colour and grouped
    /// by type, a card shown whole, three Mountains added with the stepper, and Submit - each change the server's, as the
    /// event's record says.
    /// </summary>
    private static async Task BuildOnThePhoneAsync(TablePage page, EventStore store, string eventId)
    {
        TableState state = await page.SettledAsync(state => state.InPerson?.Deck is { Shown: true, Rows.Count: > 1 });
        PhoneDeckState deck = state.InPerson!.Deck!;
        await page.PictureAsync("phone-deck");
        Assert.Equal("Deck", deck.Tab);
        Assert.All(deck.Rows, row => Assert.True(row.H >= 44, $"{row.Name}'s row is {row.H} px tall, too small to tap."));
        Assert.All(deck.Rows, row => Assert.True(row.X - (row.W / 2) >= 0 && row.X + (row.W / 2) <= TablePage.PhoneViewport.Width, $"{row.Name}'s row runs off the phone."));

        // Out of the deck with a tap, and another; the first back from the sideboard.
        PhoneDeckRow first = deck.Rows[0];
        PhoneDeckRow second = deck.Rows[1];
        await page.TapAsync(first.X, first.Y);
        state = await page.SettledAsync(state => state.InPerson!.Deck!.Rows.Any(row => row.Place == second.Place) && !state.InPerson.Deck.Rows.Any(row => row.Place == first.Place));
        PhoneDeckRow again = state.InPerson!.Deck!.Rows.First(row => row.Place == second.Place);
        await page.TapAsync(again.X, again.Y);
        await page.SettledAsync(state => state.InPerson!.Deck!.Rows.Count > 0 && !state.InPerson.Deck.Rows.Any(row => row.Place == second.Place));
        await TapDeckButtonAsync(page, "Tab Sideboard");
        state = await page.SettledAsync(state => state.InPerson!.Deck!.Tab == "Sideboard" && state.InPerson.Deck.Rows.Any(row => row.Place == first.Place));
        await page.PictureAsync("phone-deck-sideboard");
        PhoneDeckRow back = state.InPerson!.Deck!.Rows.Single(row => row.Place == first.Place);
        Assert.False(back.InDeck);
        await page.TapAsync(back.X, back.Y);
        await page.SettledAsync(state => state.InPerson!.Deck!.Rows.All(row => row.Place != first.Place));
        await ServerHasAsync(store, eventId, deck => deck.Main.Contains(first.Place) && !deck.Main.Contains(second.Place), "the deck without the second card and with the first back");

        // Narrowed to a colour, grouped by type, a card shown whole.
        await TapDeckButtonAsync(page, "Tab Deck");
        await TapDeckButtonAsync(page, "Colour red");
        await page.SettledAsync();
        await page.PictureAsync("phone-deck-red");
        await TapDeckButtonAsync(page, "Colour red");
        await TapDeckButtonAsync(page, "By type");
        state = await page.SettledAsync();
        await page.PictureAsync("phone-deck-by-type");
        PhoneDeckButton show = state.InPerson!.Deck!.Buttons.First(button => button.Label.StartsWith("Show ", StringComparison.Ordinal));
        await page.TapAsync(show.X, show.Y);
        await page.SettledAsync(state => state.InPerson!.Deck!.Preview);
        await page.PictureAsync("phone-deck-card-shown");
        await page.TapAsync(TablePage.PhoneViewport.Width / 2f, 40);
        await page.SettledAsync(state => !state.InPerson!.Deck!.Preview);

        // Three Mountains with the stepper, then Submit.
        await TapDeckButtonAsync(page, "Tab Lands");
        for (int i = 0; i < 3; i++)
        {
            await TapDeckButtonAsync(page, "Add Mountain");
            int added = i + 1;
            await page.SettledAsync(state => state.InPerson!.Deck!.Basics.Any(land => land.Name == "Mountain" && land.Count == added));
        }

        await page.PictureAsync("phone-deck-lands");
        await ServerHasAsync(store, eventId, deck => deck.Basics.GetValueOrDefault("Mountain") == 3, "three Mountains");
        state = await page.SettledAsync();
        Assert.True(state.InPerson!.Deck!.Buttons.Single(button => button.Label == "Submit deck").Enabled, "Submit is not offered for a deck big enough.");
        await TapDeckButtonAsync(page, "Submit deck");
        await ServerHasAsync(store, eventId, deck => deck.Submitted, "the deck submitted");
        await page.PictureAsync("phone-deck-submitted");
    }

    private static async Task TapDeckButtonAsync(TablePage page, string label)
    {
        TableState state = await page.SettledAsync(state => state.InPerson?.Deck?.Buttons.Any(button => button.Label == label) == true);
        PhoneDeckButton button = state.InPerson!.Deck!.Buttons.First(each => each.Label == label);
        await page.TapAsync(button.X, button.Y);
    }

    private static async Task ServerHasAsync(EventStore store, string eventId, Func<EventDeck, bool> holds, string what)
    {
        DateTime giveUp = DateTime.UtcNow + TimeSpan.FromSeconds(20);
        while (!holds(store.LoadEvent(eventId)!.Participant("you")!.Deck))
        {
            Assert.True(DateTime.UtcNow < giveUp, $"The server never had {what}.");
            await Task.Delay(100);
        }
    }

    private async Task Draft(string data)
    {
        var store = new EventStore(data);
        CreatedEvent created = DraftPod.CreateWinston(store, ContentCache.Get(WebClient.Content), new WinstonDraftSetup
        {
            Name = "Test draft",
            Cube = "rw-aggro",
            CubeCards = CubeList.Load(Path.Combine(WebClient.Root, "decks", "rw-aggro.txt")),
            Cards = 24,
            MinimumDeck = 20,
            Seed = 7,
            Players = [("you", "You"), ("bot", "Bot")],
        });

        DraftPod pod = DraftPod.Open(store, created.Record.Id, ContentCache.Get(WebClient.Content));
        await using GameServer server = await GameServer.StartAsync(new GameServerOptions
        {
            Format = Cantrip.Decks.DeckFormat.Limited(),
            Port = 0,
            Players = 2,
            ContentRoot = WebClient.Content,
            Event = pod,
        });

        var address = new Uri($"ws://localhost:{server.Port}/game");
        var bot = new GameClient(await WebSocketTransport.ConnectAsync(address));
        bot.Join("Bot", 0, created.Tickets["bot"]);
        var seat = new AutoSeat(bot);

        // The bot drafts, builds and plays for as long as the test runs.
        using var playing = new CancellationTokenSource();
        Task botPlays = Task.Run(async () =>
        {
            while (!playing.IsCancellationRequested)
            {
                if (!seat.Step())
                {
                    await Task.Delay(20);
                }
            }
        });

        await using TablePage page = await TablePage.OpenAsync(
            web, $"server={Uri.EscapeDataString(address.AbsoluteUri)}#ticket={created.Tickets["you"]}", WebClient.Pictures(nameof(DraftTests)));

        // The draft: take whatever pile is offered on this seat's turn until every card is gone. What is taken flies
        // from the pile into the pool's columns.
        TableState state = await page.SettledAsync(state => state.Phase == "Drafting");
        Assert.Equal($"Cantrip - Winston Draft ({created.Record.Id})", await page.Page.TitleAsync());
        int flownBefore = state.Flown;
        bool pictured = false;
        DateTime giveUp = DateTime.UtcNow + TimeSpan.FromMinutes(3);
        while (state.Phase == "Drafting" && DateTime.UtcNow < giveUp)
        {
            if (state.Controls.FirstOrDefault(control => control.Label.StartsWith("Take pile", StringComparison.Ordinal)) is { } take)
            {
                if (!pictured)
                {
                    await page.PictureAsync("draft-my-pick");
                    Assert.Contains(state.Controls, control => control.Label == "Pass" || control.Label.StartsWith("Pass and", StringComparison.Ordinal));
                    Assert.Contains(state.Controls, control => control.Label == "Show sideboard");
                    pictured = true;
                }

                await page.Page.Mouse.ClickAsync(take.X, take.Y);
                await Task.Delay(300);
            }
            else
            {
                await Task.Delay(100);
            }

            state = await page.StateAsync() ?? state;
        }

        Assert.True(pictured, "The page never offered this seat a pile to take.");
        Assert.True(state.Flown > flownBefore, "Nothing taken in the draft flew into the pool.");

        // Deck building: everything drafted starts in the main deck, in columns by mana value.
        state = await page.SettledAsync(state => state.Phase == "DeckBuilding");
        await page.PictureAsync("deck-building");
        int drafted = store.LoadEvent(created.Record.Id)!.Participant("you")!.Pool.Count;

        // A card dragged onto the sideboard while it is tucked away goes into it.
        CardState dragged = state.Cards.First(card => card.Visible && card.Name.Length > 0);
        ControlState tucked = state.Controls.Single(control => control.Label == "Show sideboard");
        await page.Page.Mouse.MoveAsync(dragged.X, NameStrip(dragged));
        await page.Page.Mouse.DownAsync();
        await page.Page.Mouse.MoveAsync(dragged.X + 20, NameStrip(dragged) + 10, new() { Steps = 4 });
        await page.Page.Mouse.MoveAsync(tucked.X, tucked.Y + 120, new() { Steps = 8 });
        await page.PictureAsync("dragging-to-the-sideboard");

        // Carried, it is gone from its column rather than faded there; let go, it flies from where it was dropped.
        TableState carrying = (await page.StateAsync())!;
        Assert.DoesNotContain(carrying.Cards, card => card.Visible && card.Name == dragged.Name && MathF.Abs(card.X - dragged.X) < 1 && MathF.Abs(card.Y - dragged.Y) < 1);
        int flownBeforeDrop = carrying.Flown;
        await page.Page.Mouse.UpAsync();
        await MainDeckOf(drafted - 1);
        Assert.True((await page.StateAsync())!.Flown > flownBeforeDrop, "The card dropped on the sideboard did not fly there.");

        // Pulled out, the sideboard shows it, and a double-click puts it back in the main deck.
        await page.ClickAsync("Show sideboard", "button");
        state = await page.SettledAsync(state => state.Controls.Any(control => control.Label == "Hide sideboard"));
        await page.PictureAsync("sideboard-open");
        CardState inSide = state.Cards.Where(card => card.Visible && card.Name == dragged.Name).OrderBy(card => card.X).First();
        await page.Page.Mouse.DblClickAsync(inSide.X, NameStrip(inSide));
        await MainDeckOf(drafted);

        // Along a column, a card goes in by its top against the tops of the others, as it is seen: carried down until its
        // top is just below the next card's, it goes after that card.
        state = await page.SettledAsync();
        List<CardState> column = [.. state.Cards
            .Where(card => card.Visible && card.Name.Length > 0)
            .GroupBy(card => card.X)
            .Select(group => group.OrderBy(card => card.Y).ToList())
            .First(cards => cards.Count >= 2 && cards[0].Name != cards[1].Name)];
        (CardState upper, CardState lower) = (column[0], column[1]);
        float upperTop = upper.Y - (upper.H / 2);
        float lowerTop = lower.Y - (lower.H / 2);
        await page.Page.Mouse.MoveAsync(upper.X, upperTop + 8);
        await page.Page.Mouse.DownAsync();
        await page.Page.Mouse.MoveAsync(upper.X + 6, upperTop + 14, new() { Steps = 3 });
        await page.Page.Mouse.MoveAsync(upper.X, lowerTop + 8 + 4, new() { Steps = 6 });
        await page.Page.Mouse.UpAsync();
        state = await page.SettledAsync();
        CardState upperNow = state.Cards.First(card => card.Visible && card.Name == upper.Name && MathF.Abs(card.X - upper.X) < 2);
        CardState lowerNow = state.Cards.First(card => card.Visible && card.Name == lower.Name && MathF.Abs(card.X - lower.X) < 2);
        Assert.True(upperNow.Y > lowerNow.Y, $"{upper.Name}, carried until its top was below {lower.Name}'s, did not go after it.");

        // Dragged to another column, it flies there from where it was let go and stays there.
        state = await page.SettledAsync();
        CardState leftmost = state.Cards.Where(card => card.Visible && card.Name.Length > 0).OrderBy(card => card.X).ThenBy(card => card.Y).Skip(1).First();
        CardState moving = state.Cards.Where(card => card.Visible && card.Name.Length > 0 && card.X > leftmost.X + 1).OrderBy(card => card.Y).First();
        float lastColumn = leftmost.X + (7 * (leftmost.W + 10)); // the 6 column, in view with the sideboard open
        int flownBeforeColumn = state.Flown;
        await page.Page.Mouse.MoveAsync(moving.X, NameStrip(moving));
        await page.Page.Mouse.DownAsync();
        await page.Page.Mouse.MoveAsync(moving.X + 20, NameStrip(moving) + 10, new() { Steps = 4 });
        await page.Page.Mouse.MoveAsync(lastColumn, NameStrip(leftmost) + 20, new() { Steps = 8 });
        await page.Page.Mouse.UpAsync();
        state = await page.SettledAsync();
        Assert.True(state.Flown > flownBeforeColumn, "The card dragged to another column did not fly there.");
        Assert.Contains(state.Cards, card => card.Visible && card.Name == moving.Name && MathF.Abs(card.X - lastColumn) < 2);
        await page.PictureAsync("moved-to-another-column");

        // And a double-click on a card of the main deck sends it to the sideboard.
        state = await page.SettledAsync();
        CardState toSide = state.Cards.Where(card => card.Visible && card.Name.Length > 0).OrderByDescending(card => card.X).First();
        await page.Page.Mouse.DblClickAsync(toSide.X, NameStrip(toSide));
        await MainDeckOf(drafted - 1);
        await page.SettledAsync(state => state.Controls.Any(control => control.Label.StartsWith("Submit deck", StringComparison.Ordinal)));

        // Basic lands up to the minimum, and one more.
        int lands = 20 - (drafted - 1) + 1;
        for (int i = 0; i < lands; i++)
        {
            await page.ClickAsync("Add Mountain", "button");
        }

        state = await page.SettledAsync(state => state.Controls.Any(control => control.Label == $"Submit deck ({drafted - 1 + lands})"));

        // A basic land added is a card like the others: dragged to another column it stays there, and a double-click
        // there takes that one out of the deck. An extra one is added for it.
        await page.ClickAsync("Add Mountain", "button");
        state = await page.SettledAsync(state => state.Controls.Any(control => control.Label == $"Submit deck ({drafted + lands})"));
        CardState mountain = state.Cards.Where(card => card.Visible && card.Name == "Mountain").OrderByDescending(card => card.Y).First();
        float otherColumn = mountain.X + (3 * (mountain.W + 10));
        await page.Page.Mouse.MoveAsync(mountain.X, NameStrip(mountain));
        await page.Page.Mouse.DownAsync();
        await page.Page.Mouse.MoveAsync(mountain.X + 20, NameStrip(mountain) + 10, new() { Steps = 4 });
        await page.Page.Mouse.MoveAsync(otherColumn, NameStrip(mountain) + 10, new() { Steps = 8 });
        await page.Page.Mouse.UpAsync();
        state = await page.SettledAsync();
        CardState moved = Assert.Single(state.Cards, card => card.Visible && card.Name == "Mountain" && MathF.Abs(card.X - otherColumn) < 2);
        await page.PictureAsync("basic-moved");
        await page.Page.Mouse.DblClickAsync(moved.X, NameStrip(moved));
        state = await page.SettledAsync(state => state.Controls.Any(control => control.Label == $"Submit deck ({drafted - 1 + lands})"));
        Assert.DoesNotContain(state.Cards, card => card.Visible && card.Name == "Mountain" && MathF.Abs(card.X - otherColumn) < 2);
        await page.PictureAsync("deck-built");

        // Cards picked together go as one, as in the deck editor: Ctrl-click two drafted cards by their names, then drag
        // one of them onto the sideboard - both go, and stay picked; dragged back by the other, both come back.
        state = await page.SettledAsync();
        List<CardState> feet = [.. state.Cards.Where(card => card.Visible && card.Row == "pool" && card.Name.Length > 0 && card.Name != "Mountain")
            .OrderBy(card => card.X).ThenBy(card => card.Y).Take(2)];
        Assert.Equal(2, feet.Count);
        await page.Page.Keyboard.DownAsync("Control");
        foreach (CardState foot in feet)
        {
            await page.Page.Mouse.ClickAsync(foot.X, NameStrip(foot));
        }

        await page.Page.Keyboard.UpAsync("Control");
        state = await page.SettledAsync(state => state.Cards.Count(card => card.Picked) == 2);
        Assert.Equal(feet.Select(card => card.Handle).Order(), state.Cards.Where(card => card.Picked).Select(card => card.Handle).Order());
        await page.PictureAsync("picked-together");
        CardState sided = state.Cards.First(card => card.Visible && card.Row == "sideboard");
        await DragByNameAsync(feet[0], sided.X, sided.Y + sided.H);
        await MainDeckOf(drafted - 3);
        state = await page.SettledAsync(state => state.Cards.Count(card => card.Row == "sideboard" && card.Picked) == 2);
        await page.PictureAsync("sided-together");
        CardState back = state.Cards.First(card => card.Row == "sideboard" && card.Picked && card.Handle == feet[1].Handle);
        await DragByNameAsync(back, feet[0].X, feet[0].Y + feet[0].H);
        await MainDeckOf(drafted - 1);
        state = await page.SettledAsync(state => state.Cards.Count(card => card.Row == "pool" && card.Picked) == 2);

        // A click on no card lets them go.
        await page.Page.Mouse.ClickAsync(feet[0].X, feet[0].Y + (feet[0].H * 3));
        await page.SettledAsync(state => !state.Cards.Any(card => card.Picked));

        // Saved as it was built, so a reload or another computer finds it as it is.
        EventDeck saved = store.LoadEvent(created.Record.Id)!.Participant("you")!.Deck;
        Assert.Equal(drafted - 1, saved.Main.Count);
        Assert.Equal(lands, saved.Basics["Mountain"]);
        Assert.False(saved.Submitted);

        await page.ClickAsync($"Submit deck ({drafted - 1 + lands})", "button");

        // Both decks in, the game is dealt from them: the table comes up and asks its first question.
        state = await page.SettledAsync(state => state.Phase == "Playing" && state.Prompt.Length > 0);
        Assert.Equal($"Cantrip - Game vs Bot ({created.Record.Id})", await page.Page.TitleAsync());
        await page.PictureAsync("dealt");

        EventRecord record = store.LoadEvent(created.Record.Id)!;
        Assert.Equal(EventStage.Playing, record.Stage);
        Assert.Equal(1, record.Round);
        Assert.All(record.Participants, participant => Assert.True(participant.Deck.Submitted));

        await playing.CancelAsync();
        await botPlays;
        bot.Disconnect();
        Assert.Empty(page.Errors);

        // A card in a column is covered by the ones below it but for its name, which is where it is taken hold of.
        static float NameStrip(CardState card) => card.Y - (card.H / 2) + 8;

        async Task DragByNameAsync(CardState card, float x, float y)
        {
            await page.Page.Mouse.MoveAsync(card.X, NameStrip(card));
            await page.Page.Mouse.DownAsync();
            await page.Page.Mouse.MoveAsync(card.X + 20, NameStrip(card) + 10, new() { Steps = 4 });
            await page.Page.Mouse.MoveAsync(x, y, new() { Steps = 12 });
            await page.Page.Mouse.UpAsync();
        }

        // Waits for the deck the server keeps to have this many cards of the pool in its main deck.
        async Task MainDeckOf(int count)
        {
            DateTime until = DateTime.UtcNow + TimeSpan.FromSeconds(20);
            int now = -1;
            while (DateTime.UtcNow < until)
            {
                now = store.LoadEvent(created.Record.Id)!.Participant("you")!.Deck.Main.Count;
                if (now == count)
                {
                    return;
                }

                await Task.Delay(100);
            }

            await page.PictureAsync("main-deck-not-" + count);
            Assert.Fail($"The main deck kept {now} of the pool, not {count}.");
        }
    }
}
