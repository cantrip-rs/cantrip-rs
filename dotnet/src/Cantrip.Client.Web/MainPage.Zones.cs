using Cantrip.Client;
using Cantrip.Client.Presentation;
using Cantrip.Protocol;
using Microsoft.UI.Text;

namespace Cantrip.Client.Web;

/// <summary>
/// The zones that are not drawn as rows of cards, and the ways into them: the graveyard dock between the rails and the
/// board, the piles and the mana pool in the rail, and a zone looked into in the choice window - one player's share of
/// exile, their sideboard, the cards known in a library or an opponent's hand.
/// </summary>
public sealed partial class MainPage
{
    /// <summary>The narrowest the board beside an open graveyard dock may be left.</summary>
    private const float MinBoardWidth = 320f;

    /// <summary>The dock's margins, and the room kept for its scroll bar.</summary>
    private const float GraveyardMarginX = 8f;
    private const float GraveyardScrollRoom = 8f;

    /// <summary>One player's graveyard in the dock: its column, the count in its header, and the cards.</summary>
    private sealed class GraveyardDock
    {
        public required Border Panel { get; init; }
        public required TextBlock Count { get; init; }
        public required ScrollViewer Scroll { get; init; }
        public required CardBoardElement Board { get; init; }
        public required TextBlock Empty { get; init; }
        public required DockButton Close { get; init; }

        /// <summary>How many cards the graveyard held when it was last drawn, to follow new ones down the column.</summary>
        public int Drawn { get; set; } = -1;

        /// <summary>Whether the column is to be scrolled to its end once its new cards are laid out.</summary>
        public bool ToEnd { get; set; }

        /// <summary>Whose graveyard it last drew, for laying it out again as the pointer moves along it.</summary>
        public int Seat { get; set; }
    }

    /// <summary>Whether the graveyards are open in their dock. Both open and close together.</summary>
    private bool _graveyardsOpen;

    /// <summary>How the graveyards were before they opened for a target, to put them back after; null when the player opened them.</summary>
    private bool? _graveyardsBeforeTargets;

    /// <summary>Spells and abilities on the stack aimed at cards in a graveyard, and at which.</summary>
    private readonly Dictionary<ViewHandle, List<ViewHandle>> _aimedAtGraveyards = [];

    /// <summary>Set once the graveyards have opened for the outstanding prompt's targets, so closing them is not overruled.</summary>
    private bool _peekedForTargets;

    /// <summary>A zone looked into in the choice window, while no prompt needs it: exile, a sideboard, a library, a hand.</summary>
    private (ZoneKindView Kind, int Owner)? _peek;

    private ResizeGrip _graveyardGrip = null!;
    private float? _graveyardDragFrom;
    private float _graveyardDragWidth;

