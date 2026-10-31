using System.Text.Json;
using Cantrip.Protocol;
using Microsoft.UI.Text;
using Microsoft.UI.Xaml.Automation;
using Windows.Foundation;

namespace Cantrip.Client.Web.InPerson;

/// <summary>
/// Deck building on a player's phone at an in-person table (docs/web-client.md, "In-person play"): the event's deck built
/// with a thumb on a phone held upright, rather than the remote client's columns squeezed to a phone's width. Shown over
/// the page while the table is building decks, on a phone's page alone (<see cref="MainPage"/> attaches it only there).
/// <para>
/// One list at a time - the deck, the sideboard, or the basic lands - its cards as rows a finger's height tall, grouped by
/// mana value, colour or type and filtered by colour. A tap on a row moves the card between the deck and the sideboard; its
/// eye shows the card itself, whole. Basic lands are added with steppers. Over the lists, the count against the
/// minimum, the deck's curve and what it is made of, and Submit.
/// </para>
/// <para>
/// The deck is the server's: every change goes to it as it is made (<see cref="GameClient.BuildDeck"/>, the message the
/// remote client's pool sends), and its deck is shown once it has caught up with what was sent, as the remote client's
/// pool does. Nothing of the deck is kept here beyond the changes on their way.
/// </para>
/// </summary>
public sealed class PhoneDeckBuilder : Grid
{
    /// <summary>A row's height: a finger's, and room for the name over the type line.</summary>
    private const double RowHeight = 60;

    private static readonly string[] ColourKeys = ["W", "U", "B", "R", "G", "C", "M"];

    private enum Tab
    {
        Deck,
        Sideboard,
        Lands,
    }

    private enum Grouping
    {
        ManaValue,
        Colour,
        Type,
    }

    private readonly TextBlock _title = Text(string.Empty, 18, Theme.Fg, semibold: true);
    private readonly TextBlock _count = Text(string.Empty, 15, Theme.Fg, semibold: true);
    private readonly Border _progressFill = new() { Background = Theme.Brush(Theme.Accent), CornerRadius = new CornerRadius(3) };
    private readonly Grid _progress = new() { Height = 6, Background = Theme.Brush(Theme.Line), CornerRadius = new CornerRadius(3) };
    private readonly TextBlock _status = Text(string.Empty, 13, Theme.FgSoft, wrap: true);
    private readonly DockButton _submit = new("Submit", primary: true, fontSize: 16) { MinHeight = 48, MinWidth = 120 };
    private readonly Grid _curve = new() { Height = 64, ColumnSpacing = 4 };
    private readonly TextBlock _makeup = Text(string.Empty, 12, Theme.FgSoft);
    private readonly StackPanel _tabs = new() { Orientation = Orientation.Horizontal, Spacing = 6 };
    private readonly StackPanel _filters = new() { Orientation = Orientation.Horizontal, Spacing = 6 };
    private readonly StackPanel _list = new() { Spacing = 2 };
    private readonly ScrollViewer _scroll;
    private readonly Grid _preview = new() { Visibility = Visibility.Collapsed };
    private readonly CardBoardElement _previewCard = new(300, BoardRows.CardHeight(300)) { HorizontalAlignment = HorizontalAlignment.Center, VerticalAlignment = VerticalAlignment.Center };

    private readonly Dictionary<Tab, DockButton> _tabButtons = [];
    private readonly Dictionary<string, DockButton> _colourButtons = [];
    private readonly Dictionary<Grouping, DockButton> _groupButtons = [];

    /// <summary>The rows shown now, each with its card's place in the pool (or a basic land's name) and whether it is in the deck.</summary>
    private readonly List<(FrameworkElement Row, string Name, int Place, bool InDeck)> _rows = [];
    private readonly List<(FrameworkElement Button, Func<string> Label, Func<bool> Enabled)> _buttons = [];

    /// <summary>The buttons of the list shown now - an eye, a land's steppers - made again with it.</summary>
    private readonly List<(FrameworkElement Button, Func<string> Label, Func<bool> Enabled)> _listButtons = [];

    private GameClient? _client;
    private DeckBuildingMessage? _seen;
    private string _eventId = string.Empty;
    private HashSet<int> _main = [];
    private Dictionary<string, int> _basics = new(StringComparer.Ordinal);
    private bool _submitted;
    private int _companion = -1;

