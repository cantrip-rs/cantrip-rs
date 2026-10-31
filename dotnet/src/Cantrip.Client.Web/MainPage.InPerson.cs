using Cantrip.Client.Web.InPerson;
using Cantrip.Protocol;
using Cantrip.Protocol.InPerson;

namespace Cantrip.Client.Web;

/// <summary>
/// A player's phone at an in-person table (docs/in-person.md, docs/web-client.md "In-person play"): this page, laid out
/// for a phone held upright, and the taps made on the tablets acted on here as clicks. Nothing in this file runs unless
/// the page was opened with <c>?inperson=phone</c>; every other page - the remote client above all - never reaches past
/// the first line of <see cref="AttachPhone"/>.
/// <para>
/// The phone holds what only its player may see and answers every prompt; the board lies on the tablets, where a target
/// or a mana source is tapped. The relay hands such a tap to this page as a <see cref="SeatTapMessage"/>, already in this
/// seat's handles, and this page does with it exactly what a click on that card or that player's box does here, then says
/// whether it was used.
/// </para>
/// </summary>
public sealed partial class MainPage
{
    /// <summary>How long a tap that arrived before the prompt it was routed to waits for that prompt to be drawn.</summary>
    private static readonly TimeSpan TableTapHold = TimeSpan.FromSeconds(1);

    /// <summary>How long the word that a tap did nothing stays up.</summary>
    private static readonly TimeSpan RefusalShown = TimeSpan.FromSeconds(1.6);

    /// <summary>A tap from a tablet, waiting its turn: the ones before it, or its prompt.</summary>
    private sealed record TableTap(SeatTapMessage Tap, DateTime Until, Action<bool> Report)
    {
        /// <summary>Whether it had to wait for its prompt rather than being acted on as it came.</summary>
        public bool Held { get; set; }
    }

    /// <summary>The taps from the tablets not yet acted on or let go, in the order they came.</summary>
    private readonly List<TableTap> _tableTaps = [];

    /// <summary>Every tap from a tablet this page settled, newest last, for the page's test state.</summary>
    private readonly List<(SeatTapMessage Tap, bool Used, bool Held)> _tableTapsSettled = [];

    private PhoneLink? _phoneLink;
    private DockButton _boardButton = null!;
    private bool _boardShown;
    private Border _refusal = null!;
    private TextBlock _refusalText = null!;
    private DateTime _refusalUntil;

    /// <summary>Whether this page is a phone at an in-person table.</summary>
    private static bool IsPhone => InPersonConfig.Role == InPersonRole.Phone;

    /// <summary>
    /// Makes this page a phone, if it was opened as one: laid out upright, and listening for the taps the tablets send it.
    /// Does nothing for any other page.
    /// </summary>
    private void AttachPhone()
    {
        if (!IsPhone)
        {
            return;
        }

        ApplyPhoneLayout();
        _phoneLink = new PhoneLink(TakeTableTap);

        var timer = new DispatcherTimer { Interval = TimeSpan.FromMilliseconds(30) };
        timer.Tick += (_, _) => TickPhone();
        timer.Start();
    }

    private void TickPhone()
    {
        _phoneLink?.Pump();
        SettleTableTaps();

        if (_refusal.Visibility == Visibility.Visible && DateTime.UtcNow >= _refusalUntil)
        {
            _refusal.Visibility = Visibility.Collapsed;
        }
    }

    // --- The layout ---

