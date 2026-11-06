using Cantrip.Protocol;
using Cantrip.Server.Rules;
using MessagePack;
using Xunit;

namespace Cantrip.Server.Tests;

public sealed class RetargetProtocolTests
{
    [Fact]
    public void Keeping_the_original_target_survives_both_wire_encodings()
    {
        PromptPayload prompt = new ChooseTargetPromptMessage
        {
            Player = 1, Optional = true, Count = 1, KeepOriginal = true,
            Text = "Target 1 (Sea Sprite). Choose a new target or none to keep the current one.",
        };
        var json = Assert.IsType<ChooseTargetPromptMessage>(ProtocolJson.Deserialize<PromptPayload>(ProtocolJson.Serialize(prompt)));
        var packed = Assert.IsType<ChooseTargetPromptMessage>(MessagePackSerializer.Deserialize<PromptPayload>(MessagePackSerializer.Serialize(prompt)));
        Assert.True(json.KeepOriginal);
        Assert.True(packed.KeepOriginal);
        Assert.Equal(json.Text, packed.Text);
    }
}
