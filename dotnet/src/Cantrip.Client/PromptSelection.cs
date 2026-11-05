using Cantrip.Protocol;

namespace Cantrip.Client;

public readonly record struct AttackChoice(int Player, ViewHandle Permanent);

/// <summary>Unsubmitted card choices, independent of rendering and of the transport carrying the prompt.</summary>
public sealed class PromptSelection
{
    public List<int> Cards { get; } = [];
    public List<int> Modes { get; } = [];
    public List<int> Colors { get; } = [];
    public List<TargetMessage> Targets { get; } = [];
    public List<int> TriggerOrder { get; } = [];
    public Dictionary<int, AttackChoice> Attacking { get; } = [];
    /// <summary>
    /// The blocks declared so far, each a blocker and the attacker it blocks (handle values), in the order made: any set of
    /// pairs, a creature blocking more than one attacker included, as the rules' verdict allows (CR 509.1a-c).
    /// </summary>
    public List<(int Blocker, int Attacker)> Blocking { get; } = [];

    /// <summary>
    /// Creatures picked up to attack or to block but not yet sent anywhere: clicking what to attack, or whom to block,
    /// declares every one of them at once.
    /// </summary>
    public List<int> Pending { get; } = [];

    /// <summary>
    /// The attacker (handle value) the last click sent the creatures picked up at, while they go to it one at a time as the
    /// host's verdict on each declaration allows; 0 for none. Those the verdict never lets block it stay picked up.
    /// </summary>
    public int BlockingInto { get; set; }

    /// <summary>
    /// The creatures picked up when the attacker was clicked, on their way to it one at a time (<see cref="BlockingInto"/>):
    /// no longer picked up, so releasing the pointer puts nothing down. Those it never takes are picked up again.
    /// </summary>
    public List<int> BlockQueue { get; } = [];

    public Dictionary<(int Source, int Creature), int> DamageShares { get; } = [];
    /// <summary>A divided target's own share (CR 601.2d), keyed by candidate handle.</summary>
    public Dictionary<int, int> DividedShares { get; } = [];
    /// <summary>Shares assigned to players, keyed by seat rather than object handle.</summary>
    public Dictionary<int, int> DividedPlayerShares { get; } = [];
    /// <summary>Divided targets in the order they were selected, across players and objects.</summary>
    public List<TargetMessage> DividedOrder { get; } = [];

    public void SetDividedShare(TargetMessage target, int amount)
    {
        Dictionary<int, int> shares = target.IsPlayer ? DividedPlayerShares : DividedShares;
        int key = target.IsPlayer ? target.Player : target.Object.Value;
        bool Same(TargetMessage other) => target.IsPlayer == other.IsPlayer &&
            (target.IsPlayer ? target.Player == other.Player : target.Object == other.Object);
        if (amount <= 0)
        {
            shares.Remove(key);
            DividedOrder.RemoveAll(Same);
        }
        else
        {
            if (!shares.ContainsKey(key)) DividedOrder.Add(target);
            shares[key] = amount;
        }
    }
    /// <summary>The counters taken so far for a cost that removes them from among permanents (CR 118.3), by permanent and kind.</summary>
    public Dictionary<(int Object, string Counter), int> CounterRemovals { get; } = [];
    public ActionOptionMessage? Targeting { get; set; }

    /// <summary>
    /// The colour, or one per mana, already chosen for the mana ability being aimed (<see cref="Targeting"/>): a menu row names
    /// it before the permanents its cost taps are picked (Supportive Parents' two creatures, CR 605.1a), and the answer sent
    /// once they are carries it.
    /// </summary>
    public ColorView? TargetingColor { get; set; }
    public List<ColorView> TargetingColors { get; } = [];

    /// <summary>
    /// The options of one ability whose cost taps or sacrifices a permanent of the player's choosing (Urza, Lord High
    /// Artificer; Squandered Resources), held while that permanent is picked on the board. Nothing is activated yet.
    /// </summary>
    public List<ActionOptionMessage>? CostChoice { get; set; }
    public string NameTyped { get; set; } = string.Empty;

    /// <summary>
    /// The value of X, or how many times to kick, set on the page and not announced yet (CR 601.2b): stepped here, with
    /// nothing sent, and announced at once. Null until it is changed from the value the question was asked at.
    /// </summary>
    public int? Announced { get; set; }
    /// <summary>A number being typed for a resolving choice, kept verbatim until it is valid and submitted.</summary>
    public string? NumberTyped { get; set; }
    public List<List<int>>? Placements { get; set; }
    public bool CanPick(int limit) => Cards.Count < limit || limit == 1;

    /// <summary>
    /// The simultaneous triggers not yet put on the stack, in the order they triggered, which is the order the dock
    /// lays them out for the player to click.
    /// </summary>
    public IEnumerable<int> TriggersLeft(int count) =>
        Enumerable.Range(0, count).Where(index => !TriggerOrder.Contains(index));

    /// <summary>
    /// The order shown, taken as the answer: whatever has been placed already, then everything left in the order it
    /// is laid out. First in the list goes on the stack first and so resolves last (CR 603.3b), which is exactly what
    /// clicking them left to right would have sent.
    /// </summary>
    public List<int> TriggerOrderAsShown(int count) => [.. TriggerOrder, .. TriggersLeft(count)];

    public void Pick(int handle, int limit)
    {
        if (Cards.Remove(handle)) return;
        if (limit == 1) Cards.Clear();
        if (Cards.Count < limit) Cards.Add(handle);
    }
    public void Clear()
    {
        Cards.Clear();
        Modes.Clear();
        Colors.Clear();
        Targets.Clear();
        TriggerOrder.Clear();
        Attacking.Clear();
        Blocking.Clear();
        Pending.Clear();
        BlockingInto = 0;
        BlockQueue.Clear();
        DamageShares.Clear();
        DividedShares.Clear();
        DividedPlayerShares.Clear();
        DividedOrder.Clear();
        CounterRemovals.Clear();
        Targeting = null;
        TargetingColor = null;
        TargetingColors.Clear();
        CostChoice = null;
        NameTyped = string.Empty;
        Announced = null;
        NumberTyped = null;
        Placements = null;
    }
}