    /// <summary>
    /// The phone's layout, upright: the dock over the hand along the bottom, a bar over them with the Board button, and the
    /// table above showing this player's rail (life, mana pool, piles) with the stack and any choice floating over it. The
    /// whole board - both halves, as the remote client draws it - is behind the Board button: the tablets show the public
    /// board, and the phone's is for what only its owner may see on it (their face-down cards). The side panel starts
    /// closed and opens over the table rather than beside it.
    /// </summary>
    private void ApplyPhoneLayout()
    {
        // The dock and the hand one over the other, rather than side by side, under a bar of the phone's own.
        _bottom.ColumnDefinitions.Clear();
        _bottom.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
        _bottom.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
        _bottom.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
        foreach (FrameworkElement part in _bottom.Children.OfType<FrameworkElement>())
        {
            Grid.SetColumn(part, 0);
            Grid.SetRow(part, part == _dock ? 1 : 2);
        }

        _dock.BorderThickness = new Thickness(0, 1, 0, 1);
        _dock.HorizontalAlignment = HorizontalAlignment.Stretch;
        _bottom.SizeChanged += (_, e) => FitDock(e.NewSize.Width);

        _boardButton = new DockButton("Board", primary: false, fontSize: 13);
        Microsoft.UI.Xaml.Automation.AutomationProperties.SetName(_boardButton, "Board");
        _boardButton.Tooltip = "Show the whole board, as you may see it";
        _boardButton.Clicked += () => ShowBoard(!_boardShown);
        var bar = new StackPanel
        {
            Orientation = Orientation.Horizontal,
            Spacing = 8,
            Padding = new Thickness(12, 8, 12, 8),
            Children = { _boardButton },
        };
        Grid.SetRow(bar, 0);
        _bottom.Children.Add(bar);

        // Closed, without touching the setting the remote client keeps; opened, it lies over the table, since beside it a
        // phone's width would leave the table nothing.
        _sideOpen = false;
        _sideSlide.Stop();
        _sideSlot.Width = 0;
        _sideCorner.Visibility = Visibility.Visible;
        Grid.SetColumn(_sideSlot, 0);
        Grid.SetColumnSpan(_sideSlot, 2);
        _sideSlot.HorizontalAlignment = HorizontalAlignment.Right;

        BuildRefusal();
        ShowBoard(false);
        HideLongClocks();
    }

    /// <summary>
    /// A clock with more than this left is not shown on the phone: an in-person table plays with its clocks "off", which
    /// the server keeps as clocks too long to matter (days for the table, hours for deck building), not as none.
    /// </summary>
    private static readonly TimeSpan LongestClockShown = TimeSpan.FromDays(1);

    /// <summary>
    /// Hides every clock with more than <see cref="LongestClockShown"/> left: the match clocks on the rails, and the
    /// draft's, deck building's and the next round's lines, blanked as they are written.
    /// </summary>
    private void HideLongClocks()
    {
        _myHalf.Rail.HideClockOver = LongestClockShown;
        _opponentHalf.Rail.HideClockOver = LongestClockShown;

        foreach (TextBlock clock in (TextBlock[])[_eventClock, _gridClock, _deckClock])
        {
            BlankWhenLong(clock, () => _clockBy);
        }

        BlankWhenLong(_roundClock, () => _roundBy);
    }

    private static void BlankWhenLong(TextBlock clock, Func<DateTime?> by) =>
        clock.RegisterPropertyChangedCallback(TextBlock.TextProperty, (_, _) =>
        {
            if (clock.Text.Length > 0 && by() is { } until && until - DateTime.UtcNow > LongestClockShown)
            {
                clock.Text = string.Empty;
            }
        });

    /// <summary>The dock across the phone's whole width, whatever width the remote client's dock would take.</summary>
    private void FitDock(double width)
    {
        if (width > 0)
        {
            _dock.MinWidth = width;
            _dock.MaxWidth = width;
        }
    }

    /// <summary>Shows the whole board, or only this player's rail with the stack and the choices over the table.</summary>
    private void ShowBoard(bool shown)
    {
        _boardShown = shown;
        _boardButton.On = shown;

        // The opponent's half and the hairline under it take no height while hidden; this player's half keeps its rail.
        _table.RowDefinitions[0].Height = shown ? new GridLength(1, GridUnitType.Star) : new GridLength(0);
        _table.RowDefinitions[1].Height = shown ? new GridLength(1) : new GridLength(0);
        _opponentHalf.Panel.Visibility = shown ? Visibility.Visible : Visibility.Collapsed;
        _myHalf.Scroll.Visibility = shown ? Visibility.Visible : Visibility.Collapsed;

        _needsRedraw = true;
        RedrawNow();
    }

    // --- Taps from the tablets ---

    /// <summary>
    /// A tap made on a tablet and routed to this seat (<see cref="SeatTapMessage"/>): acted on once its prompt is the one on
    /// screen, after any taps that came before it. <paramref name="report"/> hears whether it was used.
    /// </summary>
    private void TakeTableTap(SeatTapMessage tap, Action<bool> report)
    {
        _tableTaps.Add(new TableTap(tap, DateTime.UtcNow + TableTapHold, report));
        SettleTableTaps();
    }

