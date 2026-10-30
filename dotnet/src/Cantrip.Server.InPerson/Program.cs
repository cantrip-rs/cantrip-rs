using Cantrip.Server;
using Cantrip.Server.Events;
using Cantrip.Server.Host;
using Cantrip.Server.InPerson;

// In-person play's server (docs/in-person.md): two players at one table, a phone each and a tablet or two between them.
// tools\in-person.cmd builds the web client and starts this.
//
// Usage: cantrip-in-person --data PATH [--content PATH] [--client PATH] [--port N] [--table-port N] [--address IP]
//                          [--event ID] [--art-cache PATH]
//
// --data is the data directory the event is kept in (or CANTRIP_DATA). The site - the web client, the relay's socket
// and the setup page - is on --port (5200), on every interface; the setup page, http://localhost:5200/setup, only to
// this computer, since its QR codes seat whoever scans them. The table is on --table-port (7790) and card art on the
// port after it. --address is the address the devices reach this computer at, when the best guess is not it.
// --event ID hosts an event made before - a table stopped and started again - with fresh codes for every device.

if (EventStore.Configured(args) is not { } data)
{
    Console.Error.WriteLine("An event is kept in a data directory: pass --data PATH, or set CANTRIP_DATA.");
    return 1;
}

string? content = Arg("--content") ?? FindUp("content", "mtg");
if (content is null)
{
    Console.Error.WriteLine("Could not find a 'content' directory. Pass --content PATH pointing at the folder that holds the datasets.");
    return 1;
}

string? repository = FindUp(".git") is { } git ? Path.GetDirectoryName(git) : null;
string? client = Arg("--client") ?? (repository is null ? null : WebClientFiles.Built(repository));
if (client is null)
{
    Console.Error.WriteLine("There is no built web client to serve: run tools\\in-person.cmd, which builds it, or pass --client PATH.");
    return 1;
}

if (repository is not null && WebClientFiles.Stale(client, repository) is { } stale)
{
    Console.Error.WriteLine(stale);
}

await using InPersonHost host = await InPersonHost.StartAsync(new InPersonOptions
{
    ContentRoot = content,
    Store = new EventStore(data),
    ClientRoot = client,
    CubeDirectories = [.. new[] { FindUp(Path.Combine("docs", "cube-lists")), FindUp("cubes") }.OfType<string>()],
    Port = IntArg("--port", 5200),
    TablePort = IntArg("--table-port", 7790),
    Address = Arg("--address"),
    ArtCache = Arg("--art-cache"),
});

if (Arg("--event") is { } eventId)
{
    try
    {
        await host.HostEventAsync(eventId);
    }
    catch (Exception error) when (error is IOException or ArgumentException or InvalidOperationException)
    {
        Console.Error.WriteLine(error.Message);
        return 1;
    }
}

Console.WriteLine($"Set up the table at {host.Site}setup (on this computer). Ctrl+C to stop.");
Console.WriteLine("Allow the server through the firewall when asked: the phones and tablets reach it over the network.");

using var stopping = new CancellationTokenSource();
Console.CancelKeyPress += (_, eventArgs) =>
{
    eventArgs.Cancel = true;
    stopping.Cancel();
};

try
{
    await Task.Delay(Timeout.Infinite, stopping.Token);
}
catch (OperationCanceledException)
{
    Console.WriteLine(host.Pod is { } pod ? $"Stopping. Host event {pod.EventId} again with --event to carry on." : "Stopping.");
}

return 0;

string? Arg(string name)
{
    for (int i = 0; i < args.Length - 1; i++)
    {
        if (string.Equals(args[i], name, StringComparison.Ordinal)) return args[i + 1];
    }

    return null;
}

int IntArg(string name, int fallback) => int.TryParse(Arg(name), out int value) ? value : fallback;

// The nearest directory or file at this relative path above the executable, optionally holding a subdirectory.
static string? FindUp(string relative, string? holding = null)
{
    var directory = new DirectoryInfo(AppContext.BaseDirectory);
    while (directory is not null)
    {
        string candidate = Path.Combine(directory.FullName, relative);
        if (holding is null ? Directory.Exists(candidate) || File.Exists(candidate) : Directory.Exists(Path.Combine(candidate, holding)))
        {
            return candidate;
        }

        directory = directory.Parent;
    }

    return null;
}