    private Tab _tab = Tab.Deck;
    private Grouping _grouping = Grouping.ManaValue;
    private readonly HashSet<string> _colours = [];

    public PhoneDeckBuilder()
    {
        Background = Theme.Brush(Theme.Window);
        Visibility = Visibility.Collapsed;
        RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
        RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
        RowDefinitions.Add(new RowDefinition { Height = new GridLength(1, GridUnitType.Star) });

        // The deck at a glance: what it is, how many cards against the least, its curve, and Submit.
        var head = new Grid { Padding = new Thickness(16, 12, 16, 10), RowSpacing = 8, Background = Theme.Brush(Theme.Bar) };
        head.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        head.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        for (int row = 0; row < 5; row++)
        {
            head.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
        }

        var titles = new StackPanel { Spacing = 2, VerticalAlignment = VerticalAlignment.Center };
        titles.Children.Add(_title);
        titles.Children.Add(_count);
        head.Children.Add(titles);
        AutomationProperties.SetName(_submit, "Submit deck");
        _submit.Clicked += () => Send(submit: !_submitted);
        _submit.VerticalAlignment = VerticalAlignment.Center;
        Grid.SetColumn(_submit, 1);
        head.Children.Add(_submit);
        Remember(_submit, () => _submitted ? "Change deck" : "Submit deck", () => _submit.Enabled);

        _progress.ColumnDefinitions.Add(new ColumnDefinition());
        _progress.ColumnDefinitions.Add(new ColumnDefinition());
        _progress.Children.Add(_progressFill);
        Grid.SetRow(_progress, 1);
        Grid.SetColumnSpan(_progress, 2);
        head.Children.Add(_progress);

        Grid.SetRow(_status, 2);
        Grid.SetColumnSpan(_status, 2);
        head.Children.Add(_status);

        for (int column = 0; column < 8; column++)
        {
            _curve.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        }

        Grid.SetRow(_curve, 3);
        Grid.SetColumnSpan(_curve, 2);
        head.Children.Add(_curve);
        Grid.SetRow(_makeup, 4);
        Grid.SetColumnSpan(_makeup, 2);
        head.Children.Add(_makeup);
        Children.Add(head);

        // Which list, and how it is sorted and narrowed.
        var tools = new StackPanel { Spacing = 8, Padding = new Thickness(12, 10, 12, 6) };
        foreach (Tab tab in Enum.GetValues<Tab>())
        {
            var button = new DockButton(string.Empty, primary: false, fontSize: 15) { MinHeight = 44, MinWidth = 104 };
            Tab chosen = tab;
            button.Clicked += () => { _tab = chosen; Show(); };
            _tabButtons[tab] = button;
            _tabs.Children.Add(button);
        }

        tools.Children.Add(Scrolling(_tabs));

        var groups = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 6 };
        groups.Children.Add(Text("By", 13, Theme.FgSoft));
        ((TextBlock)groups.Children[0]).VerticalAlignment = VerticalAlignment.Center;
        foreach ((Grouping grouping, string label) in new[] { (Grouping.ManaValue, "Mana value"), (Grouping.Colour, "Colour"), (Grouping.Type, "Type") })
        {
            var button = new DockButton(label, primary: false, fontSize: 14) { MinHeight = 40 };
            Grouping chosen = grouping;
            button.Clicked += () => { _grouping = chosen; Show(); };
            _groupButtons[grouping] = button;
            groups.Children.Add(button);
            string said = $"By {label.ToLowerInvariant()}";
            Remember(button, () => said, () => true);
        }

        tools.Children.Add(Scrolling(groups));

        foreach (string key in ColourKeys)
        {
            var button = new DockButton(key is "C" ? "Colourless" : key is "M" ? "Multi" : $"{{{key}}}", primary: false, fontSize: 15) { MinHeight = 40, MinWidth = 44 };
            string chosen = key;
            button.Clicked += () =>
            {
                if (!_colours.Remove(chosen))
                {
                    _colours.Add(chosen);
                }

                Show();
            };
            _colourButtons[key] = button;
            _filters.Children.Add(button);
            string said = $"Colour {ColourName(chosen)}";
            Remember(button, () => said, () => true);
        }

        tools.Children.Add(Scrolling(_filters));
        Grid.SetRow(tools, 1);
        Children.Add(tools);

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

