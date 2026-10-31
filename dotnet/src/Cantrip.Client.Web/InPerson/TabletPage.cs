using Cantrip.Client.InPerson;
using Cantrip.Protocol;
using Cantrip.Protocol.InPerson;
using Microsoft.UI.Text;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using SkiaSharp;
using Windows.Foundation;

namespace Cantrip.Client.Web.InPerson;

/// <summary>
/// A public board at an in-person table (<c>?inperson=tablet</c>, docs/in-person.md): what anyone at the table may see,
/// drawn from the relay's public feed through a plain <see cref="GameClient"/>, and where the permanents, the spells on
/// the stack and the players are tapped to pick them.
/// <para>
/// The shared board (<c>side</c> absent) lies between the players: seat 2's half at the top, turned to face seat 2, the
/// stack across the middle, seat 1's half at the bottom. A player's own board (<c>side=N</c>) is that seat's half, the
/// stack, and both life totals.
/// </para>
/// <para>
/// The tablet is strict: it shows nothing of anybody's prompt. A tap is sent to the relay as the public handle or the
/// player tapped and nothing on the tablet answers it - no candidate is lit, no tap is taken or refused here; all of that
/// is on the acting player's phone. A long press holds a card up to read, on this tablet alone.
/// </para>
/// </summary>
public sealed partial class TabletPage : UserControl
{
    /// <summary>How long a press rests on a card before it holds the card up to read rather than tapping it.</summary>
    private static readonly TimeSpan HoldAfter = TimeSpan.FromMilliseconds(450);

    /// <summary>How far a press may wander and still be a tap: a fingertip is not a mouse.</summary>
    private const double TapSlop = 18;

    /// <summary>The share of the page's height the stack band takes.</summary>
    private const double StackShare = 0.2;

    /// <summary>
    /// The share the shared board's band takes: two strips, the stack once facing each player, so each strip is near what a
    /// player's own board gives the stack whole.
    /// </summary>
    private const double SharedStackShare = 0.28;

    private readonly Grid _root = new();
    private readonly FeedTransport _feed = new();
    private readonly GameClient _client;

    /// <summary>The seat whose own board this is, or zero for the shared board.</summary>
    private readonly int _side = InPersonConfig.Side;

    /// <summary>The halves shown: on the shared board the bottom one first.</summary>
    private readonly List<TabletHalf> _halves = [];
    private readonly TabletStack _stack = new();

    /// <summary>
    /// On the shared board, the stack again, turned to face seat 2: the band is mirrored, each player reading every spell
    /// and ability on it upright on the strip nearest them, in the same order, the top of the stack at their right.
    /// </summary>
    private readonly TabletStack? _turnedStack;
    private readonly TargetArrowsElement _arrows = new();
    private readonly CardFlights _flights = new();

    /// <summary>A card held up from an upright part of the board, and from the turned half: each reads upright to whoever is across from it.</summary>
    private readonly CardInspector _inspector = new();
    private readonly CardInspector _turnedInspector = new();
    private readonly CardTooltip _tooltip = new();
    private readonly CardTooltip _turnedTooltip = new();

    /// <summary>Both players' life, across the top of a player's own board.</summary>
    private readonly TextBlock _lives = new() { FontSize = 22, FontWeight = FontWeights.SemiBold, Foreground = Theme.Brush(Theme.Fg), VerticalAlignment = VerticalAlignment.Center };

    /// <summary>Where the event stands before its game, until the first public state comes.</summary>
    private readonly Border _phaseScreen;
    /// <summary>What the phase screen says - its title, the line, who is connected - once for each way it is read.</summary>
    private readonly List<(TextBlock Title, TextBlock Line, TextBlock Connected)> _phaseTexts = [];

    private readonly DispatcherTimer _frame = new() { Interval = TimeSpan.FromMilliseconds(30) };
    private readonly DispatcherTimer _hold = new() { Interval = HoldAfter };
    private bool _needsRedraw = true;

    /// <summary>A press on a card that has not yet become a tap or a hold.</summary>
    private (CardBoardElement Board, ViewHandle Handle, uint Pointer, Point At, bool Held)? _press;

    public TabletPage()
    {
        _client = new GameClient(_feed);
        _turnedStack = _side == 0 ? new TabletStack() : null;
        Content = _root;
        _root.Background = Theme.Brush(Theme.Window);

        BuildBoard();
        _phaseScreen = BuildPhaseScreen();
        _root.Children.Add(_phaseScreen);
        ShowPhase(null, "Connecting to the table…", []);

        _frame.Tick += (_, _) => Frame();
        _frame.Start();
        _hold.Tick += (_, _) => Hold();
        _root.SizeChanged += (_, _) =>
        {
            _flights.Settle();
            _needsRedraw = true;
        };

        // A right button let go anywhere puts a card held up with it back (a mouse at the tablet's page, for trying it out).
        _root.AddHandler(PointerReleasedEvent, new PointerEventHandler((_, e) =>
        {
            if (e.GetCurrentPoint(_root).Properties.PointerUpdateKind == Microsoft.UI.Input.PointerUpdateKind.RightButtonReleased)
            {
                PutDown();
            }
        }), handledEventsToo: true);

        Connect();
        StartTestState();
    }

