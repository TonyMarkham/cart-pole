using UnityEngine;

namespace Simulation.CartPole.SyntheticDataGen
{
    // The one greyscale-JPEG encoding shared by every producer of
    // detector frames - the dataset generator's training frames and the
    // simulation loop's rendered frames. The detector only sees the
    // domain it trained on when both encode identically, so there is
    // exactly one implementation.
    //
    // Greyscale is the only render path: the color readback is
    // luminance-equalized into a neutral image before encoding (BT.601
    // luma, l = 0.299 r + 0.587 g + 0.114 b). The channels stay RGB
    // (equal values, neutral chroma) because EncodeToJPG's single-channel
    // support is not guaranteed across platforms - the image carries no
    // color information either way.
    public static class GreyscaleEncoder
    {
        // The q90 contract: train and inference share the same lossy
        // statistics (DATASETS.md, Image spec).
        public const int Quality = 90;

        public static byte[] Encode(Texture2D source)
        {
            var pixels = source.GetPixels();
            for (var i = 0; i < pixels.Length; i++)
            {
                var l = pixels[i].r * 0.299f +
                    pixels[i].g * 0.587f + pixels[i].b * 0.114f;
                pixels[i] = new Color(l, l, l);
            }
            var grey = new Texture2D(
                source.width, source.height, TextureFormat.RGB24, false);
            grey.SetPixels(pixels);
            grey.Apply();
            var jpeg = grey.EncodeToJPG(Quality);
            Object.Destroy(grey);
            return jpeg;
        }
    }
}
