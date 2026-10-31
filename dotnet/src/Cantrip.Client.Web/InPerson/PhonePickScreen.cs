using System.Text.Json;
using Cantrip.Protocol;
using Microsoft.UI.Xaml.Automation;
using Windows.Foundation;

namespace Cantrip.Client.Web.InPerson;

/// <summary>
/// A player's phone at an in-person table while the grid draft is drafted (docs/in-person.md, "The grid draft"): the grid
/// lies on the tablets, where its rows and columns are tapped, so the phone says only that - "Pick on the table" - whose
/// pick it is, and shows the pool drafted so far, its cards as readable rows grouped by mana value as the deck builder
/// groups them (<see cref="PhoneDeckBuilder"/>). A tap on a row shows the card whole. Shown over the page during a grid
/// draft, on a phone's page alone (<see cref="MainPage"/> attaches it only there); deck building then follows.
/// </summary>
public sealed class PhonePickScreen : Grid
{
    private const double RowHeight = 56;

    private readonly TextBlock _title = PhoneDeckBuilder.Text("Pick on the table", 22, Theme.Fg, semibold: true);
    private readonly TextBlock _grid = PhoneDeckBuilder.Text(string.Empty, 14, Theme.FgSoft);
    private readonly TextBlock _status = PhoneDeckBuilder.Text(string.Empty, 15, Theme.Accent, semibold: true, wrap: true);
    private readonly TextBlock _counts = PhoneDeckBuilder.Text(string.Empty, 13, Theme.FgSoft, wrap: true);
    private readonly TextBlock _poolTitle = PhoneDeckBuilder.Text(string.Empty, 15, Theme.Fg, semibold: true);
    private readonly StackPanel _list = new() { Spacing = 2 };
    private readonly ScrollViewer _scroll;
    private readonly Grid _preview = new() { Visibility = Visibility.Collapsed };
    private readonly CardBoardElement _previewCard = new(300, BoardRows.CardHeight(300)) { HorizontalAlignment = HorizontalAlignment.Center, VerticalAlignment = VerticalAlignment.Center };

    /// <summary>The rows shown, each with its card's name and the group it is under.</summary>
    private readonly List<(FrameworkElement Row, string Name, string Group)> _rows = [];

    private DraftMessage? _seen;
    private bool _mine;

    public PhonePickScreen()
    {
        Background = Theme.Brush(Theme.Window);
        Visibility = Visibility.Collapsed;
        RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
        RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
        RowDefinitions.Add(new RowDefinition { Height = new GridLength(1, GridUnitType.Star) });

        var head = new StackPanel
        {
            Spacing = 6,
            Padding = new Thickness(16, 16, 16, 14),
            Background = Theme.Brush(Theme.Bar),
            Children = { _title, _grid, _status, _counts },
        };
        AutomationProperties.SetName(_title, "Pick on the table");
        Children.Add(head);

        _poolTitle.Margin = new Thickness(16, 12, 16, 4);
        Grid.SetRow(_poolTitle, 1);
        Children.Add(_poolTitle);

        _scroll = new ScrollViewer
        {
            VerticalScrollBarVisibility = ScrollBarVisibility.Auto,
            HorizontalScrollBarVisibility = ScrollBarVisibility.Disabled,
            Content = new Border { Padding = new Thickness(12, 0, 12, 24), Child = _list },
        };
        Grid.SetRow(_scroll, 2);
        Children.Add(_scroll);

        // A card read whole, over everything; a tap anywhere puts it away.
        _preview.Background = Theme.Brush(Theme.WithAlpha(Theme.Window, 0.92));
        _preview.Children.Add(_previewCard);
        _preview.Tapped += (_, e) =>
        {
            e.Handled = true;
            _preview.Visibility = Visibility.Collapsed;
        };
        Grid.SetRowSpan(_preview, 3);
        Children.Add(_preview);
    }

    /// <summary>Shown while the table drafts a grid, and brought up to date with each draft message. Called on the page's timer.</summary>
    public void Follow(GameClient? client)
    {
        bool drafting = client is { Draft: { Format: "grid" }, Table.Phase: TablePhase.Drafting };
        Visibility = drafting ? Visibility.Visible : Visibility.Collapsed;
        if (!drafting)
        {
            _seen = null;
            _preview.Visibility = Visibility.Collapsed;
            return;
        }

        DraftMessage draft = client!.Draft!;
        if (ReferenceEquals(draft, _seen))
        {
            return;
        }

        _seen = draft;
        _mine = draft.Active == client.Seat;
        string picking = draft.Seats.FirstOrDefault(seat => seat.Seat == draft.Active)?.Name ?? "Your opponent";
        _grid.Text = draft.PackNumber > 0 ? $"Grid {draft.PackNumber} of {draft.Packs}" : "The draft is over.";
        _status.Text = draft.PackNumber == 0 ? string.Empty
            : _mine
                ? draft.PickNumber == 1
                    ? "Your pick. Tap a row or a column of the grid on the table, then tap it again to take it."
                    : "Your pick from what is left. Tap a row or a column on the table, then tap it again to take it."
                : $"{picking} is picking.";
        _status.Foreground = Theme.Brush(_mine ? Theme.Accent : Theme.FgSoft);
        _counts.Text = $"Cards drafted: {string.Join("  ·  ", draft.Seats.OrderBy(seat => seat.Seat).Select(seat => $"{(seat.Seat == client.Seat ? "You" : seat.Name)} {seat.Drafted}"))}";
        _poolTitle.Text = draft.Pool.Count == 0 ? "Your pool is empty so far." : $"Your pool  ·  {draft.Pool.Count} card{(draft.Pool.Count == 1 ? string.Empty : "s")}";
        ShowPool(draft.Pool);
    }