    /// <summary>The halves, the stack band and the layers over them, as the layout this tablet shows.</summary>
    private void BuildBoard()
    {
        var board = new Grid();
        _root.Children.Add(board);

        if (_side == 0)
        {
            board.RowDefinitions.Add(new RowDefinition { Height = new GridLength(1, GridUnitType.Star) });
            board.RowDefinitions.Add(new RowDefinition { Height = new GridLength(SharedStackShare / (1 - SharedStackShare) * 2, GridUnitType.Star) });
            board.RowDefinitions.Add(new RowDefinition { Height = new GridLength(1, GridUnitType.Star) });

            var top = new TabletHalf(seat: 2, turned: true);
            var bottom = new TabletHalf(seat: 1, turned: false);
            var band = new Grid();
            band.RowDefinitions.Add(new RowDefinition { Height = new GridLength(1, GridUnitType.Star) });
            band.RowDefinitions.Add(new RowDefinition { Height = new GridLength(1, GridUnitType.Star) });
            _turnedStack!.RenderTransformOrigin = new Point(0.5, 0.5);
            _turnedStack.RenderTransform = new RotateTransform { Angle = 180 };
            Grid.SetRow(_stack, 1);
            band.Children.Add(_turnedStack);
            band.Children.Add(_stack);
            Grid.SetRow(band, 1);
            Grid.SetRow(bottom, 2);
            board.Children.Add(top);
            board.Children.Add(band);
            board.Children.Add(bottom);
            _halves.Add(bottom);
            _halves.Add(top);
        }
        else
        {
            board.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
            board.RowDefinitions.Add(new RowDefinition { Height = new GridLength(StackShare, GridUnitType.Star) });
            board.RowDefinitions.Add(new RowDefinition { Height = new GridLength(1 - StackShare, GridUnitType.Star) });

            var lives = new Border
            {
                Background = Theme.Brush(Theme.Bar),
                Padding = new Thickness(18, 10, 18, 10),
                Child = _lives,
            };
            board.Children.Add(lives);

            var mine = new TabletHalf(_side, turned: false);
            Grid.SetRow(_stack, 1);
            Grid.SetRow(mine, 2);
            board.Children.Add(_stack);
            board.Children.Add(mine);
            _halves.Add(mine);
        }

        foreach (TabletHalf half in _halves)
        {
            MakeTappable(half.Board, half.Turned);
            MakeTappable(half.Pile.Board, half.Turned);
            TabletHalf shutting = half;
            half.Pile.Close.Clicked += () => ShowPile(shutting, null);
            int seat = half.Seat;
            half.Rail.Avatar.Tapped += (_, e) =>
            {
                e.Handled = true;
                TapPlayer(seat);
            };
        }

        MakeTappable(_stack.Board, turned: false);
        if (_turnedStack is not null)
        {
            MakeTappable(_turnedStack.Board, turned: true);
        }

        _turnedInspector.RenderTransformOrigin = new Point(0.5, 0.5);
        _turnedInspector.RenderTransform = new RotateTransform { Angle = 180 };
        _turnedTooltip.RenderTransformOrigin = new Point(0.5, 0.5);
        _turnedTooltip.RenderTransform = new RotateTransform { Angle = 180 };
        foreach (FrameworkElement layer in (FrameworkElement[])[_arrows, _flights, _inspector, _turnedInspector, _tooltip, _turnedTooltip])
        {
            _root.Children.Add(layer);
        }

        _flights.FlyingChanged += () =>
        {
            foreach (CardBoardElement each in Boards())
            {
                each.Hidden = _flights.Flying;
            }
        };
        _inspector.HeldChanged += Conceal;
        _turnedInspector.HeldChanged += Conceal;
    }

    /// <summary>
    /// The screen over the board until the game begins: where the event stands, said once the right way up for the
    /// tablet's bottom edge and, on the shared board, again turned to face seat 2 across the table.
    /// </summary>
    private Border BuildPhaseScreen()
    {
        var both = new Grid();
        both.RowDefinitions.Add(new RowDefinition { Height = new GridLength(1, GridUnitType.Star) });
        if (_side == 0)
        {
            both.RowDefinitions.Add(new RowDefinition { Height = new GridLength(1, GridUnitType.Star) });
            StackPanel turned = PhaseText();
            turned.RenderTransformOrigin = new Point(0.5, 0.5);
            turned.RenderTransform = new RotateTransform { Angle = 180 };
            both.Children.Add(turned);
        }

        StackPanel upright = PhaseText();
        Grid.SetRow(upright, both.RowDefinitions.Count - 1);
        both.Children.Add(upright);
        return new Border { Background = Theme.Brush(Theme.Window), Child = both };
    }

