// paint.net OpenRaster Format Plugin
// 
// Copyright (c) 2021 Zagna https://github.com/Zagna & Nicholas Hayes https://github.com/0xC0000054
// 
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

using PaintDotNet;
using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Drawing;
using System.Drawing.Imaging;
using System.Globalization;
using System.IO;
using System.IO.Compression;
using System.Linq;
using System.Xml;

namespace OpenRasterFileType
{
    public class OraFileType : FileType
    {
        private const int thumbMaxSize = 256;

        // Base64 encoded .zip file containing just an uncompressed file called mimetype for OpenRaster.
        private readonly string mimeTypeZip = "UEsDBBQAAAAAAAAAIQDHmvCMEAAAABAAAAAIAAAAbWltZXR5cGVpbWFnZS9vcGVucmFzdGVyUEsBAhQDFAAAAAAAAAAhAMea8IwQAAAAEAAAAAgAAAAAAAAAAAAAAKSBAAAAAG1pbWV0eXBlUEsFBgAAAAABAAEANgAAADYAAAAAAA==";

        private static readonly Dictionary<LayerBlendMode, string> blendDict = new()
        {
            { LayerBlendMode.Additive, "svg:plus" },
            { LayerBlendMode.ColorBurn, "svg:color-burn" },
            { LayerBlendMode.ColorDodge, "svg:color-dodge" },
            { LayerBlendMode.Darken, "svg:darken" },
            { LayerBlendMode.Difference, "svg:difference" },
            { LayerBlendMode.Glow, "pdn:glow" },
            { LayerBlendMode.Lighten, "svg:lighten" },
            { LayerBlendMode.Multiply, "svg:multiply" },
            { LayerBlendMode.Negation, "pdn:negation" },
            { LayerBlendMode.Normal, "svg:src-over" },
            { LayerBlendMode.Overlay, "svg:overlay" },
            { LayerBlendMode.Reflect, "pdn:reflect" },
            { LayerBlendMode.Screen, "svg:screen" },
            { LayerBlendMode.Xor, "svg:xor" },
        };

        private static readonly Dictionary<string, LayerBlendMode> SVGDict = blendDict.ToDictionary(x => x.Value, x => x.Key);

        public OraFileType() : base("OpenRaster",
            new FileTypeOptions()
                {
                SupportsLayers = true,
                LoadExtensions = new string[] { ".ora" },
                SaveExtensions = new string[] { ".ora" }
                }
            )
            {
            strokeMapVersions = new string[2] { "mypaint_strokemap", "mypaint_strokemap_v2" };
            }

        private readonly string[] strokeMapVersions;

