using System.Collections.Concurrent;
using System.Diagnostics;
using System.Text;
using System.Text.Json.Nodes;

namespace Cantrip.Advisor;

/// <summary>
/// The advisor's AI host, <c>cantrip-advisor-host</c> (<c>crates/cantrip-ai-host/src/bin/advisor.rs</c>): the AI host's
/// protocol (docs/ai-host.md) over its stdin and stdout, one reply per command in order, and an <c>AiResult</c> line of
/// its own for each answer asked for. Commands are written without waiting, so mirroring a game costs the table nothing;
/// each reply goes to the callback its command was sent with, each result to <see cref="Result"/>.
/// </summary>
public sealed class AdvisorAiHost : IDisposable
{
    public const string BinaryName = "cantrip-advisor-host";

    /// <summary>The environment variable naming the binary, when it is not where cargo builds it.</summary>
    public const string PathVariable = "CANTRIP_ADVISOR_HOST";

    private readonly Process _process;
    private readonly StreamWriter _input;
    private readonly ConcurrentQueue<Action<JsonNode?>?> _pending = new();
    private readonly Thread _reader;
    private readonly object _gate = new();
    private bool _disposed;

    private AdvisorAiHost(Process process, Action<string> log)
    {
        _process = process;
        _input = process.StandardInput;
        process.ErrorDataReceived += (_, line) =>
        {
            if (line.Data is not null)
            {
                log($"[{BinaryName}] {line.Data}");
            }
        };
        process.BeginErrorReadLine();
        _reader = new Thread(Read) { IsBackground = true, Name = BinaryName };
        _reader.Start();
    }

    /// <summary>Each <c>AiResult</c> line's object, on the reader's thread.</summary>
    public event Action<JsonNode>? Result;

    /// <summary>Whether the process has gone.</summary>
    public bool Exited => _process.HasExited;

    public static AdvisorAiHost Start(string binary, IReadOnlyList<string> arguments, string workingDirectory, Action<string> log)
    {
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
            WorkingDirectory = workingDirectory,
        };
        foreach (string argument in arguments)
        {
            info.ArgumentList.Add(argument);
        }

        Process process = Process.Start(info) ?? throw new InvalidOperationException($"{binary} could not be started.");
        return new AdvisorAiHost(process, log);
    }

    /// <summary><see cref="PathVariable"/>, else the freshest of the workspace's build output.</summary>
    public static string Locate(string root)
    {
        if (Environment.GetEnvironmentVariable(PathVariable) is { Length: > 0 } configured)
        {
            return configured;
        }

        string file = OperatingSystem.IsWindows() ? BinaryName + ".exe" : BinaryName;
        string? best = null;
        foreach (string profile in (string[])["cards", "release", "debug"])
        {
            string candidate = Path.Combine(root, "target", profile, file);
            if (File.Exists(candidate) && (best is null || File.GetLastWriteTimeUtc(candidate) > File.GetLastWriteTimeUtc(best)))
            {
                best = candidate;
            }
        }

        return best ?? throw new FileNotFoundException($"No {file} under {Path.Combine(root, "target")}: cargo build --profile cards -p cantrip-ai-host.");
    }

    /// <summary>Sends a command; its reply goes to <paramref name="reply"/>, on the reader's thread, if one is given.</summary>
    public void Send(string command, Action<JsonNode?>? reply = null)
    {
        lock (_gate)
        {
            ObjectDisposedException.ThrowIf(_disposed, this);
            if (_process.HasExited)
            {
                throw new InvalidOperationException($"{BinaryName} has exited with code {_process.ExitCode}.");
            }

            // Queued before it is written, so the reader never meets a reply it has no request for.
            _pending.Enqueue(reply);
            _input.Write(command);
            _input.Write('\n');
            _input.Flush();
        }
    }

    /// <summary>Sends a command and waits for its reply.</summary>
    public JsonNode? Call(string command, TimeSpan wait)
    {
        var reply = new TaskCompletionSource<JsonNode?>(TaskCreationOptions.RunContinuationsAsynchronously);
        Send(command, node => reply.TrySetResult(node));
        return reply.Task.Wait(wait)
            ? reply.Task.Result
            : throw new TimeoutException($"{BinaryName} did not reply to {command} in {wait.TotalSeconds:N0}s.");
    }

    private void Read()
    {
        StreamReader output = _process.StandardOutput;
        while (output.ReadLine() is { } line)
        {
            JsonNode? node;
            try
            {
                node = JsonNode.Parse(line);
            }
            catch (System.Text.Json.JsonException)
            {
                continue;
            }

            if (node?["AiResult"] is JsonNode result)
            {
                Result?.Invoke(result);
                continue;
            }

            if (_pending.TryDequeue(out Action<JsonNode?>? callback))
            {
                callback?.Invoke(node);
            }
        }

        while (_pending.TryDequeue(out Action<JsonNode?>? left))
        {
            left?.Invoke(null);
        }
    }

    public void Dispose()
    {
        lock (_gate)
        {
            if (_disposed)
            {
                return;
            }

            _disposed = true;
            try
            {
                _input.Dispose();
                if (!_process.WaitForExit(TimeSpan.FromSeconds(5)))
                {
                    _process.Kill(entireProcessTree: true);
                }
            }
            catch (Exception error) when (error is InvalidOperationException or IOException or System.ComponentModel.Win32Exception)
            {
                // Already gone.
            }
        }

        _reader.Join(TimeSpan.FromSeconds(5));
        _process.Dispose();
    }
}
