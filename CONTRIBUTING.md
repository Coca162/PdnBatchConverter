By default, the project will be built without any bindings to Paint.NET, if you want to enable this build the project for windows and with the feature `pdn-sys` enabled.

This will build both a vendored version of [OpenRasterFileType](https://github.com/Zagna/openraster_pdn) and the dotnet bridge necessary for the rust side to be able to interface with Paint.NET.

These C# projects will be located under `./shared` and by default require Paint.NET to be installed in `C:\Program Files\Paint.NET` (ie the default location for installation), you can change `<HintPath>` properties located in the project's `.csproj` if you would like to have them somewhere else. Please have have dotnet 9 or higher installated as well. You might also be required to move `libnethost.lib` to the project directory, it would be located inside the `C:\Program Files\dotnet\packs\packs\Microsoft.NETCore.App.Host.win-ARCHITECTURE\DOTNET_VERSION` folder or whetever else you have dotnet installed.
