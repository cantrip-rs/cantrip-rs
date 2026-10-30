using Cantrip.Protocol;
using Cantrip.Protocol.InPerson;
using MessagePack;
using Xunit;

namespace Cantrip.Client.Tests;

/// <summary>Every in-person message survives the wire through its own union (docs/in-person.md).</summary>
public sealed class InPersonMessageTests
{
    private static T Device<T>(DeviceClientMessage message) where T : DeviceClientMessage =>
        Assert.IsType<T>(MessagePackSerializer.Deserialize<DeviceClientMessage>(MessagePackSerializer.Serialize(message)));

    private static T Server<T>(DeviceServerMessage message) where T : DeviceServerMessage =>
        Assert.IsType<T>(MessagePackSerializer.Deserialize<DeviceServerMessage>(MessagePackSerializer.Serialize(message)));

    [Fact]
    public void A_hello_round_trips()
    {
        DeviceHelloMessage hello = Device<DeviceHelloMessage>(new DeviceHelloMessage
        {
            Ticket = "ticket", Device = "device", ResumeId = "resume", Proof = [1, 2, 3],
        });

        Assert.Equal((InPersonProtocol.Version, PlayerView.ProtocolVersion), (hello.InPersonVersion, hello.ProtocolVersion));
        Assert.Equal(("ticket", "device", "resume"), (hello.Ticket, hello.Device, hello.ResumeId));
        Assert.Equal([1, 2, 3], hello.Proof);
    }

    [Fact]
    public void A_bare_hello_keeps_its_nulls()
    {
        DeviceHelloMessage hello = Device<DeviceHelloMessage>(new DeviceHelloMessage());

        Assert.Null(hello.Ticket);
        Assert.Null(hello.Device);
        Assert.Null(hello.ResumeId);
        Assert.Null(hello.Proof);
    }

    [Fact]
    public void A_table_tap_round_trips()
    {
        TableTapMessage tap = Device<TableTapMessage>(new TableTapMessage { Sequence = 42, Object = new ViewHandle(7), Player = 2 });

        Assert.Equal((42L, new ViewHandle(7), 2), (tap.Sequence, tap.Object, tap.Player));
    }

    [Fact]
    public void A_tap_result_round_trips()
    {
        TapResultMessage result = Device<TapResultMessage>(new TapResultMessage { TapId = 9, Used = true });

        Assert.Equal((9, true), (result.TapId, result.Used));
    }

    [Fact]
    public void A_welcome_round_trips()
    {
        DeviceWelcomeMessage welcome = Server<DeviceWelcomeMessage>(new DeviceWelcomeMessage
        {
            Role = DeviceRole.Tablet, Seat = 0, Side = 2, ResumeId = "resume", Proof = [4, 5], PlayerNames = ["Ann", "Bo"],
        });

        Assert.Equal((DeviceRole.Tablet, 0, 2, "resume"), (welcome.Role, welcome.Seat, welcome.Side, welcome.ResumeId));
        Assert.Equal([4, 5], welcome.Proof);
        Assert.Equal(["Ann", "Bo"], welcome.PlayerNames);
    }

    [Fact]
    public void A_public_feed_message_carries_a_server_message()
    {
        PublicFeedMessage feed = Server<PublicFeedMessage>(new PublicFeedMessage
        {
            Message = new WelcomeMessage { Seat = 0, PlayerNames = ["Ann", "Bo"] },
        });

        WelcomeMessage inner = Assert.IsType<WelcomeMessage>(feed.Message);
        Assert.Equal(0, inner.Seat);
        Assert.Equal(["Ann", "Bo"], inner.PlayerNames);

        PublicFeedMessage over = Server<PublicFeedMessage>(new PublicFeedMessage { Message = new GameOverMessage() });
        Assert.IsType<GameOverMessage>(over.Message);
    }

    [Fact]
    public void A_seat_tap_round_trips()
    {
        SeatTapMessage tap = Server<SeatTapMessage>(new SeatTapMessage { TapId = 3, RequestId = 11, Object = new ViewHandle(5), Player = 0 });

        Assert.Equal((3, 11, new ViewHandle(5), 0), (tap.TapId, tap.RequestId, tap.Object, tap.Player));
    }

    [Fact]
    public void A_phase_round_trips()
    {
        DevicePhaseMessage phase = Server<DevicePhaseMessage>(new DevicePhaseMessage
        {
            Phase = TablePhase.Drafting, Line = "Grid 3: Ann picks", Connected = ["Ann's phone", "Tablet"],
        });

        Assert.Equal((TablePhase.Drafting, "Grid 3: Ann picks"), (phase.Phase, phase.Line));
        Assert.Equal(["Ann's phone", "Tablet"], phase.Connected);
    }

    [Fact]
    public void An_error_round_trips()
    {
        DeviceErrorMessage error = Server<DeviceErrorMessage>(new DeviceErrorMessage { Code = ErrorCode.ProtocolMismatch, Detail = "v2" });

        Assert.Equal((ErrorCode.ProtocolMismatch, "v2"), (error.Code, error.Detail));
    }
}
