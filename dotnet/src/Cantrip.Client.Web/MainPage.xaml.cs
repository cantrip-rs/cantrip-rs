using Cantrip.Client;
using Cantrip.Client.Presentation;
using Cantrip.Protocol;
using Microsoft.UI.Text;

namespace Cantrip.Client.Web;

/// <summary>
/// The game table: the opponent's half over this player's, each a
/// rail beside a battlefield; the turn's steps as a bar under them; the hand and the prompt dock in a strip under
/// that; the game log down the right; and the stack floating over the table while anything is on it.
/// </summary>
public sealed partial class MainPage : Page
{
    /// <summary>As tall as the hand strip allows less its margins, as a printed card is for that height.</summary>
    private float HandCardWidth => MathF.Floor((_handHeight - (HandMarginY * 2f) - 2f) * 63f / 88f);
    private float HandCardHeight => MathF.Round(HandCardWidth * 88f / 63f);

    /// <summary>What the hand strip keeps clear above and below its cards.</summary>
    private const float HandMarginY = 12f;

    /// <summary>The hand strip's height, which the player sets by dragging its top edge.</summary>
    private float _handHeight = LocalSettings.Number(HandHeightKey, DefaultHandHeight);
    private float? _handDragFrom;
    private const string HandHeightKey = "cantrip-hand-height";
    private const float DefaultHandHeight = 168f;
    private const float MinHandHeight = 120f;
    private Grid _bottom = null!;
    /// <summary>The stack's card height until the player drags it otherwise.</summary>
    private const float DefaultStackHeight = 134;
    private const float MinStackHeight = 70;
    private const float MaxStackHeight = 600;

    /// <summary>What the stack window adds above and below its cards: its padding, title bar and gap.</summary>
    private const float StackChrome = 60;
    private const string StackHeightKey = "cantrip-stack-height";
    private const float CardGap = 8;

    private const double DockWidth = 300;

    private sealed record HalfViews(Grid Panel, PlayerRail Rail, ScrollViewer Scroll, CardBoardElement Board, Canvas Badges, bool Top, GraveyardDock Graveyard,
        Border Companion, CardBoardElement CompanionBoard);

    private GameClient? _gameClient;
    private MembershipMessage? _resuming;
    private MembershipMessage? _remembered;
    private bool _fixture;
    private string? _fixtureName;
    private FixtureTransport? _fixtureTransport;

    /// <summary>The server's log laid out under its headings.</summary>
    private GameJournal _journal = new();

    /// <summary>Whether each ability on the stack was triggered or activated, as its announcement said.</summary>
    private readonly Dictionary<ViewHandle, bool> _abilitiesTriggered = [];

    private HalfViews _opponentHalf = null!;
    private HalfViews _myHalf = null!;
    private ScrollViewer _handScroll = null!;
    private CardBoardElement _handBoard = null!;
    private TextBlock _handEmpty = null!;
    private PhaseTimelineControl _phaseTimeline = null!;
    private Grid _stackOverlay = null!;
    private TextBlock _stackTitle = null!;
    private CollapseButton _stackToggle = null!;
    private CardBoardElement _stackBoard = null!;
    private Grid _table = null!;
    private bool _stackCollapsed;
    private ResizeGrip[] _stackGrips = [];
    private float _stackHeight = LocalSettings.Number(StackHeightKey, DefaultStackHeight);
    private float? _stackDragFrom;
    private SymbolText _promptTitle = null!;
    private SymbolText _promptDetail = null!;
    private DockButton _undoButton = null!;
    private DockToggle _autoPassBox = null!;
    private DockToggle _passTurnBox = null!;
    private DockToggle _holdPriorityBox = null!;
    private readonly TurnStops _stops = new();
    private bool _autoPass;
    private bool _passTurn;
    private bool _holdPriority;

    /// <summary>The turn Pass Turn was ticked in; it clears itself once that turn is over.</summary>
    private int? _passTurnOn;

    /// <summary>What Resolve All is resolving, by handle, until it is all gone or something new joins it.</summary>
    private HashSet<ViewHandle>? _resolveAll;
    private LogPanelControl _log = null!;
    private SidePanel _side = null!;

    /// <summary>The side panel's slot, whose width slides between the panel's and nothing as it opens and closes.</summary>
    private Grid _sideSlot = null!;
    private Border _sideTab = null!;

    /// <summary>The side tab and the game menu's button beside it, in the table's top right corner while the panel is hidden.</summary>
    private StackPanel _sideCorner = null!;
    private DockButton _cornerMenu = null!;
    private bool _sideOpen = PlayerSettings.PanelOpen;
    private readonly DispatcherTimer _sideSlide = new() { Interval = TimeSpan.FromMilliseconds(8) };
    private double _sideFrom;
    private DateTime _sideStarted;

    /// <summary>The keys down now, for the hotkeys that act while held: hold priority and auto-mana.</summary>
    private readonly HashSet<Windows.System.VirtualKey> _keysDown = [];
    private TargetArrowsElement _arrows = null!;
    private readonly LobbyPanel _lobby = new();
    private readonly DeckLoadingPanel _deckLoading = new();
    private readonly CardInspector _inspector = new();
    private readonly CardFlights _flights = new();
    private readonly CardTooltip _tooltip = new();

    /// <summary>Set when something the board or dock shows has changed, and cleared by the redraw that shows it.</summary>
    private bool _needsRedraw = true;

    /// <summary>The prompt the last redraw was for; a new one clears whatever was half chosen for the old.</summary>
    private int _lastPromptId = -1;

    /// <summary>
    /// Whether the last redraw drew a prompt, and since when a redraw has been waiting for the next one:
    /// answering clears the prompt at once and the next takes a moment across the socket; drawn in between, every action flickers.
    /// </summary>
    private bool _promptDrawn;
    private DateTime? _promptAwaitedSince;
    private static readonly TimeSpan PromptGrace = TimeSpan.FromSeconds(0.15);

    public MainPage()
    {
        this.InitializeComponent();
        BuildUi();
        AttachPhone();

        // Not a reimplementation of the wire protocol - this is Cantrip.Client/Cantrip.Protocol
        // themselves, compiled into this browser-wasm build and talking to a real Cantrip.Server.Host
        // over a real WebSocket.
        _ = ClientConfig.Decks ? OpenDeckEditorAsync() : ConnectToServerAsync();
        StartTestState();
    }