    /// <summary>
    /// Acts on the waiting taps in order: each as soon as its prompt is drawn, or let go unused once it is past or has
    /// waited <see cref="TableTapHold"/> for it. A tap waits behind the one before it, so two taps are never taken out of
    /// the order they were made in.
    /// </summary>
    private void SettleTableTaps()
    {
        while (_tableTaps.Count > 0)
        {
            TableTap first = _tableTaps[0];
            TableTapOutcome outcome = TryTableTap(first.Tap);
            if (outcome == TableTapOutcome.Waiting && DateTime.UtcNow < first.Until)
            {
                first.Held = true;
                return;
            }

            _tableTaps.RemoveAt(0);
            bool taken = outcome == TableTapOutcome.Used;
            _tableTapsSettled.Add((first.Tap, taken, first.Held));
            if (_tableTapsSettled.Count > 64)
            {
                _tableTapsSettled.RemoveAt(0);
            }

            // Said only for a tap this phone's own question was asking about and could not take. A tap routed to a question
            // already answered, or to one that never reached the page, is the relay's to pass on - to this seat's next
            // question or the other seat's, which may well use it - so the phone says nothing of it.
            if (outcome == TableTapOutcome.NoChoice)
            {
                Refuse();
            }

            first.Report(taken);
        }
    }

    /// <summary>What a tap from a tablet came to on this page.</summary>
    private enum TableTapOutcome
    {
        /// <summary>Its question has not been drawn here yet.</summary>
        Waiting,

        /// <summary>Acted on, as a click on the card or the player would be.</summary>
        Used,

        /// <summary>Its question is past, or the game is: not this phone's to use any more.</summary>
        Past,

        /// <summary>Its question is the one on screen, and what was tapped is no choice in it.</summary>
        NoChoice,
    }

    /// <summary>A tap from a tablet acted on as a click on this page, if its question is the one on screen.</summary>
    private TableTapOutcome TryTableTap(SeatTapMessage tap)
    {
        if (_gameClient is not { } client || client.IsOver)
        {
            return TableTapOutcome.Past;
        }

        // Answered, and the next question - perhaps this one asked again, as a payment is after each mana ability - is
        // on its way.
        if (client.Prompt is not { } prompt || prompt.RequestId < tap.RequestId)
        {
            return TableTapOutcome.Waiting;
        }

        if (prompt.RequestId > tap.RequestId)
        {
            return TableTapOutcome.Past;
        }

        // Only a prompt this page has read and drawn, so the tap does what a click on what is drawn would.
        RedrawNow();
        if (_needsRedraw)
        {
            return TableTapOutcome.Waiting;
        }

        Action? click = tap.Object.IsNone
            ? PlayerClick(client, tap.Player).Click
            : client.Object(tap.Object) is { } card ? LookOf(client, card).Clicked : null;
        if (click is null)
        {
            return TableTapOutcome.NoChoice;
        }

        // A menu the click opens - a land's colours, a card's abilities - opens in the middle of the phone, where the
        // remote client opens it under the pointer.
        _pointer = new Windows.Foundation.Point(Root.ActualWidth / 2, Root.ActualHeight / 2);
        _autoManaHeld = false;
        click();
        RedrawNow();
        return TableTapOutcome.Used;
    }

    /// <summary>The phone's word that a tap on the table did nothing: a line over the dock, and a buzz where the phone can.</summary>
    private void Refuse()
    {
        _refusalText.Text = "That tap on the table can't be used now.";
        _refusal.Visibility = Visibility.Visible;
        _refusalUntil = DateTime.UtcNow + RefusalShown;
        PhoneFeedback.Buzz();
    }

    private void BuildRefusal()
    {
        _refusalText = new TextBlock { FontSize = 13, Foreground = Theme.Brush(Theme.Fg), TextWrapping = TextWrapping.Wrap };
        _refusal = new Border
        {
            Background = Theme.Brush(Theme.WithAlpha(Theme.Panel, 0.96)),
            BorderBrush = Theme.Brush(Theme.WithAlpha(Theme.Danger, 0.6)),
            BorderThickness = new Thickness(1),
            CornerRadius = new CornerRadius(8),
            Padding = new Thickness(14, 8, 14, 8),
            Margin = new Thickness(16, 0, 16, 16),
            HorizontalAlignment = HorizontalAlignment.Center,
            VerticalAlignment = VerticalAlignment.Bottom,
            Visibility = Visibility.Collapsed,
            IsHitTestVisible = false,
            Child = _refusalText,
        };

        // Over the bottom of the table, just above the dock, where the player is looking.
        _table.Children.Add(_refusal);
        Grid.SetRow(_refusal, 2);
    }
}
