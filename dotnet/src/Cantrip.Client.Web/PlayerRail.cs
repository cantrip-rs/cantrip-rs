using Cantrip.Protocol;
using Microsoft.UI.Text;
using Windows.UI;

namespace Cantrip.Client.Web;

/// <summary>
/// One player's rail down the left edge of their half of the table: the avatar box with name and life, a PRIORITY
/// badge across its corner, an opponent's hand as a fan of backs, the mana pool, and the library and graveyard piles at the foot.
/// </summary>
public sealed class PlayerRail : Border
{
    /// <summary>
    /// One kind of mana in the pool: how much, what it may be spent on, how much of it is going to the spell being paid
    /// for, and a click that puts one more towards it, where the server offers that.
    /// </summary>
    /// <summary>
    /// A chip in the mana row: mana in the pool, or with <paramref name="Ability"/> a mana ability the player has of
    /// their own (CR 611.2a) — no card offers it — which reads as its caption alone and activates on a click.
    /// </summary>
    public sealed record ManaChipSpec(ManaTypeView Type, int Count, string? Caption, string? Tooltip, Action? Click, bool Ability = false);

    /// <summary>
    /// A pile at the rail's foot: how many cards, the one drawn on top if any is known, what a click on it does, whether
    /// it is lit as something to do, and what its tooltip says.
    /// </summary>
    public sealed record PileSpec(int Count, ObjectView? Top, Action? Click, bool Lit, string? Note);

    /// <summary>A button over the piles into a zone that is not drawn as a pile: a share of exile, a sideboard.</summary>
    public sealed record ZoneButtonSpec(string Text, string Tooltip, bool Open, Action Click, bool Lit = false);

    public const double RailWidth = 208;
    private const double PileWidth = 85;
    private const double PileHeight = 119;

    private readonly PortraitElement _portrait;
    private readonly TextBlock _name;

    /// <summary>The player's match clock, in the room beside the name and life; empty for a game with none.</summary>
    private readonly TextBlock _clock = new()
    {
        FontSize = 22,
        FontWeight = FontWeights.SemiBold,
        Foreground = Theme.Brush(Theme.FgSoft),
        HorizontalAlignment = HorizontalAlignment.Right,
        VerticalAlignment = VerticalAlignment.Center,
        Visibility = Visibility.Collapsed,
    };

    /// <summary>The seat this rail was last filled for.</summary>
    public int Seat { get; private set; }
    private readonly TextBlock _life;
    private readonly FlowPanel _facts;
    private readonly Border _badge;
    private readonly HandFanElement _fan;
    private readonly FlowPanel _manaRow;
    private readonly StackPanel _zoneButtons;
    private readonly PileElement _library;
    private readonly CardBoardElement _libraryTop;
    private readonly TextBlock _libraryCount;
    private readonly PileElement _graveyardSlot;
    private readonly CardBoardElement _graveyardTop;
    private readonly TextBlock _graveyardCount;
    private Action? _libraryClick;
    private Action? _graveyardClick;
    private Action? _fanClick;