    private StackPanel PhaseText()
    {
        static TextBlock Text(double size, Windows.UI.Color color) => new()
        {
            FontSize = size,
            Foreground = Theme.Brush(color),
            TextAlignment = TextAlignment.Center,
            HorizontalAlignment = HorizontalAlignment.Center,
            TextWrapping = TextWrapping.WrapWholeWords,
            MaxWidth = 720,
        };

        TextBlock title = Text(30, Theme.Fg);
        title.FontWeight = FontWeights.SemiBold;
        TextBlock line = Text(20, Theme.FgSoft);
        TextBlock connected = Text(16, Theme.FgMuted);
        _phaseTexts.Add((title, line, connected));
        return new StackPanel
        {
            Spacing = 14,
            VerticalAlignment = VerticalAlignment.Center,
            HorizontalAlignment = HorizontalAlignment.Center,
            Children = { title, line, connected },
        };
    }

    /// <summary>Every row of cards on this tablet.</summary>
    private IEnumerable<CardBoardElement> Boards() =>
        [.. _halves.Select(half => half.Board), _stack.Board, .. _turnedStack is null ? [] : (CardBoardElement[])[_turnedStack.Board], .. _halves.Select(half => half.Pile.Board)];

    /// <summary>Whether a row of cards on this tablet is turned to face seat 2 across the shared board.</summary>
    private bool IsTurned(CardBoardElement board) =>
        board == _turnedStack?.Board || _halves.Any(half => half.Turned && (half.Board == board || half.Pile.Board == board));

    /// <summary>The cards held up are not drawn where they lie meanwhile.</summary>
    private void Conceal()
    {
        foreach (TabletHalf half in _halves)
        {
            half.Board.Concealed = (half.Turned ? _turnedInspector : _inspector).Concealed;
            half.Pile.Board.Concealed = half.Board.Concealed;
        }

        _stack.Board.Concealed = _inspector.Concealed;
        if (_turnedStack is not null)
        {
            _turnedStack.Board.Concealed = _turnedInspector.Concealed;
        }
    }

    /// <summary>Takes in whatever the relay has sent, and draws the board again when anything changed.</summary>
    private void Frame()
    {
        foreach (DeviceServerMessage message in Receive())
        {
            Take(message);
        }

        FollowConnection();
        _feed.IsConnected = Connected;
        if (_client.Pump() > 0)
        {
            _needsRedraw = true;
            foreach (GameEventView entry in _client.RecentEvents)
            {
                if (entry is ObjectMovedEvent moved)
                {
                    _flights.Moved(moved);
                }
            }
        }

        if (_needsRedraw)
        {
            _needsRedraw = false;
            Redraw();
        }
    }

    /// <summary>One message of the device's wire.</summary>
    private void Take(DeviceServerMessage message)
    {
        switch (message)
        {
            case DeviceWelcomeMessage welcome:
                _names = welcome.PlayerNames;
                _needsRedraw = true;
                if (_connection is not null && welcome.ResumeId.Length > 0)
                {
                    DeviceMemory.Remember(DeviceConnection.AddressFor(ClientConfig.Page), welcome);
                }

                break;

            case DevicePhaseMessage phase:
                _phase = phase;
                _needsRedraw = true;
                break;

            case PublicFeedMessage feed:
                _feed.Feed(feed.Message);
                break;

            case DeviceErrorMessage error:
                _error = error.Detail.Length > 0 ? error.Detail : error.Code.ToString();
                _needsRedraw = true;
                break;

            // A seat's tap is a phone's business; a tablet is never sent one, and would do nothing with it.
            default:
                break;
        }
    }

    private IReadOnlyList<string> _names = [];
    private DevicePhaseMessage? _phase;
    private string? _error;

    private void ShowPhase(string? title, string line, IReadOnlyList<string> connected)
    {
        _phaseScreen.Visibility = Visibility.Visible;
        foreach ((TextBlock shownTitle, TextBlock shownLine, TextBlock shownConnected) in _phaseTexts)
        {
            shownTitle.Text = title ?? (_side == 0 ? "Cantrip · the table" : $"Cantrip · {SeatName(_side)}'s board");
            shownLine.Text = line;
            shownConnected.Text = connected.Count == 0 ? string.Empty : $"Connected: {string.Join(", ", connected)}";
        }
    }