    private GraveyardDock BuildGraveyardDock(bool top)
    {
        var count = new TextBlock { Text = "0", FontSize = 11, FontWeight = FontWeights.SemiBold, Foreground = Theme.Brush(Theme.FgSoft), VerticalAlignment = VerticalAlignment.Center };
        var close = new DockButton("×", primary: false, fontSize: 15) { Visibility = top ? Visibility.Visible : Visibility.Collapsed, Tooltip = "Close the graveyards" }.Quiet();

        var header = new Grid { Height = 24, ColumnSpacing = 5 };
        header.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        header.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        header.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });

        // Apart from the count, so a narrow dock trims the word and never the number.
        header.Children.Add(new TextBlock
        {
            Text = "Graveyard",
            FontSize = 11,
            FontWeight = FontWeights.SemiBold,
            Foreground = Theme.Brush(Theme.FgMuted),
            TextTrimming = TextTrimming.CharacterEllipsis,
            VerticalAlignment = VerticalAlignment.Center,
        });
        Grid.SetColumn(count, 1);
        header.Children.Add(count);
        Grid.SetColumn(close, 2);
        close.VerticalAlignment = VerticalAlignment.Center;
        header.Children.Add(close);

        var board = new CardBoardElement(100, 140) { HorizontalAlignment = HorizontalAlignment.Left, VerticalAlignment = VerticalAlignment.Top };
        var empty = new TextBlock
        {
            Text = "Empty",
            FontSize = 11,
            Foreground = Theme.Brush(Theme.FgMuted),
            HorizontalAlignment = HorizontalAlignment.Center,
            Visibility = Visibility.Collapsed,
        };

        var scroll = new ScrollViewer
        {
            HorizontalScrollMode = ScrollMode.Disabled,
            HorizontalScrollBarVisibility = ScrollBarVisibility.Disabled,
            VerticalScrollMode = ScrollMode.Auto,
            VerticalScrollBarVisibility = ScrollBarVisibility.Auto,
            Content = new Grid { Children = { board, empty } },
        };

        var column = new Grid { RowSpacing = 6 };
        column.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
        column.RowDefinitions.Add(new RowDefinition { Height = new GridLength(1, GridUnitType.Star) });
        column.Children.Add(header);
        Grid.SetRow(scroll, 1);
        column.Children.Add(scroll);

        var panel = new Border
        {
            Background = Theme.Brush(Theme.RailIdle),
            BorderBrush = Theme.Brush(Theme.Line),
            BorderThickness = new Thickness(0, 0, 1, 0),
            Padding = new Thickness(GraveyardMarginX, 8, GraveyardMarginX - 2, 8),
            Visibility = Visibility.Collapsed,
            Child = column,
        };

        return new GraveyardDock { Panel = panel, Count = count, Scroll = scroll, Board = board, Empty = empty, Close = close };
    }

    /// <summary>The grip on the dock's right edge, down both halves: dragging it widens the dock, and the width is kept.</summary>
    private void BuildGraveyardGrip()
    {
        _graveyardGrip = new ResizeGrip("Drag to resize the graveyards", sideways: true)
        {
            HorizontalAlignment = HorizontalAlignment.Left,
            Visibility = Visibility.Collapsed,
        };

        _graveyardGrip.Started += () =>
        {
            _graveyardDragFrom = GraveyardWidth();
            _graveyardDragWidth = _graveyardDragFrom.Value;
        };

        _graveyardGrip.Moved += across =>
        {
            float width = MathF.Round(ClampGraveyardWidth((_graveyardDragFrom ?? GraveyardWidth()) + (float)across));
            if (width != _graveyardDragWidth)
            {
                _graveyardDragWidth = width;
                SettleCards();
                _needsRedraw = true;
                RedrawNow();
            }
        };

        _graveyardGrip.Ended += () =>
        {
            _graveyardDragFrom = null;
            PlayerSettings.GraveyardWidth = _graveyardDragWidth;
        };

        Grid.SetRowSpan(_graveyardGrip, 3);
        _table.Children.Add(_graveyardGrip);
    }

    /// <summary>How wide the graveyard dock is: the width being dragged to, or the saved one, within what the table leaves.</summary>
    private float GraveyardWidth() =>
        ClampGraveyardWidth(_graveyardDragFrom is not null ? _graveyardDragWidth : PlayerSettings.GraveyardWidth);

    private float ClampGraveyardWidth(float width)
    {
        float room = (float)_table.ActualWidth - (float)PlayerRail.RailWidth - MinBoardWidth;
        return Math.Clamp(
            width,
            PlayerSettings.MinGraveyardWidth,
            MathF.Max(PlayerSettings.MinGraveyardWidth, MathF.Min(PlayerSettings.MaxGraveyardWidth, room)));
    }

    /// <summary>How far in from the table's left edge the board starts: past the rail, and the graveyards when open.</summary>
    private float BoardInset() => (float)PlayerRail.RailWidth + (_graveyardsOpen ? GraveyardWidth() : 0f);

    private void SetGraveyardsOpen(bool open)
    {
        if (open && !_graveyardsOpen)
        {
            // Opened at the newest cards, which are the ones usually wanted.
            _opponentHalf.Graveyard.Drawn = -1;
            _myHalf.Graveyard.Drawn = -1;
        }

        _graveyardsOpen = open;
        _graveyardsBeforeTargets = null;
        _needsRedraw = true;
        SettleCards();
    }

    /// <summary>
    /// Opens the graveyards for a target in one, remembering how they were so <see cref="LetGraveyardsGo"/> can put them
    /// back. Anything else that opens or closes them is the player's own choice, and forgets it.
    /// </summary>
    private void OpenGraveyardsForTargets()
    {
        bool before = _graveyardsBeforeTargets ?? _graveyardsOpen;
        SetGraveyardsOpen(true);
        _graveyardsBeforeTargets = before;
    }

    /// <summary>
    /// Opens the graveyards when every target is in one, as Reanimate's are: a card in a graveyard pile cannot be clicked
    /// until the pile is looked into.
    /// </summary>
    private void PeekAtTargets(GameClient client, IReadOnlyList<TargetMessage> candidates)
    {
        if (AllInGraveyards(client, candidates))
        {
            OpenGraveyardsForTargets();
        }
    }

    private static bool AllInGraveyards(GameClient client, IReadOnlyList<TargetMessage> candidates)
    {
        List<ObjectView> objects = [.. candidates.Where(c => !c.IsPlayer).Select(c => client.Object(c.Object)).OfType<ObjectView>()];

        return objects.Count > 0
            && candidates.All(c => !c.IsPlayer)
            && objects.All(o => o.Zone == ZoneKindView.Graveyard);
    }

    /// <summary>
    /// Notes a spell or ability just put on the stack if it is aimed at a card in a graveyard, and opens the graveyards
    /// for an opponent's. The player's own were opened while they aimed, and stay however they left them.
    /// </summary>
    private void WatchGraveyardTargets(GameClient client, ViewHandle onStack, int controller, List<TargetMessage> targets)
    {
        List<ViewHandle> aimed = [.. targets
            .Where(t => !t.IsPlayer && client.Object(t.Object)?.Zone == ZoneKindView.Graveyard)
            .Select(t => t.Object)];

        if (onStack == ViewHandle.None || aimed.Count == 0 || client.Object(onStack)?.Zone != ZoneKindView.Stack)
        {
            return;
        }

        _aimedAtGraveyards[onStack] = aimed;

        if (controller != client.Seat)
        {
            OpenGraveyardsForTargets();
        }
    }

    /// <summary>
    /// Puts the graveyards back as they were before they opened for a target, once no spell or ability on the stack is
    /// aimed at a card still in one and nothing is being aimed into one now.
    /// </summary>
    private void LetGraveyardsGo(GameClient client)
    {
        foreach (ViewHandle onStack in _aimedAtGraveyards.Keys.ToList())
        {
            bool stillAimed = client.Object(onStack)?.Zone == ZoneKindView.Stack
                && _aimedAtGraveyards[onStack].Any(t => client.Object(t)?.Zone == ZoneKindView.Graveyard);

            if (!stillAimed)
            {
                _aimedAtGraveyards.Remove(onStack);
            }
        }

        if (_graveyardsBeforeTargets is not { } before
            || _aimedAtGraveyards.Count > 0
            || AimCandidates(client) is { } candidates && AllInGraveyards(client, candidates))
        {
            return;
        }

        SetGraveyardsOpen(before);
    }

    /// <summary>Follows what the latest batch of events put on the stack aimed into a graveyard.</summary>
    private void WatchEvent(GameClient client, GameEventView entry)
    {
        switch (entry)
        {
            case SpellCastEvent cast:
                WatchGraveyardTargets(client, cast.Spell, cast.Controller, cast.Targets);
                break;

            case AbilityActivatedEvent activated:
                if (!activated.Ability.IsNone)
                {
                    _abilitiesTriggered[activated.Ability] = activated.Triggered;
                }

                WatchGraveyardTargets(client, activated.Ability, activated.Controller, activated.Targets);
                break;

            default:
                break;
        }
    }

    /// <summary>
    /// One player's graveyard in the dock: a single column, oldest at the top, each card lying
    /// over the one before so its name shows, and the newest, on top of the pile, whole at the bottom. The card under the
    /// pointer is pulled out, the ones after it sliding down. Once there are too many to show a name each it scrolls.
    /// </summary>
    private void LayOutGraveyard(GameClient client, int seat, GraveyardDock dock)
    {
        dock.Seat = seat;
        dock.Panel.Visibility = _graveyardsOpen ? Visibility.Visible : Visibility.Collapsed;
        dock.Panel.Width = GraveyardWidth();

        if (!_graveyardsOpen)
        {
            dock.Board.SetCells([]);
            dock.Drawn = -1;
            return;
        }

        IReadOnlyList<ObjectView> cards = client.Zone(ZoneKindView.Graveyard, seat);
        dock.Count.Text = cards.Count.ToString(System.Globalization.CultureInfo.InvariantCulture);
        dock.Empty.Visibility = cards.Count == 0 ? Visibility.Visible : Visibility.Collapsed;

        // Keeps up with the newest card when it was already in view, as a log does, and leaves an older one being read.
        bool atEnd = dock.Scroll.VerticalOffset >= dock.Scroll.ScrollableHeight - 4;
        if (dock.Drawn < 0 || (cards.Count > dock.Drawn && atEnd))
        {
            dock.ToEnd = true;
        }

        dock.Drawn = cards.Count;

        float width = MathF.Floor(GraveyardWidth() - (GraveyardMarginX * 2) + 2 - GraveyardScrollRoom - 1f);
        float height = MathF.Round(width * 88f / 63f);
        dock.Board.SetCellSize(width, height);
        dock.Board.Width = width;

        // Enough of each card to read its name and cost, and closer only while they all fit.
        // Measured from the half rather than the scroll, which has no height yet the first time the dock opens: the half
        // less the dock's padding, its header and the gap under it.
        float half = dock.Panel.Parent is FrameworkElement { ActualHeight: > 0 } parent ? (float)parent.ActualHeight : height;
        float available = MathF.Max(height, half - 16f - 24f - 6f);
        // A column keeps no room for a card pulled out: the cards under it slide past the end until it is let go,
        // which costs less than an empty card's height under every column.
        float step = cards.Count <= 1
            ? height + 6f
            : MathF.Max(MathF.Round(width * 0.3f), MathF.Min(height + 6f, (available - height) / (cards.Count - 1)));
        float reveal = MathF.Max(0f, height - step);
        int hovered = dock.Board.Hovered is { } pointed ? cards.ToList().FindIndex(card => card.Handle == pointed) : -1;

        List<CardBoardElement.BoardCell> cells = [];
        for (int i = 0; i < cards.Count; i++)
        {
            float y = (i * step) + (hovered >= 0 && i > hovered ? reveal : 0f);
            cells.Add(Cell(client, cards[i], 0, y, tapped: false));
        }

        dock.Board.Height = cards.Count == 0 ? 0f : ((cards.Count - 1) * step) + height;
        dock.Board.SetCells(cells);

        if (dock.ToEnd)
        {
            dock.ToEnd = false;
            dock.Scroll.UpdateLayout();
            dock.Scroll.ChangeView(null, dock.Scroll.ScrollableHeight, null, disableAnimation: true);
        }
    }

    // --- The rail ---

    /// <summary>Each player's match clock in their rail, counted down from the last the server said.</summary>
    private void ShowMatchClocks()
    {
        GameClient? client = _gameClient;
        foreach (PlayerRail rail in (PlayerRail[])[_myHalf.Rail, _opponentHalf.Rail])
        {
            rail.ShowClock(client is null || rail.Seat == 0 ? null : client.ClockOf(rail.Seat));
        }
    }

    /// <summary>Fills a player's rail: the avatar, the pool, the piles, and the ways into exile and a sideboard.</summary>
    private void UpdateRail(GameClient client, PlayerRail rail, PlayerSummaryView player, bool active, bool priority)
    {
        int seat = player.Seat;
        // Another player's hand this seat may look into - a player it controls (CR 723.4) - is face up in their fan, every card
        // of it; any other, only the cards known in it.
        List<ObjectView> known = seat == client.Seat ? []
            : client.View?.Zones.FirstOrDefault(zone => zone.Kind == ZoneKindView.Hand && zone.Owner == seat) is { ContentsVisible: true }
                ? [.. client.Zone(ZoneKindView.Hand, seat)]
                : [.. client.NamedIn(ZoneKindView.Hand, seat)];

        rail.Update(
            player,
            active,
            priority,
            PileFor(client, ZoneKindView.Library, seat),
            PileFor(client, ZoneKindView.Graveyard, seat),
            ManaChips(client, player),
            ZoneButtons(client, seat),
            known,
            known.Count > 0 ? () => { TogglePeek(ZoneKindView.Hand, seat); RedrawNow(); } : null);
        rail.ShowPortrait(PortraitOf(client, seat));
    }

    /// <summary>
    /// What can be done with another player, from a right click on their box: message them on Discord, for a player who
    /// came from there. Nothing for anyone else.
    /// </summary>
    private void PlayerMenu(PlayerRail rail, Microsoft.UI.Xaml.Input.RightTappedRoutedEventArgs e)
    {
        if (_gameClient is not { } client || rail.Seat == client.Seat) return;
        string discord = client.Table?.Players.FirstOrDefault(each => each.Seat == rail.Seat)?.Discord ?? string.Empty;
        if (discord.Length == 0) return;
        _menu.Open(Root, e.GetPosition(Root), [("DM on Discord", () => { _menu.Close(); ExternalLink.Open($"https://discord.com/users/{discord}"); })]);
    }

    /// <summary>A seat's picture, as the table names it; empty for none.</summary>
    private static string PortraitOf(GameClient client, int seat) =>
        client.Table?.Players.FirstOrDefault(each => each.Seat == seat)?.Portrait ?? string.Empty;

    /// <summary>
    /// A library or graveyard pile: its top card where the viewer knows it, a click that opens it where
    /// there is anything to see, and a note when a card in a closed graveyard can be cast, played or activated.
    /// </summary>
    private PlayerRail.PileSpec PileFor(GameClient client, ZoneKindView kind, int seat)
    {
        int count = client.ZoneCount(kind, seat);
        if (count == 0)
        {
            return new PlayerRail.PileSpec(0, null, null, false, null);
        }

        IReadOnlyList<ObjectView> contents = client.Zone(kind, seat);
        IReadOnlyList<KnownCardView> known = client.KnownCards(kind, seat);

        // A library the viewer is shown one card of, with its handle, is showing its top: Bolas's Citadel's "look at the
        // top card of your library any time". Several shown at once (a search) say nothing about which is on top.
        IReadOnlyList<ObjectView> shown = kind == ZoneKindView.Library ? client.Shown(kind, seat) : [];
        ObjectView? shownTop = shown.Count == 1 ? shown[0] : null;

        ObjectView? top = contents.Count > 0
            ? contents[^1]
            : shownTop ?? known.FirstOrDefault(k => k.FromTop == 0)?.Card;

        bool browsable = contents.Count > 0 || known.Count > 0 || shownTop is not null;

        List<ActionOptionMessage> offered = kind == ZoneKindView.Graveyard
            ? [.. contents.SelectMany(card => _actions.For(card.Handle))]
            : [];

        bool offers = offered.Count > 0;
        bool plays = offered.Exists(option => option.Kind == ActionKindView.PlayLand);
        bool activates = offered.Exists(option => option.Kind == ActionKindView.ActivateAbility);

        Action? click = !browsable ? null
            : kind == ZoneKindView.Graveyard ? () => { SetGraveyardsOpen(!_graveyardsOpen); RedrawNow(); }
            : () => { TogglePeek(kind, seat); RedrawNow(); };

        bool lit = offers || (browsable && (kind == ZoneKindView.Graveyard ? _graveyardsOpen : _peek == (kind, seat)));

        string? note = contents.Count == 0 && shownTop is not null
            ? "The top card of your library. Click to look."
            : contents.Count == 0 && known.Count > 0
            ? $"You know {known.Count} card{(known.Count == 1 ? string.Empty : "s")} in this library. Click to look."
            : offers && !_graveyardsOpen
            ? $"A card here can be {(plays ? "played" : activates ? "activated" : "cast")}{GraveyardWindows(client, contents, _actions)}. Click to look."
            : null;

        return new PlayerRail.PileSpec(count, top is null ? null : Standing(top, -900_000 - seat), click, lit, note);
    }

    /// <summary>
    /// The permissions the viewer holds on cards in a graveyard (Stingcaster Mage's flashback), for the pile's note: " (Lightning
    /// Bolt: Castable this turn, then exiled)". Empty when it is no permission that lets them be cast.
    /// </summary>
    private static string GraveyardWindows(GameClient client, IReadOnlyList<ObjectView> contents, CardActions actions)
    {
        // A card's own way to be cast from here, printed on it (Wild Ride's "Harmonize {4}{R}", Faithless Looting's flashback), says what it costs.
        string[] ways = [.. contents.Where(card => !card.PlayPermissions.Any(permission => permission.Seat == client.Seat))
            .SelectMany(card => actions.For(card.Handle).Where(option => option.Kind == ActionKindView.CastSpell && option.Description.Length > 0).Select(option => $"{card.Name}: {option.Description}"))];
        string[] windows = [.. ways, .. contents.SelectMany(card => card.PlayPermissions.Where(permission => permission.Seat == client.Seat).Select(permission => $"{card.Name}: {permission.Text}"))];
        return windows.Length == 0 ? string.Empty : $" ({string.Join("; ", windows)})";
    }

    /// <summary>
    /// The buttons over a player's piles while they have cards in exile or a sideboard. Each opens its zone in the choice window, and closes it again.
    /// </summary>
    private List<PlayerRail.ZoneButtonSpec> ZoneButtons(GameClient client, int seat)
    {
        List<PlayerRail.ZoneButtonSpec> buttons = [];
        string name = NameOf(client, seat);

        List<ObjectView> exiled = ExileOf(client, seat);
        if (exiled.Count > 0)
        {
            // A card in exile the viewer may play (Bonehoard Dracosaur's, Cori's) lights the button, as a graveyard pile is lit
            // for a card in it that can be cast, and the windows they stand under are on its tooltip.
            // Only the cards the server offers an action for are counted as playable; the windows of every permission held
            // are on the tooltip, and on each card.
            List<ObjectView> playable = [.. exiled.Where(card => _actions.For(card.Handle).Count > 0)];
            string windows = string.Join("; ", exiled.Where(card => card.Known).SelectMany(card => card.PlayPermissions.Where(permission => permission.Seat == client.Seat).Select(permission => $"{card.Name}: {permission.Text}")));
            buttons.Add(new PlayerRail.ZoneButtonSpec(
                playable.Count > 0 ? $"Exile  {exiled.Count}  ·  {playable.Count} playable" : $"Exile  {exiled.Count}",
                (seat == client.Seat ? "Look at the cards you own in exile" : $"Look at the cards {name} owns in exile")
                    + (playable.Count > 0 ? $". {playable.Count} can be played from there now." : string.Empty)
                    + (windows.Length > 0 ? $" {windows}." : string.Empty),
                _peek == (ZoneKindView.Exile, seat),
                () => { TogglePeek(ZoneKindView.Exile, seat); RedrawNow(); },
                Lit: playable.Count > 0));
        }

        // A sideboard holds the cards a player owns outside the game (CR 400.11a): only its owner may look at it, so only its
        // owner's rail has the button. A revealed companion waits there too, and has a window of its own over the battlefield
        // (LayOutCompanion), where it is taken.
        IReadOnlyList<ObjectView> sideboard = client.Zone(ZoneKindView.Sideboard, seat);
        if (sideboard.Count > 0)
        {
            buttons.Add(new PlayerRail.ZoneButtonSpec(
                $"Sideboard  {sideboard.Count}",
                "Your sideboard: the cards you own outside the game. Only you can see them",
                _peek == (ZoneKindView.Sideboard, seat),
                () => { TogglePeek(ZoneKindView.Sideboard, seat); RedrawNow(); }));
        }

        return buttons;
    }

    /// <summary>
    /// The cards in exile that belong to a seat. Exile is one zone shared by everyone (CR 400.2) and the view carries it
    /// whole under seat zero, so a seat's share of it is picked out by owner.
    /// </summary>
    private static List<ObjectView> ExileOf(GameClient client, int seat) =>
        [.. client.Zone(ZoneKindView.Exile, 0).Where(card => card.Owner == seat)];

    /// <summary>
    /// The pool, one chip per kind of mana: mana that may only be spent on certain things is a chip
    /// of its own, captioned with what it is for, and so is mana a rule tells apart by the permanent that produced it
    /// (CR 106.3), captioned with that permanent's name; while this player pays for a spell each says how much is going to
    /// it, and one the server offers to put towards it sends one on a click.
    /// </summary>
    private List<PlayerRail.ManaChipSpec> ManaChips(GameClient client, PlayerSummaryView player)
    {
        PayCostsPromptMessage? paying = player.Seat == client.ActingSeat && client.Prompt?.Prompt is PayCostsPromptMessage pay ? pay : null;
        List<PlayerRail.ManaChipSpec> chips = [];

        for (int type = 0; type < player.Mana.Length; type++)
        {
            if (player.Mana[type] > 0)
            {
                chips.Add(Chip((ManaTypeView)type, ManaSpendFilterView.None, ManaSpendEffectView.None, [], 0, player.Mana[type], null, null));
            }
        }

        foreach (RestrictedManaView restricted in player.RestrictedMana)
        {
            if (!restricted.Source.IsNone)
            {
                string from = client.Object(restricted.Source)?.Name ?? "a permanent";
                chips.Add(Chip(
                    restricted.Type,
                    ManaSpendFilterView.None,
                    ManaSpendEffectView.None,
                    [],
                    0,
                    restricted.Amount,
                    $"from {from}",
                    $"Mana {from} produced, kept apart because {from} cares whether it is spent",
                    restricted.Source));
                continue;
            }

            chips.Add(Chip(
                restricted.Type,
                restricted.Filter,
                restricted.Effect,
                restricted.SpendSubtypes,
                restricted.SpendManaValue,
                restricted.Amount,
                RestrictionCaption(restricted.Filter, restricted.Effect, restricted.SpendSubtypes, restricted.SpendManaValue),
                RestrictionTooltip(restricted.Filter, restricted.Effect, restricted.SpendSubtypes, restricted.SpendManaValue)));
        }

        // The player's own mana abilities (CR 611.2a), which no card on the table offers: beside the pool they add to,
        // one chip for each that reads differently, since two Channels give two abilities that do the same thing.
        if (player.Seat == client.Seat)
        {
            foreach (IGrouping<(string, string), ActionOptionMessage> own in _actions.Own.GroupBy(option => (option.GrantedBy, option.Description)))
            {
                ActionOptionMessage first = own.First();
                ManaTypeView adds = first.Produces.Count > 0 ? first.Produces[0] : ManaTypeView.Colorless;
                string cost = first.LifeCost > 0 ? $"{first.LifeCost} life" : first.CostText;
                chips.Add(new PlayerRail.ManaChipSpec(
                    adds,
                    0,
                    cost.Length > 0 ? $"{first.GrantedBy} · {cost}" : first.GrantedBy,
                    $"{first.GrantedBy}: {first.Description}",
                    () =>
                    {
                        Choose(client, [first]);
                        RedrawNow();
                    },
                    Ability: true));
            }
        }

        return chips;

        PlayerRail.ManaChipSpec Chip(ManaTypeView type, ManaSpendFilterView filter, ManaSpendEffectView effect, List<string> subtypes, int manaValue, int count, string? caption, string? tooltip, ViewHandle source = default)
        {
            if (paying is null)
            {
                return new PlayerRail.ManaChipSpec(type, count, caption, tooltip, null);
            }

            int committed = paying.Committed
                .Where(c => c.Type == type && c.Filter == filter && c.Effect == effect && c.SpendSubtypes.SequenceEqual(subtypes) && c.SpendManaValue == manaValue && c.Source == source)
                .Sum(c => c.Amount);

            string spokenFor = committed == 0 ? string.Empty : $"{committed} to spell";
            string full = caption is null ? spokenFor : spokenFor.Length == 0 ? caption : $"{caption} · {spokenFor}";

            if (_actions.AssignFor(type, filter, effect, subtypes, manaValue, source) is not { } assign)
            {
                return new PlayerRail.ManaChipSpec(type, count, full.Length == 0 ? null : full, tooltip, null);
            }

            return new PlayerRail.ManaChipSpec(type, count, full.Length == 0 ? null : full, "Put one towards the spell", () =>
            {
                Send(client, AnswerFor(assign));
                RedrawNow();
            });
        }
    }

    private static string RestrictionCaption(ManaSpendFilterView filter, ManaSpendEffectView effect, List<string> subtypes, int manaValue) => (filter, effect) switch
    {
        (ManaSpendFilterView.NoncreatureSpells, _) => "noncreature spells",
        (ManaSpendFilterView.ManaValueSpells, _) => $"mana value {manaValue}+",
        (ManaSpendFilterView.ManaValueOrXSpells, _) => $"mana value {manaValue}+ or X",
        (ManaSpendFilterView.SubtypeSpells or ManaSpendFilterView.SubtypeCreatureSpells or ManaSpendFilterView.SubtypeSpellsOrAbilities
            or ManaSpendFilterView.SubtypeCreatureSpellsOrAbilities or ManaSpendFilterView.SubtypeSpellsOrAbilitiesOfPermanents, _)
            when subtypes.Count == 0 => "no type chosen",
        (ManaSpendFilterView.SubtypeCreatureSpellsOrAbilities, _) => $"{CardActions.Subtypes(subtypes).ToLowerInvariant()} creatures & abilities",
        (ManaSpendFilterView.SubtypeSpellsOrAbilities, _) => $"{CardActions.Subtypes(subtypes).ToLowerInvariant()} spells & abilities",
        (ManaSpendFilterView.SubtypeSpellsOrAbilitiesOfPermanents, _) =>
            $"{CardActions.Subtypes(subtypes).ToLowerInvariant()} spells & battlefield {CardActions.Subtypes(subtypes).ToLowerInvariant()} abilities",
        (ManaSpendFilterView.SubtypeSpells, ManaSpendEffectView.CantBeCountered) => $"{CardActions.Subtypes(subtypes).ToLowerInvariant()} spells · uncounterable",
        (ManaSpendFilterView.SubtypeSpells, _) => $"{CardActions.Subtypes(subtypes).ToLowerInvariant()} spells",
        (ManaSpendFilterView.SubtypeCreatureSpells, ManaSpendEffectView.CantBeCountered) => $"{CardActions.Subtypes(subtypes).ToLowerInvariant()} creatures · uncounterable",
        (ManaSpendFilterView.SubtypeCreatureSpells, _) => $"{CardActions.Subtypes(subtypes).ToLowerInvariant()} creatures",
        (ManaSpendFilterView.CreatureSpells, _) => "creatures",
        (ManaSpendFilterView.InstantOrSorcerySpells, _) => "instants & sorceries",
        (ManaSpendFilterView.ActivatedAbilities, _) => "abilities",
        (ManaSpendFilterView.LegendarySpells, _) => "legendary",
        (ManaSpendFilterView.NotNonartifactSpells, _) => "no nonartifact spells",
        (ManaSpendFilterView.SpellsFromExile, _) => "spells from exile",
        (ManaSpendFilterView.NotSpellsFromHand, _) => "not from hand",
        (ManaSpendFilterView.ArtifactSpells, _) => "artifacts",
        (ManaSpendFilterView.PlaneswalkerSpells, _) => "planeswalkers",
        (ManaSpendFilterView.LandAbilities, _) => "land abilities",
        (ManaSpendFilterView.ArtifactAbilities, _) => "artifact abilities",
        (ManaSpendFilterView.CreatureAbilities, _) => "creature abilities",
        (ManaSpendFilterView.AbilitiesOfArtifacts, _) => "battlefield artifact abilities",
        (ManaSpendFilterView.CreatureSpellsOrAbilitiesOfCreatures, _) => "creatures & battlefield creature abilities",
        (ManaSpendFilterView.CreatureSpellsOrAbilities, _) => "creatures & creature abilities",
        (ManaSpendFilterView.ArtifactSpellsOrAbilitiesOfArtifacts, _) => "artifacts & battlefield artifact abilities",
        (ManaSpendFilterView.ArtifactSpellsOrAbilities, _) => "artifact spells & source abilities",
        (ManaSpendFilterView.ArtifactSpellsOrAnyAbility, _) => "artifacts & abilities",
        (_, ManaSpendEffectView.GainsHaste) => "haste",
        (_, ManaSpendEffectView.CantBeCountered) => "uncounterable",
        _ => "restricted",
    };

    private static string RestrictionTooltip(ManaSpendFilterView filter, ManaSpendEffectView effect, List<string> subtypes, int manaValue)
    {
        string text = filter switch
        {
            ManaSpendFilterView.NoncreatureSpells => "Spend this mana only to cast a noncreature spell.",
            ManaSpendFilterView.ManaValueSpells => $"Spend this mana only to cast spells with mana value {manaValue} or greater.",
            ManaSpendFilterView.ManaValueOrXSpells =>
                $"Spend this mana only to cast spells with mana value {manaValue} or greater or spells with {{X}} in their mana costs.",
            ManaSpendFilterView.SubtypeSpells or ManaSpendFilterView.SubtypeCreatureSpells when subtypes.Count == 0 =>
                "Spend this mana only to cast a spell of the chosen type. No type was chosen, so it can't be spent on any spell.",
            ManaSpendFilterView.SubtypeSpellsOrAbilities when subtypes.Count == 0 =>
                "Spend this mana only to cast a spell of the chosen type or activate an ability of a source of the chosen type. "
                + "No type was chosen, so it can't be spent on anything.",
            ManaSpendFilterView.SubtypeCreatureSpellsOrAbilities when subtypes.Count == 0 =>
                "Spend this mana only to cast a creature spell of the chosen type or activate an ability of a creature source of the "
                + "chosen type. No type was chosen, so it can't be spent on anything.",
            ManaSpendFilterView.SubtypeSpells => $"Spend this mana only to cast {CardActions.Subtypes(subtypes)} spells.",
            ManaSpendFilterView.SubtypeCreatureSpells => $"Spend this mana only to cast {CardActions.Subtypes(subtypes)} creature spells.",
            ManaSpendFilterView.SubtypeCreatureSpellsOrAbilities =>
                $"Spend this mana only to cast {CardActions.Subtypes(subtypes)} creature spells or activate abilities of {CardActions.Subtypes(subtypes)} creature sources.",
            ManaSpendFilterView.SubtypeSpellsOrAbilities =>
                $"Spend this mana only to cast {CardActions.Subtypes(subtypes)} spells or activate abilities of {CardActions.Subtypes(subtypes)} sources.",
            ManaSpendFilterView.SubtypeSpellsOrAbilitiesOfPermanents when subtypes.Count == 0 =>
                "Spend this mana only to cast a spell of the chosen type or activate an ability of a permanent of the chosen type. "
                + "No type was chosen, so it can't be spent on anything.",
            ManaSpendFilterView.SubtypeSpellsOrAbilitiesOfPermanents =>
                $"Spend this mana only to cast {CardActions.Subtypes(subtypes)} spells or activate abilities of {CardActions.Subtypes(subtypes)} "
                + "permanents on the battlefield.",
            ManaSpendFilterView.CreatureSpells => "Spend this mana only to cast creature spells.",
            ManaSpendFilterView.InstantOrSorcerySpells => "Spend this mana only to cast instant or sorcery spells.",
            ManaSpendFilterView.ActivatedAbilities => "Spend this mana only to activate abilities.",
            ManaSpendFilterView.LegendarySpells => "Spend this mana only to cast a legendary spell.",
            ManaSpendFilterView.NotNonartifactSpells => "This mana can't be spent to cast a nonartifact spell.",
            ManaSpendFilterView.SpellsFromExile => "Spend this mana only to cast spells from exile.",
            ManaSpendFilterView.NotSpellsFromHand => "This mana can't be spent to cast spells from your hand.",
            ManaSpendFilterView.ArtifactSpells => "Spend this mana only to cast artifact spells.",
            ManaSpendFilterView.PlaneswalkerSpells => "Spend this mana only to cast a planeswalker spell.",
            ManaSpendFilterView.LandAbilities => "Spend this mana only to activate abilities of land sources.",
            ManaSpendFilterView.ArtifactAbilities => "Spend this mana only to activate abilities of artifact sources.",
            ManaSpendFilterView.CreatureAbilities => "Spend this mana only to activate abilities of creature sources.",
            ManaSpendFilterView.AbilitiesOfArtifacts =>
                "Spend this mana only to activate abilities of artifacts on the battlefield.",
            ManaSpendFilterView.CreatureSpellsOrAbilitiesOfCreatures =>
                "Spend this mana only to cast creature spells or activate abilities of creatures on the battlefield.",
            ManaSpendFilterView.CreatureSpellsOrAbilities =>
                "Spend this mana only to cast creature spells or activate abilities of creature sources.",
            ManaSpendFilterView.ArtifactSpellsOrAbilitiesOfArtifacts =>
                "Spend this mana only to cast artifact spells or activate abilities of artifacts on the battlefield.",
            ManaSpendFilterView.ArtifactSpellsOrAbilities =>
                "Spend this mana only to cast artifact spells or activate abilities of artifact sources.",
            ManaSpendFilterView.ArtifactSpellsOrAnyAbility => "Spend this mana only to cast an artifact spell or to activate an ability.",
            _ => "This mana may only be spent on certain things.",
        };

        return effect switch
        {
            ManaSpendEffectView.GainsHaste => text + " A creature spell it is spent on gains haste until end of turn.",
            ManaSpendEffectView.CantBeCountered => text + " The spell it is spent on can't be countered.",
            _ => text,
        };
    }

    // --- Looking into a zone ---

    private void TogglePeek(ZoneKindView kind, int seat)
    {
        // Clicking the open pile closes it, so the control is its own way back.
        _peek = _peek == (kind, seat) ? null : (kind, seat);
        _choiceCollapsed = false;
        _needsRedraw = true;
    }

    /// <summary>
    /// A zone the player has opened, shown in the choice window when no prompt is using it. A prompt
    /// that needs the window wins: a library search is a decision, and a pile being looked into is not.
    /// </summary>
    private void FillPeek(GameClient client)
    {
        if (_choicesOpen || _peek is not { } peek)
        {
            return;
        }

        if (peek.Kind == ZoneKindView.Exile)
        {
            FillExile(client, peek.Owner);
            return;
        }

        IReadOnlyList<ObjectView> contents = client.Zone(peek.Kind, peek.Owner);

        // The top of a library its owner is shown, with its handle, so it can be played from here (Bolas's Citadel).
        if (peek.Kind == ZoneKindView.Library && contents.Count == 0 && client.Shown(peek.Kind, peek.Owner) is { Count: 1 } top)
        {
            contents = top;
        }

        // "You's graveyard" is what a possessive built from the name gives for the viewer's own zone.
        string whose = peek.Owner == client.Seat ? "Your" : $"{NameOf(client, peek.Owner)}'s";
        string zone = peek.Kind.ToString().ToLowerInvariant();

        if (peek.Kind is ZoneKindView.Hand or ZoneKindView.Library && contents.Count == 0)
        {
            FillKnown(client, peek.Kind, peek.Owner, $"{whose} {zone}");
            return;
        }

        // A library shows only what its owner may look at, as the top card under Bolas's Citadel: never the whole of it.
        int count = client.ZoneCount(peek.Kind, peek.Owner);
        string shown = peek.Kind == ZoneKindView.Library && contents.Count < count
            ? $"the top {(contents.Count == 1 ? "card" : $"{contents.Count} cards")} of {count}"
            : $"{contents.Count} card{(contents.Count == 1 ? string.Empty : "s")}";

        OpenChoices($"{whose} {zone}  ·  {shown}", closable: true, Math.Max(1, contents.Count));
        if (contents.Count == 0)
        {
            ShowChoicesEmpty("Nothing here.");
        }

        foreach (ObjectView card in contents)
        {
            AddChoice(card, LookOf(client, card));
        }
    }

    /// <summary>
    /// One player's share of exile, opened from the button over their piles. Each card held under a permanent says which,
    /// since here it is not drawn there. Once that share is empty the window closes itself.
    /// </summary>
    private void FillExile(GameClient client, int owner)
    {
        List<ObjectView> contents = ExileOf(client, owner);
        if (contents.Count == 0)
        {
            _peek = null;
            return;
        }

        string whose = owner == client.Seat ? "Your" : $"{NameOf(client, owner)}'s";
        OpenChoices($"{whose} exile  ·  {contents.Count} card{(contents.Count == 1 ? string.Empty : "s")}", closable: true, contents.Count);

        foreach (ObjectView card in contents)
        {
            CardLook look = LookOf(client, card);
            if (card.ExiledWith != ViewHandle.None && client.Object(card.ExiledWith) is { } holder)
            {
                look = look with { Annotation = look.Annotation is null ? $"Under {holder.Name}" : $"Under {holder.Name}\n{look.Annotation}" };
            }

            _looks[card.Handle] = look;
            AddChoice(card, look);
        }
    }

    /// <summary>
    /// The cards this player knows in a zone they cannot look into, opened from another player's hand or a library. A
    /// hand shows its known cards and a back for each of the rest; a library only its known cards, each marked with
    /// where it is. Once nothing is known the window closes itself.
    /// </summary>
    private void FillKnown(GameClient client, ZoneKindView kind, int owner, string title)
    {
        bool hand = kind == ZoneKindView.Hand;
        List<KnownCardView> known = hand
            ? [.. client.NamedIn(kind, owner).Select(card => new KnownCardView { FromTop = -1, Card = card })]
            : [.. client.KnownCards(kind, owner)];

        int count = client.ZoneCount(kind, owner);
        if (known.Count == 0)
        {
            _peek = null;
            return;
        }

        int backs = hand ? Math.Max(0, count - known.Count) : 0;
        OpenChoices($"{title}  ·  {count} card{(count == 1 ? string.Empty : "s")}, {known.Count} known", closable: true, known.Count + backs);

        // A card known in a library has no handle of its own, so each is drawn under a stand-in.
        for (int i = 0; i < known.Count; i++)
        {
            AddChoice(Standing(known[i].Card, -800_000 - i), new CardLook(Annotation: hand ? null : Position(known[i].FromTop, count)));
        }

        for (int i = 0; i < backs; i++)
        {
            AddChoice(new ObjectView { Handle = new ViewHandle(-700_000 - i), Known = false, Zone = kind }, Plain);
        }

        static string Position(int fromTop, int count)
        {
            int fromBottom = count - 1 - fromTop;

            return fromTop == 0 ? "Top"
                : fromBottom == 0 ? "Bottom"
                : fromBottom < fromTop ? $"{Ordinal(fromBottom + 1)} from bottom"
                : $"{Ordinal(fromTop + 1)} from top";
        }

        static string Ordinal(int n) => (n % 100) switch
        {
            11 or 12 or 13 => $"{n}th",
            _ => (n % 10) switch
            {
                1 => $"{n}st",
                2 => $"{n}nd",
                3 => $"{n}rd",
                _ => $"{n}th",
            },
        };
    }

    /// <summary>A card with no handle of its own, as one known in a library, drawn under a stand-in handle.</summary>
    private static ObjectView Standing(ObjectView card, int standIn)
    {
        if (!card.Handle.IsNone)
        {
            return card;
        }

        ObjectView copy = MessagePack.MessagePackSerializer.Deserialize<ObjectView>(MessagePack.MessagePackSerializer.Serialize(card));
        copy.Handle = new ViewHandle(standIn);
        return copy;
    }

    /// <summary>
    /// Closes an opponent's hand looked into once a click goes past it: it is read at a glance, unlike
    /// the other windows, which a player works out of. A click on the window, or on the fan that opened it, does not.
    /// </summary>
    private void ClosePeekedHand(Windows.Foundation.Point at)
    {
        if (_peek is not { Kind: ZoneKindView.Hand } || Contains(_choiceHost, at) || Contains(_opponentHalf.Rail.HandFan, at))
        {
            return;
        }

        _peek = null;
        _needsRedraw = true;
    }

    private bool Contains(FrameworkElement element, Windows.Foundation.Point at)
    {
        if (element.Visibility != Visibility.Visible || element.XamlRoot is null || element.ActualWidth <= 0)
        {
            return false;
        }

        Windows.Foundation.Rect box = element.TransformToVisual(Root).TransformBounds(new Windows.Foundation.Rect(0, 0, element.ActualWidth, element.ActualHeight));
        return box.Contains(at);
    }
}
