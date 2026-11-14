using Cantrip.Protocol;

namespace Cantrip.Client.Presentation;

/// <summary>
/// A priority prompt and everything that decides whether it is answered for the player or left for them: the board
/// as the dock's priority settings read it.
/// </summary>
/// <remarks>
/// Split out from the table so the order of the reasons to pass can be read, and tested, on its own. None of it is
/// a rule: passing is always a legal answer to a priority prompt, so the worst any of it can do is decline to act.
/// </remarks>
public readonly record struct PriorityBoard
{
    /// <summary>The question outstanding.</summary>
    public required PriorityPromptMessage Prompt { get; init; }

    /// <summary>The stack, bottom first, so the last entry is what resolves next.</summary>
    public required IReadOnlyList<ObjectView> Stack { get; init; }

    /// <summary>The seat this client plays.</summary>
    public required int Seat { get; init; }

    /// <summary>Whether the turn being played is this player's.</summary>
    public required bool MyTurn { get; init; }

    /// <summary>The step priority is being offered in.</summary>
    public required TurnStepView Step { get; init; }

    /// <summary>Whether attackers have been declared this combat.</summary>
    public required bool Attacked { get; init; }

    /// <summary>Whether the dock's Hold Priority box is ticked, or its key held right now.</summary>
    public required bool HoldingPriority { get; init; }

    /// <summary>Whether Pass Turn is ticked.</summary>
    public required bool PassTurn { get; init; }

    /// <summary>Whether Resolve All is still resolving something.</summary>
    public required bool ResolveAll { get; init; }

    /// <summary>Whether the player has asked to be stopped in this step of this turn.</summary>
    public required bool StopsAtStep { get; init; }

    /// <summary>Whether the dock's Auto Pass box is ticked.</summary>
    public required bool AutoPass { get; init; }

    /// <summary>What resolves next, or null on an empty stack.</summary>
    public ObjectView? Topmost => Stack.Count > 0 ? Stack[^1] : null;
}

/// <summary>Whether a priority prompt is passed for the player, and why.</summary>
public static class AutoPass
{
    /// <summary>
    /// Whether to pass priority for the player rather than ask them. The reasons are in the order they beat one
    /// another in, and the first that applies decides.
    /// </summary>
    public static bool Wanted(PriorityBoard board)
    {
        // A spell or ability of this player's is on top of the stack. Unless they are holding priority, the opponent
        // is asked next without the player having to say so; that is what casting means to almost everyone.
        //
        // It holds for as long as the top is theirs, not just for the prompt after they cast: once an opponent's
        // answer has resolved off the top, the player's own spell is still their own spell and asking them about it
        // again is the same question they have already answered. The chance to respond to the opponent falls out of
        // the same rule from the other side — the moment the opponent puts something on the stack the top is no
        // longer this player's, so they are asked, whatever the stops say.
        bool mineOnTop = board.Topmost is { } top && top.Controller == board.Seat;

        if (mineOnTop && !board.HoldingPriority)
        {
            return true;
        }

        // What the player asked for by name comes before the stops: it was ticked knowing the stops were there.
        if (board.PassTurn || board.ResolveAll)
        {
            return true;
        }

        // Holding priority over something of the player's own is the one thing the box is for, so it stops them
        // whatever the step says — a trick cast on an opponent's turn is held on to in a step nobody stops in.
        if (mineOnTop)
        {
            return false;
        }

        if (board.StopsAtStep)
        {
            return false;
        }

        // Every other reason to stop below is a window to respond in, which means nothing to a player with no response.
        if (board.AutoPass && !CanAct(board.Prompt))
        {
            return true;
        }

        // Something of another player's is waiting to resolve — the only thing left it can be. A window to respond to
        // it is what priority is for, so the step's stop setting does not apply: the player is asked whatever the
        // step. Passing here on their behalf is how an opponent's spell resolves before it has even been drawn.
        if (board.Topmost is not null)
        {
            return false;
        }

        // Being attacked is always worth a look, whatever the stops say: once attackers are declared, and again once
        // blockers are, the defender is asked, since those are the windows for a trick before damage.
        if (!board.MyTurn
            && board.Step is TurnStepView.DeclareAttackers or TurnStepView.DeclareBlockers
            && board.Attacked)
        {
            return false;
        }

        return true;
    }

    /// <summary>
    /// Whether the priority prompt offers the player anything but mana: a land to play, or a spell or other ability the
    /// server says they could pay for. Mana abilities alone are not something to do, only the means of doing it.
    /// </summary>
    public static bool CanAct(PriorityPromptMessage priority)
    {
        ArgumentNullException.ThrowIfNull(priority);

        return priority.Options.Any(option => option.Kind switch
        {
            ActionKindView.PlayLand => true,
            ActionKindView.CastSpell => option.Affordable,
            ActionKindView.ActivateAbility => option.Produces.Count == 0 && option.ProducesOfAnyOneColor == 0
                && option.ProducesOfAnyColorEach == 0 && option.ProducesAlternatives.Count == 0 && option.ColorChoices.Count == 0 && option.Affordable,
            ActionKindView.TurnFaceUp => option.Affordable,
            ActionKindView.TakeCompanion => option.Affordable,
            ActionKindView.Plot => option.Affordable,
            ActionKindView.Suspend => option.Affordable,
            ActionKindView.Foretell => option.Affordable,
            ActionKindView.UnlockDoor => option.Affordable,
            _ => false,
        });
    }
}