    private string SeatName(int seat) =>
        _client.View is not null ? TabletLayout.NameOf(_client, seat)
        : seat >= 1 && seat <= _names.Count && _names[seat - 1].Length > 0 ? _names[seat - 1]
        : $"Player {seat}";

    private void Redraw()
    {
        if (_client.View is not { } view)
        {
            string line = _error is { } error ? $"The table refused this board: {error}"
                : _unreachable ? "The table cannot be reached. Scan this board's code again once its server is running."
                : !Connected && _welcomed ? "The connection to the table was lost."
                : _phase is { } phase ? PhaseLine(phase)
                : _welcomed ? "Waiting for the game to begin."
                : "Connecting to the table…";
            ShowPhase(null, line, _phase?.Connected ?? []);
            return;
        }

        _phaseScreen.Visibility = Visibility.Collapsed;
        _root.UpdateLayout();
        LayOutHalves();
        _stack.Show(_client.Zone(ZoneKindView.Stack, 0), TurnLine(view));
        _turnedStack?.Show(_client.Zone(ZoneKindView.Stack, 0), TurnLine(view));

        foreach (TabletHalf half in _halves)
        {
            if (view.Players.FirstOrDefault(player => player.Seat == half.Seat) is { } player)
            {
                UpdateRail(half, player, view);
            }

            half.Background = Theme.Brush(view.ActivePlayer == half.Seat ? Theme.HalfActive : Theme.HalfIdle);
            LayOutPile(half);
        }

        _lives.Text = string.Join("     ", view.Players.OrderBy(player => player.Seat).Select(player => $"{player.Name}  {player.Life}"));
        _root.UpdateLayout();
        Fly();
        PaintArrows();
    }

    /// <summary>What a phase message says, before the game.</summary>
    private static string PhaseLine(DevicePhaseMessage phase) => phase.Line.Length > 0 ? phase.Line : phase.Phase switch
    {
        TablePhase.Drafting => "Drafting.",
        TablePhase.DeckBuilding => "Building decks.",
        TablePhase.Playing => "The game is starting.",
        TablePhase.Finished => "The event is over.",
        _ => "Waiting for the players.",
    };

    /// <summary>Whose turn it is and in which step, or how the game ended.</summary>
    private string TurnLine(PlayerView view)
    {
        if (_client.Outcome is { } over)
        {
            string? winner = view.Players.FirstOrDefault(player => player.Team == over.WinningTeam && over.WinningTeam != 0)?.Name;
            return winner is null ? "Game over" : $"Game over · {winner} wins";
        }

        string turn = $"Turn {view.Turn} · {TabletLayout.NameOf(_client, view.ActivePlayer)}\n{StepName(view.Step)}";
        return Connected ? turn : $"{turn}\nReconnecting…";
    }

    private static string StepName(TurnStepView step) => step switch
    {
        TurnStepView.Untap => "Untap",
        TurnStepView.Upkeep => "Upkeep",
        TurnStepView.Draw => "Draw",
        TurnStepView.PrecombatMain => "Main 1",
        TurnStepView.BeginCombat => "Beginning of combat",
        TurnStepView.DeclareAttackers => "Declare attackers",
        TurnStepView.DeclareBlockers => "Declare blockers",
        TurnStepView.CombatDamage => "Combat damage",
        TurnStepView.EndOfCombat => "End of combat",
        TurnStepView.PostcombatMain => "Main 2",
        TurnStepView.End => "End step",
        TurnStepView.Cleanup => "Cleanup",
        _ => step.ToString(),
    };