    public PlayerRail(bool showsHandFan)
    {
        Width = RailWidth;
        Background = Theme.Brush(Theme.RailIdle);
        BorderBrush = Theme.Brush(Theme.Line);
        BorderThickness = new Thickness(0, 0, 1, 0);
        Padding = new Thickness(14, 12, 14, 12);

        // The rail's column, ten apart, with the piles on its floor. A stack rather than grid
        // rows for the top part, so a player with no hand fan has no gap where it would be.
        var layout = new Grid();
        layout.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
        layout.RowDefinitions.Add(new RowDefinition { Height = new GridLength(1, GridUnitType.Star) });
        layout.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
        layout.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
        Child = layout;

        var column = new StackPanel { Orientation = Orientation.Vertical, Spacing = 10 };
        layout.Children.Add(column);

        var avatar = BuildAvatar(_clock, out _portrait, out _name, out _life, out _facts, out _badge, out _box, out _badgeText, out _badgeTint);
        _pulse.Tick += (_, _) => PaintClock();
        Avatar = avatar;

        // A player is aimed at the way a permanent is: by clicking their box, outlined while they are a legal target.
        avatar.PointerEntered += (_, _) => { _hovered = true; Outline(); };
        avatar.PointerExited += (_, _) => { _hovered = false; Outline(); };
        avatar.Tapped += (_, e) =>
        {
            if (_clicked is { } clicked)
            {
                e.Handled = true;
                clicked();
            }
        };

        // A right click on the player - or a long press - asks for what can be done with them (MainPage.PlayerMenu).
        avatar.RightTapped += (_, e) =>
        {
            if (MenuAsked is { } asked)
            {
                e.Handled = true;
                asked(e);
            }
        };
        column.Children.Add(avatar);

        // As tall as a full hand's fan across the rail, whatever this one holds, so what is below does not move.
        _fan = new HandFanElement { Height = HandFanElement.HeightFor((float)(RailWidth - 28)), Visibility = showsHandFan ? Visibility.Visible : Visibility.Collapsed };
        _fan.Tapped += (_, e) =>
        {
            if (_fanClick is { } clicked)
            {
                e.Handled = true;
                clicked();
            }
        };
        column.Children.Add(_fan);

        // The pool wraps rather than running off the rail.
        _manaRow = new FlowPanel { HorizontalSpacing = 4, VerticalSpacing = 4 };
        column.Children.Add(_manaRow);

        _zoneButtons = new StackPanel { Orientation = Orientation.Vertical, Spacing = 6, Margin = new Thickness(0, 0, 0, 10) };
        Grid.SetRow(_zoneButtons, 2);
        layout.Children.Add(_zoneButtons);

        var piles = new Grid { ColumnSpacing = 10 };
        piles.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        piles.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        Grid.SetRow(piles, 3);
        layout.Children.Add(piles);

        // Each pile is a back, or an empty slot, with the card known to be on top drawn over it. A click anywhere on the
        // pile is the pile's rather than the card's: it opens what is in it.
        piles.Children.Add(Pile(Slot(out _library, out _libraryTop, () => _libraryClick), "Library", out _libraryCount));

        FrameworkElement graveyard = Pile(Slot(out _graveyardSlot, out _graveyardTop, () => _graveyardClick), "Graveyard", out _graveyardCount);
        Grid.SetColumn(graveyard, 1);
        piles.Children.Add(graveyard);
    }

    private static Grid Slot(out PileElement back, out CardBoardElement top, Func<Action?> click)
    {
        var slot = new Grid { Width = PileWidth, Height = PileHeight, Background = Theme.Brush(Color.FromArgb(0, 0, 0, 0)) };
        slot.Children.Add(back = new PileElement { Width = PileWidth, Height = PileHeight });
        slot.Children.Add(top = new CardBoardElement((float)PileWidth - 2, (float)PileHeight - 4) { Width = PileWidth, Height = PileHeight, FliesAlong = false });
        slot.Tapped += (_, e) =>
        {
            if (click() is { } clicked)
            {
                e.Handled = true;
                clicked();
            }
        };

        return slot;
    }

    /// <summary>The top of each pile, drawn as a card, for its tooltip and for holding it up to read.</summary>
    public CardBoardElement LibraryBoard => _libraryTop;

    public CardBoardElement GraveyardBoard => _graveyardTop;

    /// <summary>What each pile's tooltip adds to its top card: that there is something in it to do, or to look at.</summary>
    public string? LibraryNote { get; private set; }

    public string? GraveyardNote { get; private set; }

    /// <summary>Where this player's library, graveyard and hand of cards sit, for cards flying to and from them.</summary>
    public FrameworkElement LibraryPile => _library;

    public FrameworkElement GraveyardPile => _graveyardSlot;

    public FrameworkElement HandFan => _fan;

    /// <summary>Shows the player's picture, as their table names it; empty for none.</summary>
    public void ShowPortrait(string? source) => _portrait.Source = source ?? string.Empty;

