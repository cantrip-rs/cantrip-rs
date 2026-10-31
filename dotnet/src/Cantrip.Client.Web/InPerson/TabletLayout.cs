using Cantrip.Protocol;
using Microsoft.UI.Text;
using Microsoft.UI.Xaml.Media;

namespace Cantrip.Client.Web.InPerson;

/// <summary>
/// One seat's half of a tablet's board: the seat's rail - name, life, hand count, mana, library and graveyard - beside
/// its battlefield, laid out by <see cref="BoardRows"/> exactly as the remote client lays out a half, with the creatures
/// against the edge nearest the other player. The shared tablet turns seat 2's half round (<see cref="Turned"/>) so it
/// faces seat 2 across the table; every hit test and every place on the page goes through the turn, since the half is
/// laid out and hit-tested in its own frame.
/// </summary>
public sealed class TabletHalf : Grid
{
    public TabletHalf(int seat, bool turned)
    {
        Seat = seat;
        Turned = turned;
        Background = Theme.Brush(Theme.HalfIdle);
        ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });

        // The rail at its own size where the half is tall enough, and shrunk whole where it is not - the shared board's
        // halves are half as tall as a remote table's - so its piles and counts are never cut off.
        Rail = new PlayerRail(showsHandFan: true);
        Children.Add(new Border
        {
            Background = Theme.Brush(Theme.RailIdle),
            BorderBrush = Theme.Brush(Theme.Line),
            BorderThickness = new Thickness(0, 0, 1, 0),
            Child = new Viewbox
            {
                Stretch = Stretch.Uniform,
                StretchDirection = StretchDirection.DownOnly,
                VerticalAlignment = VerticalAlignment.Top,
                Child = Rail,
            },
        });

        Board = new CardBoardElement(BoardRows.StandardCardWidth, BoardRows.CardHeight(BoardRows.StandardCardWidth)) { Slides = true };
        Badges = new Canvas { IsHitTestVisible = false };

        // The rows never scroll on a tablet: a drag across a board would fight a tap on a card. They are fitted to the
        // half instead, and what still does not fit is clipped.
        Area = new Grid { Background = new SolidColorBrush(Microsoft.UI.Colors.Transparent) };
        Area.Children.Add(Board);
        Area.Children.Add(new ArrivalsElement(Board));
        Area.Children.Add(Badges);
        Area.SizeChanged += (_, e) => Area.Clip = new RectangleGeometry { Rect = new Windows.Foundation.Rect(0, 0, e.NewSize.Width, e.NewSize.Height) };
        Grid.SetColumn(Area, 1);
        Children.Add(Area);

        // A graveyard or exile opened from the rail lies over the battlefield, turned with the half.
        Pile = new TabletPile();
        Grid.SetColumn(Pile, 1);
        Children.Add(Pile);

        if (turned)
        {
            RenderTransformOrigin = new Windows.Foundation.Point(0.5, 0.5);
            RenderTransform = new RotateTransform { Angle = 180 };
        }
    }

    /// <summary>The seat whose half this is.</summary>
    public int Seat { get; }

    /// <summary>Turned half round, to face the player across the table from the tablet's own bottom edge.</summary>
    public bool Turned { get; }

    public PlayerRail Rail { get; }

    public CardBoardElement Board { get; }

    /// <summary>The piles' counts and the combat captions, over the board.</summary>
    public Canvas Badges { get; }

    /// <summary>The battlefield's part of the half, which the rows are fitted to.</summary>
    public Grid Area { get; }

    /// <summary>The seat's graveyard or exile, opened from its rail over the battlefield.</summary>
    public TabletPile Pile { get; }

    /// <summary>Lays the battlefield's rows out at a card width, as <see cref="BoardRows.Lay"/> places them.</summary>
    public void Show(BoardRows.Laid laid, float cardWidth)
    {
        Board.SetCellSize(cardWidth, BoardRows.CardHeight(cardWidth));
        Board.Width = laid.Width;
        Board.Height = Area.ActualHeight;
        Board.HorizontalAlignment = HorizontalAlignment.Left;
        Badges.Width = laid.Width;
        Badges.Height = Area.ActualHeight;

        Board.SetCells([.. laid.Cards.Select(placed => new CardBoardElement.BoardCell(
            placed.Card,
            placed.Left,
            placed.Top,
            placed.Card.Tapped,
            // A phased-out permanent is drawn faded, as on every table (CR 702.26b). Nothing else is dimmed or lit: a tablet
            // says nothing of anybody's prompt.
            Dimmed: placed.Card.PhasedOut))]);
        Board.Bands = laid.Bands;

        Badges.Children.Clear();
        foreach (BoardRows.Band band in laid.Bands)
        {
            if (band.Caption is { Length: > 0 } caption)
            {
                var text = new TextBlock
                {
                    Text = caption.ToUpperInvariant(),
                    FontSize = 12,
                    FontWeight = FontWeights.SemiBold,
                    Foreground = Theme.Brush(Theme.FromHex("#ee8f8a")),
                };

                Canvas.SetLeft(text, band.Left + 10);
                Canvas.SetTop(text, band.OpensDownward ? band.Top + 5 : band.Bottom - 21);
                Badges.Children.Add(text);
            }
        }

        foreach (BoardRows.Badge badge in laid.Badges)
        {
            Border pill = TabletLayout.PileCount(badge.Count);
            pill.Measure(new Windows.Foundation.Size(double.PositiveInfinity, double.PositiveInfinity));
            Canvas.SetLeft(pill, badge.GrowsLeft ? badge.X - pill.DesiredSize.Width : badge.X);
            Canvas.SetTop(pill, badge.Y);
            Badges.Children.Add(pill);
        }
    }
}

