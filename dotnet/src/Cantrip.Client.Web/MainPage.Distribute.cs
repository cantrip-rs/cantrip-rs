using Cantrip.Protocol;
using Microsoft.UI.Text;

namespace Cantrip.Client.Web;

/// <summary>
/// Looking at cards and saying where each one goes: scry (CR 701.22), surveil (CR 701.25), Ponder's "in any order",
/// Stock Up's "two into your hand and the rest on the bottom", and the order of cards put into a graveyard together (CR 404.3).
/// Shown as a labelled pile per destination above, and
/// below, the cards still to place, each with a button per pile that will take it. The answer is checked by the prompt itself - the same <c>CardDistribution</c> check the engine makes - so Done is
/// enabled exactly when it would be taken.
/// </summary>
public sealed partial class MainPage
{
    private void FillDistribute(GameClient client, DistributeCardsPromptMessage prompt)
    {
        int places = prompt.Destinations.Count;

        // One list per destination, held across redraws while the prompt is open.
        if (_selection.Placements is null || _selection.Placements.Count != places)
        {
            _selection.Placements = [.. prompt.Destinations.Select(_ => new List<int>())];
        }

        List<List<int>> piles = _selection.Placements;
        List<int> unplaced =
        [
            .. prompt.Cards
                .Select(card => card.Value)
                .Where(handle => !piles.Any(pile => pile.Contains(handle))),
        ];

        CardsDistributedAnswer answer = new()
        {
            Placements = [.. piles.Select(pile => pile.Select(handle => new ViewHandle(handle)).ToList())],
        };

        string? problem = prompt.Problem(answer);
        string source = client.Object(prompt.Source)?.Name ?? "A spell";

        Prompt(
            client.Prompt?.Presentation is null || GraveyardOrder(prompt) ? Headline(prompt, source) : $"{source}: place {prompt.Cards.Count} card{(prompt.Cards.Count == 1 ? "" : "s")}",
            unplaced.Count > 0
                ? places > 1
                    ? $"Choose where each card goes  ·  drag to reorder or move between piles  ·  {unplaced.Count} left"
                    : $"Choose {PlaceName(prompt.Destinations[0])} for each card in turn; the last one chosen goes {LastGoes(prompt.Destinations[0])}  ·  drag to reorder  ·  {unplaced.Count} left"
                : problem ?? "Every card has a place  ·  drag to reorder or move between piles");

        OpenChoices(
            $"{CardChoicePresentation.Origins(client, prompt.Cards, client.Prompt?.Presentation)}  ·  {prompt.Cards.Count} card{(prompt.Cards.Count == 1 ? string.Empty : "s")}",
            closable: false,
            prompt.Cards.Count);

        const float Separation = 10f;
        const float PileChrome = 42f;
        const float ButtonRoom = 36f;

        // Sized so the piles and the row fit the window together: no wider than the row of cards leaves room for, and
        // no taller than two cards fit in the height the window may take.
        float inner = ChoiceWidth() - 28f - 14f;
        int count = Math.Max(1, prompt.Cards.Count);
        float room = (float)_table.ActualHeight - 40f - 190f;
        float byHeight = MathF.Floor((room - PileChrome - ButtonRoom - Separation) / 2f * 63f / 88f);
        float byWidth = MathF.Floor((inner - ((count - 1) * ChoiceGap)) / count);
        float width = MathF.Max(60f, MathF.Min(ChoiceCardWidth(), MathF.Min(byHeight, byWidth)));
        float height = MathF.Round(width * 88f / 63f);

        var column = new StackPanel { Orientation = Orientation.Vertical, Spacing = Separation, Width = inner, HorizontalAlignment = HorizontalAlignment.Left };

        var spread = new Grid { ColumnSpacing = ChoiceGap };
        column.Children.Add(spread);

        float pileInner = ((inner - ((places - 1) * ChoiceGap)) / places) - 16f - 2f;
        for (int i = 0; i < places; i++)
        {
            spread.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
            (FrameworkElement pile, CardBoardElement board) = Pile(client, prompt.Destinations[i], piles[i], width, height, pileInner);
            PromptBoard(board);
            Grid.SetColumn(pile, i);
            spread.Children.Add(pile);

            // A pile takes any of the cards being placed that it would take by its button, dragged from the row below,
            // along the pile, or from another pile.
            CardDestinationMessage destination = prompt.Destinations[i];
            List<int> cards = piles[i];
            HashSet<int> looked = [.. prompt.Cards.Select(card => card.Value)];
            HashSet<int> accepts = [.. destination.Eligible.Select(card => card.Value)];
            MakeDraggable(board);
            _promptDropZones.Add(new DropZone(
                board,
                handle => looked.Contains(handle.Value)
                    && accepts.Contains(handle.Value)
                    && FitsGroups(destination, [.. cards.Where(card => card != handle.Value)], handle.Value),
                (handle, at) =>
                {
                    // Out of whichever pile it was in; into this one where it was dropped.
                    foreach (List<int> other in piles.Where(other => !ReferenceEquals(other, cards)))
                    {
                        other.Remove(handle.Value);
                    }

                    Place(cards, handle.Value, at);
                }));
        }

        // The cards still to place, as tall as a card with its buttons whether or not any are left, so the window does
        // not jump as the last one goes.
        var row = new StackPanel
        {
            Orientation = Orientation.Horizontal,
            Spacing = ChoiceGap,
            HorizontalAlignment = HorizontalAlignment.Center,
            MinHeight = height + ButtonRoom,
        };

        column.Children.Add(row);

        if (unplaced.Count == 0)
        {
            row.Children.Add(new TextBlock
            {
                Text = "Every card has a place.",
                FontSize = 12,
                Foreground = Theme.Brush(Theme.FgMuted),
                VerticalAlignment = VerticalAlignment.Center,
            });
        }

        foreach (int handle in unplaced)
        {
            var slot = new StackPanel { Orientation = Orientation.Vertical, Spacing = 4 };
            row.Children.Add(slot);

            if (client.Object(new ViewHandle(handle)) is { } card)
            {
                var face = new CardBoardElement(width, height) { Width = width, Height = height, HorizontalAlignment = HorizontalAlignment.Center };
                face.SetCells([new CardBoardElement.BoardCell(card, 0, 0, Tapped: false)]);
                MakeDraggable(face);
                slot.Children.Add(PromptBoard(face));
            }

            var choices = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 4, HorizontalAlignment = HorizontalAlignment.Center };
            slot.Children.Add(choices);

            for (int i = 0; i < places; i++)
            {
                CardDestinationMessage destination = prompt.Destinations[i];

                // A pile that will not take this card, or has no room left for it, offers no button for it.
                bool accepts = destination.Eligible.Any(eligible => eligible.Value == handle)
                    && piles[i].Count < destination.Max
                    && FitsGroups(destination, piles[i], handle);
                if (!accepts)
                {
                    continue;
                }

                List<int> pile = piles[i];
                int at = TopFirst(destination) ? 0 : pile.Count;

                var button = new DockButton(PlaceName(destination), primary: false) { Tooltip = PlaceTooltip(destination) };
                button.Clicked += () =>
                {
                    pile.Remove(handle);
                    pile.Insert(Math.Clamp(at, 0, pile.Count), handle);
                    _needsRedraw = true;
                    RedrawNow();
                };

                choices.Children.Add(button);
            }
        }

