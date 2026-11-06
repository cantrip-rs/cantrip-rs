using Cantrip.Protocol;
using Xunit;

namespace Cantrip.Client.Tests;

public sealed class NumberChoiceTests
{
    [Theory]
    [InlineData("0", true)]
    [InlineData("65535", true)]
    [InlineData("4294967295", true)]
    [InlineData("4294967296", false)]
    [InlineData("-1", false)]
    [InlineData("1.5", false)]
    [InlineData("", false)]
    [InlineData("no", false)]
    public void Full_range_input_is_exact_and_never_clamped(string text, bool valid)
    {
        var prompt = new ChooseNumberPromptMessage { Least = 0, Most = uint.MaxValue };
        Assert.Equal(valid, NumberChoice.TryValue(prompt, text, out uint value));
        if (valid) Assert.Equal(uint.Parse(text), value);
    }

    [Fact]
    public void Bounds_and_steps_preserve_both_endpoints_without_overflow()
    {
        var prompt = new ChooseNumberPromptMessage { Least = 100, Most = uint.MaxValue };
        Assert.False(NumberChoice.TryValue(prompt, "99", out _));
        Assert.Null(NumberChoice.Step(prompt, "100", up: false));
        Assert.Equal(101u, NumberChoice.Step(prompt, "100", up: true));
        Assert.Null(NumberChoice.Step(prompt, uint.MaxValue.ToString(), up: true));
        Assert.Equal(uint.MaxValue - 1, NumberChoice.Step(prompt, uint.MaxValue.ToString(), up: false));
        Assert.Null(NumberChoice.Step(prompt, "invalid", up: true));
        Assert.Equal(100u, Assert.IsType<NumberChosenAnswer>(prompt.Default()).Value);
    }

    [Fact]
    public void Random_play_samples_unsigned_values_and_both_inclusive_endpoints()
    {
        // The random chooser itself is under test; no game position is reached by a seed.
        var bot = new RandomPlay(seed: 7);
        var wide = new ChooseNumberPromptMessage { Least = 0, Most = uint.MaxValue };
        var values = Enumerable.Range(0, 400).Select(_ => Assert.IsType<NumberChosenAnswer>(bot.Decide(wide)).Value).ToList();
        Assert.Contains(values, value => value > int.MaxValue);
        Assert.Contains(values, value => value < int.MaxValue);
        foreach (uint least in new[] { 0u, uint.MaxValue - 1 })
        {
            var edge = new ChooseNumberPromptMessage { Least = least, Most = least + 1 };
            var seen = Enumerable.Range(0, 400).Select(_ => Assert.IsType<NumberChosenAnswer>(bot.Decide(edge)).Value).ToHashSet();
            Assert.True(seen.SetEquals(new[] { least, least + 1 }));
        }
    }
}
