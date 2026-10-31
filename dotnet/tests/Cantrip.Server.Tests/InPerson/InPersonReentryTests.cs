using Cantrip.Protocol;
using Cantrip.Protocol.InPerson;
using Cantrip.Server.InPerson;
using Xunit;

namespace Cantrip.Server.Tests.InPerson;

/// <summary>
/// A phone's re-entry code (docs/in-person.md, "Full screen"): what brings a phone added to an iPhone's home screen - an
/// app with storage of its own, knowing nothing of the browser tab's ticket or welcome - back to its seat on its first
/// launch. Good once, revocable, compared in constant time, and traded for a fresh seat ticket the app keeps.
/// </summary>
public sealed class InPersonReentryTests
{
    private const string AliceTicket = "A11CE";
    private const string BobTicket = "B0B";

    private readonly List<int> _issued = [];
    private readonly InPersonRelay _relay;

    public InPersonReentryTests()
    {
        _relay = Relay(issue: true);
    }

    private InPersonRelay Relay(bool issue) => new(new InPersonRelayOptions
    {
        PlayerNames = ["Alice", "Bob"],
        SeatForTicket = ticket => ticket switch { AliceTicket => 1, BobTicket => 2, _ => 0 },
        IssueTicket = issue
            ? seat =>
            {
                _issued.Add(seat);
                return $"fresh-ticket-{seat}-{_issued.Count}";
            }
            : null,
    });

    private static List<DeviceServerMessage> Drain(RelayDevice device)
    {
        List<DeviceServerMessage> messages = [];
        while (device.Outbox.TryRead(out DeviceServerMessage? message)) messages.Add(message);
        return messages;
    }

    private static RelayDevice Join(InPersonRelay relay, DeviceHelloMessage hello)
    {
        RelayDevice device = relay.Attach();
        relay.Receive(device, hello);
        return device;
    }

    private RelayDevice Phone(string ticket, InPersonRelay? relay = null)
    {
        RelayDevice phone = Join(relay ?? _relay, new DeviceHelloMessage { Ticket = ticket });
        Assert.IsType<DeviceWelcomeMessage>(Assert.Single(Drain(phone)));
        return phone;
    }

    private string CodeFor(RelayDevice phone, InPersonRelay? relay = null)
    {
        (relay ?? _relay).Receive(phone, new ReentryRequestMessage());
        string code = Assert.IsType<ReentryCodeMessage>(Assert.Single(Drain(phone))).Code;
        Assert.Equal(64, code.Length);
        return code;
    }

    private static void AssertRefused(RelayDevice device)
    {
        Assert.Equal(ErrorCode.MembershipRefused, Assert.IsType<DeviceErrorMessage>(Assert.Single(Drain(device))).Code);
    }

    [Fact]
    public void A_code_brings_the_home_screen_app_to_the_seat_with_a_fresh_ticket_and_takes_the_tab_s_place()
    {
        RelayDevice tab = Phone(BobTicket);
        string code = CodeFor(tab);

        RelayDevice app = Join(_relay, new DeviceHelloMessage { Reentry = code });
        DeviceWelcomeMessage welcome = Assert.IsType<DeviceWelcomeMessage>(Assert.Single(Drain(app)));
        Assert.Equal((DeviceRole.Phone, 2), (welcome.Role, welcome.Seat));
        Assert.Equal("fresh-ticket-2-1", welcome.Ticket);
        Assert.Equal([2], _issued);
        Assert.Equal(32, welcome.Proof.Length);

        // The tab it came from is told its place was taken, and let go.
        AssertRefused(tab);
        Assert.True(tab.Outbox.Completion.IsCompleted);

        // The app resumes as any device does after that, with its own welcome's proof.
        _relay.Detach(app);
        RelayDevice again = Join(_relay, new DeviceHelloMessage { ResumeId = welcome.ResumeId, Proof = welcome.Proof });
        DeviceWelcomeMessage resumed = Assert.IsType<DeviceWelcomeMessage>(Assert.Single(Drain(again)));
        Assert.Null(resumed.Ticket);
        Assert.Equal([2], _issued);
    }

    [Fact]
    public void A_code_is_good_once()
    {
        string code = CodeFor(Phone(AliceTicket));
        RelayDevice first = Join(_relay, new DeviceHelloMessage { Reentry = code });
        Assert.IsType<DeviceWelcomeMessage>(Assert.Single(Drain(first)));

        RelayDevice second = Join(_relay, new DeviceHelloMessage { Reentry = code });
        AssertRefused(second);
        Assert.Equal([1], _issued);
    }