    private void ShowPool(IReadOnlyList<ObjectView> pool)
    {
        _list.Children.Clear();
        _rows.Clear();
        foreach (IGrouping<(int Order, string Name), (ObjectView Card, int Place)> group in pool
            .Select((card, place) => (card, place))
            .GroupBy(each => PhoneDeckBuilder.ByManaValue(each.card))
            .OrderBy(group => group.Key.Order))
        {
            TextBlock heading = PhoneDeckBuilder.Text($"{group.Key.Name}  ·  {group.Count()}", 13, Theme.FgSoft, semibold: true);
            heading.Margin = new Thickness(4, 12, 0, 4);
            _list.Children.Add(heading);
            foreach ((ObjectView card, int place) in group.OrderBy(each => each.Card.ManaValue).ThenBy(each => each.Card.Name, StringComparer.Ordinal))
            {
                _list.Children.Add(CardRow(card, place, group.Key.Name));
            }
        }
    }

    /// <summary>One card of the pool: its name and cost over its type line; a tap shows it whole.</summary>
    private FrameworkElement CardRow(ObjectView card, int place, string group)
    {
        var row = new Grid
        {
            Height = RowHeight,
            Padding = new Thickness(12, 0, 10, 0),
            Background = Theme.Brush(Theme.Panel),
            CornerRadius = new CornerRadius(8),
            BorderBrush = Theme.Brush(Theme.Line),
            BorderThickness = new Thickness(1),
        };

        var words = new StackPanel { Spacing = 2, VerticalAlignment = VerticalAlignment.Center };
        var top = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 8 };
        top.Children.Add(PhoneDeckBuilder.Text(card.Name, 15, Theme.Fg, semibold: true));
        if (card.ManaCost.Length > 0)
        {
            top.Children.Add(new SymbolText(13, Theme.Fg) { Text = card.ManaCost, VerticalAlignment = VerticalAlignment.Center });
        }

        words.Children.Add(top);
        string type = CardFace.TypeLine(card);
        if (card.Power is { } power && card.Toughness is { } toughness)
        {
            type = $"{type}  ·  {power}/{toughness}";
        }

        words.Children.Add(PhoneDeckBuilder.Text(type, 12, Theme.FgSoft));
        row.Children.Add(words);
        AutomationProperties.SetName(row, card.Name);
        row.Tapped += (_, e) =>
        {
            e.Handled = true;
            Preview(card, place);
        };
        _rows.Add((row, card.Name, group));
        return row;
    }

    private void Preview(ObjectView card, int place)
    {
        double width = Math.Min(320, Math.Max(200, ActualWidth - 48));
        _previewCard.SetCellSize((float)width, BoardRows.CardHeight((float)width));
        _previewCard.Width = width;
        _previewCard.Height = BoardRows.CardHeight((float)width);
        ObjectView shown = MessagePack.MessagePackSerializer.Deserialize<ObjectView>(MessagePack.MessagePackSerializer.Serialize(card));
        shown.Handle = new ViewHandle(3_100_000 + place);
        _previewCard.SetCells([new CardBoardElement.BoardCell(shown, 0, 0, Tapped: false)]);
        _preview.Visibility = Visibility.Visible;
    }

    // --- For a page driven by a test (MainPage.Test.cs) ---

    /// <summary>Whether rows were made again and are not laid out yet: what a test waits out.</summary>
    public bool Laying => Visibility == Visibility.Visible && _rows.Any(row => row.Row.XamlRoot is null || row.Row.ActualWidth <= 0);

    /// <summary>
    /// The screen as a test reads it, under the phone's <c>inPerson</c>: whether it is up, whether the pick is this seat's,
    /// what it says, and the pool's rows in view, each with its group and its box.
    /// </summary>
    public void WriteTest(Utf8JsonWriter json, UIElement page)
    {
        json.WriteStartObject("draft");
        bool shown = Visibility == Visibility.Visible && _seen is not null;
        json.WriteBoolean("shown", shown);
        json.WriteBoolean("mine", shown && _mine);
        json.WriteString("title", shown ? _title.Text : string.Empty);
        json.WriteString("grid", shown ? _grid.Text : string.Empty);
        json.WriteString("status", shown ? _status.Text : string.Empty);
        json.WriteNumber("pool", shown ? _seen!.Pool.Count : 0);
        json.WriteBoolean("preview", _preview.Visibility == Visibility.Visible);
        json.WriteStartArray("rows");
        if (shown)
        {
            Rect view = _scroll.XamlRoot is null ? default : _scroll.TransformToVisual(page).TransformBounds(new Rect(0, 0, _scroll.ActualWidth, _scroll.ActualHeight));
            foreach ((FrameworkElement row, string name, string group) in _rows)
            {
                if (row.XamlRoot is null || row.ActualWidth <= 0)
                {
                    continue;
                }

                Rect box = row.TransformToVisual(page).TransformBounds(new Rect(0, 0, row.ActualWidth, row.ActualHeight));
                json.WriteStartObject();
                json.WriteString("name", name);
                json.WriteString("group", group);
                json.WriteBoolean("inView", box.Y + (box.Height / 2) >= view.Y && box.Y + (box.Height / 2) <= view.Y + view.Height);
                json.WriteNumber("x", Math.Round(box.X + (box.Width / 2), 1));
                json.WriteNumber("y", Math.Round(box.Y + (box.Height / 2), 1));
                json.WriteNumber("w", Math.Round(box.Width, 1));
                json.WriteNumber("h", Math.Round(box.Height, 1));
                json.WriteEndObject();
            }
        }

        json.WriteEndArray();
        json.WriteEndObject();
    }
}
