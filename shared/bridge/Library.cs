using System;
using System.IO;
using System.Reflection;
using System.Runtime.InteropServices;
using Microsoft.Win32.SafeHandles;
using OpenRasterFileType;
using PaintDotNet;

namespace PdnBridge;

public static class Library {
    [StructLayoutAttribute(LayoutKind.Sequential)]
    public struct IO {
        public nint input;
        public nint output;
    }

    [UnmanagedCallersOnly]
    public static unsafe void PdnToOra(IO pair) {
        Document doc;
        using (var input = new FileStream(new SafeFileHandle(pair.input, true), FileAccess.Read)) {
            doc = Document.FromStream(input);
        }

        using var output = new FileStream(new SafeFileHandle(pair.output, true), FileAccess.ReadWrite);
        typeof(OraFileType).InvokeMember("OnSave", BindingFlags.NonPublic | BindingFlags.Instance | BindingFlags.InvokeMethod, null, new OraFileType(), [doc, output, null, null, null]);
    }
}
