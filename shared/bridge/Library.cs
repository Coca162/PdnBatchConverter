using System;
using System.IO;
using System.Reflection;
using System.Runtime.InteropServices;
using Microsoft.Win32.SafeHandles;
using OpenRasterFileType;
using PaintDotNet;

namespace PdnBridge;

public static class Library {
    private static unsafe delegate*<char*, int, byte*> CopyToCString;

    [UnmanagedCallersOnly]
    public static unsafe void SetCopyToCStringFunctionPtr(delegate*<char*, int, byte*> copyToCString) => CopyToCString = copyToCString;

    [StructLayoutAttribute(LayoutKind.Sequential)]
    public struct IO {
        public nint input;
        public nint output;
    }

    [UnmanagedCallersOnly]
    public static unsafe byte* PdnToOra(IO pair) {
        try {
            Document doc;
            using (var input = new FileStream(new SafeFileHandle(pair.input, true), FileAccess.Read)) {
                doc = Document.FromStream(input);
            }

            using var output = new FileStream(new SafeFileHandle(pair.output, true), FileAccess.ReadWrite);
            typeof(OraFileType).InvokeMember("OnSave", BindingFlags.NonPublic | BindingFlags.Instance | BindingFlags.InvokeMethod, null, new OraFileType(), [doc, output, null, null, null]);
        } catch (Exception e) {
            var message = e.ToString();
            fixed (char* ptr = message) {
                return CopyToCString(ptr, message.Length);
            }
        }

        return null;
    }
}