        protected override Document OnLoad(Stream input)
        {
            using ZipArchive file = new(input, ZipArchiveMode.Read);
            try
            {
                ZipArchiveEntry mimeEntry = file.GetEntry("mimetype");
                using StreamReader reader = new(mimeEntry.Open());
                string mimeType = reader.ReadToEnd();
                if (!mimeType.Equals("image/openraster", StringComparison.Ordinal))
                {
                    throw new FormatException("Incorrect mimetype: " + mimeType);
                }
            }
            catch (NullReferenceException)
            {
                throw new FormatException("No mimetype found in OpenRaster file");
            }

            XmlDocument stackXml = new();
            try
            {
                stackXml.Load(file.GetEntry("stack.xml").Open());
            }
            catch (NullReferenceException)
            {
                throw new FormatException("No 'stack.xml' found in OpenRaster file");
            }

            XmlElement imageElement = stackXml.DocumentElement;
            int width = int.Parse(imageElement.GetAttribute("w"), CultureInfo.InvariantCulture);
            int height = int.Parse(imageElement.GetAttribute("h"), CultureInfo.InvariantCulture);

            Document doc = new(width, height)
            {
                DpuUnit = MeasurementUnit.Inch,
                DpuX = double.Parse(GetAttribute(imageElement, "xres", "72"), CultureInfo.InvariantCulture),
                DpuY = double.Parse(GetAttribute(imageElement, "yres", "72"), CultureInfo.InvariantCulture)
            };

            XmlElement stackElement = (XmlElement)stackXml.GetElementsByTagName("stack")[0];
            XmlNodeList layerElements = stackElement.GetElementsByTagName("layer");

            if (layerElements.Count == 0)
            {
                throw new FormatException("No layers found in OpenRaster file");
            }

            int layerCount = layerElements.Count - 1;

            for (int i = layerCount; i >= 0; i--) // The last layer in the list is the background so load in reverse
            {
                XmlElement layerElement = (XmlElement)layerElements[i];
                int x = int.Parse(GetAttribute(layerElement, "x", "0"), CultureInfo.InvariantCulture); // the x offset within the layer
                int y = int.Parse(GetAttribute(layerElement, "y", "0"), CultureInfo.InvariantCulture); // the y offset within the layer

                int layerNum = layerCount - i;

                ZipArchiveEntry layerEntry = file.GetEntry(layerElement.GetAttribute("src"));

                using Stream s = layerEntry.Open();
                using Bitmap BMP = GetBitmapFromOraLayer(x, y, s, width, height);

                BitmapLayer myLayer = i == layerCount ? Layer.CreateBackgroundLayer(width, height) : new(width, height);

                myLayer.Surface.CopyFromGdipBitmap(BMP, false);

                myLayer.Name = GetAttribute(layerElement, "name", $"Layer {layerNum}");
                myLayer.Opacity = (byte)(255.0 * double.Parse(GetAttribute(layerElement, "opacity", "1"), CultureInfo.InvariantCulture));
                myLayer.Visible = GetAttribute(layerElement, "visibility", "visible") == "visible"; // newer ora files have this

                string compOp = GetAttribute(layerElement, "composite-op", "svg:src-over");

                compOp = compOp.Contains("pdn-") ? compOp.Replace("pdn-", "pdn:") : compOp;

                if (SVGDict.ContainsKey(compOp))
                {
                    myLayer.BlendMode = SVGDict[compOp];
                }
                else
                {
                    string pdnCompOp = "pdn:" + compOp.Split(':')[1];
                    myLayer.BlendMode = SVGDict.ContainsKey(pdnCompOp) ? SVGDict[pdnCompOp] : LayerBlendMode.Normal;
                }

                string backTile = GetAttribute(layerElement, "background_tile", string.Empty);

                if (!string.IsNullOrEmpty(backTile))
                {
                    // convert the tile image to a Base64String and then save it in the layer's MetaData.
                    myLayer.Metadata.SetUserValue("OraBackgroundTile", ToBase64(file.GetEntry(backTile)));
                }

                foreach (string version in strokeMapVersions)
                {
                    string strokeMap = GetAttribute(layerElement, version, string.Empty);

                    if (!string.IsNullOrEmpty(strokeMap))
                    {
                        // convert the stroke map to a Base64String and then save it in the layer's MetaData.
                        myLayer.Metadata.SetUserValue("OraMyPaintStrokeMapData", ToBase64(file.GetEntry(strokeMap)));
                        // Save the version of the stroke map in the MetaData
                        myLayer.Metadata.SetUserValue("OraMyPaintStrokeMapVersion", version);
                    }
                }
                doc.Layers.Insert(layerNum, myLayer);
            }
            return doc;
        }

