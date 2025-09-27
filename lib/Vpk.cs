using System.Runtime.InteropServices;
using System.Text;
using SteamDatabase.ValvePak;

namespace VpkInfo;

/*
 * 读取指定的vpk文件，获取其文件中的addoninfo.txt和missions/文件夹下的文本文件的内容
 */
public class Vpk : IDisposable
{
    private readonly PackageEntry? _addoninfoEntry;
    private readonly PackageEntry? _missionEntry;
    private readonly Package _package;

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

    public void Dispose()
    {
        _package.Dispose();
    }

    public string? GetMissionContent()
    {
        if (_missionEntry is null) return null;
        _package.ReadEntry(_missionEntry, out byte[] buffer, false);
        return Encoding.UTF8.GetString(buffer);
    }

    public string? GetAddonInfoContent()
    {
        if (_addoninfoEntry is null) return null;
        _package.ReadEntry(_addoninfoEntry, out byte[] buffer, false);
        return Encoding.UTF8.GetString(buffer);
    }
}

public static class NativeExports
{
    private static string _lastErrMsg = string.Empty;

    [UnmanagedCallersOnly(EntryPoint = "GetLastErrorMessage")]
    public static unsafe void* GetLastErrorMessage() => Marshal.StringToCoTaskMemUTF8(_lastErrMsg).ToPointer();

    [UnmanagedCallersOnly(EntryPoint = "FreeString")]
    public static unsafe void FreeString(void* strPtr) => Marshal.FreeCoTaskMem((nint) strPtr);

    // 返回值为-1 有错误
    [UnmanagedCallersOnly(EntryPoint = "CreateVpk")]
    public static unsafe int CreateVpk(void* pathPtr, void** handle)
    {
        *handle = (void*) 0;
        try
        {
            if (pathPtr == (void*) 0)
            {
                throw new Exception("path ptr is not zero");
            }

            string path = Marshal.PtrToStringUTF8((nint) pathPtr)!;
            var vpk = new Vpk(path);
            *handle = GCHandle.ToIntPtr(GCHandle.Alloc(vpk)).ToPointer();
            return 0;
        }
        catch (Exception e)
        {
            _lastErrMsg = e.Message;
            return -1;
        }
    }

    [UnmanagedCallersOnly(EntryPoint = "DestroyVpk")]
    public static unsafe void DestroyVpk(void* handle)
    {
        if (handle == (void*) 0) return;
        var gch = GCHandle.FromIntPtr((nint) handle);
        if (gch.Target is IDisposable dis)
        {
            dis.Dispose();
        }

        gch.Free();
    }

    // 返回值为-1 有错误  *contentPtr 为0时  文件不存在
    [UnmanagedCallersOnly(EntryPoint = "GetMissionContent")]
    public static unsafe int GetMissionContent(void* handle, void** contentPtr)
    {
        *contentPtr = (void*) 0;
        try
        {
            if (handle == (void*) 0)
            {
                throw new Exception("handle is not zero");
            }

            var vpk = (Vpk) GCHandle.FromIntPtr((nint) handle).Target!;
            string? content = vpk.GetMissionContent();
            *contentPtr = Marshal.StringToCoTaskMemUTF8(content).ToPointer();
            return 0;
        }
        catch (Exception e)
        {
            _lastErrMsg = e.Message;
            return -1;
        }
    }

    // 返回值为-1 有错误  *contentPtr 为0时 文件不存在
    [UnmanagedCallersOnly(EntryPoint = "GetAddonInfoContent")]
    public static unsafe int GetAddonInfoContent(void* handle, void** contentPtr)
    {
        *contentPtr = (void*) 0;
        try
        {
            if (handle == (void*) 0)
            {
                throw new Exception("handle is not zero");
            }

            var vpk = (Vpk) GCHandle.FromIntPtr((nint) handle).Target!;
            string? content = vpk.GetAddonInfoContent();
            *contentPtr = Marshal.StringToCoTaskMemUTF8(content).ToPointer();
            return 0;
        }
        catch (Exception e)
        {
            _lastErrMsg = e.Message;
            return -1;
        }
    }
}
