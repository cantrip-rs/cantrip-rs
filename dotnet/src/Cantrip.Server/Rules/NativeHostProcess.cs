using System.Diagnostics;
using System.Text;

namespace Cantrip.Server.Rules;

/// <summary>
/// One request line in, one reply line out: what <see cref="NativeRulesHost"/> talks to, so its logic can be tested
/// against a fake that answers from a script rather than against the real binary.
/// </summary>
public interface IHostTransport
{
    /// <summary>Sends one request, as a JSON line without its newline, and returns the one reply line.</summary>
    string Exchange(string request);
}

/// <summary>
/// The <c>cantrip-native-host</c> sidecar: one process per server, holding any number of games, spoken to over its
/// stdin and stdout one JSON object per line, one reply per request, in order (<c>docs/engine-design.md</c>). Its
/// stderr is its log, forwarded line by line.
/// <para>
/// Calls are serialised with a lock, because the protocol is: a reply belongs to the request before it, and two
/// tables on one server would otherwise read each other's. Found by <c>CANTRIP_NATIVE_HOST</c>, then beside the server
/// in the workspace's own build output, then on the path.
/// </para>
/// </summary>
public sealed class NativeHostProcess : IHostTransport, IDisposable
{
    /// <summary>
    /// The environment variable naming a file to append every exchange to, request line then reply line, each
    /// prefixed <c>&gt; </c> or <c>&lt; </c>: what a refusal is reproduced from, since the request that drew it is
    /// otherwise gone by the time the client reports it.
    /// </summary>
    public const string TraceVariable = "CANTRIP_NATIVE_TRACE";

    private const string BinaryName = "cantrip-native-host";

    /// <summary>The environment variable naming the binary, when it is not where cargo builds it.</summary>
    public const string PathVariable = "CANTRIP_NATIVE_HOST";

    private static readonly object SharedGate = new();
    private static NativeHostProcess? _shared;

    private readonly object _gate = new();
    private readonly Process _process;
    private readonly StreamWriter _input;
    private readonly StreamReader _output;
    private readonly Action<string> _log;
    private readonly StreamWriter? _trace;
    private bool _disposed;

    private NativeHostProcess(Process process, Action<string> log)
    {
        _process = process;
        _log = log;
        _input = process.StandardInput;
        _output = process.StandardOutput;
        _trace = Environment.GetEnvironmentVariable(TraceVariable) is { Length: > 0 } trace
            ? new StreamWriter(trace, append: true, new UTF8Encoding(encoderShouldEmitUTF8Identifier: false))
            : null;
        process.ErrorDataReceived += (_, line) =>
        {
            if (line.Data is not null)
            {
                _log(line.Data);
            }
        };
        process.BeginErrorReadLine();
    }

    /// <summary>Where the binary was started from.</summary>
    public string Path { get; private init; } = string.Empty;

    /// <summary>
    /// Where <see cref="Shared"/>'s log lines go: the server's log, once it has one to offer; standard error until then.
    /// Read when the shared sidecar starts, so it is set before the first game is dealt.
    /// </summary>
    public static Action<string>? SharedLog { get; set; }

    /// <summary>
    /// The command-line arguments <see cref="Shared"/> is started with (the host's <c>--ai-record DIR</c>, say: what
    /// the bot lab records training games with). Read when the shared sidecar starts, so set before the first game.
    /// </summary>
    public static IReadOnlyList<string> SharedArguments { get; set; } = [];

    /// <summary>
    /// Stops <see cref="Shared"/>, if it runs, by closing its input and waiting up to <paramref name="wait"/> for it to
    /// exit: a host recording games writes out the last of them once its input ends, which can take longer than the
    /// usual grace.
    /// </summary>
    public static void CloseShared(TimeSpan wait)
    {
        lock (SharedGate)
        {
            _shared?.Close(wait);
            _shared = null;
        }
    }

    /// <summary>
    /// The one sidecar this server runs, started on first use and kept for the process's life: every table of every
    /// event deals its games through it.
    /// </summary>
    public static NativeHostProcess Shared
    {
        get
        {
            lock (SharedGate)
            {
                if (_shared is null || _shared._disposed || _shared._process.HasExited)
                {
                    _shared = Start(log: SharedLog, arguments: SharedArguments);
                }

                return _shared;
            }
        }
    }

