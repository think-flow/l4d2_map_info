using System.Buffers;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using SteamDatabase.ValvePak;

namespace VpkInfo;

/*
 * 读取指定的vpk文件，获取其文件中的addoninfo.txt和missions/文件夹下的文本文件的内容
 */
public class Vpk : IDisposable
{
    private readonly Package _package;
    private readonly PackageEntry? _missionEntry;
    private readonly PackageEntry? _addoninfoEntry;

    public Vpk(string path)
    {
        ArgumentNullException.ThrowIfNull(path);
        if (!File.Exists(path))
        {
            throw new FileNotFoundException(path);
        }

        _package = new Package();
        _package.Read(path);

        var entries = _package.Entries;
        if (entries is null)
        {
            throw new Exception("No entries found");
        }

        if (!entries.TryGetValue("txt", out var txtEntries)) return;

        foreach (var entry in txtEntries)
        {
            if (entry.DirectoryName.Equals("missions", StringComparison.OrdinalIgnoreCase))
            {
                _missionEntry = entry;
            }
            else if (entry.FileName.Equals("addoninfo", StringComparison.OrdinalIgnoreCase))
            {
                _addoninfoEntry = entry;
            }
        }
    }

    public uint GetMissionContentLength() => _missionEntry?.Length ?? 0;

    public uint GetAddonInfoContentLength() => _addoninfoEntry?.Length ?? 0;

    public void GetMissionContent(byte[] buffer)
    {
        if (_missionEntry is null) return;
        _package.ReadEntry(_missionEntry, buffer, false);
    }

    public void GetAddonInfoContent(byte[] buffer)
    {
        if (_addoninfoEntry is null) return;
        _package.ReadEntry(_addoninfoEntry, buffer, false);
    }

    public void Dispose()
    {
        _package.Dispose();
    }
}

public static class NativeExports
{
    [UnmanagedCallersOnly(EntryPoint = "create_vpk", CallConvs = [typeof(CallConvCdecl)])]
    public static nint CreateVpk(nint pathPtr)
    {
        string path = Marshal.PtrToStringUTF8(pathPtr)!;
        var vpk = new Vpk(path);
        GCHandle handle = GCHandle.Alloc(vpk);
        return (nint) handle;
    }

    [UnmanagedCallersOnly(EntryPoint = "destroy_vpk", CallConvs = [typeof(CallConvCdecl)])]
    public static void DestroyVpk(nint handle)
    {
        if (handle == nint.Zero) return;
        var gch = (GCHandle) handle;
        if (gch.Target is IDisposable dis)
        {
            dis.Dispose();
        }

        gch.Free();
    }

    [UnmanagedCallersOnly(EntryPoint = "get_mission_content_length", CallConvs = [typeof(CallConvCdecl)])]
    public static uint GetMissionContentLength(nint handle) => handle.ToVpk().GetMissionContentLength();

    [UnmanagedCallersOnly(EntryPoint = "get_addoninfo_content_length", CallConvs = [typeof(CallConvCdecl)])]
    public static uint GetAddonInfoContentLength(nint handle) => handle.ToVpk().GetAddonInfoContentLength();

    [UnmanagedCallersOnly(EntryPoint = "get_mission_content", CallConvs = [typeof(CallConvCdecl)])]
    public static int GetMissionContent(nint handle, nint buffer, int bufferSize)
    {
        if (handle == nint.Zero) return -1;
        if (buffer == nint.Zero) return -2;
        var vpk = handle.ToVpk();
        int length = (int) vpk.GetMissionContentLength();
        if (bufferSize < length) return -3; // 提供的缓冲区过小
        byte[] bytes = ArrayPool<byte>.Shared.Rent(length);
        vpk.GetMissionContent(bytes);
        int toCopy = Math.Min(length, bufferSize);
        Marshal.Copy(bytes, 0, buffer, toCopy);
        ArrayPool<byte>.Shared.Return(bytes);
        return toCopy;
    }

    [UnmanagedCallersOnly(EntryPoint = "get_addoninfo_content", CallConvs = [typeof(CallConvCdecl)])]
    public static int GetAddonInfoContent(nint handle, nint buffer, int bufferSize)
    {
        if (handle == nint.Zero) return -1;
        if (buffer == nint.Zero) return -2;
        var vpk = handle.ToVpk();
        int length = (int) vpk.GetAddonInfoContentLength();
        if (bufferSize < length) return -3; // 提供的缓冲区过小
        byte[] bytes = ArrayPool<byte>.Shared.Rent(length);
        vpk.GetAddonInfoContent(bytes);
        int toCopy = Math.Min(length, bufferSize);
        Marshal.Copy(bytes, 0, buffer, toCopy);
        ArrayPool<byte>.Shared.Return(bytes);
        return toCopy;
    }

    private static Vpk ToVpk(this nint handle)
    {
        if (handle == nint.Zero) throw new ArgumentException("handle is zero");
        return (Vpk) ((GCHandle) handle).Target!;
    }
}