    private void BuildUi()
    {
        Root.Background = Theme.Brush(Theme.Window);
        Root.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        Root.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        Root.RowDefinitions.Add(new RowDefinition { Height = new GridLength(1, GridUnitType.Star) });
        Root.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
        Root.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });

        Grid table = BuildTable();
        Root.Children.Add(table);

        _phaseTimeline = new PhaseTimelineControl(_stops);
        Grid.SetRow(_phaseTimeline, 1);
        Root.Children.Add(_phaseTimeline);

        Grid bottom = BuildBottom();
        Grid.SetRow(bottom, 2);
        Root.Children.Add(bottom);

        // Straddling the top edge of the hand's strip, across its whole width, and dragged to make the hand taller or
        // shorter. It does not light up: the pointer crosses it all the time.
        var handGrip = new ResizeGrip("Drag to resize the hand", lights: false)
        {
            VerticalAlignment = VerticalAlignment.Top,
            Margin = new Thickness(0, -ResizeGrip.Thickness / 2, 0, 0),
        };
        handGrip.Started += () => _handDragFrom = _handHeight;
        handGrip.Moved += down => SetHandHeight((_handDragFrom ?? _handHeight) - (float)down);
        handGrip.Ended += () =>
        {
            _handDragFrom = null;
            LocalSettings.Store(HandHeightKey, _handHeight);
        };
        Grid.SetRow(handGrip, 2);
        Root.Children.Add(handGrip);
        Root.SizeChanged += (_, _) => SetHandHeight(_handDragFrom is null ? LocalSettings.Number(HandHeightKey, DefaultHandHeight) : _handHeight);

        // The log, the settings and the game's menu as tabs of one panel down the right, which slides away and back.
        _log = new LogPanelControl();
        _side = new SidePanel(_log, new SettingsPanel(), new GameMenuPanel());
        _side.Menu.ConcedeRequested += () => _gameClient?.Concede();
        _side.Menu.DrawRequested += OfferOrAcceptDraw;
        _side.MenuRequested += () => OpenGameMenu(_side.MenuButton);
        _side.Menu.ReturnRequested += GoBack;
        _side.HideRequested += () => SetSideOpen(false);
        _sideSlot = new Grid { Width = _sideOpen ? SidePanel.PanelWidth : 0, Children = { _side } };
        _sideSlot.SizeChanged += (_, e) => _sideSlot.Clip = new Microsoft.UI.Xaml.Media.RectangleGeometry
        {
            Rect = new Windows.Foundation.Rect(0, 0, e.NewSize.Width, e.NewSize.Height),
        };
        _sideSlide.Tick += (_, _) => SlideSide();
        Grid.SetColumn(_sideSlot, 1);
        Grid.SetRowSpan(_sideSlot, 3);
        Root.Children.Add(_sideSlot);

        // Over everything, stack window included, so the arrows are added last. The tail starts faint, so
        // over the spell's own card it barely shows.
        _arrows = new TargetArrowsElement();
        Grid.SetRowSpan(_arrows, 3);
        Grid.SetColumnSpan(_arrows, 2);
        Root.Children.Add(_arrows);

        // The creatures picked up to attack or block, with an arrow from each, and the rectangle dragged to pick them.
        BuildSelection();

        // What each arrow's spell does to its target, in words over the arrow's head.
        Grid.SetRowSpan(_arrowLabels, 3);
        Grid.SetColumnSpan(_arrowLabels, 2);
        Root.Children.Add(_arrowLabels);

        // Each ability as it resolves, over the table and its arrows like them.
        BuildResolutions();

        // The outcome over the table and its arrows, under a card held up to be read.
        Grid outcome = BuildOutcomeOverlay();
        Grid.SetRowSpan(outcome, 3);
        Grid.SetColumnSpan(outcome, 2);
        Root.Children.Add(outcome);

        // An event's draft and deck building, over the table, and under a card held up to be read from them.
        BuildEventPanel();
        Grid.SetRowSpan(_eventPanel, 3);
        Grid.SetColumnSpan(_eventPanel, 2);
        Root.Children.Add(_eventPanel);

        // An event's pairings and standings between its rounds, over the table, as the draft is.
        BuildPairingsPanel();
        Grid.SetRowSpan(_pairingsPanel, 3);
        Grid.SetColumnSpan(_pairingsPanel, 2);
        Root.Children.Add(_pairingsPanel);

        // The constructed deck editor, for a page opened for it (?decks) rather than for a table.
        if (ClientConfig.Decks)
        {
            BuildDeckEditor();
            Grid.SetRowSpan(_deckPanel, 3);
            Grid.SetColumnSpan(_deckPanel, 2);
            Root.Children.Add(_deckPanel);
        }

        // The advisor's panel, for a page at the advisor's table (?advisor=) and no other (MainPage.Advisor.cs).
        if (ClientConfig.Advisor is not null)
        {
            BuildAdvisor();
        }

        // A card held up to be read, a card's tooltip, and a card being carried, over everything but the lobby.
        foreach (FrameworkElement layer in (FrameworkElement[])[_flights, _inspector, _tooltip, _carryLayer])
        {
            Grid.SetRowSpan(layer, 3);
            Grid.SetColumnSpan(layer, 2);
            Root.Children.Add(layer);
        }

        WireCardBoards();

        // Over the whole window while the table is being set up.
        Grid.SetRowSpan(_lobby, 3);
        Grid.SetColumnSpan(_lobby, 2);
        Root.Children.Add(_lobby);

        // Over the whole window, and over the lobby, while this player's deck is got ready at the start of a game.
        Grid.SetRowSpan(_deckLoading, 3);
        Grid.SetColumnSpan(_deckLoading, 2);
        Root.Children.Add(_deckLoading);

        foreach (CardBoardElement board in (CardBoardElement[])[_opponentHalf.Board, _myHalf.Board, _handBoard, _stackBoard, _opponentHalf.Graveyard.Board, _myHalf.Graveyard.Board, _opponentHalf.CompanionBoard, _myHalf.CompanionBoard])
        {
            board.CardClicked += handle =>
            {
                NotePress(board);
                OnCardClicked(handle);
            };
        }
        _stackBoard.HoverChanged += () =>
        {
            if (_gameClient is { } client)
            {
                LayOutStack(client, _stackItems);
            }

            PaintTargetArrows(_gameClient);
        };
        foreach (GraveyardDock dock in (GraveyardDock[])[_opponentHalf.Graveyard, _myHalf.Graveyard])
        {
            dock.Board.HoverChanged += () =>
            {
                if (_gameClient is { } client)
                {
                    LayOutGraveyard(client, dock.Seat, dock);
                }
            };
            dock.Close.Clicked += () =>
            {
                SetGraveyardsOpen(false);
                RedrawNow();
            };
        }

        _opponentHalf.Board.DamageStepped += OnShareStepped;
        _myHalf.Board.DamageStepped += OnShareStepped;
        // A permanent under the pointer that chose another as it entered points at it (PaintTargetArrows).
        _opponentHalf.Board.HoverChanged += () => PaintTargetArrows(_gameClient);
        _myHalf.Board.HoverChanged += () => PaintTargetArrows(_gameClient);
        _handBoard.HoverChanged += () =>
        {
            if (_gameClient is { } client)
            {
                LayOutHand(client);
            }
        };

        // The hand is dragged along itself into whatever order its player likes; the server never hears of it.
        MakeDraggable(_handBoard);
        _dropZones.Add(new DropZone(
            _handBoard,
            handle => _hand.Any(card => card.Handle == handle),
            (handle, index) =>
            {
                List<ViewHandle> shown = [.. _hand.Select(card => card.Handle)];
                Place(shown, handle, index);
                _handOrder.RemoveAll(shown.Contains);
                _handOrder.InsertRange(0, shown);
            }));

        // Where a card's menu opens, and whether the Auto-mana key was held for the click that opened it. Handled events
        // too, since the boards take their own presses.
        Root.AddHandler(PointerPressedEvent, new Microsoft.UI.Xaml.Input.PointerEventHandler((_, e) =>
        {
            _pointer = e.GetCurrentPoint(Root).Position;
            _autoManaHeld = Held(Hotkey.AutoMana, e.KeyModifiers);

            if (!e.GetCurrentPoint(Root).Properties.IsRightButtonPressed)
            {
                ClosePeekedHand(_pointer);
            }
        }), handledEventsToo: true);

        // Keys as the browser window hears them, wherever the focus is (KeyboardBridge), read every frame.
        var keys = new DispatcherTimer { Interval = TimeSpan.FromMilliseconds(16) };
        keys.Tick += (_, _) => ReadKeys();
        keys.Start();
        Root.SizeChanged += (_, _) => _needsRedraw = true;

        // A settings change - the card size, auto-mana, a key - is drawn at once.
        PlayerSettings.Changed += () =>
        {
            CardBoardElement.Animates = PlayerSettings.AnimateCards;
            _flights.Enabled = PlayerSettings.AnimateCards;
            _resolutions.Enabled = PlayerSettings.AnimateCards;
            _opponentHalf.Board.Slides = PlayerSettings.AnimateCards;
            _myHalf.Board.Slides = PlayerSettings.AnimateCards;
            SettleCards();
            DescribeKeys();
            _needsRedraw = true;
            RedrawNow();
        };
        CardBoardElement.Animates = PlayerSettings.AnimateCards;
        _flights.Enabled = PlayerSettings.AnimateCards;
        _resolutions.Enabled = PlayerSettings.AnimateCards;

        BuildSideTab();
        SetSideOpen(_sideOpen, animate: false);
    }

    /// <summary>
    /// Takes where a card's press landed, and whether the Auto-mana key was held for it, from the board it landed on:
    /// the board raises the click before the page's own handler hears the press, so that would still be the last one.
    /// </summary>
    private void NotePress(CardBoardElement board)
    {
        if (board.XamlRoot is not null)
        {
            _pointer = board.TransformToVisual(Root).TransformPoint(board.LastPress);
        }

        _autoManaHeld = Held(Hotkey.AutoMana, board.LastModifiers);
    }

    /// <summary>Every row of cards on the page, each with the looks its clicks and notes come from.</summary>
    private IEnumerable<(CardBoardElement Board, Func<ViewHandle, CardLook?> Look)> CardBoards() =>
    [
        (_opponentHalf.Board, LookOn),
        (_myHalf.Board, LookOn),
        (_handBoard, LookOn),
        (_stackBoard, LookOn),
        (_opponentHalf.Graveyard.Board, LookOn),
        (_myHalf.Graveyard.Board, LookOn),
        (_opponentHalf.CompanionBoard, LookOn),
        (_myHalf.CompanionBoard, LookOn),
        (_opponentHalf.Rail.LibraryBoard, _ => new CardLook(Note: _opponentHalf.Rail.LibraryNote)),
        (_opponentHalf.Rail.GraveyardBoard, _ => new CardLook(Note: _opponentHalf.Rail.GraveyardNote)),
        (_myHalf.Rail.LibraryBoard, _ => new CardLook(Note: _myHalf.Rail.LibraryNote)),
        (_myHalf.Rail.GraveyardBoard, _ => new CardLook(Note: _myHalf.Rail.GraveyardNote)),
        (_choiceBoard, handle => _choiceLooks.GetValueOrDefault(handle)),
        (_choicePickedBoard, handle => _pickedLooks.GetValueOrDefault(handle)),
        .. _promptBoards.Select(board => (board, (Func<ViewHandle, CardLook?>)(_ => null))),
        .. EventBoards(),
    ];

    private CardLook? LookOn(ViewHandle handle) => _looks.GetValueOrDefault(handle);

    /// <summary>
    /// Puts every card laid out over the next moment where it belongs, neither flown nor slid there: the page laid out
    /// again, not the game moving anything.
    /// </summary>
    private void SettleCards()
    {
        _flights.Settle();
        foreach ((CardBoardElement board, _) in CardBoards())
        {
            board.Settle();
        }
    }

    /// <summary>
    /// Right-press to hold a card up and read it, let go anywhere to put it back; and a
    /// tooltip for the card the pointer rests on, with whatever note the board has added to it.
    /// </summary>
    private void WireCardBoards()
    {
        foreach ((CardBoardElement board, _) in CardBoards())
        {
            MakeInspectable(board);
        }

        // A card known in an opponent's hand is read the same way, from the fan in their rail.
        if (_opponentHalf.Rail.HandFan is HandFanElement fan)
        {
            fan.CardInspected += (card, box, angle) =>
            {
                if (fan.XamlRoot is not null)
                {
                    _tooltip.Hide();
                    _inspector.Inspect(card, fan.TransformToVisual(_inspector).TransformBounds(box), angle);
                }
            };
        }

        _flights.FlyingChanged += () =>
        {
            foreach ((CardBoardElement board, _) in CardBoards())
            {
                board.Hidden = _flights.Flying;
            }

            // The cards gathered into a stack being dragged show in it as they land there.
            if (_carried is { } carried)
            {
                carried.Hidden = _flights.Flying;
            }
        };

        // A change of layout - the window, the panel sliding, the hand's strip - puts cards where they are going.
        _table.SizeChanged += (_, _) => SettleCards();

        _inspector.HeldChanged += () =>
        {
            foreach ((CardBoardElement board, _) in CardBoards())
            {
                board.Concealed = _inspector.Concealed;
            }
        };

        Root.AddHandler(PointerReleasedEvent, new Microsoft.UI.Xaml.Input.PointerEventHandler((_, e) =>
        {
            if (e.GetCurrentPoint(Root).Properties.PointerUpdateKind == Microsoft.UI.Input.PointerUpdateKind.RightButtonReleased)
            {
                _inspector.Release();
            }
        }), handledEventsToo: true);

        Root.AddHandler(PointerMovedEvent, new Microsoft.UI.Xaml.Input.PointerEventHandler((_, e) =>
        {
            Windows.Foundation.Point at = e.GetCurrentPoint(Root).Position;

            // A card held up is already the whole face, large; a box of the same words over it is in the way.
            if (_inspector.Held is not null)
            {
                _tooltip.Hide();
                return;
            }

            // A card named in a line of text - the game log, a draft's picks - reads as the card does.
            if (MentionUnder(e.OriginalSource as DependencyObject) is { } mention)
            {
                foreach ((CardBoardElement row, Func<ViewHandle, CardLook?> _) in CardBoards())
                {
                    row.Unhover();
                }

                _tooltip.Rest(CardTooltip.TextFor(mention.Card, null), at);
                return;
            }

            // Only the row the pointer is over speaks. Another can still think a card is under the pointer when it never
            // heard the pointer leave - the game's hand, covered by the deck builder while the pointer rested on a
            // card - and would otherwise show that card's tooltip wherever the pointer went.
            CardBoardElement? under = BoardUnder(e.OriginalSource as DependencyObject);
            (string Text, Windows.Foundation.Point At)? rest = null;

            foreach ((CardBoardElement board, Func<ViewHandle, CardLook?> look) in CardBoards())
            {
                if (board != under)
                {
                    board.Unhover();
                }
                else if (board.Hovered is { } handle && board.CardOf(handle) is { } card)
                {
                    rest = (board.OverEye() ? EyeTooltip : CardTooltip.TextFor(card, look(handle)?.Note), at);
                }
            }

            if (rest is { } shown)
            {
                _tooltip.Rest(shown.Text, shown.At);
            }
            else
            {
                _tooltip.Hide();
            }
        }), handledEventsToo: true);

        Root.AddHandler(PointerPressedEvent, new Microsoft.UI.Xaml.Input.PointerEventHandler((_, e) =>
        {
            _tooltip.Hide();

            // Right-press on a card's name holds the card up from the name, as it would from the card.
            if (e.GetCurrentPoint(Root).Properties.IsRightButtonPressed
                && MentionUnder(e.OriginalSource as DependencyObject) is { } mention
                && mention.Name.XamlRoot is not null)
            {
                Windows.Foundation.Rect box = mention.Name.TransformToVisual(_inspector).TransformBounds(new Windows.Foundation.Rect(0, 0, mention.Name.ActualWidth, mention.Name.ActualHeight));
                _inspector.InspectName(mention.Card, box);
            }
        }), handledEventsToo: true);
    }

    /// <summary>Right-press on one of a row's cards holds it up; wired once per row.</summary>
    private void MakeInspectable(CardBoardElement board) =>
        board.CardInspected += (handle, box, angle) =>
        {
            if (board.CardOf(handle) is { } card && board.XamlRoot is not null)
            {
                _tooltip.Hide();
                _inspector.Inspect(card, board.TransformToVisual(_inspector).TransformBounds(box), angle);
            }
        };

    /// <summary>A row the dock's prompt made: read and held up like the page's own rows until the dock is redrawn.</summary>
    private CardBoardElement PromptBoard(CardBoardElement board)
    {
        MakeInspectable(board);
        board.Concealed = _inspector.Concealed;
        _promptBoards.Add(board);
        return board;
    }

    /// <summary>
    /// The card named by the words a pointer event came from, if they name one (<see cref="CardMention"/>), with the
    /// words themselves: the card as it is now while this seat can see it, and as the line was written otherwise.
    /// </summary>
    private (ObjectView Card, FrameworkElement Name)? MentionUnder(DependencyObject? source)
    {
        for (DependencyObject? at = source; at is not null; at = VisualTreeHelper.GetParent(at))
        {
            if (at is TextBlock { Tag: CardMention { Card: { } named } } name)
            {
                ObjectView? card = !named.Object.IsNone && _gameClient?.Object(named.Object) is { Known: true } now ? now : named.Card;
                return card is { Known: true } ? (card, name) : null;
            }

            if (at is CardBoardElement or SymbolText)
            {
                return null;
            }
        }

        return null;
    }

    /// <summary>The row of cards a pointer event came from, if any: the element itself or the nearest one around it.</summary>
    private static CardBoardElement? BoardUnder(DependencyObject? source)
    {
        for (DependencyObject? at = source; at is not null; at = VisualTreeHelper.GetParent(at))
        {
            if (at is CardBoardElement board)
            {
                return board;
            }
        }

        return null;
    }

    /// <summary>What the eye on a card says when the pointer rests on it.</summary>
    private const string EyeTooltip = "Revealed to an opponent\nThis card was revealed to your opponent, who knows it is here.";

    /// <summary>
    /// The page's keys: each hotkey as the settings bind it, and L for the side panel. A
    /// settings row waiting for a key takes the press first, so the key chosen is never also the key that acts; and a
    /// key typed into a text field - a card name - is the field's.
    /// </summary>
    private void ReadKeys()
    {
        foreach (KeyPress? heard in KeyboardBridge.Take())
        {
            if (heard is not { } press)
            {
                // The window lost focus: whatever was held is no longer known to be.
                _keysDown.Clear();
                continue;
            }

            if (!press.Down)
            {
                _keysDown.Remove(KeyBinding.Plain(press.Key));
                continue;
            }

            if (!press.Typing)
            {
                _keysDown.Add(KeyBinding.Plain(press.Key));
            }

            OnKey(press);
        }

        FollowHoldPriorityKey();
    }

    private void OnKey(KeyPress press)
    {
        Windows.System.VirtualKey key = KeyBinding.Plain(press.Key);
        (bool ctrl, bool shift, bool alt) = (press.Ctrl, press.Shift, press.Alt);

        if (_side.Settings.Capturing is { } row)
        {
            if (!press.Repeat)
            {
                row.Take(press.Key, ctrl, shift, alt);
            }

            return;
        }

        if (press.Repeat || press.Typing)
        {
            return;
        }

        var pressed = new KeyBinding(
            key,
            ctrl && key != Windows.System.VirtualKey.Control,
            shift && key != Windows.System.VirtualKey.Shift,
            alt && key != Windows.System.VirtualKey.Menu);

        if (pressed == Hotkeys.TogglePanel)
        {
            if (!_sideOpen)
            {
                _side.Select(SidePanel.Tab.Log);
            }

            SetSideOpen(!_sideOpen);
            return;
        }

        Hotkey? bound = Hotkeys.All.Where(hotkey => !Hotkeys.IsHeld(hotkey) && PlayerSettings.KeyFor(hotkey) == pressed)
            .Select(hotkey => (Hotkey?)hotkey)
            .FirstOrDefault();

        Action? act = bound switch
        {
            Hotkey.Pass => _pass,
            Hotkey.Cancel => _cancel,
            Hotkey.AttackAll => _attackAll,
            Hotkey.Undo => OnUndoClick,

            // Clicks the box, so the key and the mouse go through the same Toggled handler.
            Hotkey.PassTurn when _gameClient is not null => _passTurnBox.Flip,
            Hotkey.AutoPass when _gameClient is not null => _autoPassBox.Flip,
            _ => null,
        };

        if (act is not null)
        {
            act();
            RedrawNow();
        }
    }

    /// <summary>
    /// Whether a hotkey that acts while held is down now: its key among those held, or, for a bare modifier, the
    /// modifier a pointer event carried, which is known even when the key went down before the page had focus.
    /// </summary>
    private bool Held(Hotkey hotkey, Windows.System.VirtualKeyModifiers? pointer = null)
    {
        KeyBinding key = PlayerSettings.KeyFor(hotkey);
        bool fromPointer = pointer is { } modifiers && key.Key switch
        {
            Windows.System.VirtualKey.Shift => modifiers.HasFlag(Windows.System.VirtualKeyModifiers.Shift),
            Windows.System.VirtualKey.Control => modifiers.HasFlag(Windows.System.VirtualKeyModifiers.Control),
            Windows.System.VirtualKey.Menu => modifiers.HasFlag(Windows.System.VirtualKeyModifiers.Menu),
            _ => false,
        };

        return fromPointer || _keysDown.Contains(key.Key);
    }

    /// <summary>
    /// The tooltips that end with a key - the dock's toggles and Undo - written again when a key is rebound.
    /// </summary>
    private void DescribeKeys()
    {
        _autoPassBox.Tooltip =
            "Pass priority for you whenever there is nothing you could do but tap for mana: no land to play,\n" +
            "and no spell or ability you could pay for. Steps you have set a stop in still stop you." +
            Hotkeys.Suffix(PlayerSettings.KeyFor(Hotkey.AutoPass));

        _passTurnBox.Tooltip =
            "Pass priority at every chance until this turn ends, stops and your opponent's spells included,\n" +
            "and declare no attackers. You are still asked to declare blockers when attacked, and to make any\n" +
            "choice that cannot be passed. Clears itself when the next turn begins." +
            Hotkeys.Suffix(PlayerSettings.KeyFor(Hotkey.PassTurn));

        _holdPriorityBox.Tooltip =
            "Keep priority for as long as a spell or ability of yours is on top of the stack, to act again before your\n" +
            "opponent may respond, and again each time something of theirs resolves off yours. Without it those windows\n" +
            "are passed for you, but you are asked the moment your opponent puts something on top of yours.\n" +
            "Holding the key does the same while it is down." +
            Hotkeys.Suffix(PlayerSettings.KeyFor(Hotkey.HoldPriority));

        _undoButton.Tooltip = "Take back the last mana ability" + Hotkeys.Suffix(PlayerSettings.KeyFor(Hotkey.Undo));
    }

    /// <summary>
    /// Ticks the Hold Priority box while its key is held and clears it when the key is let go, so the box always says
    /// what will happen when a spell is cast. Only on a change, so a click on the box sticks while the key is up.
    /// </summary>
    private void FollowHoldPriorityKey()
    {
        bool down = _side.Settings.Capturing is null && Held(Hotkey.HoldPriority);
        if (down == _holdKeyDown)
        {
            return;
        }

        _holdKeyDown = down;
        _holdPriority = down;
        _holdPriorityBox.SetOn(down);
        _needsRedraw = true;
    }

    private bool _holdKeyDown;

    /// <summary>
    /// Whether this seat has a game to concede: one being played, that it is still in, where everyone else still in is
    /// of one team - the only concession the server takes, since it ends the game.
    /// </summary>
    private static bool CanConcede(GameClient client)
    {
        if (client.View is not { } view || client.IsOver || !client.IsConnected || client.Table is { Phase: not TablePhase.Playing })
        {
            return false;
        }

        if (view.Players.FirstOrDefault(player => player.Seat == client.Seat) is not { Status: PlayerStatusView.Active })
        {
            return false;
        }

        return view.Players.Where(player => player.Seat != client.Seat && player.Status == PlayerStatusView.Active)
            .Select(player => player.Team).Distinct().Count() == 1;
    }

    /// <summary>
    /// Slides the side panel in or out: its slot's width eases between the panel's and nothing, cubic ease-out over a
    /// little over a quarter of a second, so it arrives quickly and settles.
    /// </summary>
    private void SetSideOpen(bool open, bool animate = true)
    {
        _sideOpen = open;
        PlayerSettings.PanelOpen = open;
        _sideCorner.Visibility = open ? Visibility.Collapsed : Visibility.Visible;

        if (!open)
        {
            _side.Settings.StopCapturing();
        }

        if (!animate)
        {
            _sideSlide.Stop();
            _sideSlot.Width = open ? SidePanel.PanelWidth : 0;
            return;
        }

        _sideFrom = _sideSlot.Width;
        _sideStarted = DateTime.UtcNow;
        _sideSlide.Start();
    }

    private void SlideSide()
    {
        double t = Math.Min(1, (DateTime.UtcNow - _sideStarted).TotalSeconds / 0.28);
        double eased = 1 - Math.Pow(1 - t, 3);
        double to = _sideOpen ? SidePanel.PanelWidth : 0;
        _sideSlot.Width = _sideFrom + ((to - _sideFrom) * eased);

        if (t >= 1)
        {
            _sideSlide.Stop();
        }
    }

    /// <summary>
    /// The way back to the side panel once it is hidden: an arrow in the table's top right corner, pointing the way the
    /// panel comes from. It reopens whichever tab was showing.
    /// </summary>
    private void BuildSideTab()
    {
        var arrow = new TextBlock { Text = "‹", FontSize = 18, Foreground = Theme.Brush(Theme.FgSoft), VerticalAlignment = VerticalAlignment.Center };
        _sideTab = new Border
        {
            Child = arrow,
            Padding = new Thickness(11, 0, 11, 2),
            CornerRadius = new CornerRadius(6),
            BorderThickness = new Thickness(1),
        };

        void Paint(bool hovered, bool pressed)
        {
            _sideTab.Background = Theme.Brush(pressed ? Theme.ControlPressed : hovered ? Theme.ControlHover : Theme.WithAlpha(Theme.Panel, 0.9));
            _sideTab.BorderBrush = Theme.Brush(hovered || pressed ? Theme.ControlBorder : Theme.Line);
            arrow.Foreground = Theme.Brush(hovered ? Theme.Fg : Theme.FgSoft);
        }

        Paint(false, false);
        ToolTipService.SetToolTip(_sideTab, "Show the log and settings" + Hotkeys.Suffix(Hotkeys.TogglePanel));
        _sideTab.PointerEntered += (_, _) => Paint(true, false);
        _sideTab.PointerExited += (_, _) => Paint(false, false);
        _sideTab.PointerPressed += (_, _) => Paint(true, true);
        _sideTab.Tapped += (_, e) =>
        {
            e.Handled = true;
            SetSideOpen(true);
        };

        // The game menu beside it, as the panel's own header has it, so the menu is a click away with the panel hidden too.
        _cornerMenu = GameMenuButton(quiet: false);
        _cornerMenu.Clicked += () => OpenGameMenu(_cornerMenu);
        _sideCorner = new StackPanel
        {
            Orientation = Orientation.Horizontal,
            Spacing = 6,
            HorizontalAlignment = HorizontalAlignment.Right,
            VerticalAlignment = VerticalAlignment.Top,
            Margin = new Thickness(0, 10, 12, 0),
            Visibility = Visibility.Collapsed,
            Children = { _cornerMenu, _sideTab },
        };

        Grid.SetRowSpan(_sideCorner, 3);
        _table.Children.Add(_sideCorner);
    }

    /// <summary>The two halves split by a hairline, sharing the height between them, with the stack floating over both.</summary>
    private Grid BuildTable()
    {
        var table = new Grid();
        _table = table;
        table.RowDefinitions.Add(new RowDefinition { Height = new GridLength(1, GridUnitType.Star) });
        table.RowDefinitions.Add(new RowDefinition { Height = new GridLength(1) });
        table.RowDefinitions.Add(new RowDefinition { Height = new GridLength(1, GridUnitType.Star) });

        _opponentHalf = BuildHalf(showsHandFan: true, top: true);
        table.Children.Add(_opponentHalf.Panel);
        _opponentHalf.Rail.MenuAsked += e => PlayerMenu(_opponentHalf.Rail, e);

        var rule = new Border { Background = Theme.Brush(Theme.Line) };
        Grid.SetRow(rule, 1);
        table.Children.Add(rule);

        _myHalf = BuildHalf(showsHandFan: false, top: false);
        Grid.SetRow(_myHalf.Panel, 2);
        table.Children.Add(_myHalf.Panel);

        // Bottom left of this player's board, for as long as the server offers to take back a mana ability: each press
        // takes back one.
        _undoButton = new DockButton("↺  Undo", primary: false, fontSize: 12)
        {
            HorizontalAlignment = HorizontalAlignment.Left,
            VerticalAlignment = VerticalAlignment.Bottom,
            Margin = new Thickness(18, 0, 0, 14),
            Visibility = Visibility.Collapsed,
        };

        _undoButton.Clicked += OnUndoClick;
        Grid.SetColumn(_undoButton, 2);
        _myHalf.Panel.Children.Add(_undoButton);

        _stackOverlay = BuildStackOverlay();
        Grid.SetRowSpan(_stackOverlay, 3);
        table.Children.Add(_stackOverlay);

        _choiceHost = BuildChoiceOverlay();
        Grid.SetRowSpan(_choiceHost, 3);
        table.Children.Add(_choiceHost);

        BuildGraveyardGrip();
        BuildToast();
        return table;
    }

    private HalfViews BuildHalf(bool showsHandFan, bool top)
    {
        var half = new Grid { Background = Theme.Brush(Theme.HalfIdle) };
        half.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(PlayerRail.RailWidth) });
        half.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        half.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });

        var rail = new PlayerRail(showsHandFan);
        half.Children.Add(rail);

        // The graveyard dock, between the rail and the board, built for both halves and shown together, so it splits on
        // the same line the rails and the board do.
        GraveyardDock graveyard = BuildGraveyardDock(top);
        Grid.SetColumn(graveyard.Panel, 1);
        half.Children.Add(graveyard.Panel);

        // Its cards slide along it as the game moves them - tapped into a pile, making room for one arriving - while a
        // card that comes from another zone flies there (CardFlights).
        var board = new CardBoardElement(BoardRows.StandardCardWidth, BoardRows.CardHeight(BoardRows.StandardCardWidth)) { Slides = PlayerSettings.AnimateCards };

        // The piles' counts, over the board in the same scrolling content so they move with it.
        var badges = new Canvas { IsHitTestVisible = false };
        var content = new Grid();
        content.Children.Add(board);

        // The marks of creatures just arrived, on their own layer over the row, so they rise and fall without the row
        // being drawn again for them.
        content.Children.Add(new ArrivalsElement(board));
        content.Children.Add(badges);

        // Across only: the rows are fitted to the half's height, or clipped at it. The wheel scrolls it across too.
        var scroll = Scroller(content);
        ScrollAcrossWithWheel(scroll);
        Grid.SetColumn(scroll, 2);
        half.Children.Add(scroll);

        // The companion this player revealed before the game (CR 103.2b), waiting outside the game: a small window over the
        // corner of their battlefield nearest the rail - the bottom left of the player's own, the top left of an opponent's.
        // Its card is clicked to take it (CR 116.2g). Shown only while there is one.
        var companionBoard = new CardBoardElement(CompanionCardWidth, BoardRows.CardHeight(CompanionCardWidth)) { Slides = false };
        companionBoard.SetCellSize(CompanionCardWidth, BoardRows.CardHeight(CompanionCardWidth));
        companionBoard.Width = CompanionCardWidth;
        companionBoard.Height = BoardRows.CardHeight(CompanionCardWidth);
        var companionContent = new StackPanel { Spacing = 4 };
        companionContent.Children.Add(new TextBlock { Text = "Companion", FontSize = 12, FontWeight = Microsoft.UI.Text.FontWeights.SemiBold, Foreground = Theme.Brush(Theme.FgSoft) });
        companionContent.Children.Add(companionBoard);
        var companion = new Border
        {
            Background = Theme.Brush(Theme.Panel),
            BorderBrush = Theme.Brush(Theme.Line),
            BorderThickness = new Thickness(1),
            CornerRadius = new CornerRadius(6),
            Padding = new Thickness(6),
            Margin = new Thickness(8),
            HorizontalAlignment = HorizontalAlignment.Left,
            VerticalAlignment = top ? VerticalAlignment.Top : VerticalAlignment.Bottom,
            Visibility = Visibility.Collapsed,
            Child = companionContent,
        };
        Microsoft.UI.Xaml.Automation.AutomationProperties.SetName(companion, top ? "Opponent's companion" : "Your companion");
        Grid.SetColumn(companion, 2);
        half.Children.Add(companion);

        return new HalfViews(half, rail, scroll, board, badges, top, graveyard, companion, companionBoard);
    }

    private static ScrollViewer Scroller(FrameworkElement board) => new()
    {
        HorizontalScrollMode = ScrollMode.Auto,
        HorizontalScrollBarVisibility = ScrollBarVisibility.Auto,
        VerticalScrollMode = ScrollMode.Disabled,
        VerticalScrollBarVisibility = ScrollBarVisibility.Disabled,
        Content = board,
    };

    /// <summary>
    /// Turns the mouse wheel over a strip that only scrolls across into scrolling it across: down the wheel is right
    /// along the strip. A wheel or pad that scrolls sideways already does so as it is.
    /// </summary>
    private static void ScrollAcrossWithWheel(ScrollViewer scroll)
    {
        scroll.AddHandler(PointerWheelChangedEvent, new Microsoft.UI.Xaml.Input.PointerEventHandler((_, e) =>
        {
            Microsoft.UI.Input.PointerPointProperties wheel = e.GetCurrentPoint(scroll).Properties;
            if (wheel.IsHorizontalMouseWheel || wheel.MouseWheelDelta == 0 || scroll.ScrollableWidth <= 0)
            {
                return;
            }

            double to = Math.Clamp(scroll.HorizontalOffset - wheel.MouseWheelDelta, 0, scroll.ScrollableWidth);
            scroll.ChangeView(to, null, null, disableAnimation: true);
            e.Handled = true;
        }), handledEventsToo: true);
    }

    /// <summary>The strip under the table: the hand, centred, and the dock that says what is being asked.</summary>
    private Border _dock = null!;

    /// <summary>
    /// The dock at its own width, or wider when a button asks for more: the button flow is as wide as its widest
    /// button, and the dock as wide as the flow, with the hand giving up the room.
    /// </summary>
    private void SizeDock()
    {
        double widest = 0;
        foreach (UIElement child in _dockButtons.Children)
        {
            child.Measure(new Windows.Foundation.Size(double.PositiveInfinity, double.PositiveInfinity));
            widest = Math.Max(widest, child.DesiredSize.Width);
        }

        // The dock's padding either side and its hairline on the left.
        _dock.Width = Math.Max(DockWidth, Math.Ceiling(widest) + 32 + 1);
    }

    private Grid BuildBottom()
    {
        // As tall as the hand strip, and taller while the dock needs it.
        var bottom = _bottom = new Grid { MinHeight = _handHeight, Background = Theme.Brush(Theme.Bar), BorderBrush = Theme.Brush(Theme.Line), BorderThickness = new Thickness(0, 1, 0, 0) };
        bottom.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        bottom.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });

        var hand = new Grid();
        _handBoard = new CardBoardElement(HandCardWidth, HandCardHeight) { Slides = true };
        _handScroll = Scroller(_handBoard);
        _handScroll.Height = _handHeight - 1;
        _handScroll.VerticalAlignment = VerticalAlignment.Center;
        hand.Children.Add(_handScroll);

        _handEmpty = new TextBlock
        {
            Text = "Your hand is empty",
            FontSize = 12,
            Foreground = Theme.Brush(Theme.FgMuted),
            HorizontalAlignment = HorizontalAlignment.Center,
            VerticalAlignment = VerticalAlignment.Center,
            Visibility = Visibility.Collapsed,
        };

        hand.Children.Add(_handEmpty);
        bottom.Children.Add(hand);

        var dock = _dock = new Border
        {
            Background = Theme.Brush(Theme.Panel),
            BorderBrush = Theme.Brush(Theme.Line),
            BorderThickness = new Thickness(1, 0, 0, 0),
            Padding = new Thickness(16, 14, 16, 14),
        };

        var column = new StackPanel { Orientation = Orientation.Vertical, Spacing = 10 };
        dock.Child = column;

        var toggles = new FlowPanel { HorizontalSpacing = 14, VerticalSpacing = 2 };
        column.Children.Add(toggles);

        _autoPassBox = new DockToggle("Auto Pass");
        _autoPassBox.Toggled += on =>
        {
            _autoPass = on;
            _needsRedraw = true;
        };
        toggles.Children.Add(_autoPassBox);

        _passTurnBox = new DockToggle("Pass Turn");
        _passTurnBox.Toggled += on =>
        {
            _passTurn = on;
            _passTurnOn = null;
            _needsRedraw = true;
        };
        toggles.Children.Add(_passTurnBox);

        _holdPriorityBox = new DockToggle("Hold Priority");
        _holdPriorityBox.Toggled += on =>
        {
            _holdPriority = on;
            _needsRedraw = true;
        };
        toggles.Children.Add(_holdPriorityBox);
        DescribeKeys();

        // What is being asked, and where the answer stands. Prompts name costs ("pay {3}"), so they are drawn with symbols.
        var text = new StackPanel { Orientation = Orientation.Vertical, Spacing = 4 };
        column.Children.Add(text);

        _promptTitle = new SymbolText(15, Theme.Fg, semibold: true, wrap: true) { Text = "Connecting…" };
        text.Children.Add(_promptTitle);

        _promptDetail = new SymbolText(12, Theme.FgSoft, wrap: true);
        text.Children.Add(_promptDetail);

        _errorLabel = new TextBlock { FontSize = 11, Foreground = Theme.Brush(Theme.Danger), TextWrapping = TextWrapping.Wrap, Visibility = Visibility.Collapsed };
        column.Children.Add(_errorLabel);

        _dockButtons = new FlowPanel { HorizontalSpacing = 8, VerticalSpacing = 6 };
        _buttons = _dockButtons;
        column.Children.Add(_dockButtons);

        Grid.SetColumn(dock, 1);
        bottom.Children.Add(dock);

        return bottom;
    }

    /// <summary>
    /// The stack, floating over the table on a translucent window ground, centred on the table's middle
    /// line. Its title bar folds it up; the grips on its top and bottom edges make its cards
    /// taller or shorter, both edges moving together because the window stays centred.
    /// </summary>
    private Grid BuildStackOverlay()
    {
        float height = ClampStackHeight(_stackHeight);
        _stackBoard = new CardBoardElement(StackCardWidth(height), height) { Height = height, Slides = true };

        var window = new Border
        {
            Background = Theme.Brush(Theme.WithAlpha(Theme.Panel, 0.75)),
            BorderBrush = Theme.Brush(Theme.ControlBorder),
            BorderThickness = new Thickness(1),
            CornerRadius = new CornerRadius(10),
            Padding = new Thickness(14, 12, 14, 12),
            Margin = new Thickness(0, ResizeGrip.Thickness / 2, 0, ResizeGrip.Thickness / 2),
        };

        var column = new StackPanel { Orientation = Orientation.Vertical, Spacing = 8 };
        window.Child = column;

        var header = new Grid { ColumnSpacing = 10, Background = Theme.Brush(Windows.UI.Color.FromArgb(0, 0, 0, 0)) };
        header.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        header.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        header.Tapped += (_, _) => ToggleStack();
        column.Children.Add(header);

        _stackTitle = new TextBlock { Text = "Stack", FontSize = 13, FontWeight = FontWeights.SemiBold, Foreground = Theme.Brush(Theme.Fg), VerticalAlignment = VerticalAlignment.Center };
        header.Children.Add(_stackTitle);

        _stackToggle = new CollapseButton();
        Grid.SetColumn(_stackToggle, 1);
        header.Children.Add(_stackToggle);

        column.Children.Add(_stackBoard);

        var host = new Grid
        {
            HorizontalAlignment = HorizontalAlignment.Left,
            VerticalAlignment = VerticalAlignment.Center,
            Margin = new Thickness(PlayerRail.RailWidth + 20, 0, 0, 0),
            Visibility = Visibility.Collapsed,
        };

        host.Children.Add(window);
        _stackGrips = [StackGrip(outwards: -1, VerticalAlignment.Top), StackGrip(outwards: 1, VerticalAlignment.Bottom)];
        foreach (ResizeGrip grip in _stackGrips)
        {
            host.Children.Add(grip);
        }

        return host;
    }

    private ResizeGrip StackGrip(int outwards, VerticalAlignment edge)
    {
        var grip = new ResizeGrip("Drag to resize the stack") { VerticalAlignment = edge };
        grip.Started += () => _stackDragFrom = _stackHeight;
        grip.Moved += down =>
        {
            _stackHeight = MathF.Round(ClampStackHeight((_stackDragFrom ?? _stackHeight) + (2f * (float)down * outwards)));
            SizeStack();
        };

        grip.Ended += () =>
        {
            _stackDragFrom = null;
            LocalSettings.Store(StackHeightKey, _stackHeight);
        };

        return grip;
    }

    private void ToggleStack()
    {
        _stackCollapsed = !_stackCollapsed;
        _stackToggle.Collapsed = _stackCollapsed;
        _stackBoard.Visibility = _stackCollapsed ? Visibility.Collapsed : Visibility.Visible;

        // Folded, there are no cards to make taller or shorter.
        foreach (ResizeGrip grip in _stackGrips)
        {
            grip.Visibility = _stackCollapsed ? Visibility.Collapsed : Visibility.Visible;
        }
    }

    /// <summary>The stack's card height, within what the table leaves room for.</summary>
    private float ClampStackHeight(float height)
    {
        float room = _table is null || _table.ActualHeight <= 0 ? MaxStackHeight : (float)_table.ActualHeight - 40f - StackChrome;
        return Math.Clamp(height, MinStackHeight, MathF.Max(MinStackHeight, MathF.Min(MaxStackHeight, room)));
    }

    /// <summary>A card as wide as a printed card is for its height, 63 by 88, rounded down.</summary>
    private static float StackCardWidth(float height) => MathF.Floor(height * 63f / 88f);

    /// <summary>How far each item on the stack overlaps the one under it: 34 of a standard 96-wide card, scaled.</summary>
    private static float StackStep(float width) => MathF.Round(34f * width / 96f);

    private void SizeStack()
    {
        float height = ClampStackHeight(_stackHeight);
        _stackBoard.SetCellSize(StackCardWidth(height), height);
        _stackBoard.Height = height;

        // The window is as wide as its cards at their new size, so it follows a drag of its edges.
        if (_gameClient is { } client)
        {
            LayOutStack(client, _stackItems);
        }
    }

    private async Task ConnectToServerAsync()
    {
        if (ClientConfig.Fixture is { } fixture)
        {
            // A canned position for one dock, with nothing on the other end: answers go nowhere and nothing is passed
            // for the player, so the dock stays exactly as the fixture put it.
            _fixture = true;
            LoadFixture(fixture);
            StartPumping();
            return;
        }

        try
        {
            var transport = await WebSocketTransport.ConnectAsync(ClientConfig.Server);
            _gameClient = new GameClient(transport);

            if (SeatMemory.Recall() is { } seat)
            {
                _resuming = seat;
                _gameClient.Resume(seat);
            }
            else
            {
                _gameClient.Join(ClientConfig.Name, preferredSeat: ClientConfig.Seat, ClientConfig.Ticket);
            }

            StartPumping();
        }
        catch (Exception ex)
        {
            _promptTitle.Text = "Could not connect";
            _promptDetail.Text = ex.Message;
        }
    }

    /// <summary>Whether this page has set off for the table its seat was moved to.</summary>
    private bool _movedAway;

    /// <summary>
    /// Remembers the seat once the server grants it, and when a remembered one is refused - the table it belonged to
    /// is gone - forgets it and asks for a seat afresh.
    /// </summary>

    private void KeepSeat(GameClient client)
    {
        // A seat given up, or moved to another table, is not this tab's any more: forgotten, so it is neither resumed on a
        // reload nor chased when the connection closes; and a move is followed to the new table.
        if (client.Released is not null || client.Moved is not null)
        {
            if (_remembered is not null)
            {
                _remembered = null;
                SeatMemory.Forget();
            }

            if (client.Moved is { } moved && !_movedAway)
            {
                _movedAway = true;
                TableMove.Go(moved.Server, moved.Ticket);
            }

            return;
        }

        if (client.Membership is { } granted && !ReferenceEquals(granted, _remembered))
        {
            _remembered = granted;
            _resuming = null;
            SeatMemory.Remember(granted);
            return;
        }

        if (_resuming is not null && client.Errors.Any(e => e.Code is ErrorCode.MembershipRefused or ErrorCode.MembershipRequired))
        {
            _resuming = null;
            SeatMemory.Forget();
            client.Join(ClientConfig.Name, preferredSeat: ClientConfig.Seat, ClientConfig.Ticket);
        }
    }

    /// <summary>
    /// Puts a fixture's position in front of the client, starting over: a new client over the fixture's messages, and
    /// the log and any half-made choice cleared. A gallery changes fixture by the URL's fragment, without reloading.
    /// </summary>
    private void LoadFixture(string name)
    {
        _fixtureName = name;
        _staged = false;
        _menu.Close();
        _fixtureTransport = new FixtureTransport(PromptFixtures.Build(name), PromptFixtures.BlocksPerBlocker(name));
        _gameClient = new GameClient(_fixtureTransport);
        _journal = new GameJournal();
        _abilitiesTriggered.Clear();
        _landingTriggers = null;
        _outcomeDismissed = false;
        _log.Clear();
        _selection.Clear();
        _lastPromptId = -1;
        _needsRedraw = true;
    }

    private string? WantedFixture()
    {
        string hash = System.Runtime.InteropServices.JavaScript.JSHost.GlobalThis.GetPropertyAsJSObject("location")?.GetPropertyAsString("hash") ?? string.Empty;
        return hash.Length > 1 ? Uri.UnescapeDataString(hash[1..]) : ClientConfig.Fixture;
    }

    private void StartPumping()
    {
        var timer = new DispatcherTimer { Interval = TimeSpan.FromMilliseconds(30) };
        timer.Tick += (_, _) => PumpGameClient();
        timer.Start();
    }

    private void PumpGameClient()
    {
        if (_fixture && WantedFixture() is { } wanted && wanted != _fixtureName && PromptFixtures.Known(wanted))
        {
            LoadFixture(wanted);
        }

        if (_gameClient is not { } client)
        {
            return;
        }

        // Everything received is taken here rather than read during the redraw, since the client keeps only the latest
        // batch. The log is the server's: all of it again after a reload or a reconnect, which comes with the first view.
        bool live = client.View is not null;
        if (client.Pump() > 0)
        {
            _needsRedraw = true;
            _journal.Follow(client);

            // Where card art is, as this table says: never what the page was opened with or came from.
            if (client.Table is { } table && ClientConfig.TableArt(table.Art))
            {
                CardArt.Moved();
            }

            foreach (GameEventView entry in client.RecentEvents)
            {
                WatchEvent(client, entry);

                if (entry is ObjectMovedEvent moved)
                {
                    _flights.Moved(moved);
                }
            }

            FollowResolutions(client, live);

            foreach (ViewHandle ability in _abilitiesTriggered.Keys.ToList())
            {
                if (client.Object(ability)?.Zone != ZoneKindView.Stack)
                {
                    _abilitiesTriggered.Remove(ability);
                }
            }

            LetGraveyardsGo(client);
        }

        LandTriggers(client);

        if (!_fixture)
        {
            FollowRestore(client);
            KeepSeat(client);
            FollowConnection(client);
            FollowGoingBack(client);

            // Answered for the player where they would only have passed; the view it leaves is the one drawn below.
            AutoAnswer(client);
            PlayMyself(client);
            PlayBots(client);
        }

        // Not while a card is being carried: the row it was picked up from would be rebuilt under the pointer.
        if (_needsRedraw && !AwaitingPrompt(client) && _carried is null && !_resolutions.HoldingTable)
        {
            _needsRedraw = false;
            Redraw(client);
        }

        // A fixture whose dock only appears after a click is put in that state once its prompt is drawn.
        if (_fixture && !_staged && client.Prompt is not null && _fixtureName is { } fixture)
        {
            _staged = true;
            Stage(client, fixture);
            RedrawNow();
        }
    }

    /// <summary>Whether the fixture on screen has been put in the state a click would.</summary>
    private bool _staged;

    /// <summary>
    /// Puts the client into the state a click would, for the fixtures whose dock only appears after one: a spell announced and waiting for its target, a land's menu open, the permanent a
    /// cost takes being picked, and a blocker held up waiting for an attacker.
    /// </summary>
    private void Stage(GameClient client, string fixture)
    {
        switch (fixture)
        {
            case "targeting" or "targeting-optional"
                when client.Prompt?.Prompt is PriorityPromptMessage aiming
                    && aiming.Options.FirstOrDefault(CardActions.NeedsATarget) is { } announced:
                _targeting = announced;
                _needsRedraw = true;
                break;

            case "priority-animated-land"
                when client.Prompt?.Prompt is PriorityPromptMessage offered
                    && offered.Options.FirstOrDefault(o => o.Description.Contains("becomes a 4/4", StringComparison.Ordinal)) is { } animation:
                _pointer = new Windows.Foundation.Point(_myHalf.Rail.ActualWidth, _table.ActualHeight * 0.4);
                Choose(client, _actions.For(animation.Source));
                break;

            case "priority-tap-for-mana" or "priority-sacrifice-for-mana"
                when client.Prompt?.Prompt is PriorityPromptMessage costing
                    && costing.Options.FirstOrDefault(o => o.Tap is not null || o.SacrificeCandidates.Count > 0) is { } chosen:
                Choose(client, _actions.For(chosen.Source));
                break;

            case "blockers" or "blockers-restricted"
                when client.Prompt?.Prompt is DeclareBlockersPromptMessage asked && asked.Eligible.Count > 0:
                ViewHandle held = asked.Eligible.FirstOrDefault(
                    blocker => asked.Allowed.Count(pair => pair.Blocker == blocker) < asked.Attackers.Count);

                _pending.Add((held.IsNone ? asked.Eligible[0] : held).Value);
                _needsRedraw = true;
                break;

            default:
                break;
        }
    }

    /// <summary>Draws at once what a click just changed, rather than on the next tick.</summary>
    private void RedrawNow()
    {
        if (_gameClient is { } client && _needsRedraw && !AwaitingPrompt(client) && _carried is null)
        {
            _needsRedraw = false;
            Redraw(client);
        }
    }

    /// <summary>Whether a redraw should wait because the prompt on screen has been answered and the next has not come yet.</summary>
    private bool AwaitingPrompt(GameClient client)
    {
        if (!_promptDrawn || client.Prompt is not null || client.IsOver || !client.IsConnected)
        {
            _promptAwaitedSince = null;
            return false;
        }

        _promptAwaitedSince ??= DateTime.UtcNow;
        return DateTime.UtcNow - _promptAwaitedSince < PromptGrace;
    }

    /// <summary>The whole table from the client's view and the outstanding prompt.</summary>
    private void Redraw(GameClient client)
    {
        _promptDrawn = client.Prompt is not null;

        // A new prompt means any selection made for the previous one is meaningless.
        int promptId = client.Prompt?.RequestId ?? -1;
        if (promptId != _lastPromptId)
        {
            _lastPromptId = promptId;
            _choiceCollapsed = false;
            _selection.Clear();
            _answerWhenJudged = false;
            _peekedForTargets = false;
        }

        ReadPrompt(client);
        _log.Show(_journal.Lines, _journal.Revision);
        _lobby.Show(client);
        ShowEvent(client);
        ShowPairings(client);
        _side.Menu.ShowPlaying(CanConcede(client));
        ShowDrawOffer(client);
        _deckLoading.Show(client);
        ShowOutcome(client);
        BrowserTitle.Show(PageTitle.For(client.Table, client.Seat, client.Draft, client.DeckBuilding, client.Pairings));

        // Hidden while a target is being picked, so the only ways out of that are choosing one or cancelling.
        _undoButton.Visibility = !client.IsOver && AimCandidates(client) is null && _actions.Undo is not null
            ? Visibility.Visible
            : Visibility.Collapsed;

        FillPrompt(client);
        FillPeek(client);
        SizeDock();

        if (client.View is not { } view)
        {
            return;
        }

        bool myTurn = view.ActivePlayer == client.Seat;
        _phaseTimeline.Follow(view.Step, myTurn);
        _phaseTimeline.Designate(view.DayNight);

        PlayerSummaryView? opponent = client.Opponents.FirstOrDefault();
        bool opponentActive = !client.IsOver && opponent is not null && view.ActivePlayer == opponent.Seat;

        _opponentHalf.Panel.Background = Theme.Brush(opponentActive ? Theme.HalfActive : Theme.HalfIdle);
        _myHalf.Panel.Background = Theme.Brush(!client.IsOver && myTurn ? Theme.HalfActive : Theme.HalfIdle);

        void Aimable(PlayerRail rail, int seat)
        {
            (Action? click, bool chosen) = PlayerClick(client, seat);
            rail.SetTarget(click, chosen);
        }

        Aimable(_myHalf.Rail, client.Seat);
        if (opponent is not null)
        {
            Aimable(_opponentHalf.Rail, opponent.Seat);
        }

        if (opponent is { } them)
        {
            UpdateRail(client, _opponentHalf.Rail, them, opponentActive, view.PriorityPlayer == them.Seat);
        }

        if (client.Me is { } me)
        {
            UpdateRail(client, _myHalf.Rail, me, myTurn, view.PriorityPlayer == client.Seat);
        }

        // The graveyards first, since the board beside them has what they leave.
        LayOutGraveyard(client, opponent?.Seat ?? 0, _opponentHalf.Graveyard);
        LayOutGraveyard(client, client.Seat, _myHalf.Graveyard);
        float inset = BoardInset();
        _graveyardGrip.Visibility = _graveyardsOpen ? Visibility.Visible : Visibility.Collapsed;
        _graveyardGrip.Margin = new Thickness(inset - (ResizeGrip.Thickness / 2), 0, 0, 0);
        _stackOverlay.Margin = new Thickness(inset + 20, 0, 0, 0);
        PlaceToast(inset);
        _table.UpdateLayout();

        // Combat first, because the creatures it takes are left out of the rows; and the creatures left in them are
        // given the same width in both halves, so the groups between stand at the same place in each.
        // Auras, Equipment and what a permanent has exiled until it leaves are drawn under it, not in a row of their own.
        BoardRows.Tucked = card => Tucked(client, card);
        float cardWidth = BattlefieldWidth((float)_myHalf.Scroll.ActualHeight);
        List<ObjectView> theirs;
        List<ObjectView> mine;
        float aligned;

        // The battlefield never scrolls down. Fitted to the table, the cards are drawn smaller until both halves' rows
        // fit their height: the size two rows have room for is only a start, since a stack of Auras or a row of piles
        // can stand taller than a card. At a size the settings fix, what does not fit is clipped instead.
        _fitted.Clear();
        for (int attempt = 0; ; attempt++)
        {
            FillCombat(client, view, cardWidth);
            theirs = [.. (opponent is null ? [] : client.BattlefieldShownOf(opponent.Seat)).Where(card => !_inCombat.Contains(card.Handle) && !Nested(client, card))];
            mine = [.. client.BattlefieldShownOf(client.Seat).Where(card => !_inCombat.Contains(card.Handle) && !Nested(client, card))];
            aligned = _myCombat.Count + _opponentCombat.Count == 0
                ? 0f
                : MathF.Max(BoardRows.CreaturesWidth(theirs, cardWidth, Look), BoardRows.CreaturesWidth(mine, cardWidth, Look));

            if (!PlayerSettings.AutoCardScale || attempt == 4)
            {
                break;
            }

            float fits = MathF.Min(
                FittingWidth(_opponentHalf, theirs, _opponentCombat, aligned, cardWidth),
                FittingWidth(_myHalf, mine, _myCombat, aligned, cardWidth));
            if (fits >= cardWidth)
            {
                break;
            }

            cardWidth = fits;
        }

        LayOutHalf(client, _opponentHalf, theirs, _opponentCombat, aligned, cardWidth);
        LayOutHalf(client, _myHalf, mine, _myCombat, aligned, cardWidth);
        LayOutCompanion(client, _opponentHalf, opponent);
        LayOutCompanion(client, _myHalf, client.View?.Players.FirstOrDefault(player => player.Seat == client.Seat));

        // A spell being paid for is on the stack already (CR 601.2a), and so is an ability (CR 602.2a): its source stays
        // where it is.
        List<ObjectView> hand = InHandOrder([.. client.MyHand()]);
        _hand = hand;
        LayOutHand(client);
        _handEmpty.Visibility = hand.Count == 0 ? Visibility.Visible : Visibility.Collapsed;

        LayOutStack(client, [.. client.Zone(ZoneKindView.Stack, 0), .. StagedTriggers(client)]);
        LayOutChoices();

        // After the flights' layout pass, so a stack window that has just appeared has a size to aim from.
        Fly(client);
        RememberPlaces();
        PaintTargetArrows(client);
        PaintSelection(client);
    }

    /// <summary>
    /// What a click on a player's box does now, and whether they are a target already picked: a player is aimed at by
    /// clicking their box while they are among the candidates, or attacked, by the creatures picked up to attack. No click
    /// while they are neither. The box on the rail, and a player tapped on an in-person table's tablet (MainPage.InPerson.cs).
    /// </summary>
    private (Action? Click, bool Chosen) PlayerClick(GameClient client, int seat)
    {
        TargetMessage? candidate = AimCandidates(client)?.FirstOrDefault(c => c.IsPlayer && c.Player == seat);

        if (candidate is null && client.Prompt?.Prompt is DeclareAttackersPromptMessage attacking
            && _pending.Any(creature => MayAttack(attacking, creature, new AttackChoice(seat, ViewHandle.None))))
        {
            return (
                () =>
                {
                    AssignAttack(client, attacking, new AttackChoice(seat, ViewHandle.None));
                    RedrawNow();
                },
                false);
        }

        return (
            candidate is null ? null : () =>
            {
                Aim(client, candidate);
                RedrawNow();
            },
            candidate is not null && IsPickedTarget(candidate));
    }

    /// <summary>The width a companion is drawn at in its window: small, beside the battlefield's cards.</summary>
    private const float CompanionCardWidth = 72f;

    /// <summary>
    /// The companion window over a half: the card the player revealed as their companion (CR 103.2b) while it waits outside
    /// the game, clickable as any card is - its owner's offers taking it for {3} at sorcery timing (CR 116.2g). Gone once it has
    /// been taken, or with none revealed.
    /// </summary>
    private void LayOutCompanion(GameClient client, HalfViews half, PlayerSummaryView? player)
    {
        ObjectView? card = player is { Companion.IsNone: false } ? client.Object(player.Companion) : null;
        if (card is null)
        {
            half.Companion.Visibility = Visibility.Collapsed;
            half.CompanionBoard.SetCells([]);
            return;
        }

        half.Companion.Visibility = Visibility.Visible;
        half.CompanionBoard.SetCells([Cell(client, card, 0, 0, tapped: false)]);
    }

    /// <summary>
    /// Hands every card laid out now to the flight layer, which flies whatever the game moved. After
    /// a layout pass, so where each row sits on the page is where it is now.
    /// </summary>
    private void Fly(GameClient client)
    {
        Root.UpdateLayout();
        List<CardFlights.Placed> placed = [];

        foreach (CardBoardElement board in (CardBoardElement[])[_opponentHalf.Board, _myHalf.Board, _opponentHalf.Graveyard.Board, _myHalf.Graveyard.Board, _handBoard, _stackBoard, _choiceBoard])
        {
            if (board.XamlRoot is null || board.ActualWidth <= 0 || !IsShown(board))
            {
                continue;
            }

            GeneralTransform toPage = board.TransformToVisual(Root);
            foreach ((ViewHandle handle, ObjectView card, float x, float y, float width, float angle) in board.Laid())
            {
                Windows.Foundation.Point at = toPage.TransformPoint(new Windows.Foundation.Point(x, y));
                placed.Add(new CardFlights.Placed(handle, card, new CardFlights.Pose((float)at.X, (float)at.Y, width, angle), board, x, y));
            }
        }

        _flights.Look(placed, (zone, owner) =>
        {
            PlayerRail? rail = owner == client.Seat ? _myHalf.Rail
                : client.Opponents.FirstOrDefault()?.Seat == owner ? _opponentHalf.Rail
                : null;

            FrameworkElement? pile = (zone, rail) switch
            {
                (_, null) => null,
                (ZoneKindView.Library, { } r) => r.LibraryPile,
                (ZoneKindView.Graveyard, { } r) => r.GraveyardPile,

                // This player's hand is drawn as cards, so its cards were seen; an opponent's is the fan in their rail.
                (ZoneKindView.Hand, { } r) when owner != client.Seat => r.HandFan,
                _ => null,
            };

            if (pile is null || pile.ActualWidth <= 0 || pile.XamlRoot is null)
            {
                return null;
            }

            Windows.Foundation.Point centre = pile.TransformToVisual(Root).TransformPoint(new Windows.Foundation.Point(pile.ActualWidth / 2, pile.ActualHeight / 2));

            // A fan is several cards wide; a card leaves from, or goes into, one of its cards' size.
            float width = pile is HandFanElement
                ? HandFanElement.CardWidthFor((float)pile.ActualWidth)
                : (float)Math.Min(pile.ActualWidth, pile.ActualHeight * 63 / 88);
            return new CardFlights.Pose((float)centre.X, (float)centre.Y, width, 0f);
        });
    }

    /// <summary>Whether an element and everything around it are showing, so a card on a folded window is not a place.</summary>
    private static bool IsShown(FrameworkElement element)
    {
        for (DependencyObject? at = element; at is not null; at = VisualTreeHelper.GetParent(at))
        {
            if (at is UIElement { Visibility: Visibility.Collapsed })
            {
                return false;
            }
        }

        return true;
    }

    /// <summary>
    /// How wide a card on the battlefield is drawn: as large as the half's two rows have room for, or at the size chosen
    /// in the settings.
    /// </summary>
    private static float BattlefieldWidth(float halfHeight) => PlayerSettings.AutoCardScale
        ? BoardRows.CardWidth(halfHeight)
        : MathF.Round(BoardRows.StandardCardWidth * PlayerSettings.CardScale);

    private string Look(ObjectView card) => BoardRows.Likeness(card, PromptLikeness(card));

    /// <summary>The smallest a card is drawn to fit the battlefield: past this the half clips rather than shrink it further.</summary>
    private const float SmallestFittedCard = 24f;

    /// <summary>
    /// The card width at which a half's rows fit its height, found from how tall they stand at this width: only the rows
    /// grow with the cards, not the margins or the gap between them. This width if they fit already.
    /// </summary>
    private float FittingWidth(HalfViews half, List<ObjectView> permanents, List<BoardRows.CombatGroup> combat, float aligned, float cardWidth)
    {
        float halfHeight = (float)half.Scroll.ActualHeight;
        if (halfHeight <= 0 || cardWidth <= SmallestFittedCard)
        {
            return cardWidth;
        }

        BoardRows.Laid laid = BoardRows.Lay(permanents, half.Top, (float)half.Scroll.ViewportWidth, halfHeight, cardWidth, Look, IsSpent, combat, aligned);
        _fitted.Add(new Fitted(half, permanents, combat, aligned, cardWidth, (float)half.Scroll.ViewportWidth, halfHeight, laid));
        float needed = laid.Height;
        if (needed <= halfHeight)
        {
            return cardWidth;
        }

        float fixedPart = (2f * BoardRows.MarginY) + BoardRows.RowGap;
        float scaled = MathF.Floor(cardWidth * MathF.Max(0f, halfHeight - fixedPart) / MathF.Max(1f, needed - fixedPart));

        // Rounding can leave it a pixel short of fitting; each try is at least a pixel smaller than the last.
        return MathF.Max(SmallestFittedCard, MathF.Min(scaled, cardWidth - 1f));
    }

    /// <summary>
    /// A half laid out while its card size was being found, kept for this redraw: at the size that fits, it is the very
    /// layout the half is then given, and a board of two hundred permanents is not laid out a second time to get it.
    /// </summary>
    private sealed record Fitted(
        HalfViews Half, List<ObjectView> Permanents, List<BoardRows.CombatGroup> Combat, float Aligned, float CardWidth, float Viewport, float Height, BoardRows.Laid Laid);

    private readonly List<Fitted> _fitted = [];

    private void LayOutHalf(GameClient client, HalfViews half, IEnumerable<ObjectView> permanents, List<BoardRows.CombatGroup> combat, float aligned, float cardWidth)
    {
        float viewportWidth = (float)half.Scroll.ViewportWidth;
        float halfHeight = (float)half.Scroll.ActualHeight;
        if (viewportWidth <= 0 || halfHeight <= 0)
        {
            return;
        }

        half.Board.SetCellSize(cardWidth, BoardRows.CardHeight(cardWidth));

        BoardRows.Laid laid = _fitted.FindLast(fitted =>
                ReferenceEquals(fitted.Half, half)
                && ReferenceEquals(fitted.Permanents, permanents)
                && ReferenceEquals(fitted.Combat, combat)
                && fitted.Aligned == aligned
                && fitted.CardWidth == cardWidth
                && fitted.Viewport == viewportWidth
                && fitted.Height == halfHeight)?.Laid
            ?? BoardRows.Lay(
                permanents,
                half.Top,
                viewportWidth,
                halfHeight,
                cardWidth,
                Look,
                IsSpent,
                combat,
                aligned);

        // Exactly as tall as the half: rows that need more are clipped at its outer edge, never scrolled to.
        half.Board.Width = laid.Width;
        half.Board.Height = halfHeight;
        half.Badges.Width = laid.Width;
        half.Badges.Height = half.Board.Height;

        half.Board.SetCells([.. laid.Cards.Select(placed => Cell(client, placed.Card, placed.Left, placed.Top, placed.Card.Tapped))]);
        half.Board.Bands = laid.Bands;

        half.Badges.Children.Clear();

        // Each attacking group's caption, along the band's far edge, away from the line (CombatZone).
        foreach (BoardRows.Band band in laid.Bands)
        {
            if (band.Caption is { Length: > 0 } caption)
            {
                var text = new TextBlock
                {
                    Text = caption.ToUpperInvariant(),
                    FontSize = 10,
                    FontWeight = FontWeights.SemiBold,
                    Foreground = Theme.Brush(Theme.FromHex("#ee8f8a")),
                };

                Canvas.SetLeft(text, band.Left + 10);
                Canvas.SetTop(text, band.OpensDownward ? band.Top + 5 : band.Bottom - 19);
                half.Badges.Children.Add(text);
            }
        }

        foreach (BoardRows.Badge badge in laid.Badges)
        {
            Border pill = PileCount(badge.Count);
            pill.Measure(new Windows.Foundation.Size(double.PositiveInfinity, double.PositiveInfinity));
            Canvas.SetLeft(pill, badge.GrowsLeft ? badge.X - pill.DesiredSize.Width : badge.X);
            Canvas.SetTop(pill, badge.Y);
            half.Badges.Children.Add(pill);
        }
    }

    /// <summary>A pile's count: a small pill on the panel colour, edged, with "×3" in it.</summary>
    private static Border PileCount(int count) => Theme.Pill(new Border
    {
        Background = Theme.Brush(Theme.WithAlpha(Theme.Panel, 0.94)),
        BorderBrush = Theme.Brush(Theme.ControlBorder),
        BorderThickness = new Thickness(1),
        Padding = new Thickness(7, 1, 7, 1),
        Child = new TextBlock { Text = $"×{count}", FontSize = 10, FontWeight = FontWeights.SemiBold, Foreground = Theme.Brush(Theme.Fg) },
    });

    /// <summary>
    /// Sets the hand strip's height, between the least a card can be read at and half the window, so the table always
    /// keeps the other half. A saved height too tall for this window is held down, not changed.
    /// </summary>
    private void SetHandHeight(float height)
    {
        float most = MathF.Max(MinHandHeight, (float)Root.ActualHeight * 0.5f);
        height = MathF.Round(Math.Clamp(height, MinHandHeight, Root.ActualHeight > 0 ? most : float.MaxValue));
        if (height == _handHeight)
        {
            return;
        }

        _handHeight = height;
        _bottom.MinHeight = height;
        _handScroll.Height = height - 1;
        _handBoard.SetCellSize(HandCardWidth, HandCardHeight);
        _needsRedraw = true;
        RedrawNow();
    }

    /// <summary>The hand as last drawn, for laying it out again when the pointer moves along it.</summary>
    private List<ObjectView> _hand = [];

    /// <summary>What the hand strip keeps clear on either side of its cards.</summary>
    private const float HandMarginX = 18f;

    /// <summary>
    /// The hand, spread: side by side a gap apart while they fit, laid over
    /// one another left to right once they do not, and the card under the pointer pulled out, the ones after it sliding
    /// aside, so it can be read whole. Only the cards after it move, so the pointed-at card stays under the pointer.
    /// </summary>
    private void LayOutHand(GameClient client)
    {
        float viewportWidth = (float)_handScroll.ViewportWidth;
        float viewportHeight = (float)_handScroll.ActualHeight;
        if (viewportWidth <= 0 || viewportHeight <= 0)
        {
            return;
        }

        _handBoard.Width = viewportWidth;
        _handBoard.Height = viewportHeight;

        float available = viewportWidth - (2 * HandMarginX);
        int count = _hand.Count;
        int hovered = _handBoard.Hovered is { } pointed ? _hand.FindIndex(card => card.Handle == pointed) : -1;

        float step = SpreadStep(count, available, HandCardWidth, HandCardWidth + CardGap);
        float reveal = MathF.Max(0f, HandCardWidth - step);
        float lying = ((count - 1) * step) + HandCardWidth;

        // Centred on the cards as they lie, but never so far right that pulling one out would push the last past the edge.
        float offset = HandMarginX + MathF.Max(0f, MathF.Min((available - lying) / 2f, available - lying - reveal));
        float top = (viewportHeight - HandCardHeight) / 2;

        List<CardBoardElement.BoardCell> cells = [];
        for (int i = 0; i < count; i++)
        {
            float x = offset + (i * step) + (hovered >= 0 && i > hovered ? reveal : 0f);
            cells.Add(Cell(client, _hand[i], x, top, tapped: false));
        }

        _handBoard.SetCells(cells);
    }

    /// <summary>
    /// How far apart the cards of a spread stand: the spacing while they fit, less once they do not,
    /// and room kept to pull one out whole - never less than a strip of each.
    /// </summary>
    private static float SpreadStep(int count, float available, float card, float spacing)
    {
        const float MinimumStep = 10f;

        if (count <= 1)
        {
            return spacing;
        }

        float step = MathF.Min(spacing, (available - card) / (count - 1));
        if (step < card && count > 2)
        {
            step = MathF.Min(step, (available - (card * 2f)) / (count - 2));
        }

        return MathF.Max(step, MinimumStep);
    }

    /// <summary>
    /// Arrows from one spell on the stack to what it is aimed at: the one under the pointer
    /// in the stack window, or else the top of the stack, since what resolves next is what a player wants to know about
    /// before they think to ask. Targets no longer legal are left out; the spell will do nothing to them (CR 608.2b).
    /// A permanent under the pointer on the battlefield that chose another as it entered (CR 614.12a, 607.2d) points at the
    /// one it chose instead, the arrow labelled with what its text calls it: "Chosen creature".
    /// </summary>
    private void PaintTargetArrows(GameClient? client)
    {
        IReadOnlyList<ObjectView> stack = client?.Zone(ZoneKindView.Stack, 0) ?? [];
        ObjectView? spell = _stackBoard.Hovered is { } hovered
            ? stack.FirstOrDefault(item => item.Handle == hovered && item.Targets.Count > 0)
            : null;

        spell ??= stack.Count > 0 && stack[^1].Targets.Count > 0 ? stack[^1] : null;

        _arrowLabels.Children.Clear();
        if (PaintChosenArrows())
        {
            return;
        }

        if (client is null || spell is null || _stackCollapsed || BoundsOn(_stackBoard, spell.Handle) is not { } source)
        {
            _arrows.Show([]);
            return;
        }

        var from = new SkiaSharp.SKPoint(source.MidX, source.MidY);
        List<(SkiaSharp.SKPoint From, SkiaSharp.SKPoint To)> arrows = [];

        foreach (SpellTargetView target in spell.Targets)
        {
            if (target.StillLegal && TargetBounds(client, target) is { } box)
            {
                SkiaSharp.SKPoint to = TargetArrowsElement.Edge(box, from);
                arrows.Add((from, to));

                if (target.Text.Length > 0)
                {
                    LabelArrow(target.Text, TargetArrowsElement.LabelAt(from, to));
                }
            }
        }

        _arrows.Show(arrows);
    }

    private readonly Canvas _arrowLabels = new() { IsHitTestVisible = false };

    /// <summary>
    /// Arrows from the battlefield permanent under the pointer to each permanent it chose as it entered (a choice with an
    /// object, <see cref="ChoiceView.Object"/>), if it has any: whether any were drawn.
    /// </summary>
    private bool PaintChosenArrows()
    {
        foreach (CardBoardElement board in (CardBoardElement[])[_opponentHalf.Board, _myHalf.Board])
        {
            if (board.Hovered is not { } hovered || board.CardOf(hovered) is not { } card
                || !card.Choices.Exists(choice => choice.Object != ViewHandle.None) || BoundsOn(board, hovered) is not { } source)
            {
                continue;
            }

            var from = new SkiaSharp.SKPoint(source.MidX, source.MidY);
            List<(SkiaSharp.SKPoint From, SkiaSharp.SKPoint To)> arrows = [];
            foreach (ChoiceView choice in card.Choices)
            {
                if (choice.Object != ViewHandle.None && CardBounds(choice.Object) is { } box)
                {
                    SkiaSharp.SKPoint to = TargetArrowsElement.Edge(box, from);
                    arrows.Add((from, to));
                    LabelArrow(Cantrip.Core.ObjectChoice.Label(choice.Key), TargetArrowsElement.LabelAt(from, to));
                }
            }

            _arrows.Show(arrows);
            return true;
        }

        return false;
    }

    /// <summary>
    /// The clause of the spell that applies to one target, on its own backing so it reads over a card, centred just
    /// behind the arrow's head and kept inside the window.
    /// </summary>
    private void LabelArrow(string text, SkiaSharp.SKPoint at)
    {
        var label = new Border
        {
            Background = Theme.Brush(Theme.WithAlpha(Theme.Window, 0.94)),
            BorderBrush = Theme.Brush(Theme.WithAlpha(Theme.Accent, 0.55)),
            BorderThickness = new Thickness(1),
            Padding = new Thickness(8, 5, 8, 5),
            Child = new TextBlock
            {
                Text = text,
                FontSize = 11,
                FontWeight = FontWeights.SemiBold,
                Foreground = Theme.Brush(Theme.Fg),
                TextWrapping = TextWrapping.WrapWholeWords,
                MaxLines = 3,
                TextTrimming = TextTrimming.CharacterEllipsis,
                MaxWidth = 190,
            },
        };

        label.Measure(new Windows.Foundation.Size(double.PositiveInfinity, double.PositiveInfinity));
        Windows.Foundation.Size size = label.DesiredSize;
        double most = Math.Max(6, Root.ActualWidth - size.Width - 6);
        double lowest = Math.Max(6, Root.ActualHeight - size.Height - 6);
        Canvas.SetLeft(label, Math.Clamp(at.X - (size.Width / 2), 6, most));
        Canvas.SetTop(label, Math.Clamp(at.Y - (size.Height / 2), 6, lowest));
        _arrowLabels.Children.Add(label);
    }

    /// <summary>Where a target is on screen: a card on any row, or the avatar box that stands for a player.</summary>
    private SkiaSharp.SKRect? TargetBounds(GameClient client, SpellTargetView target) =>
        target.IsPlayer ? PlayerBounds(client, target.Player) : CardBounds(target.Object);

    /// <summary>Where the box that stands for a player is, if the table draws them.</summary>
    private SkiaSharp.SKRect? PlayerBounds(GameClient client, int seat)
    {
        PlayerRail? rail = seat == client.Seat ? _myHalf.Rail
            : client.Opponents.FirstOrDefault()?.Seat == seat ? _opponentHalf.Rail
            : null;

        return rail is null ? null : BoundsOf(rail.Avatar, new Windows.Foundation.Rect(0, 0, rail.Avatar.ActualWidth, rail.Avatar.ActualHeight));
    }

    /// <summary>Where a card is on screen, on whichever row it stands.</summary>
    private SkiaSharp.SKRect? CardBounds(ViewHandle card)
    {
        foreach (CardBoardElement board in (CardBoardElement[])[_opponentHalf.Board, _myHalf.Board, _opponentHalf.Graveyard.Board, _myHalf.Graveyard.Board, _handBoard, _stackBoard])
        {
            if (BoundsOn(board, card) is { } box)
            {
                return box;
            }
        }

        return null;
    }

    private SkiaSharp.SKRect? BoundsOn(CardBoardElement board, ViewHandle handle) =>
        board.Visibility == Visibility.Visible && board.Bounds(handle) is { } local ? BoundsOf(board, local) : null;

    /// <summary>A rectangle in an element, in the arrow layer's coordinates.</summary>
    private SkiaSharp.SKRect? BoundsOf(FrameworkElement element, Windows.Foundation.Rect local)
    {
        if (element.ActualWidth <= 0 || element.XamlRoot is null)
        {
            return null;
        }

        Windows.Foundation.Rect r = element.TransformToVisual(_arrows).TransformBounds(local);
        return SkiaSharp.SKRect.Create((float)r.X, (float)r.Y, (float)r.Width, (float)r.Height);
    }

    /// <summary>The stack as an overlapping pile, oldest at the left and the top of the stack fully showing at the right.</summary>
    /// <summary>What the stack window shows, for laying it out again as it is resized or pointed along.</summary>
    private List<ObjectView> _stackItems = [];

    /// <summary>
    /// The stack as an overlapping row, oldest at the left: each item lies over the
    /// one it was put on top of, the item under the pointer pulls out whole, and the row overlaps further rather than
    /// running past the table. The window is as wide as the row, so it grows and shrinks with its cards.
    /// </summary>
    private void LayOutStack(GameClient client, IReadOnlyList<ObjectView> stack)
    {
        _stackItems = [.. stack];
        _stackOverlay.Visibility = stack.Count > 0 ? Visibility.Visible : Visibility.Collapsed;
        _stackTitle.Text = stack.Count == 1 ? "Stack" : $"Stack  ·  {stack.Count}";

        // Open while triggers are being put on it, since that is where each one clicked goes.
        _stackBoard.Visibility = !_stackCollapsed || stack.Any(item => item.Handle.Value < 0) ? Visibility.Visible : Visibility.Collapsed;

        float height = ClampStackHeight(_stackHeight);
        float width = StackCardWidth(height);
        _stackBoard.SetCellSize(width, height);
        _stackBoard.Height = height;

        // The panel's own margins and a gutter either side are what the cards may not have.
        float most = MathF.Max(width, (float)_table.ActualWidth - BoardInset() - 36f - 28f);
        float step = SpreadStep(stack.Count, most, width, StackStep(width));
        float reveal = MathF.Max(0f, width - step);
        int hovered = _stackBoard.Hovered is { } pointed ? stack.ToList().FindIndex(item => item.Handle == pointed) : -1;

        List<CardBoardElement.BoardCell> cells = [];
        float right = 0f;
        for (int i = 0; i < stack.Count; i++)
        {
            float x = (i * step) + (hovered >= 0 && i > hovered ? reveal : 0f);
            cells.Add(Cell(client, stack[i], x, 0, tapped: false));
            right = x + width;
        }

        _stackBoard.Width = right;
        _stackBoard.SetCells(cells);
    }

    /// <summary>
    /// Passes priority for the player when the dock's settings and stops say they would only pass (deciding through
    /// AutoPass.Wanted), and declares the chosen attackers, if any, for Pass Turn.
    /// </summary>
    private void AutoAnswer(GameClient client)
    {
        if (client.View is not { } view)
        {
            return;
        }

        IReadOnlyList<ObjectView> stack = client.Zone(ZoneKindView.Stack, 0);
        FollowPassTurn(view);
        FollowResolveAll(stack);

        if (_targeting is not null || _selection.CostChoice is not null)
        {
            return;
        }

        // The mana planned for X as it was announced, made now the cost is paid (CR 601.2g).
        if (TapPlanned(client))
        {
            return;
        }

        // Passing the turn attacks with what was chosen to, though the attack was not locked in, and what must attack, a
        // goaded creature (CR 701.15b).
        if (_passTurn && client.Prompt?.Prompt is DeclareAttackersPromptMessage passing)
        {
            Send(client, PassTurnAttack(passing));
            return;
        }

        if (client.Prompt?.Prompt is not PriorityPromptMessage priority)
        {
            return;
        }

        // At the advisor's table the advisor answers what this page would pass by itself (MainPage.Advisor.cs).
        if (AdvisorHoldsPriority())
        {
            return;
        }

        // The priority of a player this seat controls (CR 723.5) is passed as that player's own: it is their turn.
        bool myTurn = view.ActivePlayer == client.ActingSeat;
        if (AutoPass.Wanted(new PriorityBoard
        {
            Prompt = priority,
            Stack = stack,
            Seat = client.ActingSeat,
            MyTurn = myTurn,
            Step = view.Step,
            Attacked = view.Combat.Attackers.Count > 0,
            HoldingPriority = _holdPriority || Held(Hotkey.HoldPriority),
            PassTurn = _passTurn,
            ResolveAll = _resolveAll is not null,
            StopsAtStep = _stops.StopsAt(view.Step, myTurn),
            AutoPass = _autoPass,
        }))
        {
            Send(client, new PassAnswer());
        }
    }

    /// <summary>Notes which turn Pass Turn was ticked in, and clears the box once that turn is over.</summary>
    private void FollowPassTurn(PlayerView view)
    {
        if (!_passTurn)
        {
            return;
        }

        _passTurnOn ??= view.Turn;
        if (_passTurnOn != view.Turn)
        {
            _passTurn = false;
            _passTurnOn = null;
            _passTurnBox.SetOn(false);
        }
    }

    /// <summary>Ends Resolve All once what it was resolving is gone, or as soon as anything new joins it on the stack.</summary>
    private void FollowResolveAll(IReadOnlyList<ObjectView> stack)
    {
        if (_resolveAll is { } resolving && (stack.Count == 0 || stack.Any(item => !resolving.Contains(item.Handle))))
        {
            _resolveAll = null;
        }
    }

    private void OnUndoClick()
    {
        if (_gameClient is { } client && _actions.Undo is { } undo)
        {
            Send(client, AnswerFor(undo));
            RedrawNow();
        }
    }

    /// <summary>
    /// What passing priority will do from here: resolve what is on
    /// top of the stack, or with the stack empty, move on to wherever the player is next stopped. A forecast, not a
    /// promise: the opponent may act before then.
    /// </summary>
    private string PassLabel(GameClient client)
    {
        if (client.View is not { } view)
        {
            return "Pass";
        }

        if (client.ZoneCount(ZoneKindView.Stack, 0) > 0)
        {
            return "Resolve";
        }

        // For a player this seat controls (CR 723.5), as that player: their turn, their creatures.
        bool mine = view.ActivePlayer == client.ActingSeat;
        bool canAttack = client.Zone(ZoneKindView.Battlefield, 0).Any(card =>
            card.Controller == client.ActingSeat
            && card.CardTypes.Contains(CardTypeView.Creature)
            && !card.Tapped
            && !card.SummoningSick);

        TurnStepView step = view.Step;
        bool stepMine = mine;
        bool turnPassed = false;

        // Every step of this turn and the next, which is as far as a stop can be.
        for (int i = 0; i < 24; i++)
        {
            if (step == TurnStepView.Cleanup)
            {
                step = TurnStepView.Untap;
                stepMine = !stepMine;
                turnPassed = true;
            }
            else
            {
                step++;
            }

            bool attacking = stepMine && !turnPassed && canAttack && step == TurnStepView.DeclareAttackers;

            // Being attacked stops this player whatever their stops say, once the attackers are on the table.
            bool blocking = !stepMine
                && !turnPassed
                && view.Combat.Attackers.Count > 0
                && step is TurnStepView.DeclareAttackers or TurnStepView.DeclareBlockers;

            if (!attacking && !blocking && !_stops.StopsAt(step, stepMine))
            {
                continue;
            }

            if (turnPassed)
            {
                return mine ? "End Turn" : "Go to My Turn";
            }

            return step switch
            {
                TurnStepView.BeginCombat or TurnStepView.DeclareAttackers => "Go to Combat",
                TurnStepView.DeclareBlockers => "Go to Blockers",
                TurnStepView.CombatDamage => "Go to Damage",
                TurnStepView.EndOfCombat => "Go to End of Combat",
                TurnStepView.PostcombatMain => "Go to Main 2",
                TurnStepView.End => "Go to End Step",
                TurnStepView.Cleanup => "Go to Cleanup",
                _ => "Pass",
            };
        }

        return "Pass";
    }
}