    /// <summary>The box that stands for the player, which is what an arrow aimed at them points to.</summary>
    public FrameworkElement Avatar { get; }

    /// <summary>The player's box was right-clicked, or long-pressed.</summary>
    public event Action<Microsoft.UI.Xaml.Input.RightTappedRoutedEventArgs>? MenuAsked;

    private readonly Border _box;
    private readonly TextBlock _badgeText;
    private readonly Border _badgeTint;
    private bool _hovered;
    private Action? _clicked;
    private bool _chosen;

    /// <summary>
    /// Makes the player a target of what is being aimed, or not: outlined as a targetable card is,
    /// and aimed at by a click. <paramref name="chosen"/> marks one already picked for an "up to" choice.
    /// </summary>
    public void SetTarget(Action? clicked, bool chosen)
    {
        _clicked = clicked;
        _chosen = chosen;
        Outline();
    }

    /// <summary>The box's edge: a hairline, green while the player can be aimed at, and blue under the pointer then.</summary>
    private void Outline()
    {
        bool live = _clicked is not null;
        double width = live ? 2 : 1;
        _box.BorderBrush = Theme.Brush(live ? _hovered ? Theme.Hover : Theme.Ready : Theme.Line);
        _box.BorderThickness = new Thickness(width);

        // The content stays where it was however thick the edge.
        _box.Padding = new Thickness(7 - width);
    }

    /// <summary>
    /// Shows the player's match clock - "24:13" - lit while it runs, and red in its last five minutes; nothing for a game
    /// with no clock.
    /// </summary>
    public void ShowClock((TimeSpan Left, bool Running)? clock)
    {
        // A clock with more time on it than this is no clock to the player: shown as none (an in-person phone's, whose
        // table's clocks are off).
        if (clock is { } far && HideClockOver is { } over && far.Left > over)
        {
            clock = null;
        }

        _clockLeft = clock?.Left;
        _clockRunning = clock?.Running ?? false;
        if (clock is not { } shown)
        {
            _clock.Text = string.Empty;
            _clock.Visibility = Visibility.Collapsed;
            PaintClock();
            return;
        }

        _clock.Visibility = Visibility.Visible;
        TimeSpan left = shown.Left;
        _clock.Text = $"{(int)left.TotalMinutes}:{left.Seconds:00}";
        ToolTipService.SetToolTip(_clock, shown.Running
            ? "Match clock, running: the game is waiting on this player. At zero they lose the match."
            : "Match clock: time left for the whole match. It runs while the game waits on this player.");
        PaintClock();
    }

    /// <summary>
    /// A match clock with more left than this is not shown, as for a game with none; null (every page but an in-person
    /// phone, MainPage.InPerson.cs) shows every clock.
    /// </summary>
    public TimeSpan? HideClockOver { get; set; }

    /// <summary>Under this much left on the match clock, it and the PRIORITY badge turn red.</summary>
    private static readonly TimeSpan ClockLow = TimeSpan.FromMinutes(5);

    private static readonly Color RedDark = Theme.FromHex("#8b0000");
    private static readonly Color RedPure = Theme.FromHex("#ff0000");
    private static readonly Color RedMedium = Theme.FromHex("#c62828");

    private TimeSpan? _clockLeft;
    private bool _clockRunning;
    private bool _priorityShown;

    /// <summary>Repaints the clock and the badge while they pulse, a frame at a time.</summary>
    private readonly DispatcherTimer _pulse = new() { Interval = TimeSpan.FromMilliseconds(33) };

