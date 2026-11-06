using Cantrip.Protocol;
using Xunit;
using static Cantrip.Client.Llm.Tests.Board;

namespace Cantrip.Client.Llm.Tests;

public sealed class NumberDecisionTests
{
    private static NumberQuestion Full => new() { Heading = "Choose a number", Least = 0, Most = uint.MaxValue };

    [Theory]
    [InlineData("0", 0u)]
    [InlineData("65535", 65535u)]
    [InlineData("4294967295", uint.MaxValue)]
    [InlineData("Keep all of them.\nAnswer: 4294967295", uint.MaxValue)]
    public void Scalar_replies_preserve_the_exact_unsigned_value_in_one_call(string reply, uint expected)
    {
        var model = new ScriptedModel(reply);
        var chooser = new ModelChooser(model, "You are playing Magic.");
        Assert.Equal(expected, chooser.ChooseNumber(Full));
        Assert.Equal(1, model.Usage.Calls); Assert.Equal(0, model.Usage.Retries);
        Assert.Contains("4294967295", model.Heard.Single(), StringComparison.Ordinal);
        Assert.True(model.Heard.Single().Length < 512);
    }

    [Theory]
    [InlineData("4294967296")]
    [InlineData("-1")]
    [InlineData("1.5")]
    [InlineData("1e3")]
    [InlineData("I think 123 is best.")]
    [InlineData("123\nnot an answer")]
    public void Invalid_scalar_replies_are_retried_without_clamping_or_reading_a_reasoning_number(string invalid)
    {
        var model = new ScriptedModel(invalid, "4294967295");
        Assert.Equal(uint.MaxValue, new ModelChooser(model, "You are playing Magic.").ChooseNumber(Full));
        Assert.Equal(2, model.Usage.Calls); Assert.Equal(1, model.Usage.Retries);
    }

    [Fact]
    public void Out_of_range_replies_stop_at_the_existing_two_attempt_limit()
    {
        var model = new ScriptedModel("299", "4294967295", "300");
        var chooser = new ModelChooser(model, "You are playing Magic.");
        Assert.Throws<NoChoiceException>(() => chooser.ChooseNumber(new NumberQuestion { Heading = "Put counters", Least = 300, Most = 65535 }));
        Assert.Equal(2, model.Usage.Calls); Assert.Equal(2, model.Usage.Retries);
    }

    [Fact]
    public void The_real_decision_path_keeps_source_bounds_and_the_exact_scalar_answer()
    {
        var model = new ScriptedModel("4294967295");
        var chooser = new ModelChooser(model, "You are playing Magic.") { Position = "The current board." };
        var prompt = new ChooseNumberPromptMessage { Player = Me, Source = Handle(2), Heading = "Put counters", Least = 300, Most = uint.MaxValue };
        var answer = Assert.IsType<NumberChosenAnswer>(new Decisions(chooser).Answer(Seated(Card(2, "Clockwork Beast", "{6}")), prompt));
        Assert.Equal(uint.MaxValue, answer.Value);
        Assert.Contains("Clockwork Beast", model.Heard.Single(), StringComparison.Ordinal);
        Assert.Contains("300", model.Heard.Single(), StringComparison.Ordinal);
        Assert.Contains("The current board.", model.Heard.Single(), StringComparison.Ordinal);
        Assert.Equal(string.Empty, chooser.Position);
    }

    [Fact]
    public void A_forced_scalar_answer_does_not_spend_a_model_call()
    {
        var model = new ScriptedModel();
        var chooser = new ModelChooser(model, "You are playing Magic.");
        var answer = Assert.IsType<NumberChosenAnswer>(new Decisions(chooser).Answer(Seated(),
            new ChooseNumberPromptMessage { Least = uint.MaxValue, Most = uint.MaxValue }));
        Assert.Equal(uint.MaxValue, answer.Value); Assert.Equal(0, model.Usage.Calls);
    }
}
