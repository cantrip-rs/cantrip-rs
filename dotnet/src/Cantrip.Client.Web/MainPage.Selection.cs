using Cantrip.Protocol;
using Microsoft.UI.Xaml.Input;

namespace Cantrip.Client.Web;

/// <summary>
/// Picking creatures up to attack or block: a click on one, or a rectangle dragged out from an empty part of the
/// battlefield over several, and a click on what to attack or whom to block to send them all. A red arrow runs from each
/// creature picked up to the pointer; a press anywhere else on the battlefield puts them all down.
/// </summary>
public sealed partial class MainPage
{
    private SelectionOverlayElement _selectionOverlay = null!;

    /// <summary>Where the pointer last was, in the page's coordinates, for the arrows to point at.</summary>
    private Windows.Foundation.Point? _mouse;

    /// <summary>Where a press on an empty part of the battlefield began, while the button is held.</summary>
    private Windows.Foundation.Point? _dragFrom;

    /// <summary>The rectangle being dragged out, once the pointer has moved far enough for it to be one.</summary>
    private SkiaSharp.SKRect? _marquee;

    /// <summary>How far the pointer moves with the button down before a press is a drag rather than a click.</summary>
    private const double MarqueeDistance = 5;

    private void BuildSelection()
    {
        _selectionOverlay = new SelectionOverlayElement();
        Grid.SetRowSpan(_selectionOverlay, 3);
        Grid.SetColumnSpan(_selectionOverlay, 2);
        Root.Children.Add(_selectionOverlay);

        Root.AddHandler(PointerPressedEvent, new PointerEventHandler(OnSelectionPressed), handledEventsToo: true);
        Root.AddHandler(PointerMovedEvent, new PointerEventHandler(OnSelectionMoved), handledEventsToo: true);
        Root.AddHandler(PointerReleasedEvent, new PointerEventHandler(OnSelectionReleased), handledEventsToo: true);
        Root.AddHandler(PointerCaptureLostEvent, new PointerEventHandler((_, _) => EndDrag()), handledEventsToo: true);
    }

    /// <summary>Whether creatures are being picked: attackers or blockers are being declared and some creature may be.</summary>
    private bool Picking(GameClient client) => client.Prompt?.Prompt switch
    {
        DeclareAttackersPromptMessage attackers => attackers.Eligible.Count > 0,
        DeclareBlockersPromptMessage blockers => blockers.Eligible.Count > 0,
        _ => false,
    };

    /// <summary>The scrolling rows of the battlefield, which a press on an empty part of begins a rectangle.</summary>
    private IEnumerable<ScrollViewer> BattlefieldScrolls => [_opponentHalf.Scroll, _myHalf.Scroll];

    private void OnSelectionPressed(object sender, PointerRoutedEventArgs e)
    {
        if (_gameClient is not { } client || !Picking(client) || !e.GetCurrentPoint(Root).Properties.IsLeftButtonPressed)
        {
            return;
        }

        Windows.Foundation.Point at = e.GetCurrentPoint(Root).Position;
        _mouse = at;

        // A press on the advisor's panel, over the battlefield at the advisor's table, is the panel's (MainPage.Advisor.cs).
        if (AdvisorCovers(at))
        {
            return;
        }

        ScrollViewer? scroll = BattlefieldScrolls.FirstOrDefault(candidate => Covers(candidate, at));
        bool inHand = Covers(_handScroll, at);
        if (scroll is null && !inHand)
        {
            return;
        }

        // A creature or planeswalker that answers the click does so itself; every other press puts them all down.
        if (scroll is not null && ActingCardAt(at))
        {
            return;
        }

        if (_pending.Count > 0)
        {
            PutDown();
            RedrawNow();
        }

        // Or begins a rectangle, from a press where there is no card.
        if (scroll is not null && !OnBattlefieldCard(at))
        {
            _dragFrom = at;
            Root.CapturePointer(e.Pointer);
        }
    }

    private void OnSelectionMoved(object sender, PointerRoutedEventArgs e)
    {
        if (_gameClient is not { } client)
        {
            return;
        }

        Windows.Foundation.Point at = e.GetCurrentPoint(Root).Position;
        _mouse = at;

        if (_dragFrom is { } from)
        {
            if (!e.GetCurrentPoint(Root).Properties.IsLeftButtonPressed)
            {
                EndDrag();
                return;
            }

            if (_marquee is not null || Math.Abs(at.X - from.X) >= MarqueeDistance || Math.Abs(at.Y - from.Y) >= MarqueeDistance)
            {
                _marquee = new SkiaSharp.SKRect(
                    (float)Math.Min(from.X, at.X),
                    (float)Math.Min(from.Y, at.Y),
                    (float)Math.Max(from.X, at.X),
                    (float)Math.Max(from.Y, at.Y));
            }
        }

        if (_pending.Count > 0 || _marquee is not null)
        {
            PaintSelection(client);
        }
    }

    private void OnSelectionReleased(object sender, PointerRoutedEventArgs e)
    {
        if (_dragFrom is null)
        {
            return;
        }

        SkiaSharp.SKRect? swept = _marquee;
        Root.ReleasePointerCaptures();
        EndDrag();

        if (swept is { } box && _gameClient is { } client && Picking(client))
        {
            PickUpWithin(client, box);
            RedrawNow();
        }
    }