    /// <summary>
    /// Colours the match clock, and the PRIORITY badge with it: low on time, a medium red, pulsing from dark red to pure
    /// red once a second while the clock runs; otherwise lit while it runs.
    /// </summary>
    private void PaintClock()
    {
        bool low = _clockLeft is { } left && left < ClockLow;
        bool pulsing = low && _clockRunning;
        if (pulsing != _pulse.IsEnabled)
        {
            if (pulsing) _pulse.Start();
            else _pulse.Stop();
        }

        // The same phase on every rail, from the wall clock, so whatever pulses pulses together.
        Color red = pulsing ? Blend(RedDark, RedPure, 0.5 - 0.5 * Math.Cos(2 * Math.PI * (DateTime.UtcNow.TimeOfDay.TotalMilliseconds % 1000) / 1000)) : RedMedium;
        if (_clockLeft is not null)
        {
            _clock.Foreground = Theme.Brush(low ? red : _clockRunning ? Theme.Accent : Theme.FgSoft);
        }

        if (_priorityShown)
        {
            Color tint = low ? red : Theme.Accent;
            _badgeText.Foreground = Theme.Brush(tint);
            _badgeTint.Background = Theme.Brush(Theme.WithAlpha(tint, 0.18));
        }
    }

    /// <summary>A colour part way from one to another, in a few dozen steps so the brushes made for it stay few.</summary>
    private static Color Blend(Color from, Color to, double part)
    {
        double step = Math.Round(part * 32) / 32;
        byte Mix(byte a, byte b) => (byte)Math.Round(a + (b - a) * step);
        return Color.FromArgb(255, Mix(from.R, to.R), Mix(from.G, to.G), Mix(from.B, to.B));
    }

    public void Update(
        PlayerSummaryView player,
        bool active,
        bool hasPriority,
        PileSpec library,
        PileSpec graveyard,
        IReadOnlyList<ManaChipSpec> mana,
        IReadOnlyList<ZoneButtonSpec> zones,
        IReadOnlyList<ObjectView> knownInHand,
        Action? fanClick)
    {
        Background = Theme.Brush(active ? Theme.RailActive : Theme.RailIdle);
        Seat = player.Seat;
        _name.Text = player.Name;
        _life.Text = player.Life.ToString();
        _life.Foreground = Theme.Brush(player.Life <= 5 ? Theme.Danger : Theme.Fg);
        ShowFacts(player);
        // One badge over the corner, most telling first: a player out of the game, one chosen as a target,
        // then priority.
        (string? badge, Color tint) = player.Status != PlayerStatusView.Active ? (player.Status.ToString(), Theme.Danger)
            : _chosen ? ("Chosen", Theme.Ready)
            : hasPriority ? ("Priority", Theme.Accent)
            : ((string?)null, Theme.Accent);

        _badge.Visibility = badge is null ? Visibility.Collapsed : Visibility.Visible;
        if (badge is not null)
        {
            _badgeText.Text = badge.ToUpperInvariant();
            _badgeText.Foreground = Theme.Brush(tint);
            _badgeTint.Background = Theme.Brush(Theme.WithAlpha(tint, 0.18));
        }

        _priorityShown = badge == "Priority";
        PaintClock();
        _fan.Count = player.HandCount;
        _fan.Known = knownInHand;
        _fanClick = fanClick;
        string held = player.HandCount switch
        {
            0 => "No cards in hand",
            1 => "1 card in hand",
            var n => $"{n} cards in hand",
        };
        ToolTipService.SetToolTip(_fan, knownInHand.Count == 0 ? held : $"{held}, {knownInHand.Count} known. Click to look at them.");

        ShowMana(mana);

        // Rebuilt only when what the buttons say has changed; a click goes to the latest action given for its place.
        _zones = zones;
        if (!zones.Select(zone => (zone.Text, zone.Tooltip, zone.Open, zone.Lit)).SequenceEqual(_zonesShown))
        {
            _zonesShown = [.. zones.Select(zone => (zone.Text, zone.Tooltip, zone.Open, zone.Lit))];
            _zoneButtons.Children.Clear();
            for (int i = 0; i < zones.Count; i++)
            {
                int place = i;
                var button = new DockButton(zones[i].Text, primary: false, fontSize: 12) { On = zones[i].Open, Lit = zones[i].Lit, Tooltip = zones[i].Tooltip }.Filling();
                button.Clicked += () => _zones[place].Click();
                _zoneButtons.Children.Add(button);
            }
        }

        _zoneButtons.Visibility = zones.Count > 0 ? Visibility.Visible : Visibility.Collapsed;

        ShowPile(library, _library, _libraryTop, _libraryCount);
        _libraryClick = library.Click;
        LibraryNote = library.Note;

        ShowPile(graveyard, _graveyardSlot, _graveyardTop, _graveyardCount);
        _graveyardClick = graveyard.Click;
        GraveyardNote = graveyard.Note;
        GraveyardLit = graveyard.Lit;
    }