    /// <summary>
    /// Starts the sidecar: at <paramref name="path"/>, or wherever <see cref="Locate"/> finds it. Its log lines go to
    /// <paramref name="log"/>, or to standard error prefixed with the binary's name; <paramref name="arguments"/> are
    /// its command line.
    /// </summary>
    public static NativeHostProcess Start(string? path = null, Action<string>? log = null, IReadOnlyList<string>? arguments = null)
    {
        string binary = path ?? Locate();
        var info = new ProcessStartInfo(binary)
        {
            UseShellExecute = false,
            RedirectStandardInput = true,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            StandardInputEncoding = new UTF8Encoding(encoderShouldEmitUTF8Identifier: false),
            StandardOutputEncoding = Encoding.UTF8,
            StandardErrorEncoding = Encoding.UTF8,
            CreateNoWindow = true,

            // From the repository root when there is one, so the host finds its card data where the tools put it
            // (its card export) however the server was started.
            WorkingDirectory = RepositoryRoot() ?? System.IO.Path.GetDirectoryName(System.IO.Path.GetFullPath(binary)) ?? string.Empty,
        };

        foreach (string argument in arguments ?? [])
        {
            info.ArgumentList.Add(argument);
        }

        Process process = Process.Start(info)
            ?? throw new InvalidOperationException($"{binary} could not be started.");

        return new NativeHostProcess(process, log ?? (line => Console.Error.WriteLine($"[{BinaryName}] {line}")))
        {
            Path = binary,
        };
    }

    /// <summary>
    /// The binary to run: <see cref="PathVariable"/> when set; else the workspace's build output under
    /// the repository this server runs from; else the bare name, for the path to resolve.
    /// </summary>
    public static string Locate()
    {
        if (Environment.GetEnvironmentVariable(PathVariable) is { Length: > 0 } configured)
        {
            return configured;
        }

        string name = BinaryName;
        string file = OperatingSystem.IsWindows() ? name + ".exe" : name;
        if (RepositoryRoot() is { } root)
        {
            // The native workspace builds the host in its "cards" profile (release code, incremental) by default; the
            // freshest of what is there is the one that was just built.
            string[] profiles = ["cards", "release", "debug"];
            string? best = null;
            foreach (string profile in profiles)
            {
                string candidate = System.IO.Path.Combine(root, "target", profile, file);
                if (File.Exists(candidate) && (best is null || File.GetLastWriteTimeUtc(candidate) > File.GetLastWriteTimeUtc(best)))
                {
                    best = candidate;
                }
            }

            if (best is not null)
            {
                return best;
            }
        }

        return name;
    }

    /// <summary>The repository above this process, found by the engine's workspace at its root; null when the server runs installed.</summary>
    private static string? RepositoryRoot()
    {
        var directory = new DirectoryInfo(AppContext.BaseDirectory);
        while (directory is not null)
        {
            if (File.Exists(System.IO.Path.Combine(directory.FullName, "Cargo.toml")) && Directory.Exists(System.IO.Path.Combine(directory.FullName, "crates")))
            {
                return directory.FullName;
            }

            directory = directory.Parent;
        }

        return null;
    }

    public string Exchange(string request)
    {
        ArgumentNullException.ThrowIfNull(request);

        lock (_gate)
        {
            ObjectDisposedException.ThrowIf(_disposed, this);
            if (_process.HasExited)
            {
                throw new InvalidOperationException($"{BinaryName} has exited with code {_process.ExitCode}.");
            }

            _input.Write(request);
            _input.Write('\n');
            _input.Flush();
            Trace('>', request);

            string reply = _output.ReadLine()
                ?? throw new InvalidOperationException(
                    $"{BinaryName} closed its output without replying to {request}" + (_process.HasExited ? $" (exit code {_process.ExitCode})." : "."));

            Trace('<', reply);
            return reply;
        }
    }

    private void Trace(char direction, string line)
    {
        if (_trace is null)
        {
            return;
        }

        _trace.Write(direction);
        _trace.Write(' ');
        _trace.WriteLine(line);
        _trace.Flush();
    }

    public void Dispose() => Close(TimeSpan.FromSeconds(2));

    private void Close(TimeSpan wait)
    {
        lock (_gate)
        {
            if (_disposed)
            {
                return;
            }

            _disposed = true;
            _trace?.Dispose();
            try
            {
                // Closing its input is how the host is told there is nothing more; a host that does not take the hint
                // is stopped.
                _input.Dispose();
                if (!_process.WaitForExit(wait))
                {
                    _process.Kill(entireProcessTree: true);
                }
            }
            catch (Exception error) when (error is InvalidOperationException or System.ComponentModel.Win32Exception or IOException)
            {
                // Already gone, which is what was wanted.
            }

            _process.Dispose();
        }
    }
}
