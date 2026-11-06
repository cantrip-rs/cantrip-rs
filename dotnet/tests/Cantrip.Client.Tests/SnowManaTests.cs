using Cantrip.Client.Presentation;
using Cantrip.Protocol;
using MessagePack;
using Xunit;

namespace Cantrip.Client.Tests;

public sealed class SnowManaTests
{
    [Fact]
    public void Snow_allocation_choices_keep_their_exact_quality_symbol_and_wire_identity()
    {
        ActionOptionMessage ordinary = new() { Kind = ActionKindView.AssignMana, Produces = [ManaTypeView.Blue], Snow = true, OptionId = 1 };
        ActionOptionMessage snow = new() { Kind = ActionKindView.AssignMana, Produces = [ManaTypeView.Blue], Snow = true, PaySnow = true, OptionId = 2 };
        ActionOptionMessage plain = new() { Kind = ActionKindView.AssignMana, Produces = [ManaTypeView.Blue], OptionId = 3 };
        var prompt = new PayCostsPromptMessage { Remaining = "{U}{S}", RemainingColored = [ColorView.Blue], RemainingSnow = 1, Options = [ordinary, snow, plain] };
        var actions = new CardActions(prompt);
        Assert.Equal(new[] { ordinary, snow }, actions.AssignmentsFor(ManaTypeView.Blue, ManaSpendFilterView.None, ManaSpendEffectView.None, snow: true));
        Assert.Same(plain, actions.AssignFor(ManaTypeView.Blue, ManaSpendFilterView.None, ManaSpendEffectView.None));
        foreach (ActionOptionMessage option in prompt.Options)
        {
            TakeActionAnswer answer = AutoPlay.Assign(option);
            AnswerPayload back = MessagePackSerializer.Deserialize<AnswerPayload>(MessagePackSerializer.Serialize<AnswerPayload>(answer));
            var taken = Assert.IsType<TakeActionAnswer>(back);
            Assert.Equal(option.Snow, taken.Snow);
            Assert.Equal(option.PaySnow, taken.PaySnow);
            Assert.Equal(option.OptionId, taken.OptionId);
            Assert.Equal(ManaTypeView.Blue, taken.Mana);
        }
        Assert.True(AutoPlay.Owing(prompt));
        Assert.True(AutoPlay.Helps(prompt, snow));
        Assert.True(ManaOwed.Of(prompt)!.Any);
        Assert.Equal(1u, ManaOwed.Of(prompt)!.Snow);
        prompt.RemainingColored.Clear();
        Assert.True(AutoPlay.Owing(prompt));
        Assert.True(ManaOwed.Of(prompt)!.Any);
        Assert.False(AutoPlay.Helps(prompt, ordinary));
        Assert.True(AutoPlay.Helps(prompt, snow));
    }
}