    /// <summary>
    /// Chips about the player beside their life total, wrapping under it: poison, energy, counters
    /// on the player, the designations they hold, and where they are in a dungeon.
    /// </summary>
    private void ShowFacts(PlayerSummaryView player)
    {
        List<(string Text, Color Tint, double Fill, string? Tooltip)> facts = [];

        void Add(string text, Color tint, double fill = 0.15, string? tooltip = null) => facts.Add((text, tint, fill, tooltip));

        // Ten poison counters lose the game (CR 704.5c), so this is never decoration.
        if (player.PoisonCounters > 0)
        {
            Add($"{player.PoisonCounters} poison", Theme.Danger);
        }

        // Energy is a resource a player keeps between turns (CR 107.16, 122.1), so it has to be visible to spend.
        if (player.Energy > 0)
        {
            Add($"{player.Energy} energy", Theme.Accent, tooltip: "Energy counters. Cards that print {E} in a cost or a choice spend these.");
        }

        // Counters on the player themselves (CR 122.1), named, since a card may print one nothing here has heard of.
        foreach (CounterView counter in player.Counters)
        {
            Add($"{counter.Count} {counter.Name}", Theme.Accent, tooltip: $"{counter.Name} counters on this player. Cards that count them read this number.");
        }

        if (player.IsMonarch)
        {
            Add("Monarch", Theme.Accent, tooltip: "Draws a card at their end step. A creature dealing them combat damage takes the title.");
        }

        if (player.HasInitiative)
        {
            Add("Initiative", Theme.Accent, tooltip: "Ventures into Undercity at the beginning of their upkeep. Creatures dealing them combat damage take it.");
        }

        if (player.HasCitysBlessing)
        {
            Add("City's blessing", Theme.Accent, tooltip: "Has the city's blessing for the rest of the game. Cards with ascend and others check for it.");
        }

        if (player.HasEnduringStory)
        {
            Add("Enduring story", Theme.Accent, tooltip: "Has an enduring story for the rest of the game. Cards with storied and others check for it.");
        }

        // Speed (CR 702.179): shown once the player has any, since max speed abilities work only at 4 (CR 702.178a).
        if (player.Speed is int speed)
        {
            Add(
                speed >= 4 ? "Max speed" : $"Speed {speed}",
                Theme.Accent,
                tooltip: speed >= 4
                    ? "Speed 4, the most there is. Their max speed abilities work."
                    : $"Speed {speed} of 4. It goes up by 1 once on each of their turns when an opponent loses life. Max speed abilities work at 4.");
        }

        // Effects waiting on the player that sit on no object (CR 611.2c): "the next spell you cast can't be countered".
        foreach (string pending in player.PendingEffects)
        {
            Add(pending.TrimEnd('.'), Theme.Accent, tooltip: pending + " It ends with the turn if no spell is cast.");
        }

        // Where their venture marker is (CR 309.4), and how many dungeons they have completed (CR 309.7): the room alone,
        // which fits beside the life total, and the dungeon it is in in the tooltip.
        if (player.Dungeon.Length > 0 || player.DungeonsCompleted > 0)
        {
            string where = player.Dungeon.Length > 0 ? player.DungeonRoom : "Dungeon";
            Add(
                player.DungeonsCompleted > 0 ? $"{where} · {player.DungeonsCompleted} done" : where,
                Theme.Accent,
                fill: 0.08,
                tooltip: ((player.Dungeon.Length > 0 ? $"In {player.Dungeon}, on the room {player.DungeonRoom}." : string.Empty)
                    + (player.DungeonsCompleted > 0 ? $" Has completed {player.DungeonsCompleted} dungeon(s)." : string.Empty)).Trim());
        }

        // Made again only when they say something else.
        if (facts.SequenceEqual(_factsShown))
        {
            return;
        }

        _factsShown = facts;
        while (_facts.Children.Count > 1)
        {
            _facts.Children.RemoveAt(_facts.Children.Count - 1);
        }

        foreach ((string text, Color tint, double fill, string? tooltip) in facts)
        {
            Border chip = Fact(text, tint, fill);
            if (tooltip is not null)
            {
                ToolTipService.SetToolTip(chip, tooltip);
            }

            _facts.Children.Add(chip);
        }
    }

