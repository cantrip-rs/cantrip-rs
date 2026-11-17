using Cantrip.Protocol;
using Xunit;

namespace Cantrip.Client.Tests;

/// <summary>
/// A rejection is shown while it is about the question at hand (<see cref="GameClient.StandingError"/>): through the refused
/// question asked again, and no longer once another is asked. <see cref="GameClient.Errors"/> keeps every one.
/// </summary>
public sealed class StandingErrorTests
{
    private sealed class Fed : IGameTransport
    {
        public Queue<ServerMessage> Inbound { get; } = [];

        public bool IsConnected => true;

        public void Send(ClientMessage message)
        {
        }

        public IReadOnlyList<ServerMessage> Receive()
        {
            List<ServerMessage> messages = [.. Inbound];
            Inbound.Clear();
            return messages;
        }

        public void Dispose()
        {
        }
    }

    private static PromptMessage Priority(int requestId) => new() { RequestId = requestId, Prompt = new PriorityPromptMessage { Player = 1 } };

    [Fact]
    public void A_refusal_stands_through_its_question_asked_again_and_goes_with_the_next_question()
    {
        var transport = new Fed();
        var client = new GameClient(transport);
        transport.Inbound.Enqueue(Priority(7));
        client.Pump();
        Assert.Null(client.StandingError);

        client.Pass();
        transport.Inbound.Enqueue(new ErrorMessage { Code = ErrorCode.IllegalAnswer, Detail = "that is not an answer to this question" });
        transport.Inbound.Enqueue(Priority(7));
        client.Pump();
        Assert.Equal(ErrorCode.IllegalAnswer, client.StandingError?.Code);

        client.Pass();
        transport.Inbound.Enqueue(Priority(8));
        client.Pump();
        Assert.Null(client.StandingError);
        Assert.Single(client.Errors);
    }
}
