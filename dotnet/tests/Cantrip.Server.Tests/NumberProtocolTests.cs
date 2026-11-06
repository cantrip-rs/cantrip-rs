using Cantrip.Protocol;
using Cantrip.Server.Rules;
using MessagePack;
using Xunit;

namespace Cantrip.Server.Tests;

public sealed class NumberProtocolTests
{
    [Fact]
    public void Full_unsigned_range_and_nondefault_answer_survive_both_wire_encodings()
    {
        PromptPayload prompt = new ChooseNumberPromptMessage { Player = 2, Source = new ViewHandle(7), Least = 300, Most = uint.MaxValue, Heading = "Put counters" };
        var json = Assert.IsType<ChooseNumberPromptMessage>(ProtocolJson.Deserialize<PromptPayload>(ProtocolJson.Serialize(prompt)));
        Assert.Equal((300u, uint.MaxValue, "Put counters", 2, new ViewHandle(7)), (json.Least, json.Most, json.Heading, json.Player, json.Source));
        var packed = Assert.IsType<ChooseNumberPromptMessage>(MessagePackSerializer.Deserialize<PromptPayload>(MessagePackSerializer.Serialize(prompt)));
        Assert.Equal(json.Most, packed.Most);
        AnswerPayload answer = new NumberChosenAnswer { Value = uint.MaxValue };
        Assert.Equal(uint.MaxValue, Assert.IsType<NumberChosenAnswer>(ProtocolJson.Deserialize<AnswerPayload>(ProtocolJson.Serialize(answer))).Value);
        Assert.Equal(uint.MaxValue, Assert.IsType<NumberChosenAnswer>(MessagePackSerializer.Deserialize<AnswerPayload>(MessagePackSerializer.Serialize(answer))).Value);
        Assert.Equal(300u, Assert.IsType<NumberChosenAnswer>(prompt.Default()).Value);
    }
}