        protected override void OnSave(Document input, Stream output, SaveConfigToken token, Surface scratchSurface, ProgressEventHandler callback)
        {
            ArgumentNullException.ThrowIfNull(input);
            ArgumentNullException.ThrowIfNull(output);

            byte[] zipBytes = Convert.FromBase64String(mimeTypeZip);

            output.Write(zipBytes, 0, zipBytes.Length);

            using ZipArchive archive = new(output, ZipArchiveMode.Update, true);

            LayerInfo[] layerInfo = new LayerInfo[input.Layers.Count];

            for (int i = 0; i < input.Layers.Count; i++)
            {
                BitmapLayer layer = (BitmapLayer)input.Layers[i];
                Rectangle bounds = layer.Surface.Bounds;
                ColorBgra pixel;

                int left = layer.Width;
                int top = layer.Height;
                int right = 0;
                int bottom = 0;
                for (int y = 0; y < layer.Height; y++)
                {
                    for (int x = 0; x < layer.Width; x++)
                    {
                        pixel = layer.Surface[x,y];
                        if (pixel.A > 0)
                        {
                            left = x < left ? x : left;
                            right = x > right ? x : right;
                            top = y < top ? y : top;
                            bottom = y > bottom ? y : bottom;
                        }
                    }
                }

                if (left < layer.Width && top < layer.Height) // is the layer not empty
                {
                    bounds = new Rectangle(left, top, right - left + 1, bottom - top + 1); // clip it to the visible rectangle
                    layerInfo[i] = new LayerInfo(left, top);
                }
                else
                {
                    layerInfo[i] = new LayerInfo(0, 0);
                }

                string tileData = layer.Metadata.GetUserValue("OraBackgroundTile");

                if (!string.IsNullOrEmpty(tileData)) // save the background_tile png if it exists
                {
                    ZipArchiveEntry bgTile = archive.CreateEntry("data/background_tile.png");

                    using Stream bg = bgTile.Open();
                    byte[] tileBytes = Convert.FromBase64String(tileData);
                    bg.Write(tileBytes, 0, tileBytes.Length);
                }

                string strokeData = layer.Metadata.GetUserValue("OraMyPaintStrokeMapData");

                if (!string.IsNullOrEmpty(strokeData)) // save MyPaint's stroke data if it exists
                {
                    ZipArchiveEntry strokeMap = archive.CreateEntry("data/layer" + i.ToString(CultureInfo.InvariantCulture) + "_strokemap.dat");

                    using Stream stroke = strokeMap.Open();
                    byte[] strokeBytes = Convert.FromBase64String(strokeData);
                    stroke.Write(strokeBytes, 0, strokeBytes.Length);
                }

                using MemoryStream layerStream = new();
                layer.Surface.CreateAliasedBitmap(bounds, true).Save(layerStream, ImageFormat.Png);
                byte[] layerBuf = layerStream.ToArray();

                ZipArchiveEntry layerPNG = archive.CreateEntry("data/layer" + i.ToString(CultureInfo.InvariantCulture) + ".png");

                using Stream pngStream = layerPNG.Open();
                pngStream.Write(layerBuf, 0, layerBuf.Length);
            }

            ZipArchiveEntry stackXML = archive.CreateEntry("stack.xml");

            using (Stream sXML = stackXML.Open())
            {
                double dpiX;
                double dpiY;

                switch (input.DpuUnit)
                {
                    case MeasurementUnit.Centimeter:
                        dpiX = Document.DotsPerCmToDotsPerInch(input.DpuX);
                        dpiY = Document.DotsPerCmToDotsPerInch(input.DpuY);
                        break;

                    case MeasurementUnit.Inch:
                        dpiX = input.DpuX;
                        dpiY = input.DpuY;
                        break;

                    case MeasurementUnit.Pixel:
                        dpiX = Document.GetDefaultDpu(MeasurementUnit.Inch);
                        dpiY = Document.GetDefaultDpu(MeasurementUnit.Inch);
                        break;

                    default:
                        throw new InvalidEnumArgumentException("Invalid measurement unit.");
                }

                byte[] stackBytes = GetLayerXmlData(input.Layers, layerInfo, dpiX, dpiY);
                sXML.Write(stackBytes, 0, stackBytes.Length);
            }

            using Surface flat = new(input.Width, input.Height);

            input.Flatten(flat);

            using MemoryStream aliasStream = new();
            flat.CreateAliasedBitmap().Save(aliasStream, ImageFormat.Png);
            byte[] aliasBytes = aliasStream.ToArray();

            ZipArchiveEntry mergeEntry = archive.CreateEntry("mergedimage.png");

            using Stream merge = mergeEntry.Open();
            merge.Write(aliasBytes, 0, aliasBytes.Length);

            Size thumbSize = GetThumbDimensions(input.Width, input.Height);

            using Surface scale = new(thumbSize);
            scale.FitSurface(ResamplingAlgorithm.SuperSampling, flat);

            using MemoryStream scaleStream = new();
            scale.CreateAliasedBitmap().Save(scaleStream, ImageFormat.Png);

            byte[] scaleBytes = scaleStream.ToArray();

            ZipArchiveEntry thumbnailEntry = archive.CreateEntry("Thumbnails/thumbnail.png");
            using Stream thumbStream = thumbnailEntry.Open();
            thumbStream.Write(scaleBytes, 0, scaleBytes.Length);
        }

        // A struct to store the new x,y offsets
        private struct LayerInfo
        {
            public int x;
            public int y;

            public LayerInfo(int x, int y)
            {
                this.x = x;
                this.y = y;
            }
        }

        private static Bitmap GetBitmapFromOraLayer(int xofs, int yofs, Stream inStream, int baseWidth, int baseHeight)
        {
            using Bitmap layer = new(baseWidth, baseHeight, PixelFormat.Format32bppArgb);
            using Bitmap bmp = new(inStream);
            using Graphics graphics = Graphics.FromImage(layer);

            graphics.DrawImage(bmp, new Rectangle(xofs, yofs, bmp.Width, bmp.Height));

            return (Bitmap)layer.Clone();
        }