    private List<(string Text, Color Tint, double Fill, string? Tooltip)> _factsShown = [];
    private IReadOnlyList<ZoneButtonSpec> _zones = [];
    private List<(string Text, string Tooltip, bool Open, bool Lit)> _zonesShown = [];

    /// <summary>The zone buttons shown with what they say and whether they are lit, for a page driven by a test (MainPage.Test.cs).</summary>
    public IEnumerable<(string Text, string Tooltip, bool Lit, DockButton Button)> ZoneButtonsShown() =>
        _zoneButtons.Children.OfType<DockButton>().Select((button, i) => (_zonesShown[i].Text, _zonesShown[i].Tooltip, _zonesShown[i].Lit, button));

    /// <summary>The words of the facts shown beside the life total, for a page driven by a test.</summary>
    public IEnumerable<string> FactsShown() => _factsShown.Select(fact => fact.Text);

    /// <summary>What the graveyard pile's tooltip adds, and whether it is lit, for a page driven by a test.</summary>
    public bool GraveyardLit { get; private set; }

    /// <summary>What each mana chip shown was made for, and the text that says its count.</summary>
    private readonly List<(ManaChipSpec Spec, TextBlock Count)> _chips = [];
    private IReadOnlyList<ManaChipSpec> _mana = [];

    /// <summary>
    /// The mana pool as chips, the ones already shown kept where only their count has changed: tapping a land for mana
    /// changes one number, not the row.
    /// </summary>
    /// <summary>The mana chips shown, each with the element a click lands on, for a page driven by a test (MainPage.Test.cs).</summary>
    public IEnumerable<(ManaChipSpec Spec, FrameworkElement Chip)> Chips() =>
        _chips.Select((chip, i) => (chip.Spec, (FrameworkElement)_manaRow.Children[i]));

    private void ShowMana(IReadOnlyList<ManaChipSpec> mana)
    {
        _mana = mana;
        for (int i = 0; i < mana.Count; i++)
        {
            ManaChipSpec spec = mana[i];
            if (i < _chips.Count && Alike(_chips[i].Spec, spec))
            {
                _chips[i].Count.Text = ChipText(spec);
                _chips[i] = (spec, _chips[i].Count);
                continue;
            }

            (Border chip, TextBlock count) = ManaChip(spec, i);
            if (i < _chips.Count)
            {
                _manaRow.Children[i] = chip;
                _chips[i] = (spec, count);
            }
            else
            {
                _manaRow.Children.Add(chip);
                _chips.Add((spec, count));
            }
        }

        while (_chips.Count > mana.Count)
        {
            _manaRow.Children.RemoveAt(_chips.Count - 1);
            _chips.RemoveAt(_chips.Count - 1);
        }

        static bool Alike(ManaChipSpec shown, ManaChipSpec wanted) =>
            shown.Type == wanted.Type
            && shown.Caption == wanted.Caption
            && shown.Tooltip == wanted.Tooltip
            && shown.Ability == wanted.Ability
            && (shown.Click is null) == (wanted.Click is null);
    }