    /// <summary>
    /// Shown while the table is building decks and this seat has a deck to build, and brought up to date with the server's
    /// deck once it has caught up with the changes sent from here. Called on the page's timer.
    /// </summary>
    public void Follow(GameClient? client)
    {
        bool building = client is { DeckBuilding: not null, Table.Phase: TablePhase.DeckBuilding };
        Visibility = building ? Visibility.Visible : Visibility.Collapsed;
        if (!building)
        {
            _seen = null;
            return;
        }

        _client = client;
        DeckBuildingMessage deck = client!.DeckBuilding!;
        if (ReferenceEquals(deck, _seen))
        {
            return;
        }

        bool first = _seen is null || deck.EventId != _eventId;
        _seen = deck;
        _eventId = deck.EventId;
        if (first || client.DeckCaughtUp)
        {
            _main = [.. deck.Main];
            _basics = deck.Basics.ToDictionary(land => land.Name, land => land.Count, StringComparer.Ordinal);
            _submitted = deck.Submitted;
            _companion = deck.Companion;
        }

        Show();
    }

    /// <summary>Sends the deck as it now stands, and shows it at once rather than when the server's answer comes.</summary>
    private void Send(bool submit)
    {
        if (_client is not { DeckBuilding: not null } client)
        {
            return;
        }

        // A companion stays one only while it is in the sideboard (CR 702.139a).
        if (_main.Contains(_companion))
        {
            _companion = -1;
        }

        client.BuildDeck([.. _main.Order()], _basics, submit, _companion);
        _submitted = submit;
        Show();
    }

    private void Move(int place)
    {
        if (_submitted)
        {
            return;
        }

        if (!_main.Remove(place))
        {
            _main.Add(place);
        }

        Send(submit: false);
    }

    private void ChangeBasic(string land, int by)
    {
        if (_submitted)
        {
            return;
        }

        _basics[land] = Math.Clamp(_basics.GetValueOrDefault(land) + by, 0, 99);
        Send(submit: false);
    }

    // --- Drawing ---

    private void Show()
    {
        if (_seen is not { } deck || _client is not { } client)
        {
            return;
        }

        IReadOnlyList<ObjectView> pool = deck.Pool;
        int basics = _basics.Values.Sum();
        int size = _main.Count + basics;
        _title.Text = deck.Title.Length > 0 ? deck.Title : _submitted ? "Your deck is submitted" : "Build your deck";
        _count.Text = $"{size} / {deck.Minimum} cards";
        _count.Foreground = Theme.Brush(size >= deck.Minimum ? Theme.Ready : Theme.Fg);
        double part = deck.Minimum == 0 ? 1.0 : Math.Min(1.0, size / (double)deck.Minimum);
        _progress.ColumnDefinitions[0].Width = new GridLength(Math.Max(0.0001, part), GridUnitType.Star);
        _progress.ColumnDefinitions[1].Width = new GridLength(1 - part, GridUnitType.Star);
        _progressFill.Background = Theme.Brush(size >= deck.Minimum ? Theme.Ready : Theme.Accent);
        _submit.Caption = _submitted ? "Change" : "Submit";
        _submit.Enabled = _submitted || size >= deck.Minimum;

        List<string> waiting = [.. deck.Seats.Where(seat => seat.Seat != client.Seat && !seat.Submitted).Select(seat => seat.Name)];
        _status.Text = _submitted
            ? waiting.Count == 0 ? "Everyone has submitted. Dealing..." : $"Waiting for {string.Join(" and ", waiting)}. You can still change your deck until then."
            : size < deck.Minimum
                ? $"Add {deck.Minimum - size} more to submit."
                : "Ready to submit.";

        List<ObjectView> main = [.. _main.Where(place => place >= 0 && place < pool.Count).Select(place => pool[place])];
        ShowCurve(main);
        int creatures = main.Count(card => card.CardTypes.Contains(CardTypeView.Creature));
        int lands = main.Count(IsLand) + basics;
        int others = main.Count - creatures - main.Count(IsLand);
        _makeup.Text = $"{Many(creatures, "creature")}  ·  {Many(others, "other spell")}  ·  {Many(lands, "land")}";

        int side = pool.Count - _main.Count;
        _tabButtons[Tab.Deck].Caption = $"Deck  {_main.Count}";
        _tabButtons[Tab.Sideboard].Caption = $"Sideboard  {side}";
        _tabButtons[Tab.Lands].Caption = $"Basics  {basics}";
        foreach ((Tab tab, DockButton button) in _tabButtons)
        {
            button.On = tab == _tab;
        }

        foreach ((Grouping grouping, DockButton button) in _groupButtons)
        {
            button.On = grouping == _grouping;
        }

        foreach ((string key, DockButton button) in _colourButtons)
        {
            button.On = _colours.Contains(key);
        }

        _filters.Visibility = _tab == Tab.Lands ? Visibility.Collapsed : Visibility.Visible;
        ((FrameworkElement)_groupButtons[Grouping.ManaValue].Parent).Visibility = _filters.Visibility;
        ShowList(deck);
    }

