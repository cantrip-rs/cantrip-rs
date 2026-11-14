using Cantrip.Protocol;
using Xunit;

namespace Cantrip.Client.Tests;

/// <summary>
/// What a bot declares once its declaration before was taken back unpaid (CR 508.1j, 733.1): only attacks that cost
/// nothing, so a seat that answers alike each time does not make the same one forever.
/// </summary>
public sealed class AttackCostsTests
{
    private static readonly ViewHandle Elf = new(3);
    private static readonly ViewHandle Knight = new(4);

    private static DeclareAttackersPromptMessage Prompt(bool takenBack) => new()
    {
        Player = 1,
        Eligible = [Elf, Knight],
        AttackablePlayers = [2],
        Costs = [new AttackCostMessage { Attacker = Elf, TargetPlayer = 2, Nonmana = "sacrifice a land" }],
        CanPay = 0,
        TakenBack = takenBack,
    };

    private static List<AttackMessage> Both() =>
    [
        new AttackMessage { Attacker = Elf, TargetPlayer = 2 },
        new AttackMessage { Attacker = Knight, TargetPlayer = 2 },
    ];

    [Fact]
    public void Permanents_to_sacrifice_are_left_to_the_hosts_verdict()
    {
        Assert.Equal(2, AttackCosts.Within(Prompt(takenBack: false), Both(), 0).Count);
    }

    [Fact]
    public void After_a_declaration_taken_back_only_what_costs_nothing_is_declared()
    {
        AttackMessage kept = Assert.Single(AttackCosts.Within(Prompt(takenBack: true), Both(), int.MaxValue));
        Assert.Equal(Knight, kept.Attacker);
    }
}