    /// <summary>
    /// Both halves at one card size, the largest at which every half's rows fit its height and width; combat groups taken
    /// out of the creature rows, and the creatures left in them given the same width in both halves while there is a
    /// fight, so the groups between stand across from each other.
    /// </summary>
    private void LayOutHalves()
    {
        BoardRows.Tucked = card => TabletLayout.Tucked(_client, card);
        float height = (float)_halves.Min(half => half.Area.ActualHeight);
        if (height <= 0)
        {
            return;
        }

        float cardWidth = BoardRows.CardWidth(height);
        Dictionary<TabletHalf, BoardRows.Laid> laid = [];
        for (int attempt = 0; ; attempt++)
        {
            (Dictionary<int, List<BoardRows.CombatGroup>> groups, HashSet<ViewHandle> inCombat) = TabletLayout.Combat(_client, cardWidth);
            Dictionary<int, List<ObjectView>> rows = _client.View!.Players.ToDictionary(
                player => player.Seat,
                player => _client.BattlefieldShownOf(player.Seat).Where(card => !inCombat.Contains(card.Handle) && !TabletLayout.Nested(_client, card)).ToList());
            float aligned = groups.Count == 0 ? 0f : rows.Values.Max(row => BoardRows.CreaturesWidth(row, cardWidth, Look));

            laid.Clear();
            float fits = cardWidth;
            foreach (TabletHalf half in _halves)
            {
                float viewport = (float)half.Area.ActualWidth;
                float halfHeight = (float)half.Area.ActualHeight;
                laid[half] = BoardRows.Lay(
                    rows.GetValueOrDefault(half.Seat) ?? [],
                    top: false,
                    viewport,
                    halfHeight,
                    cardWidth,
                    Look,
                    card => TabletLayout.Spent(_client, card),
                    groups.GetValueOrDefault(half.Seat),
                    aligned);
                fits = MathF.Min(fits, TabletLayout.Fitting(laid[half], viewport, halfHeight, cardWidth));
            }

            if (fits >= cardWidth || attempt == 5)
            {
                break;
            }

            cardWidth = fits;
        }

        foreach ((TabletHalf half, BoardRows.Laid rows) in laid)
        {
            half.Show(rows, cardWidth);
        }
    }

    private static string Look(ObjectView card) => BoardRows.Likeness(card, string.Empty);

    /// <summary>A rail as anyone at the table may see it: no pile opens, no mana is spent from here.</summary>
    private void UpdateRail(TabletHalf half, PlayerSummaryView player, PlayerView view)
    {
        int seat = player.Seat;
        IReadOnlyList<ObjectView> graveyard = _client.Zone(ZoneKindView.Graveyard, seat);
        List<PlayerRail.ManaChipSpec> mana = [];
        for (int type = 0; type < player.Mana.Length; type++)
        {
            if (player.Mana[type] > 0)
            {
                mana.Add(new PlayerRail.ManaChipSpec((ManaTypeView)type, player.Mana[type], null, null, null));
            }
        }

        foreach (RestrictedManaView restricted in player.RestrictedMana)
        {
            mana.Add(new PlayerRail.ManaChipSpec(restricted.Type, restricted.Amount, "restricted", null, null));
        }

        half.Rail.Update(
            player,
            view.ActivePlayer == seat,
            view.PriorityPlayer == seat,
            new PlayerRail.PileSpec(_client.ZoneCount(ZoneKindView.Library, seat), null, null, false, null),
            new PlayerRail.PileSpec(
                _client.ZoneCount(ZoneKindView.Graveyard, seat),
                graveyard.Count > 0 ? graveyard[^1] : null,
                graveyard.Count > 0 ? () => TogglePile(half, ZoneKindView.Graveyard) : null,
                false,
                null),
            mana,
            ExileOf(seat) is { Count: > 0 } exiled
                ? [new PlayerRail.ZoneButtonSpec(
                    $"Exile  {exiled.Count}",
                    $"The cards {TabletLayout.NameOf(_client, seat)} owns in exile",
                    half.Pile.Open == (ZoneKindView.Exile, seat),
                    () => TogglePile(half, ZoneKindView.Exile))]
                : [],
            // A card in a hand that everyone at the table has been shown is face up in the fan.
            _client.NamedIn(ZoneKindView.Hand, seat),
            null);
    }

    /// <summary>The cards a seat owns in exile, as the public sees them: a card exiled face down is a back.</summary>
    private List<ObjectView> ExileOf(int seat) => [.. _client.Zone(ZoneKindView.Exile, 0).Where(card => card.Owner == seat)];

    /// <summary>What a pile holds now: a graveyard's cards bottom first, or a seat's cards in exile.</summary>
    private IReadOnlyList<ObjectView> PileCards(ZoneKindView zone, int owner) =>
        zone == ZoneKindView.Graveyard ? _client.Zone(ZoneKindView.Graveyard, owner) : ExileOf(owner);

    /// <summary>A tap on a seat's graveyard pile or exile button: its list opened over the half, or shut if it was open.</summary>
    private void TogglePile(TabletHalf half, ZoneKindView zone) =>
        ShowPile(half, half.Pile.Open == (zone, half.Seat) ? null : (zone, half.Seat));

    private void ShowPile(TabletHalf half, (ZoneKindView Zone, int Owner)? pile)
    {
        PutDown();
        half.Pile.Show(pile);
        _needsRedraw = true;
    }

    /// <summary>The open pile's cards laid out again, and the list shut once the pile is empty.</summary>
    private void LayOutPile(TabletHalf half)
    {
        if (half.Pile.Open is not { } open)
        {
            return;
        }

        IReadOnlyList<ObjectView> cards = PileCards(open.Zone, open.Owner);
        if (cards.Count == 0)
        {
            half.Pile.Show(null);
            return;
        }

        string whose = TabletLayout.NameOf(_client, open.Owner);
        half.Pile.Lay(open.Zone == ZoneKindView.Graveyard ? $"{whose}'s graveyard" : $"{whose}'s exile", cards);
    }