    [Fact]
    public void A_newer_code_revokes_the_one_before()
    {
        RelayDevice tab = Phone(AliceTicket);
        string older = CodeFor(tab);
        string newer = CodeFor(tab);
        Assert.NotEqual(older, newer);

        AssertRefused(Join(_relay, new DeviceHelloMessage { Reentry = older }));
        Assert.IsType<DeviceWelcomeMessage>(Assert.Single(Drain(Join(_relay, new DeviceHelloMessage { Reentry = newer }))));
    }

    [Fact]
    public void A_code_is_revoked_by_the_host_by_the_seat_s_ticket_taking_the_place_anew_and_by_the_table_closing()
    {
        string revoked = CodeFor(Phone(AliceTicket));
        Assert.True(_relay.Recognizes(new DeviceHelloMessage { Reentry = revoked }));
        _relay.RevokeReentry(1);
        Assert.False(_relay.Recognizes(new DeviceHelloMessage { Reentry = revoked }));
        AssertRefused(Join(_relay, new DeviceHelloMessage { Reentry = revoked }));

        // The setup page's code scanned again leaves no older credential working.
        string taken = CodeFor(Phone(BobTicket));
        Phone(BobTicket);
        AssertRefused(Join(_relay, new DeviceHelloMessage { Reentry = taken }));

        string closed = CodeFor(Phone(AliceTicket));
        _relay.CloseAll("over");
        Assert.False(_relay.Recognizes(new DeviceHelloMessage { Reentry = closed }));
        Assert.Empty(_issued);
    }

    [Fact]
    public void Asking_whether_a_hello_is_for_this_table_does_not_spend_its_code()
    {
        string code = CodeFor(Phone(AliceTicket));
        Assert.True(_relay.Recognizes(new DeviceHelloMessage { Reentry = code }));
        Assert.True(_relay.Recognizes(new DeviceHelloMessage { Reentry = code }));
        Assert.IsType<DeviceWelcomeMessage>(Assert.Single(Drain(Join(_relay, new DeviceHelloMessage { Reentry = code }))));
    }

    [Fact]
    public void Only_the_exact_code_for_a_seat_takes_that_seat()
    {
        string alice = CodeFor(Phone(AliceTicket));
        string bob = CodeFor(Phone(BobTicket));

        // Compared as fixed-length hashes of what was presented, against every phone's: a prefix, the code with a character
        // changed or added, another case, nothing at all - none is anything.
        foreach (string wrong in (string[])[alice[..63], alice[..32], alice + "0", Flip(alice), alice.ToLowerInvariant(), string.Empty, "0"])
        {
            Assert.False(_relay.Recognizes(new DeviceHelloMessage { Reentry = wrong }), wrong);
        }

        DeviceWelcomeMessage welcome = Assert.IsType<DeviceWelcomeMessage>(Assert.Single(Drain(Join(_relay, new DeviceHelloMessage { Reentry = bob }))));
        Assert.Equal(2, welcome.Seat);
        Assert.IsType<DeviceWelcomeMessage>(Assert.Single(Drain(Join(_relay, new DeviceHelloMessage { Reentry = alice }))));

        static string Flip(string code) => code[..^1] + (code[^1] == 'A' ? 'B' : 'A');
    }

    [Fact]
    public void A_tablet_a_device_not_yet_welcomed_and_a_relay_that_issues_no_tickets_get_no_code()
    {
        RelayDevice tablet = Join(_relay, new DeviceHelloMessage { Device = _relay.MintTablet(0) });
        Drain(tablet);
        _relay.Receive(tablet, new ReentryRequestMessage());
        Assert.Equal(ErrorCode.IllegalAnswer, Assert.IsType<DeviceErrorMessage>(Assert.Single(Drain(tablet))).Code);

        RelayDevice stranger = _relay.Attach();
        _relay.Receive(stranger, new ReentryRequestMessage());
        Assert.Equal(ErrorCode.MembershipRequired, Assert.IsType<DeviceErrorMessage>(Assert.Single(Drain(stranger))).Code);

        InPersonRelay plain = Relay(issue: false);
        RelayDevice phone = Phone(AliceTicket, plain);
        plain.Receive(phone, new ReentryRequestMessage());
        Assert.Equal(ErrorCode.IllegalAnswer, Assert.IsType<DeviceErrorMessage>(Assert.Single(Drain(phone))).Code);
    }

    [Fact]
    public void A_code_is_never_described()
    {
        string code = CodeFor(Phone(AliceTicket));
        Assert.DoesNotContain(code, new ReentryCodeMessage { Code = code }.Describe(), StringComparison.Ordinal);
        Assert.DoesNotContain(code, new DeviceHelloMessage { Reentry = code }.Describe(), StringComparison.Ordinal);
        Assert.DoesNotContain("fresh", new DeviceWelcomeMessage { Ticket = "fresh-ticket" }.Describe(), StringComparison.Ordinal);
    }
}
