using Cantrip.Client.InPerson;
using Cantrip.Protocol;
using Cantrip.Protocol.InPerson;
using Cantrip.Server.InPerson;
using Cantrip.Server.Rules;
using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Hosting;
using Xunit;

namespace Cantrip.Server.Tests.InPerson;

/// <summary>
/// The relay's socket, <c>/inperson</c>, over real sockets: a phone and a tablet connect with <see cref="DeviceConnection"/>,
/// the client the in-person pages use, and a tap made on the tablet reaches the phone. On the wall clock because it is a
/// real server.
/// </summary>
[Trait("Category", "WallClock")]
public sealed class InPersonSocketTests
{
    private static readonly TimeSpan Patience = TimeSpan.FromSeconds(20);

    private static async Task<List<DeviceServerMessage>> UntilAsync(DeviceConnection connection, Func<List<DeviceServerMessage>, bool> done, List<DeviceServerMessage>? seen = null)
    {
        seen ??= [];
        DateTime deadline = DateTime.UtcNow + Patience;
        while (!done(seen))
        {
            if (DateTime.UtcNow > deadline)
                throw new TimeoutException($"Got only: {string.Join(" | ", seen.Select(message => message.Describe()))}; {connection.LastError}");
            seen.AddRange(connection.Receive());
            await Task.Delay(5);
        }

        return seen;
    }

    [Fact]
    public async Task A_tap_on_the_tablet_reaches_the_phone_over_the_relays_socket()
    {
        var relay = new InPersonRelay(new InPersonRelayOptions
        {
            PlayerNames = ["Alice", "Bob"],
            SeatForTicket = ticket => ticket == "A11CE" ? 1 : 0,
        });
        string token = relay.MintTablet(0);

        WebApplicationBuilder builder = WebApplication.CreateBuilder();
        builder.WebHost.UseUrls("http://127.0.0.1:0");
        await using WebApplication app = builder.Build();
        app.UseWebSockets();
        app.MapInPerson(relay);
        await app.StartAsync();
        Uri address = DeviceConnection.AddressFor(new Uri(app.Urls.First() + "/?inperson=tablet"));
        Assert.Equal("/inperson", address.AbsolutePath);
        Assert.Equal("ws", address.Scheme);

        using DeviceConnection phone = await DeviceConnection.ConnectAsync(address);
        phone.Send(DeviceConnection.Hello(ticket: "A11CE"));
        DeviceWelcomeMessage welcome = (await UntilAsync(phone, seen => seen.Count > 0)).OfType<DeviceWelcomeMessage>().Single();
        Assert.Equal(1, welcome.Seat);

        using DeviceConnection tablet = await DeviceConnection.ConnectAsync(address);
        tablet.Send(DeviceConnection.Hello(device: token));
        List<DeviceServerMessage> fed = await UntilAsync(tablet, seen => seen.OfType<DevicePhaseMessage>().Any(phase => phase.Connected.Count == 2));

        var inner = new PublicHost { Opening = PublicHost.Step([new RulesRequest(6, 1, new PriorityPromptMessage(), false, null)]) };
        new ObservedRulesHost(inner, relay).Start();
        await UntilAsync(tablet, seen => seen.OfType<PublicFeedMessage>().Any(feed => feed.Message is StateMessage), fed);

        tablet.Send(new TableTapMessage { Object = new(902) });
        SeatTapMessage tap = (await UntilAsync(phone, seen => seen.OfType<SeatTapMessage>().Any())).OfType<SeatTapMessage>().Single();
        Assert.Equal((6, new ViewHandle(102)), (tap.RequestId, tap.Object));

        // The phone's reload takes its place back with its proof, and the old line is closed behind it.
        using DeviceConnection reloaded = await DeviceConnection.ConnectAsync(address);
        reloaded.Send(DeviceConnection.Hello(resumeId: welcome.ResumeId, proof: welcome.Proof));
        await UntilAsync(reloaded, seen => seen.OfType<DeviceWelcomeMessage>().Any());
        await UntilAsync(phone, seen => seen.OfType<DeviceErrorMessage>().Any());
        DateTime deadline = DateTime.UtcNow + Patience;
        while (phone.IsConnected && DateTime.UtcNow < deadline) await Task.Delay(5);
        Assert.False(phone.IsConnected);

        await app.StopAsync();
    }
}