/// <summary>
/// How a tablet's board is worked out from the public view: what each permanent has tucked under it, the combat groups
/// across the line, and the card size the halves are fitted at. The remote client's own versions of these are its page's
/// and are not shared; these are the same reading of the view without anything a seat's prompt adds.
/// </summary>
public static class TabletLayout
{
    /// <summary>The smallest a card is drawn to fit a half: past this the half clips rather than shrink it further.</summary>
    private const float SmallestCard = 28f;

    /// <summary>Whether a permanent is attached to another one on the battlefield, and so is drawn tucked under it.</summary>
    public static bool Nested(GameClient client, ObjectView card) =>
        card.AttachedTo != ViewHandle.None && client.Object(card.AttachedTo) is { Zone: ZoneKindView.Battlefield };

    /// <summary>
    /// Everything tucked under a permanent, furthest back first: the cards it holds in exile until it leaves (CR 610.3),
    /// then each Aura and Equipment attached to it (CR 301.5c, 303.4) after whatever that holds in turn.
    /// </summary>
    public static IReadOnlyList<ObjectView> Tucked(GameClient client, ObjectView holder)
    {
        (_, ILookup<ViewHandle, ObjectView> held, ILookup<ViewHandle, ObjectView> worn) = Holdings(client);
        List<ObjectView> under = [.. held[holder.Handle]];
        foreach (ObjectView attached in worn[holder.Handle])
        {
            under.AddRange(Tucked(client, attached));
            under.Add(attached);
        }

        return under;
    }

    private static (PlayerView? View, ILookup<ViewHandle, ObjectView> Held, ILookup<ViewHandle, ObjectView> Worn) _holdings;

    /// <summary>What each permanent holds in exile and has attached to it, gathered once per view rather than once per permanent.</summary>
    private static (PlayerView? View, ILookup<ViewHandle, ObjectView> Held, ILookup<ViewHandle, ObjectView> Worn) Holdings(GameClient client)
    {
        if (!ReferenceEquals(_holdings.View, client.View) || _holdings.Held is null)
        {
            _holdings = (
                client.View,
                client.Zone(ZoneKindView.Exile, 0).Where(card => card.ExiledWith != ViewHandle.None).ToLookup(card => card.ExiledWith),
                client.Battlefield().Where(card => card.AttachedTo != ViewHandle.None).ToLookup(card => card.AttachedTo));
        }

        return _holdings;
    }

    /// <summary>A tapped land with nothing tucked under it, which joins the one pile of spent lands.</summary>
    public static bool Spent(GameClient client, ObjectView land) => land.Tapped && Tucked(client, land).Count == 0;