    /// <summary>Hands every card laid out now to the flight layer, which flies whatever the game moved.</summary>
    private void Fly()
    {
        List<CardFlights.Placed> placed = [];
        foreach (TabletHalf half in _halves)
        {
            Place(half.Board, half.Turned ? 180f : 0f);
        }

        Place(_stack.Board, 0f);

        _flights.Look(placed, (zone, owner) =>
        {
            TabletHalf? half = _halves.FirstOrDefault(each => each.Seat == owner);
            FrameworkElement? pile = (zone, half) switch
            {
                (_, null) => null,
                (ZoneKindView.Library, { } h) => h.Rail.LibraryPile,
                (ZoneKindView.Graveyard, { } h) => h.Rail.GraveyardPile,
                (ZoneKindView.Hand, { } h) => h.Rail.HandFan,
                _ => null,
            };

            if (pile is null || pile.ActualWidth <= 0 || pile.XamlRoot is null)
            {
                return null;
            }

            Point centre = pile.TransformToVisual(_root).TransformPoint(new Point(pile.ActualWidth / 2, pile.ActualHeight / 2));
            float width = pile is HandFanElement
                ? HandFanElement.CardWidthFor((float)pile.ActualWidth)
                : (float)Math.Min(pile.ActualWidth, pile.ActualHeight * 63 / 88);
            return new CardFlights.Pose((float)centre.X, (float)centre.Y, width, half!.Turned ? 180f : 0f);
        });

        void Place(CardBoardElement board, float turn)
        {
            if (board.XamlRoot is null || board.ActualWidth <= 0 || board.Visibility != Visibility.Visible)
            {
                return;
            }

            GeneralTransform toPage = board.TransformToVisual(_root);
            foreach ((ViewHandle handle, ObjectView card, float x, float y, float width, float angle) in board.Laid())
            {
                Point at = toPage.TransformPoint(new Point(x, y));
                placed.Add(new CardFlights.Placed(handle, card, new CardFlights.Pose((float)at.X, (float)at.Y, width, angle + turn), board, x, y));
            }
        }
    }

    /// <summary>
    /// Arrows from the top of the stack to what it is aimed at, where this tablet shows it: the public face of a spell
    /// says what it targets (CR 601.2c). A target no longer legal is left out (CR 608.2b).
    /// </summary>
    private void PaintArrows()
    {
        IReadOnlyList<ObjectView> stack = _client.Zone(ZoneKindView.Stack, 0);
        if (stack.Count == 0 || stack[^1].Targets.Count == 0 || Box(_stack.Board, stack[^1].Handle) is not { } upright)
        {
            _arrows.Show([]);
            return;
        }

        // On the shared board an arrow starts from the strip of the stack nearest what it points at.
        SKRect? turnedSource = _turnedStack is null ? null : Box(_turnedStack.Board, stack[^1].Handle);
        List<(SKPoint From, SKPoint To)> arrows = [];
        foreach (SpellTargetView target in stack[^1].Targets)
        {
            TabletHalf? playerHalf = target.IsPlayer ? _halves.FirstOrDefault(half => half.Seat == target.Player) : null;
            CardBoardElement? holder = target.IsPlayer ? null : Boards().Where(board => board != _stack.Board && board != _turnedStack?.Board)
                .FirstOrDefault(board => Box(board, target.Object) is not null);
            SKRect? box = !target.StillLegal ? null
                : playerHalf is { } half ? BoxOf(half.Rail.Avatar, new Rect(0, 0, half.Rail.Avatar.ActualWidth, half.Rail.Avatar.ActualHeight))
                : target.IsPlayer ? null
                : holder is not null ? Box(holder, target.Object) : Box(_stack.Board, target.Object);
            bool towardsTop = target.IsPlayer ? playerHalf?.Turned == true : holder is not null && IsTurned(holder);
            SKRect source = towardsTop && turnedSource is { } turned ? turned : upright;
            var from = new SKPoint(source.MidX, source.MidY);
            if (box is { } to)
            {
                arrows.Add((from, TargetArrowsElement.Edge(to, from)));
            }
        }

        _arrows.Show(arrows);
    }

    private SKRect? Box(CardBoardElement board, ViewHandle handle) =>
        board.Visibility == Visibility.Visible && board.Bounds(handle) is { } local ? BoxOf(board, local) : null;