    private static string ChipText(ManaChipSpec spec) => spec.Ability ? spec.Caption ?? string.Empty : spec.Caption is null
        ? spec.Count.ToString(System.Globalization.CultureInfo.InvariantCulture)
        : $"{spec.Count.ToString(System.Globalization.CultureInfo.InvariantCulture)} · {spec.Caption}";

    /// <summary>A fact about the player as a chip: a rounded tint of its colour, the words in it.</summary>
    private static Border Fact(string text, Color tint, double fill) => Theme.Pill(new Border
    {
        Background = Theme.Brush(Theme.WithAlpha(tint, fill)),
        Padding = new Thickness(8, 2, 8, 2),
        VerticalAlignment = VerticalAlignment.Center,
        Child = new TextBlock { Text = text, FontSize = 11, FontWeight = FontWeights.SemiBold, Foreground = Theme.Brush(tint) },
    });

    private static void ShowPile(PileSpec pile, PileElement back, CardBoardElement top, TextBlock count)
    {
        back.Filled = pile.Count > 0;
        back.Lit = pile.Lit && pile.Top is null;
        count.Text = pile.Count.ToString(System.Globalization.CultureInfo.InvariantCulture);
        top.SetCells(pile.Top is null
            ? []
            : [new CardBoardElement.BoardCell(pile.Top, 1, 1, Tapped: false, Actionable: pile.Lit, Clickable: pile.Click is not null)]);

        // A back has no card to carry the note, so the pile says it.
        ToolTipService.SetToolTip(back, pile.Top is null && pile.Note is { Length: > 0 } note ? note : null);
    }

    private static Grid BuildAvatar(TextBlock clock, out PortraitElement portrait, out TextBlock name, out TextBlock life, out FlowPanel facts, out Border badge, out Border box, out TextBlock badgeText, out Border badgeTint)
    {
        var host = new Grid { Background = Theme.Brush(Color.FromArgb(0, 0, 0, 0)) };

        box = new Border
        {
            // As tall as the portrait and its padding, and taller while the facts beside the life total wrap.
            MinHeight = 70,
            Background = Theme.Brush(Theme.WithAlpha(Color.FromArgb(255, 255, 255, 255), 0.03)),
            BorderBrush = Theme.Brush(Theme.Line),
            BorderThickness = new Thickness(1),
            CornerRadius = new CornerRadius(8),
            Padding = new Thickness(6),
        };

        host.Children.Add(box);

        // The column beside the portrait takes the rest of the box's width, which is what the facts wrap at, less the
        // match clock's on the right when there is one.
        var row = new Grid { ColumnSpacing = 8 };
        row.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        row.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        row.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        box.Child = row;

        row.Children.Add(portrait = new PortraitElement { Width = 56, Height = 56, VerticalAlignment = VerticalAlignment.Top });

        var column = new StackPanel { Orientation = Orientation.Vertical, VerticalAlignment = VerticalAlignment.Center };
        Grid.SetColumn(column, 1);
        row.Children.Add(column);

        name = new TextBlock
        {
            FontSize = 14,
            FontWeight = FontWeights.SemiBold,
            Foreground = Theme.Brush(Theme.Fg),
            TextTrimming = TextTrimming.CharacterEllipsis,
        };

        column.Children.Add(name);
        Grid.SetColumn(clock, 2);
        row.Children.Add(clock);

        // Life first on its line, with whatever else there is to say about the player beside it and wrapping under it.
        life = new TextBlock { FontSize = 28, FontWeight = FontWeights.SemiBold, Foreground = Theme.Brush(Theme.Fg), LineHeight = 34, MinHeight = 36, VerticalAlignment = VerticalAlignment.Center };
        ToolTipService.SetToolTip(life, "Life");
        facts = new FlowPanel { HorizontalSpacing = 4, VerticalSpacing = 2, Children = { life } };
        column.Children.Add(facts);

        // Across the box's top right corner on a solid ground of its own, so it neither moves the rail nor takes width
        // from the name when it comes and goes.
        // A badge: the colour's tint behind its caption, on a solid ground of its own because it sits across the box's
        // edge and a tint alone would show the outline through it.
        badge = Theme.Pill(new Border
        {
            Background = Theme.Brush(Theme.Bar),
            HorizontalAlignment = HorizontalAlignment.Right,
            VerticalAlignment = VerticalAlignment.Top,
            Margin = new Thickness(0, -8, 4, 0),
            Visibility = Visibility.Collapsed,
            Child = badgeTint = Theme.Pill(new Border
            {
                Background = Theme.Brush(Theme.WithAlpha(Theme.Accent, 0.18)),
                Padding = new Thickness(8, 1, 8, 1),
                Child = badgeText = new TextBlock
                {
                    Text = "PRIORITY",
                    FontSize = 10,
                    FontWeight = FontWeights.SemiBold,
                    Foreground = Theme.Brush(Theme.Accent),
                },
            }),
        });

        host.Children.Add(badge);
        return host;
    }