    /// <summary>The deck's spells by mana value, a bar for each from nothing to seven and more, with how many.</summary>
    private void ShowCurve(IReadOnlyList<ObjectView> main)
    {
        int[] counts = new int[8];
        foreach (ObjectView card in main.Where(card => !IsLand(card)))
        {
            counts[Math.Min(7, card.ManaValue)]++;
        }

        int most = Math.Max(1, counts.Max());
        _curve.Children.Clear();
        for (int value = 0; value < counts.Length; value++)
        {
            var column = new Grid();
            column.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
            column.RowDefinitions.Add(new RowDefinition { Height = new GridLength(1, GridUnitType.Star) });
            column.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
            TextBlock count = Text(counts[value] == 0 ? string.Empty : counts[value].ToString(System.Globalization.CultureInfo.InvariantCulture), 11, Theme.Fg, centred: true);
            column.Children.Add(count);
            var bar = new Border
            {
                Background = Theme.Brush(counts[value] == 0 ? Theme.Line : Theme.Accent),
                CornerRadius = new CornerRadius(2),
                VerticalAlignment = VerticalAlignment.Bottom,
                Height = counts[value] == 0 ? 2 : 4 + (30.0 * counts[value] / most),
            };
            Grid.SetRow(bar, 1);
            column.Children.Add(bar);
            TextBlock label = Text(value == 7 ? "7+" : value.ToString(System.Globalization.CultureInfo.InvariantCulture), 11, Theme.FgMuted, centred: true);
            Grid.SetRow(label, 2);
            column.Children.Add(label);
            Grid.SetColumn(column, value);
            _curve.Children.Add(column);
        }
    }

    private void ShowList(DeckBuildingMessage deck)
    {
        _list.Children.Clear();
        _rows.Clear();
        _listButtons.Clear();

        if (_tab == Tab.Lands)
        {
            _list.Children.Add(Text("Basic lands", 14, Theme.FgSoft, semibold: true));
            foreach (BasicLandView land in deck.Basics)
            {
                _list.Children.Add(BasicRow(land.Name, _basics.GetValueOrDefault(land.Name)));
            }

            return;
        }

        bool inDeck = _tab == Tab.Deck;
        List<(ObjectView Card, int Place)> cards = [.. deck.Pool
            .Select((card, place) => (card, place))
            .Where(each => _main.Contains(each.place) == inDeck && Passes(each.card))];
        if (cards.Count == 0)
        {
            _list.Children.Add(Text(
                _colours.Count > 0 ? "No cards of those colours here." : inDeck ? "No cards in the deck yet: tap a card in the sideboard to add it." : "The sideboard is empty.",
                14,
                Theme.FgMuted,
                wrap: true));
            return;
        }

        foreach (IGrouping<(int Order, string Name), (ObjectView Card, int Place)> group in cards
            .GroupBy(each => GroupOf(each.Card))
            .OrderBy(group => group.Key.Order))
        {
            TextBlock heading = Text($"{group.Key.Name}  ·  {group.Count()}", 13, Theme.FgSoft, semibold: true);
            heading.Margin = new Thickness(4, 12, 0, 4);
            _list.Children.Add(heading);
            foreach ((ObjectView card, int place) in group.OrderBy(each => each.Card.ManaValue).ThenBy(each => each.Card.Name, StringComparer.Ordinal))
            {
                _list.Children.Add(CardRow(card, place, inDeck));
            }
        }
    }

