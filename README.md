# PdnBatchConverter

<p align="center">
    <img src="icon.ico" alt="Icon" />
</p>

**PdnBatchConverter** is a application that can be used to convert .pdn files to other formats.

It hooks into your installation of [Paint.NET](https://getpaint.net) to read the provided files and convert multiple at once.

Supports conversions to both PNG and JPEG as well as layer export via OpenRaster. OpenRaster is a widely used image editing format in programs such as [Pinta](https://www.pinta-project.com) and [Krita](https://krita.org).

## Download

- **Graphical user interface** (desktop app):
  - 🟢 **[Stable release](https://github.com/Coca162/PdnBatchConverter/releases/latest)**: look for `PdnBatchConverter-*.exe`
- **Command-line interface** (terminal app):
  - 🟢 **[Stable release](https://github.com/Coca162/PdnBatchConverter/releases/latest)**: look for `PdnBatchConverter-Cli-*.exe`

[CI builds](https://github.com/Coca162/PdnBatchConverter/actions/workflows/build.yml) are available for testing unreleased changes.

> **Note**:
> If you're unsure which build is right for your system, consult with [this page](https://useragent.cc) to determine your CPU architecture.

## Features

- Graphical interface with files/folder selection
- Recursive folder option which preserves folder layout when converting
- Parallel processing of multiple files at once
- Controllable quality for JPEG conversions
- Installs [OpenRaster Filetype Plugin](https://forums.getpaint.net/topic/20984-openraster-filetype) in Paint.NET
- Command-line interface for automation use cases

## See Also / Special Thanks
- [**OpenRaster Filetype Plugin**](https://github.com/Zagna/openraster_pdn) - used for converting to OpenRaster files.
- [**Convert-pdn-to-ora**](https://github.com/marvin1099/Convert-pdn-to-ora) - for having this similar idea before me.
- [**DiscordChatExporter**](https://github.com/Tyrrrz/DiscordChatExporter) - for reference on the meta parts of making the project