        private static byte[] GetLayerXmlData(LayerList layers, LayerInfo[] info, double dpiX, double dpiY) // OraFormat.cs - some changes
        {
            using MemoryStream xmlStream = new();

            XmlWriterSettings settings = new()
            {
                Indent = true,
                OmitXmlDeclaration = false,
                ConformanceLevel = ConformanceLevel.Document,
                CloseOutput = false
            };
            XmlWriter writer = XmlWriter.Create(xmlStream, settings);

            writer.WriteStartDocument();

            writer.WriteStartElement("image");
            writer.WriteAttributeString("w", layers.GetAt(0).Width.ToString(CultureInfo.InvariantCulture));
            writer.WriteAttributeString("h", layers.GetAt(0).Height.ToString(CultureInfo.InvariantCulture));
            writer.WriteAttributeString("version", "0.0.5"); // mandatory

            writer.WriteAttributeString("xres", dpiX.ToString(CultureInfo.InvariantCulture));
            writer.WriteAttributeString("yres", dpiY.ToString(CultureInfo.InvariantCulture));

            writer.WriteStartElement("stack");
            writer.WriteAttributeString("name", "root");

            // ORA stores layers top to bottom
            for (int i = layers.Count - 1; i >= 0; i--)
            {
                Layer layer = layers[i];

                writer.WriteStartElement("layer");

                string backTile = layer.Metadata.GetUserValue("OraBackgroundTile");

                if (!string.IsNullOrEmpty(backTile))
                {
                    writer.WriteAttributeString("background_tile", "data/background_tile.png");
                }
                string strokeMapVersion = layer.Metadata.GetUserValue("OraMyPaintStrokeMapVersion");

                if (!string.IsNullOrEmpty(strokeMapVersion))
                {
                    writer.WriteAttributeString(strokeMapVersion, "data/layer" + i.ToString(CultureInfo.InvariantCulture) + "_strokemap.dat");
                }
                if (string.IsNullOrEmpty(strokeMapVersion)) // the stroke map layer does not have a name
                {
                    writer.WriteAttributeString("name", layer.Name);
                }

                writer.WriteAttributeString("opacity", double.Clamp(layer.Opacity / 255.0, 0.0, 1.0).ToString("N2", CultureInfo.InvariantCulture)); // this is even more bizarre :D

                writer.WriteAttributeString("src", "data/layer" + i.ToString(CultureInfo.InvariantCulture) + ".png");
                writer.WriteAttributeString("visibility", layer.Visible ? "visible" : "hidden");

                writer.WriteAttributeString("x", info[i].x.ToString(CultureInfo.InvariantCulture));
                writer.WriteAttributeString("y", info[i].y.ToString(CultureInfo.InvariantCulture));

                if (blendDict.ContainsKey(layer.BlendMode))
                {
                    writer.WriteAttributeString("composite-op", blendDict[layer.BlendMode]);
                }
                else
                {
                    writer.WriteAttributeString("composite-op", "svg:src-over");
                }

                writer.WriteEndElement();
            }

            writer.WriteEndElement(); // stack
            writer.WriteEndElement(); // image
            writer.WriteEndDocument();

            writer.Close();

            return xmlStream.ToArray();
        }

        private static Size GetThumbDimensions(int width, int height) // OraFormat.cs
        {
            return width <= thumbMaxSize && height <= thumbMaxSize
                ? new Size(width, height)
                : width > height
                ? new Size(thumbMaxSize, (int)((double)height / width * thumbMaxSize))
                : new Size((int)((double)width / height * thumbMaxSize), thumbMaxSize);
        }

        private static string GetAttribute(XmlElement element, string attribute, string defValue) // OraFormat.cs
        {
            string ret = element.GetAttribute(attribute);
            return string.IsNullOrEmpty(ret) ? defValue : ret;
        }

        private static string ToBase64(ZipArchiveEntry zipEntry)
        {
            using Stream stream = zipEntry.Open();
            using MemoryStream memory = new();
            stream.CopyTo(memory);
            byte[] bytes = memory.ToArray();
            return Convert.ToBase64String(bytes);
        }
    }

    public class MyFileTypeFactory : IFileTypeFactory
    {
        public FileType[] GetFileTypeInstances() => new FileType[] { new OraFileType() };
    }
}