        _choiceCustom.Content = column;

        // The height the window would have given a row of cards is not this one's.
        float tall = PileChrome + height + Separation + height + ButtonRoom;
        _choiceTall = MathF.Min(tall, MathF.Max(room, height));

        if (problem is null)
        {
            _pass = () => Send(client, answer);
            AddButton("Done", () => Send(client, answer), primary: true).Tooltip = "Put the cards where the piles show" + Suffix(PassKey);
        }
        else
        {
            AddButton("Done", () => { }, enabled: false).Tooltip = problem;
        }

        if (piles.Any(pile => pile.Count > 0))
        {
            AddButton("Start over", () =>
            {
                foreach (List<int> pile in piles)
                {
                    pile.Clear();
                }

                _needsRedraw = true;
            });
        }

        if (prompt.MayShuffle)
        {
            AddButton("Shuffle instead", () => Send(client, new CardsDistributedAnswer { Shuffle = true }));
        }
    }

    /// <summary>A pile: a labelled box whose cards overlap left to right, and the row of them, which takes drops.</summary>
    private static (FrameworkElement Pile, CardBoardElement Board) Pile(GameClient client, CardDestinationMessage destination, List<int> cards, float width, float height, float room)
    {
        var inside = new StackPanel { Orientation = Orientation.Vertical, Spacing = 4 };

        var header = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 8 };
        inside.Children.Add(header);

        header.Children.Add(new TextBlock { Text = PileName(destination), FontSize = 12, FontWeight = FontWeights.SemiBold, Foreground = Theme.Brush(Theme.Fg) });
        header.Children.Add(new TextBlock { Text = Tally(destination, cards.Count), FontSize = 12, Foreground = Theme.Brush(Theme.FgMuted) });

        if (Hint(destination, cards.Count) is string hint)
        {
            header.Children.Add(new TextBlock { Text = $"·  {hint}", FontSize = 11, Foreground = Theme.Brush(Theme.FgSoft), VerticalAlignment = VerticalAlignment.Center });
        }

        // Overlapping by a little under half a card, closer still when that would run past the pile's edge.
        List<ObjectView> faces = [.. cards.Select(handle => client.Object(new ViewHandle(handle))).OfType<ObjectView>()];
        float step = MathF.Round(width * 0.55f);
        if (faces.Count > 1 && width + (step * (faces.Count - 1)) > room)
        {
            step = MathF.Max(0f, (room - width) / (faces.Count - 1));
        }

        var board = new CardBoardElement(width, height) { Height = height, Width = MathF.Max(0f, room), HorizontalAlignment = HorizontalAlignment.Left };
        board.SetCells([.. faces.Select((card, i) => new CardBoardElement.BoardCell(card, i * step, 0, Tapped: false))]);
        inside.Children.Add(board);

        return (new Border
        {
            Background = Theme.Brush(Theme.WithAlpha(Theme.Window, 0.55)),
            BorderBrush = Theme.Brush(Theme.ControlBorder),
            BorderThickness = new Thickness(1),
            CornerRadius = new CornerRadius(6),
            Padding = new Thickness(8, 6, 8, 6),
            Child = inside,
        }, board);
    }

    /// <summary>What the dock is called, in the words of the card doing it.</summary>
    private static string Headline(DistributeCardsPromptMessage prompt, string source)
    {
        int count = prompt.Cards.Count;

        if (GraveyardOrder(prompt))
        {
            return $"{count} cards go to your graveyard together: put them in any order";
        }

        return prompt.Destinations.Select(destination => destination.Kind).ToList() switch
        {
            [CardDestinationKindView.LibraryTop] => count == 1
                ? $"{source}: put the card on top of the library"
                : $"{source}: put all {count} cards on top of the library in any order",
            [CardDestinationKindView.LibraryBottom] => $"{source}: put {(count == 1 ? "it" : $"all {count}")} on the bottom in any order",
            [CardDestinationKindView.LibraryTop, CardDestinationKindView.LibraryBottom] => $"{source}: scry {count}",
            [CardDestinationKindView.LibraryTop, CardDestinationKindView.Graveyard] => $"{source}: surveil {count}",
            [CardDestinationKindView.BattlefieldFaceDown, CardDestinationKindView.Graveyard] => count == 1
                ? $"{source}: manifest dread — this card goes onto the battlefield face down"
                : $"{source}: manifest dread — choose one to put onto the battlefield face down, the other goes to your graveyard",
            _ => $"{source}: look at {count} and place {(count == 1 ? "it" : "them")}",
        };
    }

    private static string PileName(CardDestinationMessage destination) => destination.Kind switch
    {
        CardDestinationKindView.LibraryTop => "Top of library",
        CardDestinationKindView.LibraryBottom => "Bottom of library",
        CardDestinationKindView.Graveyard => "Graveyard",
        CardDestinationKindView.Hand => "Hand",
        CardDestinationKindView.BattlefieldFaceDown => "Face down, as a 2/2",
        _ => "Exile",
    };

    private static string PlaceName(CardDestinationMessage destination) => destination.Kind switch
    {
        CardDestinationKindView.LibraryTop => "Top",
        CardDestinationKindView.Graveyard when destination.Ordered => "On top",
        CardDestinationKindView.LibraryBottom => "Bottom",
        CardDestinationKindView.Graveyard => "Graveyard",
        CardDestinationKindView.Hand => "Hand",
        CardDestinationKindView.BattlefieldFaceDown => "Manifest",
        _ => "Exile",
    };

    /// <summary>
    /// The rules' question of the order of cards put into a graveyard together (CR 404.3): one ordered graveyard pile, which
    /// takes them all, the leftmost on top.
    /// </summary>
    private static bool GraveyardOrder(DistributeCardsPromptMessage prompt) =>
        prompt.Destinations is [{ Kind: CardDestinationKindView.Graveyard, Ordered: true }];

    /// <summary>Whether a pile's leftmost card is its top one, so a card placed there goes on top of those already in it.</summary>
    private static bool TopFirst(CardDestinationMessage destination) =>
        destination.Kind == CardDestinationKindView.LibraryTop
        || destination.Kind == CardDestinationKindView.Graveyard && destination.Ordered;

    private static string LastGoes(CardDestinationMessage destination) =>
        destination.Kind == CardDestinationKindView.LibraryBottom ? "on the bottom" : "on top";

    private static string PlaceTooltip(CardDestinationMessage destination) => destination.Kind switch
    {
        CardDestinationKindView.LibraryTop => "Put this card on top of the ones already there",
        CardDestinationKindView.LibraryBottom => "Put this card on the bottom, under the ones already there",
        CardDestinationKindView.Graveyard when destination.Ordered => "Put this card into your graveyard, on top of the ones already there",
        CardDestinationKindView.Graveyard => "Put this card into your graveyard",
        CardDestinationKindView.Hand => "Put this card into your hand",
        CardDestinationKindView.BattlefieldFaceDown =>
            "Put this card onto the battlefield face down as a 2/2 creature; if it is a creature card you can turn it face up for its mana cost",
        _ => "Exile this card",
    };

    private static string Tally(CardDestinationMessage destination, int placed)
    {
        string here = placed.ToString(System.Globalization.CultureInfo.InvariantCulture);

        return destination.Min == destination.Max
            ? $"{here} of {destination.Min}"
            : destination.Min > 0 ? $"{here}, at least {destination.Min}" : here;
    }

    /// <summary>Whether a pile taking one card of each kind can still count this card as a kind none of the others is.</summary>
    private static bool FitsGroups(CardDestinationMessage destination, IReadOnlyList<int> pile, int handle) =>
        destination.Groups.Count == 0
        || Cantrip.Core.Decisions.CardDistribution.Fits(
            [.. pile, handle],
            [.. destination.Groups.Select(group => (IReadOnlyList<int>)[.. group.Select(card => card.Value)])]);

    private static string? Hint(CardDestinationMessage destination, int placed) =>
        destination.GroupNames.Count > 0
            ? $"one of each: {string.Join(", ", destination.GroupNames)}"
            : !destination.Ordered || placed <= 1
            ? destination.Randomized && placed > 1 ? "in a random order" : null
            : TopFirst(destination) ? "leftmost on top" : "rightmost at the bottom";
}
