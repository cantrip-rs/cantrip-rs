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
            Ticket = "ticket", Device = "device", ResumeId = "resume", Proof = [1, 2, 3], Reentry = "reentry",
        });

        Assert.Equal((InPersonProtocol.Version, PlayerView.ProtocolVersion), (hello.InPersonVersion, hello.ProtocolVersion));
        Assert.Equal(("ticket", "device", "resume"), (hello.Ticket, hello.Device, hello.ResumeId));
        Assert.Equal([1, 2, 3], hello.Proof);
        Assert.Equal("reentry", hello.Reentry);
    }

    [Fact]
    public void A_bare_hello_keeps_its_nulls()
    {
        DeviceHelloMessage hello = Device<DeviceHelloMessage>(new DeviceHelloMessage());

        Assert.Null(hello.Ticket);
        Assert.Null(hello.Device);
        Assert.Null(hello.ResumeId);
        Assert.Null(hello.Proof);
        Assert.Null(hello.Reentry);
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
            Role = DeviceRole.Tablet, Seat = 0, Side = 2, ResumeId = "resume", Proof = [4, 5], PlayerNames = ["Ann", "Bo"], Ticket = "ticket",
        });

        Assert.Equal((DeviceRole.Tablet, 0, 2, "resume"), (welcome.Role, welcome.Seat, welcome.Side, welcome.ResumeId));
        Assert.Equal([4, 5], welcome.Proof);
        Assert.Equal(["Ann", "Bo"], welcome.PlayerNames);
        Assert.Equal("ticket", welcome.Ticket);
        Assert.Null(Server<DeviceWelcomeMessage>(new DeviceWelcomeMessage()).Ticket);
    }

    [Fact]
    public void A_re_entry_request_and_its_code_round_trip()
    {
        Device<ReentryRequestMessage>(new ReentryRequestMessage());
        Assert.Equal("code", Server<ReentryCodeMessage>(new ReentryCodeMessage { Code = "code" }).Code);
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

    [Fact]
    public void A_table_line_round_trips()
    {
        TableLineMessage line = Device<TableLineMessage>(new TableLineMessage { Decision = 7, Line = 4, Take = true });

        Assert.Equal((7, 4, true), (line.Decision, line.Line, line.Take));
    }

    [Fact]
    public void A_seat_line_round_trips()
    {
        SeatLineMessage line = Server<SeatLineMessage>(new SeatLineMessage { Decision = 7, Line = 2, Take = false });

        Assert.Equal((7, 2, false), (line.Decision, line.Line, line.Take));
    }

    [Fact]
    public void A_public_grid_round_trips()
    {
        PublicGridMessage grid = Server<PublicGridMessage>(new PublicGridMessage
        {
            Decision = 5, Grid = 3, Grids = 18, Pick = 2, Active = 1,
            Places = [new ObjectView { Name = "Lightning Bolt", Known = true }, new ObjectView { Known = false }],
            Drafted = [6, 3],
            Recent = ["Bo took row 2 of grid 3: [Lightning Bolt]."],
            RecentCards = [[new LogReferenceView { Name = "Lightning Bolt" }]],
            LastLine = 1,
            PickLeftMs = 30_000,
        });

        Assert.Equal((5, 3, 18, 2, 1, 1, 30_000), (grid.Decision, grid.Grid, grid.Grids, grid.Pick, grid.Active, grid.LastLine, grid.PickLeftMs));
        Assert.Equal([("Lightning Bolt", true), (string.Empty, false)], grid.Places.Select(place => (place.Name, place.Known)));
        Assert.Equal([6, 3], grid.Drafted);
        Assert.Equal("Lightning Bolt", Assert.Single(Assert.Single(grid.RecentCards)).Name);
        Assert.Single(grid.Recent);
    }
}