    /// <summary>One card as a row: its cost, its name over its type line, and where a tap sends it.</summary>
    private FrameworkElement CardRow(ObjectView card, int place, bool inDeck)
    {
        var row = new Grid
        {
            Height = RowHeight,
            ColumnSpacing = 10,
            Padding = new Thickness(4, 0, 10, 0),
            Background = Theme.Brush(Theme.Panel),
            CornerRadius = new CornerRadius(8),
            BorderBrush = Theme.Brush(Theme.Line),
            BorderThickness = new Thickness(1),
        };
        row.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(44) });
        row.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        row.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });

        // The eye shows the card whole; the rest of the row moves it.
        var eye = new Border
        {
            Width = 44,
            Height = 44,
            CornerRadius = new CornerRadius(22),
            Background = Theme.Brush(Theme.ControlFill),
            VerticalAlignment = VerticalAlignment.Center,
            Child = Text("View", 12, Theme.FgSoft, semibold: true, centred: true),
        };
        ((TextBlock)eye.Child).VerticalAlignment = VerticalAlignment.Center;
        AutomationProperties.SetName(eye, $"Show {card.Name}");
        eye.Tapped += (_, e) =>
        {
            e.Handled = true;
            Preview(card, place);
        };
        row.Children.Add(eye);
        _listButtons.Add((eye, () => $"Show {card.Name}", () => true));

        var words = new StackPanel { Spacing = 2, VerticalAlignment = VerticalAlignment.Center };
        var top = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 8 };
        top.Children.Add(Text(card.Name, 15, Theme.Fg, semibold: true));
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

        words.Children.Add(Text(type, 12, Theme.FgSoft));
        Grid.SetColumn(words, 1);
        row.Children.Add(words);

        TextBlock where = Text(inDeck ? "To side" : "To deck", 13, _submitted ? Theme.FgMuted : Theme.Accent, semibold: true);
        where.VerticalAlignment = VerticalAlignment.Center;
        Grid.SetColumn(where, 2);
        row.Children.Add(where);

        AutomationProperties.SetName(row, $"{card.Name}, {(inDeck ? "in the deck" : "in the sideboard")}");
        row.Tapped += (_, e) =>
        {
            e.Handled = true;
            Move(place);
        };
        _rows.Add((row, card.Name, place, inDeck));
        return row;
    }

    /// <summary>One basic land: how many, with steppers a thumb can hit.</summary>
    private FrameworkElement BasicRow(string land, int count)
    {
        var row = new Grid { Height = RowHeight, ColumnSpacing = 10, Padding = new Thickness(12, 0, 8, 0), Background = Theme.Brush(Theme.Panel), CornerRadius = new CornerRadius(8) };
        row.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        row.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(52) });
        row.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(40) });
        row.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(52) });

        var name = new SymbolText(16, Theme.Fg, semibold: true) { Text = $"{Pip(land)} {land}".Trim(), VerticalAlignment = VerticalAlignment.Center };
        row.Children.Add(name);

        var less = new DockButton("−", primary: false, fontSize: 20, enabled: !_submitted && count > 0) { Height = 48 }.Filling();
        AutomationProperties.SetName(less, $"Remove {land}");
        less.Clicked += () => ChangeBasic(land, -1);
        Grid.SetColumn(less, 1);
        row.Children.Add(less);
        _listButtons.Add((less, () => $"Remove {land}", () => less.Enabled));

        TextBlock shown = Text(count.ToString(System.Globalization.CultureInfo.InvariantCulture), 18, Theme.Fg, semibold: true, centred: true);
        shown.VerticalAlignment = VerticalAlignment.Center;
        Grid.SetColumn(shown, 2);
        row.Children.Add(shown);

        var more = new DockButton("+", primary: false, fontSize: 20, enabled: !_submitted && count < 99) { Height = 48 }.Filling();
        AutomationProperties.SetName(more, $"Add {land}");
        more.Clicked += () => ChangeBasic(land, +1);
        Grid.SetColumn(more, 3);
        row.Children.Add(more);
        _listButtons.Add((more, () => $"Add {land}", () => more.Enabled));
        return row;
    }

    private void Preview(ObjectView card, int place)
    {
        double width = Math.Min(320, Math.Max(200, ActualWidth - 48));
        _previewCard.SetCellSize((float)width, BoardRows.CardHeight((float)width));
        _previewCard.Width = width;
        _previewCard.Height = BoardRows.CardHeight((float)width);
        ObjectView shown = MessagePack.MessagePackSerializer.Deserialize<ObjectView>(MessagePack.MessagePackSerializer.Serialize(card));
        shown.Handle = new ViewHandle(3_000_000 + place);
        _previewCard.SetCells([new CardBoardElement.BoardCell(shown, 0, 0, Tapped: false)]);
        _preview.Visibility = Visibility.Visible;
    }

    // --- Sorting and narrowing ---

    private bool Passes(ObjectView card)
    {
        if (_colours.Count == 0)
        {
            return true;
        }

        string key = ColourKey(card);
        return _colours.Contains(key) || (key == "M" && card.Colors.Any(colour => _colours.Contains(Letter(colour))));
    }

    private (int Order, string Name) GroupOf(ObjectView card) => _grouping switch
    {
        Grouping.Colour => ColourKey(card) switch
        {
            "W" => (0, "White"),
            "U" => (1, "Blue"),
            "B" => (2, "Black"),
            "R" => (3, "Red"),
            "G" => (4, "Green"),
            "M" => (5, "Multicolour"),
            _ => (IsLand(card) ? 7 : 6, IsLand(card) ? "Lands" : "Colourless"),
        },
        Grouping.Type => card.CardTypes.Contains(CardTypeView.Creature) ? (0, "Creatures")
            : card.CardTypes.Contains(CardTypeView.Planeswalker) ? (1, "Planeswalkers")
            : card.CardTypes.Contains(CardTypeView.Instant) || card.CardTypes.Contains(CardTypeView.Sorcery) ? (2, "Instants and sorceries")
            : IsLand(card) ? (4, "Lands")
            : (3, "Other permanents"),
        _ => ByManaValue(card),
    };

    /// <summary>
    /// A card's group by mana value - "2 mana", "7+ mana", the lands last - as the deck's list groups it first, and as the
    /// pool is shown on the phone during the draft (<see cref="PhonePickScreen"/>).
    /// </summary>
    internal static (int Order, string Name) ByManaValue(ObjectView card) =>
        IsLand(card) ? (100, "Lands") : (Math.Min(7, card.ManaValue), card.ManaValue >= 7 ? "7+ mana" : $"{card.ManaValue} mana");

    private static bool IsLand(ObjectView card) => card.CardTypes.Contains(CardTypeView.Land);

    /// <summary>A card's colour as a filter knows it: one letter, C for none, M for more than one.</summary>
    private static string ColourKey(ObjectView card) => card.Colors.Count switch
    {
        0 => "C",
        1 => Letter(card.Colors[0]),
        _ => "M",
    };

    private static string Letter(ColorView colour) => colour switch
    {
        ColorView.White => "W",
        ColorView.Blue => "U",
        ColorView.Black => "B",
        ColorView.Red => "R",
        ColorView.Green => "G",
        _ => "C",
    };

    private static string ColourName(string key) => key switch
    {
        "W" => "white",
        "U" => "blue",
        "B" => "black",
        "R" => "red",
        "G" => "green",
        "M" => "multi",
        _ => "colourless",
    };

    private static string Many(int count, string what) => count == 1 ? $"1 {what}" : $"{count} {what}s";

    private static string Pip(string land) => land switch
    {
        "Plains" => "{W}",
        "Island" => "{U}",
        "Swamp" => "{B}",
        "Mountain" => "{R}",
        "Forest" => "{G}",
        _ => string.Empty,
    };

    // --- Small parts ---

    private void Remember(FrameworkElement button, Func<string> label, Func<bool> enabled) => _buttons.Add((button, label, enabled));

    private static ScrollViewer Scrolling(UIElement content) => new()
    {
        HorizontalScrollBarVisibility = ScrollBarVisibility.Hidden,
        VerticalScrollBarVisibility = ScrollBarVisibility.Disabled,
        HorizontalScrollMode = ScrollMode.Enabled,
        VerticalScrollMode = ScrollMode.Disabled,
        Content = content,
    };

    internal static TextBlock Text(string text, double size, Windows.UI.Color color, bool semibold = false, bool wrap = false, bool centred = false) => new()
    {
        Text = text,
        FontSize = size,
        FontWeight = semibold ? FontWeights.SemiBold : FontWeights.Normal,
        Foreground = Theme.Brush(color),
        TextWrapping = wrap ? TextWrapping.WrapWholeWords : TextWrapping.NoWrap,
        TextTrimming = wrap ? TextTrimming.None : TextTrimming.CharacterEllipsis,
        HorizontalAlignment = centred ? HorizontalAlignment.Center : HorizontalAlignment.Left,
        TextAlignment = centred ? TextAlignment.Center : TextAlignment.Left,
    };

    // --- For a page driven by a test (MainPage.Test.cs) ---

    /// <summary>Whether the list shown was made again and its rows are not laid out yet: what a test waits out.</summary>
    public bool Laying =>
        Visibility == Visibility.Visible
        && _rows.Select(row => row.Row).Concat(_listButtons.Select(button => button.Button)).Any(element => element.XamlRoot is null || element.ActualWidth <= 0);

    /// <summary>
    /// The builder as a test reads it, under the phone's <c>inPerson</c>: whether it is up, the list shown, the count
    /// against the least, whether the deck is submitted, each row in view with its card and whether it is in the deck,
    /// each button with its label, and the basic lands.
    /// </summary>
    public void WriteTest(Utf8JsonWriter json, UIElement page)
    {
        json.WriteStartObject("deck");
        bool shown = Visibility == Visibility.Visible && _seen is not null;
        json.WriteBoolean("shown", shown);
        json.WriteString("tab", _tab.ToString());
        json.WriteNumber("size", _main.Count + _basics.Values.Sum());
        json.WriteNumber("minimum", _seen?.Minimum ?? 0);
        json.WriteBoolean("submitted", _submitted);
        json.WriteBoolean("preview", _preview.Visibility == Visibility.Visible);
        json.WriteStartArray("rows");
        if (shown)
        {
            foreach ((FrameworkElement row, string name, int place, bool inDeck) in _rows)
            {
                if (Box(row, page, inList: true) is { } box)
                {
                    json.WriteStartObject();
                    json.WriteString("name", name);
                    json.WriteNumber("place", place);
                    json.WriteBoolean("inDeck", inDeck);
                    WriteBox(json, box);
                    json.WriteEndObject();
                }
            }
        }

        json.WriteEndArray();
        json.WriteStartArray("buttons");
        if (shown)
        {
            List<(FrameworkElement Button, Func<string> Label, Func<bool> Enabled, bool InList)> buttons =
            [
                .. _buttons.Select(each => (each.Button, each.Label, each.Enabled, false)),
                .. _tabButtons.Select(each => ((FrameworkElement)each.Value, (Func<string>)(() => $"Tab {each.Key}"), (Func<bool>)(() => true), false)),
                .. _listButtons.Select(each => (each.Button, each.Label, each.Enabled, true)),
            ];
            foreach ((FrameworkElement button, Func<string> label, Func<bool> enabled, bool inList) in buttons)
            {
                if (Box(button, page, inList) is { } box)
                {
                    json.WriteStartObject();
                    json.WriteString("label", label());
                    json.WriteBoolean("enabled", enabled());
                    WriteBox(json, box);
                    json.WriteEndObject();
                }
            }
        }

        json.WriteEndArray();
        json.WriteStartArray("basics");
        foreach ((string land, int count) in _basics)
        {
            json.WriteStartObject();
            json.WriteString("name", land);
            json.WriteNumber("count", count);
            json.WriteEndObject();
        }

        json.WriteEndArray();
        json.WriteEndObject();
    }

    /// <summary>Where an element is on the page, if it is laid out and at least partly in the list's view.</summary>
    private Rect? Box(FrameworkElement element, UIElement page, bool inList)
    {
        if (element.XamlRoot is null || element.ActualWidth <= 0 || element.Visibility != Visibility.Visible)
        {
            return null;
        }

        Rect box = element.TransformToVisual(page).TransformBounds(new Rect(0, 0, element.ActualWidth, element.ActualHeight));
        if (inList)
        {
            Rect view = _scroll.TransformToVisual(page).TransformBounds(new Rect(0, 0, _scroll.ActualWidth, _scroll.ActualHeight));
            if (box.Y + (box.Height / 2) < view.Y || box.Y + (box.Height / 2) > view.Y + view.Height)
            {
                return null;
            }
        }

        return box;
    }

    private static void WriteBox(Utf8JsonWriter json, Rect box)
    {
        json.WriteNumber("x", Math.Round(box.X + (box.Width / 2), 1));
        json.WriteNumber("y", Math.Round(box.Y + (box.Height / 2), 1));
        json.WriteNumber("w", Math.Round(box.Width, 1));
        json.WriteNumber("h", Math.Round(box.Height, 1));
    }
}