    private void EndDrag()
    {
        bool drawn = _marquee is not null;
        _dragFrom = null;
        _marquee = null;

        if (drawn && _gameClient is { } client)
        {
            PaintSelection(client);
        }
    }

    /// <summary>Whether a point falls within an element, in the page's coordinates.</summary>
    private bool Covers(FrameworkElement element, Windows.Foundation.Point at)
    {
        if (element.ActualWidth <= 0 || element.XamlRoot is null || element.Visibility != Visibility.Visible)
        {
            return false;
        }

        Windows.Foundation.Rect box = element.TransformToVisual(Root).TransformBounds(new Windows.Foundation.Rect(0, 0, element.ActualWidth, element.ActualHeight));
        return box.Contains(at);
    }

    /// <summary>The card of either battlefield row under a point, if one is.</summary>
    private (CardBoardElement Board, ViewHandle Card)? BattlefieldCardAt(Windows.Foundation.Point at)
    {
        foreach (CardBoardElement board in (CardBoardElement[])[_opponentHalf.Board, _myHalf.Board])
        {
            if (board.Visibility == Visibility.Visible && Covers(board, at)
                && board.TransformToVisual(Root).Inverse.TransformPoint(at) is var local
                && board.CardAtPoint(local) is { } card)
            {
                return (board, card);
            }
        }

        return null;
    }

    private bool OnBattlefieldCard(Windows.Foundation.Point at) => BattlefieldCardAt(at) is not null;

    /// <summary>Whether the card under a point answers a click: a creature to pick up, or something to send them at.</summary>
    private bool ActingCardAt(Windows.Foundation.Point at) =>
        BattlefieldCardAt(at) is { } found && _looks.TryGetValue(found.Card, out CardLook? look) && look.Clicked is not null;

    /// <summary>
    /// Picks up every creature that may be declared whose card the rectangle touches. Where there is only one thing to
    /// attack nothing is left to choose, so the attackers are sent at it as a click on each would.
    /// </summary>
    private void PickUpWithin(GameClient client, SkiaSharp.SKRect box)
    {
        SkiaSharp.SKRect? view = BoundsOf(_myHalf.Scroll, new Windows.Foundation.Rect(0, 0, _myHalf.Scroll.ActualWidth, _myHalf.Scroll.ActualHeight));
        List<int> inside = [];

        foreach (ObjectView card in client.BattlefieldOf(client.Seat))
        {
            int handle = card.Handle.Value;
            if (_eligible.Contains(handle)
                && !_attacking.ContainsKey(handle)
                && !_blocking.Exists(pair => pair.Blocker == handle)
                && !_blockQueue.Contains(handle)
                && BoundsOn(_myHalf.Board, card.Handle) is { } at
                && at.IntersectsWith(box)
                && (view is null || at.IntersectsWith(view.Value)))
            {
                inside.Add(handle);
            }
        }

        _pending.Clear();
        if (_blockQueue.Count == 0)
        {
            _blockingInto = 0;
        }

        if (client.Prompt?.Prompt is DeclareAttackersPromptMessage attackers && !ChoosesWhatToAttack(attackers))
        {
            foreach (int handle in inside)
            {
                if (AttackChoices(attackers, new ViewHandle(handle)) is { Count: > 0 } choices)
                {
                    _attacking[handle] = choices[0];
                }
            }
        }
        else
        {
            _pending.AddRange(inside);
        }

        _needsRedraw = true;
    }

    /// <summary>The arrows from the creatures picked up to the pointer, and the rectangle being dragged out.</summary>
    private void PaintSelection(GameClient client)
    {
        List<SkiaSharp.SKPoint> from = [];

        foreach (int handle in _pending)
        {
            if (BoundsOn(_myHalf.Board, new ViewHandle(handle)) is { } box)
            {
                from.Add(new SkiaSharp.SKPoint(box.MidX, box.MidY));
            }
        }

        SkiaSharp.SKPoint? pointer = from.Count > 0 && _mouse is { } at ? new SkiaSharp.SKPoint((float)at.X, (float)at.Y) : null;
        _selectionOverlay.Hold(from, pointer, _marquee);
    }

    /// <summary>
    /// The creatures just sent somewhere: their arrows run on to the edge of what they were sent at and draw themselves
    /// in there. Taken before the creatures move, since they leave their row for the combat.
    /// </summary>
    private void RetireArrows(IEnumerable<int> creatures, SkiaSharp.SKRect? target)
    {
        if (target is not { } box || !CardBoardElement.Animates)
        {
            return;
        }

        List<(SkiaSharp.SKPoint From, SkiaSharp.SKPoint To)> arrows = [];

        foreach (int handle in creatures)
        {
            if (BoundsOn(_myHalf.Board, new ViewHandle(handle)) is { } at)
            {
                var from = new SkiaSharp.SKPoint(at.MidX, at.MidY);
                arrows.Add((from, TargetArrowsElement.Edge(box, from)));
            }
        }

        _selectionOverlay.Retire(arrows);
    }

    /// <summary>Where an attack is aimed on the screen: a seat's box, or a planeswalker or battle's card.</summary>
    private SkiaSharp.SKRect? AttackBounds(GameClient client, AttackChoice choice) =>
        choice.Permanent.IsNone ? PlayerBounds(client, choice.Player) : CardBounds(choice.Permanent);
}
