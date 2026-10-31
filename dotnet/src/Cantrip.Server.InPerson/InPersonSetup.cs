using Cantrip.Datasets;
using Cantrip.Server.Events;

namespace Cantrip.Server.InPerson;

/// <summary>
/// What starting an in-person table takes, wherever it is hosted - the standalone server's setup page
/// (<see cref="InPersonHost"/>) or the bot's site (<see cref="InPersonTables"/>): the form's limits, the grid draft it
/// makes, the relay and credentials for its devices, and the devices' links.
/// </summary>
public static class InPersonSetup
{
    /// <summary>The most grids a draft lays out; the cube's size may allow fewer (<see cref="DraftPod.CreateGrid"/>).</summary>
    public const int MostPacks = 60;

    /// <summary>The longest name a seat is given.</summary>
    public const int LongestName = 32;

    /// <summary>A layout's key in the event's host data, so hosting it again lays out the same devices.</summary>
    internal const string LayoutKey = "inperson.layout";

    internal const string ClocksKey = "inperson.clocks";

    /// <summary>The cube lists in <paramref name="directories"/>, by name: the only cubes the form may name.</summary>
    public static List<string> Cubes(IEnumerable<string> directories) =>
        [.. directories.Where(Directory.Exists)
            .SelectMany(directory => Directory.EnumerateFiles(directory, "*.txt"))
            .Select(Path.GetFileNameWithoutExtension).OfType<string>()
            .Distinct(StringComparer.Ordinal).Order(StringComparer.OrdinalIgnoreCase)];

    /// <summary>Throws <see cref="ArgumentException"/> for a form outside its limits: two names, three or four devices, a cube of <paramref name="cubes"/>.</summary>
    public static void Validate(SetupRequest request, IReadOnlyCollection<string> cubes)
    {
        ArgumentNullException.ThrowIfNull(request);
        ArgumentNullException.ThrowIfNull(cubes);
        if (request.Names is not { Count: 2 } || request.Names.Any(name => !ValidName(name)))
            throw new ArgumentException($"Each seat needs a name of 1-{LongestName} printable characters.");
        if (request.Layout is not (3 or 4)) throw new ArgumentException("The devices are three or four.");
        if (request.Packs is < 1 or > MostPacks) throw new ArgumentException($"A grid draft lays out 1-{MostPacks} grids.");
        if (request.Cube is null || !cubes.Contains(request.Cube, StringComparer.Ordinal)) throw new ArgumentException($"There is no cube '{request.Cube}'.");
    }

    /// <summary>
    /// Makes the grid draft a validated form asks for, its players given <paramref name="playerIds"/> (seat 1's, then seat
    /// 2's), and returns the event's id.
    /// </summary>
    internal static string CreateEvent(EventStore store, ContentLibrary library, IReadOnlyList<string> cubeDirectories, SetupRequest request,
        (string First, string Second) playerIds, TimeProvider time)
    {
        string cube = CubeList.Find(request.Cube, [.. cubeDirectories]);
        CreatedEvent created = DraftPod.CreateGrid(store, library, new GridDraftSetup
        {
            Name = $"In person: {request.Cube}",
            Cube = request.Cube,
            CubeCards = CubeList.Load(cube),
            PickOrder = CubeList.PickOrder(cube),
            Packs = request.Packs,
            Players = [(playerIds.First, request.Names[0].Trim()), (playerIds.Second, request.Names[1].Trim())],
        }, time);
        return created.Record.Id;
    }

    /// <summary>
    /// The relay of a two-player table and its devices' credentials: a ticket per seat, judged by the event, and a token per
    /// tablet - one shared tablet for three devices, one a seat for four.
    /// </summary>
    internal static Seating Seat(DraftPod pod, int devices, TimeProvider time, Action<string> log)
    {
        List<(string PlayerId, string Name, int Seat)> players = [.. pod.Players.OrderBy(player => player.Seat)];
        Dictionary<int, string> tickets = players.ToDictionary(player => player.Seat, player => pod.IssueTicket(player.PlayerId));
        var relay = new InPersonRelay(new InPersonRelayOptions
        {
            PlayerNames = [.. players.Select(player => player.Name)],
            SeatForTicket = pod.SeatFor,
            TimeProvider = time,
            Log = log,
        });
        List<int> sides = devices == 4 ? [1, 2] : [0];
        List<(int Side, string Token)> tablets = [.. sides.Select(side => (side, relay.MintTablet(side)))];
        return new Seating(players, tickets, relay, tablets);
    }

    /// <summary>
    /// Each device's name and link, phones then tablets, in the relay's order (<see cref="InPersonRelay.Devices"/>). A phone's
    /// link names the table (<c>?inperson=phone&amp;server=...#ticket=T</c>); a tablet's, which is sent no table message,
    /// where card art is (<c>?inperson=tablet&amp;side=N&amp;art=...#device=D</c>). The credential is after the <c>#</c>
    /// either way, so it is in no request and no log.
    /// </summary>
    internal static List<DeviceLink> Links(Seating seating, Uri site, Uri table, Uri art)
    {
        string game = Uri.EscapeDataString(table.AbsoluteUri);
        string tabletArt = Uri.EscapeDataString(art.AbsoluteUri);
        string page = site.GetLeftPart(UriPartial.Path);
        return
        [
            .. seating.Players.Select(player => new DeviceLink($"{player.Name}'s phone",
                $"{page}?inperson=phone&server={game}#ticket={seating.Tickets[player.Seat]}")),
            .. seating.Tablets.Select(tablet => new DeviceLink(tablet.Side == 0 ? "The shared tablet" : $"{seating.Players[tablet.Side - 1].Name}'s tablet",
                $"{page}?inperson=tablet&side={tablet.Side}&art={tabletArt}#device={tablet.Token}")),
        ];
    }

    private static bool ValidName(string? name) =>
        name is not null && name.Trim().Length is >= 1 and <= LongestName && !name.Any(char.IsControl);

    internal sealed record Seating(
        IReadOnlyList<(string PlayerId, string Name, int Seat)> Players,
        IReadOnlyDictionary<int, string> Tickets,
        InPersonRelay Relay,
        IReadOnlyList<(int Side, string Token)> Tablets);
}

/// <summary>A device's name and its link, which holds its credential; its QR code drawn once.</summary>
internal sealed record DeviceLink(string Name, string Url)
{
    public string Qr { get; } = InPersonHost.QrSvg(Url);
}