    private SKRect? BoxOf(FrameworkElement element, Rect local)
    {
        if (element.ActualWidth <= 0 || element.XamlRoot is null)
        {
            return null;
        }

        Rect r = element.TransformToVisual(_arrows).TransformBounds(local);
        return SKRect.Create((float)r.X, (float)r.Y, (float)r.Width, (float)r.Height);
    }

    /// <summary>
    /// A row whose cards are tapped and held: a press let go where it started is a tap, sent to the relay; a press that
    /// rests holds the card up to read until it is let go. A right button holds a card up too, for a mouse.
    /// </summary>
    private void MakeTappable(CardBoardElement board, bool turned)
    {
        board.AddHandler(PointerPressedEvent, new PointerEventHandler((_, e) =>
        {
            Microsoft.UI.Input.PointerPoint point = e.GetCurrentPoint(board);
            if (point.Properties.IsRightButtonPressed || board.CardAtPoint(point.Position) is not { } handle)
            {
                return;
            }

            PutDown();
            _press = (board, handle, e.Pointer.PointerId, point.Position, false);
            board.CapturePointer(e.Pointer);
            _hold.Stop();
            _hold.Start();
            e.Handled = true;
        }), handledEventsToo: true);

        board.AddHandler(PointerMovedEvent, new PointerEventHandler((_, e) =>
        {
            if (_press is { Held: false } press && press.Board == board && press.Pointer == e.Pointer.PointerId)
            {
                Point at = e.GetCurrentPoint(board).Position;
                if (Math.Abs(at.X - press.At.X) > TapSlop || Math.Abs(at.Y - press.At.Y) > TapSlop)
                {
                    _hold.Stop();
                    _press = null;
                }
            }
        }), handledEventsToo: true);

        board.AddHandler(PointerReleasedEvent, new PointerEventHandler((_, e) =>
        {
            if (_press is not { } press || press.Board != board || press.Pointer != e.Pointer.PointerId)
            {
                return;
            }

            _hold.Stop();
            _press = null;
            board.ReleasePointerCapture(e.Pointer);
            if (press.Held)
            {
                PutDown();
            }
            else
            {
                TapCard(press.Handle);
            }

            e.Handled = true;
        }), handledEventsToo: true);

        board.AddHandler(PointerCanceledEvent, new PointerEventHandler((_, _) => Abandon(board)), handledEventsToo: true);
        board.AddHandler(PointerCaptureLostEvent, new PointerEventHandler((_, _) => Abandon(board)), handledEventsToo: true);

        // A mouse's right button holds a card up as on any table.
        board.CardInspected += (handle, box, angle) => Inspect(board, turned, handle, box, angle, null);
    }

    /// <summary>A press that ends without being let go over its row: neither a tap nor, any longer, a hold.</summary>
    private void Abandon(CardBoardElement board)
    {
        if (_press is { } press && press.Board == board)
        {
            _hold.Stop();
            _press = null;
            if (press.Held)
            {
                PutDown();
            }
        }
    }

    /// <summary>A press has rested on its card long enough: the card is held up to read.</summary>
    private void Hold()
    {
        _hold.Stop();
        if (_press is not { Held: false } press || press.Board.Bounds(press.Handle) is not { } box || press.Board.Placement(press.Handle) is not { } placed)
        {
            return;
        }

        _press = press with { Held = true };
        bool turned = IsTurned(press.Board);
        Inspect(press.Board, turned, press.Handle, box, placed.Angle, press.At);
    }

    private void Inspect(CardBoardElement board, bool turned, ViewHandle handle, Rect box, float angle, Point? at)
    {
        if (board.CardOf(handle) is not { } card || board.XamlRoot is null)
        {
            return;
        }

        CardInspector inspector = turned ? _turnedInspector : _inspector;
        CardTooltip tooltip = turned ? _turnedTooltip : _tooltip;
        inspector.Inspect(card, board.TransformToVisual(inspector).TransformBounds(box), angle);
        if (at is { } pressed)
        {
            tooltip.Rest(CardTooltip.TextFor(card, null), board.TransformToVisual(tooltip).TransformPoint(pressed));
        }
    }

    /// <summary>Puts down whatever card is held up.</summary>
    private void PutDown()
    {
        _inspector.Release();
        _turnedInspector.Release();
        _tooltip.Hide();
        _turnedTooltip.Hide();
    }

    /// <summary>A permanent or a spell tapped: its public handle, for the relay to give whoever is being asked.</summary>
    private void TapCard(ViewHandle handle) =>
        Send(new TableTapMessage { Sequence = _client.EventSequence, Object = handle, Player = 0 });

    /// <summary>A player tapped, by their box on their rail.</summary>
    private void TapPlayer(int seat) =>
        Send(new TableTapMessage { Sequence = _client.EventSequence, Object = ViewHandle.None, Player = seat });

    // --- The device's wire: the relay's /inperson/socket, or a test's feed. ---