    /// <summary>
    /// The declared attacks and blocks, as combat groups for each seat's half: an attacker by whom it attacks in its
    /// controller's half, its blockers straight across in the other's, column for column; and the creatures they take
    /// out of the rows.
    /// </summary>
    public static (Dictionary<int, List<BoardRows.CombatGroup>> Groups, HashSet<ViewHandle> InCombat) Combat(GameClient client, float cardWidth)
    {
        Dictionary<int, List<BoardRows.CombatGroup>> halves = [];
        HashSet<ViewHandle> inCombat = [];
        if (client.View is not { } view)
        {
            return (halves, inCombat);
        }

        // Groups keep the order their target was first seen in.
        List<(string Label, List<(ObjectView Attacker, List<ObjectView> Blockers)> Attackers)> groups = [];
        foreach (AttackerView attacker in view.Combat.Attackers)
        {
            if (client.Object(attacker.Attacker) is not { } creature)
            {
                continue;
            }

            List<ObjectView> blockers = [.. (view.Combat.Blocks.FirstOrDefault(b => b.Attacker == attacker.Attacker)?.Blockers ?? [])
                .Select(client.Object).OfType<ObjectView>()];
            string label = attacker.TargetKind == AttackTargetKindView.Player
                ? NameOf(client, attacker.TargetPlayer)
                : client.Object(attacker.TargetPermanent)?.Name ?? "?";

            int index = groups.FindIndex(g => g.Label == label && g.Attackers[0].Attacker.Controller == creature.Controller);
            if (index < 0)
            {
                groups.Add((label, []));
                index = groups.Count - 1;
            }

            groups[index].Attackers.Add((creature, blockers));
            inCombat.Add(creature.Handle);
            inCombat.UnionWith(blockers.Select(blocker => blocker.Handle));
        }

        foreach ((string label, List<(ObjectView Attacker, List<ObjectView> Blockers)> attackers) in groups)
        {
            int attacking = attackers[0].Attacker.Controller;
            int defending = view.Players.Select(player => player.Seat).FirstOrDefault(seat => seat != attacking);

            // Unblocked twins share a pile; a blocked attacker stands alone across from its own blockers.
            List<(List<ObjectView> Attackers, List<ObjectView> Blockers)> columns = [];
            Dictionary<string, int> unblocked = [];
            foreach ((ObjectView creature, List<ObjectView> blockers) in attackers)
            {
                if (blockers.Count == 0)
                {
                    string look = BoardRows.Likeness(creature, string.Empty);
                    if (unblocked.TryGetValue(look, out int at))
                    {
                        columns[at].Attackers.Add(creature);
                        continue;
                    }

                    unblocked[look] = columns.Count;
                }

                columns.Add(([creature], blockers));
            }

            List<BoardRows.CombatColumn> offence = [];
            List<BoardRows.CombatColumn> defence = [];
            foreach ((List<ObjectView> fighters, List<ObjectView> blockers) in columns)
            {
                float blockersWidth = blockers.Sum(blocker => BoardRows.PileWidthOf([blocker], cardWidth))
                    + (BoardRows.BlockerGap * Math.Max(0, blockers.Count - 1));
                float width = MathF.Max(BoardRows.PileWidthOf(fighters, cardWidth), blockersWidth);
                offence.Add(new BoardRows.CombatColumn(width, fighters, Pile: true));
                defence.Add(new BoardRows.CombatColumn(width, blockers, Pile: false));
            }

            Of(attacking).Add(new BoardRows.CombatGroup($"Attacking {label}", offence));
            Of(defending).Add(new BoardRows.CombatGroup(null, defence));
        }

        return (halves, inCombat);

        List<BoardRows.CombatGroup> Of(int seat)
        {
            if (!halves.TryGetValue(seat, out List<BoardRows.CombatGroup>? list))
            {
                list = [];
                halves[seat] = list;
            }

            return list;
        }
    }

    /// <summary>A player's name as the view gives it, or "Player N".</summary>
    public static string NameOf(GameClient client, int seat) =>
        client.View?.Players.FirstOrDefault(player => player.Seat == seat)?.Name is { Length: > 0 } name ? name : $"Player {seat}";

    /// <summary>
    /// The card width at which a half's rows fit both its height and its width, from how they stand at this one: smaller
    /// in proportion to how far over they are, and at least a pixel smaller than this, so a few tries find it.
    /// </summary>
    public static float Fitting(BoardRows.Laid laid, float viewportWidth, float halfHeight, float cardWidth)
    {
        float fixedHeight = (2f * BoardRows.MarginY) + BoardRows.RowGap;
        float fixedWidth = 2f * BoardRows.MarginX;
        float byHeight = laid.Height <= halfHeight ? cardWidth : cardWidth * MathF.Max(0f, halfHeight - fixedHeight) / MathF.Max(1f, laid.Height - fixedHeight);
        float byWidth = laid.Width <= viewportWidth + 0.5f ? cardWidth : cardWidth * MathF.Max(0f, viewportWidth - fixedWidth) / MathF.Max(1f, laid.Width - fixedWidth);
        float fits = MathF.Floor(MathF.Min(byHeight, byWidth));
        return fits >= cardWidth ? cardWidth : MathF.Max(SmallestCard, MathF.Min(fits, cardWidth - 1f));
    }

    /// <summary>A pile's count: a small pill on the panel colour, edged, with "×3" in it.</summary>
    public static Border PileCount(int count) => Theme.Pill(new Border
    {
        Background = Theme.Brush(Theme.WithAlpha(Theme.Panel, 0.94)),
        BorderBrush = Theme.Brush(Theme.ControlBorder),
        BorderThickness = new Thickness(1),
        Padding = new Thickness(8, 2, 8, 2),
        Child = new TextBlock { Text = $"×{count}", FontSize = 12, FontWeight = FontWeights.SemiBold, Foreground = Theme.Brush(Theme.Fg) },
    });
}