    private static StackPanel Pile(FrameworkElement face, string caption, out TextBlock countLabel)
    {
        var column = new StackPanel { Orientation = Orientation.Vertical, Spacing = 4, HorizontalAlignment = HorizontalAlignment.Center };
        column.Children.Add(face);

        var label = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 5, HorizontalAlignment = HorizontalAlignment.Center };
        label.Children.Add(new TextBlock { Text = caption, FontSize = 10, Foreground = Theme.Brush(Theme.FgMuted) });

        countLabel = new TextBlock { Text = "0", FontSize = 10, FontWeight = FontWeights.SemiBold, Foreground = Theme.Brush(Theme.FgSoft) };
        label.Children.Add(countLabel);

        column.Children.Add(label);
        return column;
    }

    /// <summary>
    /// Mana in the pool: the symbol, and how many, on a tint of its colour, followed by what it is for or how much is
    /// going to the spell. One the server offers to put towards the spell is edged in the ready colour and sends one on
    /// a click.
    /// </summary>
    private (Border Chip, TextBlock Count) ManaChip(ManaChipSpec spec, int place)
    {
        string symbol = Letter(spec.Type).ToString();
        SkiaSharp.SKColor fill = ManaSymbols.Fill(symbol);
        Color tint = Color.FromArgb(255, fill.Red, fill.Green, fill.Blue);

        var row = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 4 };
        row.Children.Add(new ManaPipElement(symbol, 14));
        var text = new TextBlock
        {
            Text = ChipText(spec),
            FontSize = 11,
            FontWeight = FontWeights.SemiBold,
            Foreground = Theme.Brush(tint),
            VerticalAlignment = VerticalAlignment.Center,
        };
        row.Children.Add(text);

        bool live = spec.Click is not null;
        Border chip = Theme.Pill(new Border
        {
            Background = Theme.Brush(Theme.WithAlpha(tint, 0.16)),
            BorderBrush = live ? Theme.Brush(Theme.Ready) : null,
            BorderThickness = new Thickness(live ? 1 : 0),
            Padding = live ? new Thickness(5, 1, 5, 1) : new Thickness(6, 2, 6, 2),
            HorizontalAlignment = HorizontalAlignment.Left,
            Child = row,
        });

        if (spec.Tooltip is { Length: > 0 } tooltip)
        {
            ToolTipService.SetToolTip(chip, tooltip);
        }

        if (spec.Click is not null)
        {
            // The action given for this place now, which a later update may have replaced while keeping the chip.
            chip.Tapped += (_, e) =>
            {
                e.Handled = true;
                if (place < _mana.Count)
                {
                    _mana[place].Click?.Invoke();
                }
            };
        }

        return (chip, text);
    }

    private static char Letter(ManaTypeView type) => type switch
    {
        ManaTypeView.White => 'W',
        ManaTypeView.Blue => 'U',
        ManaTypeView.Black => 'B',
        ManaTypeView.Red => 'R',
        ManaTypeView.Green => 'G',
        _ => 'C',
    };
}