    private Func<IReadOnlyList<DeviceServerMessage>>? _receive;
    private Action<DeviceClientMessage>? _send;
    private Func<bool>? _isConnected;

    /// <summary>Whether the relay has welcomed this tablet.</summary>
    private bool _welcomed;

    private bool Connected => _isConnected?.Invoke() ?? false;

    private void Connect()
    {
        if (FedByTest)
        {
            _receive = TestFeed;
            _send = message =>
            {
                if (message is TableTapMessage tap)
                {
                    _sent.Add(tap);
                }
            };
            _isConnected = () => true;
            return;
        }

        _ = ConnectToRelayAsync();
    }

    private IReadOnlyList<DeviceServerMessage> Receive()
    {
        IReadOnlyList<DeviceServerMessage> received = _receive?.Invoke() ?? [];
        _welcomed |= received.Any(message => message is DeviceWelcomeMessage);
        return received;
    }

    private void Send(DeviceClientMessage message) => _send?.Invoke(message);

    /// <summary>How many times a lost connection is tried again, and how long apart, before the tablet says it is lost.</summary>
    private const int Retries = 5;
    private static readonly TimeSpan RetryAfter = TimeSpan.FromSeconds(2);

    private DeviceConnection? _connection;
    private bool _connecting;

    /// <summary>Every try to reach the relay failed.</summary>
    private bool _unreachable;

    /// <summary>
    /// Connects to the relay's <c>/inperson/socket</c> beside this page, and says who this tablet is:
    /// the token from its QR code, and the resume id and proof of an earlier welcome in this tab, which the relay prefers
    /// while it still knows them - a reload, whose address no longer holds the token, comes back on those alone. A lost
    /// connection is tried again a few times; the relay sends the board whole again on a welcome.
    /// </summary>
    private async Task ConnectToRelayAsync()
    {
        if (_connecting)
        {
            return;
        }

        _connecting = true;
        Uri address = DeviceConnection.AddressFor(ClientConfig.Page);
        try
        {
            for (int attempt = 0; attempt <= Retries; attempt++)
            {
                if (attempt > 0)
                {
                    await Task.Delay(RetryAfter);
                }

                try
                {
                    DeviceConnection connection = await DeviceConnection.ConnectAsync(address);
                    (string? resumeId, byte[]? proof) = DeviceMemory.Recall(address);
                    connection.Send(DeviceConnection.Hello(device: InPersonConfig.Device, resumeId: resumeId, proof: proof));
                    _connection?.Dispose();
                    _connection = connection;
                    _receive = connection.Receive;
                    _send = connection.Send;
                    _isConnected = () => connection.IsConnected;
                    return;
                }
                catch (Exception error) when (error is System.Net.WebSockets.WebSocketException or InvalidOperationException or OperationCanceledException)
                {
                    // Not up, or not reachable: tried again after a moment.
                }
            }

            _unreachable = true;
        }
        finally
        {
            _connecting = false;
            _needsRedraw = true;
        }
    }

    /// <summary>A connection that was up and is not, and nothing refused this tablet: it is tried again.</summary>
    private void FollowConnection()
    {
        if (_connection is { IsConnected: false } && !_connecting && _error is null && _lost < Retries)
        {
            _lost++;
            _needsRedraw = true;
            _ = ConnectToRelayAsync();
        }
        else if (_connection is { IsConnected: true })
        {
            _lost = 0;
        }
    }

    private int _lost;

    /// <summary>
    /// This tab's place at the relay - the resume id and proof it was welcomed with - kept across a reload in
    /// sessionStorage, which lives as long as the tab, as a seat's membership is (<see cref="SeatMemory"/>).
    /// </summary>
    private static class DeviceMemory
    {
        private static System.Runtime.InteropServices.JavaScript.JSObject Storage =>
            System.Runtime.InteropServices.JavaScript.JSHost.GlobalThis.GetPropertyAsJSObject("sessionStorage")!;

        private static string Key(Uri relay) => $"cantrip-device:{relay}";

        public static (string? ResumeId, byte[]? Proof) Recall(Uri relay)
        {
            string[] parts = (Storage.GetPropertyAsString(Key(relay)) ?? string.Empty).Split('\n');
            if (parts.Length != 2 || parts[0].Length == 0)
            {
                return (null, null);
            }

            try
            {
                return (parts[0], Convert.FromBase64String(parts[1]));
            }
            catch (FormatException)
            {
                return (null, null);
            }
        }

        public static void Remember(Uri relay, DeviceWelcomeMessage welcome) =>
            Storage.SetProperty(Key(relay), $"{welcome.ResumeId}\n{Convert.ToBase64String(welcome.Proof)}");
    }
}
