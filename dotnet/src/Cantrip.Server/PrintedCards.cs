using Cantrip.Protocol;

namespace Cantrip.Server;

/// <summary>
/// The cards one viewer has been sent as printed, in the order sent: what a <see cref="StateMessage.Printed"/> carries. A
/// seat's deck comes with its welcome; every other card is sent the first time the viewer could see it, and all of them
/// again whenever the viewer is brought back up to date. Kept for each seat by a <see cref="GameSession"/>, and for the
/// public board by in-person play's relay (docs/in-person.md).
/// </summary>
public sealed class PrintedCards
{
    private readonly HashSet<string> _ids;
    private readonly List<ObjectView> _cards;

    /// <param name="sent">The cards the viewer already has, which are never sent again as newly seen: a seat's own deck.</param>
    public PrintedCards(IEnumerable<ObjectView>? sent = null)
    {
        _cards = [.. sent ?? []];
        _ids = [.. _cards.Select(card => card.CardId)];
    }

    /// <summary>Every card sent so far, in the order sent.</summary>
    public IReadOnlyList<ObjectView> Cards => _cards;

    /// <summary>
    /// The oracle of each card in a view that the viewer is seeing for the first time, noted as sent. A card is one the
    /// viewer may identify: a face-down card it may not look at carries no card id, and is not in it.
    /// </summary>
    /// <param name="oracle">A card as printed by its id, or null for one the rules do not know (<see cref="Rules.IRulesHost.Oracle"/>).</param>
    public List<ObjectView> NewlySeen(PlayerView view, Func<string, ObjectView?> oracle)
    {
        ArgumentNullException.ThrowIfNull(view);
        ArgumentNullException.ThrowIfNull(oracle);
        List<ObjectView> fresh = [];
        foreach (ObjectView seen in view.Objects)
        {
            if (seen.CardId.Length > 0 && _ids.Add(seen.CardId) && oracle(seen.CardId) is { } printed)
            {
                fresh.Add(printed);
                _cards.Add(printed);
            }
        }

        return fresh;
    }
}
