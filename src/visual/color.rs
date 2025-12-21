// Continuous color mapping schemes (colormaps) - for numerical data
// The color was got from https://hauselin.github.io/colorpalettejs/ and
// https://docs.rs/colorous/latest/colorous/index.html

/// Continuous color mapping schemes (colormaps) for numerical data visualization.
///
/// This enum provides various predefined color mapping schemes that can be used
/// to map continuous numerical values to colors. These colormaps are commonly
/// used in data visualization to represent gradients and value ranges.
///
/// Each variant represents a different color scheme with specific characteristics:
/// - **Perceptually uniform colormaps** (Viridis, Inferno, Magma, Plasma, Cividis):
///   Designed to be perceptually uniform and colorblind-friendly
/// - **Sequential colormaps** (Blues, Greens, Greys, Oranges, Purples, Reds):
///   Single-hue gradients from light to dark
/// - **Diverging colormaps** (BuGn, BuPu, GnBu, OrRd, PuBuGn, PuBu, PuRd, RdPu,
///   YlGnBu, YlGn, YlOrBr, YlOrRd): Multi-hue gradients combining two or more colors
/// - **Cyclic colormaps** (Rainbow): Full spectrum color cycles
/// - **Specialized colormaps** (Jet, Hot, Cool): Traditional scientific visualization schemes
///
/// # Examples
///
/// ```
/// use charton::visual::color::ColorMap;
///
/// // Get a color from the Viridis colormap at 75% position
/// let color = ColorMap::Viridis.get_color(0.75);
/// assert_eq!(color, "#54c568");
/// ```
#[derive(Clone, Debug)]
pub enum ColorMap {
    Viridis,
    Inferno,
    Magma,
    Plasma,
    Cividis,

    Blues,
    Greens,
    Greys,
    Oranges,
    Purples,
    Reds,

    BuGn,   // Blue Green
    BuPu,   // Blue Purple
    GnBu,   // Green Blue
    OrRd,   // Orange Red
    PuBuGn, // Purple Blue Green
    PuBu,   // Purple Blue
    PuRd,   // Purple Red
    RdPu,   // Red Purple
    YlGnBu, // Yellow Green Blue
    YlGn,   // Yellow Green
    YlOrBr, // Yellow Orange Brown
    YlOrRd, // Yellow Orange Red

    Rainbow,
    Jet,
    Hot,
    Cool,
}

impl ColorMap {
    /// Returns a color from the colormap based on a value between 0 and 1
    pub(crate) fn get_color(&self, value: f64) -> String {
        // Clamp value between 0 and 1
        let clamped_value = value.clamp(0.0, 1.0);

        match self {
            ColorMap::Viridis => {
                // Viridis colormap - purple to yellow
                Self::interpolate_color_stops(
                    &[
                        (0.00000, 0x44, 0x01, 0x54), // #440154
                        (0.06667, 0x48, 0x1a, 0x6c), // #481a6c
                        (0.13333, 0x47, 0x2f, 0x7d), // #472f7d
                        (0.20000, 0x41, 0x44, 0x87), // #414487
                        (0.26667, 0x39, 0x56, 0x8c), // #39568c
                        (0.33333, 0x31, 0x68, 0x8e), // #31688e
                        (0.40000, 0x2a, 0x78, 0x8e), // #2a788e
                        (0.46667, 0x23, 0x88, 0x8e), // #23888e
                        (0.53333, 0x1f, 0x98, 0x8b), // #1f988b
                        (0.60000, 0x22, 0xa8, 0x84), // #22a884
                        (0.66667, 0x35, 0xb7, 0x79), // #35b779
                        (0.73333, 0x54, 0xc5, 0x68), // #54c568
                        (0.80000, 0x7a, 0xd1, 0x51), // #7ad151
                        (0.86667, 0xa5, 0xdb, 0x36), // #a5db36
                        (0.93333, 0xd2, 0xe2, 0x1b), // #d2e21b
                        (1.00000, 0xfd, 0xe7, 0x25), // #fde725
                    ],
                    clamped_value,
                )
            }
            ColorMap::Inferno => {
                // Inferno colormap - black to red to yellow
                Self::interpolate_color_stops(
                    &[
                        (0.00000, 0x00, 0x00, 0x04), // #000004
                        (0.06667, 0x0c, 0x08, 0x26), // #0c0826
                        (0.13333, 0x24, 0x0c, 0x4f), // #240c4f
                        (0.20000, 0x42, 0x0a, 0x68), // #420a68
                        (0.26667, 0x5d, 0x12, 0x6e), // #5d126e
                        (0.33333, 0x78, 0x1c, 0x6d), // #781c6d
                        (0.40000, 0x93, 0x26, 0x67), // #932667
                        (0.46667, 0xae, 0x30, 0x5c), // #ae305c
                        (0.53333, 0xc7, 0x3e, 0x4c), // #c73e4c
                        (0.60000, 0xdd, 0x51, 0x3a), // #dd513a
                        (0.66667, 0xed, 0x69, 0x25), // #ed6925
                        (0.73333, 0xf8, 0x85, 0x0f), // #f8850f
                        (0.80000, 0xfc, 0xa5, 0x0a), // #fca50a
                        (0.86667, 0xfa, 0xc6, 0x2d), // #fac62d
                        (0.93333, 0xf2, 0xe6, 0x61), // #f2e661
                        (1.00000, 0xfc, 0xff, 0xa4), // #fcffa4
                    ],
                    clamped_value,
                )
            }
            ColorMap::Magma => {
                // Magma colormap - black to red to white
                Self::interpolate_color_stops(
                    &[
                        (0.00000, 0x00, 0x00, 0x04), // #000004
                        (0.06667, 0x0b, 0x09, 0x24), // #0b0924
                        (0.13333, 0x20, 0x11, 0x4b), // #20114b
                        (0.20000, 0x3b, 0x0f, 0x70), // #3b0f70
                        (0.26667, 0x57, 0x15, 0x7e), // #57157e
                        (0.33333, 0x72, 0x1f, 0x81), // #721f81
                        (0.40000, 0x8c, 0x29, 0x81), // #8c2981
                        (0.46667, 0xa8, 0x32, 0x7d), // #a8327d
                        (0.53333, 0xc4, 0x3c, 0x75), // #c43c75
                        (0.60000, 0xde, 0x49, 0x68), // #de4968
                        (0.66667, 0xf1, 0x60, 0x5d), // #f1605d
                        (0.73333, 0xfa, 0x7f, 0x5e), // #fa7f5e
                        (0.80000, 0xfe, 0x9f, 0x6d), // #fe9f6d
                        (0.86667, 0xfe, 0xbf, 0x84), // #febf84
                        (0.93333, 0xfd, 0xde, 0xa0), // #fddea0
                        (1.00000, 0xfc, 0xfd, 0xbf), // #fcfdbf
                    ],
                    clamped_value,
                )
            }
            ColorMap::Plasma => {
                // Plasma colormap - purple to pink to yellow
                Self::interpolate_color_stops(
                    &[
                        (0.00000, 0x0d, 0x08, 0x87), // #0d0887
                        (0.06667, 0x33, 0x05, 0x97), // #330597
                        (0.13333, 0x50, 0x02, 0xa2), // #5002a2
                        (0.20000, 0x6a, 0x00, 0xa8), // #6a00a8
                        (0.26667, 0x84, 0x05, 0xa7), // #8405a7
                        (0.33333, 0x9c, 0x17, 0x9e), // #9c179e
                        (0.40000, 0xb1, 0x2a, 0x90), // #b12a90
                        (0.46667, 0xc3, 0x3d, 0x80), // #c33d80
                        (0.53333, 0xd3, 0x51, 0x71), // #d35171
                        (0.60000, 0xe1, 0x64, 0x62), // #e16462
                        (0.66667, 0xed, 0x79, 0x53), // #ed7953
                        (0.73333, 0xf6, 0x8f, 0x44), // #f68f44
                        (0.80000, 0xfc, 0xa6, 0x36), // #fca636
                        (0.86667, 0xfe, 0xc0, 0x29), // #fec029
                        (0.93333, 0xf9, 0xdc, 0x24), // #f9dc24
                        (1.00000, 0xf0, 0xf9, 0x21), // #f0f921
                    ],
                    clamped_value,
                )
            }
            ColorMap::Cividis => {
                // Cividis colormap - blue to yellow
                Self::interpolate_color_stops(
                    &[
                        (0.00000, 0x00, 0x20, 0x51), // #002051
                        (0.06667, 0x02, 0x2c, 0x65), // #022c65
                        (0.13333, 0x14, 0x38, 0x6d), // #14386d
                        (0.20000, 0x2b, 0x44, 0x6e), // #2b446e
                        (0.26667, 0x42, 0x50, 0x6e), // #42506e
                        (0.33333, 0x57, 0x5c, 0x6e), // #575c6e
                        (0.40000, 0x69, 0x69, 0x70), // #696970
                        (0.46667, 0x78, 0x75, 0x73), // #787573
                        (0.53333, 0x86, 0x82, 0x76), // #868276
                        (0.60000, 0x94, 0x8f, 0x78), // #948f78
                        (0.66667, 0xa4, 0x9d, 0x78), // #a49d78
                        (0.73333, 0xb6, 0xab, 0x73), // #b6ab73
                        (0.80000, 0xca, 0xba, 0x6a), // #caba6a
                        (0.86667, 0xe0, 0xc9, 0x5d), // #e0c95d
                        (0.93333, 0xf2, 0xd9, 0x50), // #f2d950
                        (1.00000, 0xfd, 0xea, 0x45), // #fdea45
                    ],
                    clamped_value,
                )
            }
            ColorMap::Blues => {
                // Blues colormap - white to blue
                Self::interpolate_color_stops(
                    &[
                        (0.00000, 0xf7, 0xfb, 0xff), // #f7fbff
                        (0.12500, 0xde, 0xeb, 0xf7), // #deebf7
                        (0.25000, 0xc6, 0xdb, 0xef), // #c6dbef
                        (0.37500, 0x9e, 0xca, 0xe1), // #9ecae1
                        (0.50000, 0x6b, 0xae, 0xd6), // #6baed6
                        (0.62500, 0x42, 0x92, 0xc6), // #4292c6
                        (0.75000, 0x21, 0x71, 0xb5), // #2171b5
                        (0.87500, 0x08, 0x51, 0x9c), // #08519c
                        (1.00000, 0x08, 0x30, 0x6b), // #08306b
                    ],
                    clamped_value,
                )
            }
            ColorMap::Greens => {
                // Greens colormap - white to green
                Self::interpolate_color_stops(
                    &[
                        (0.00000, 0xf7, 0xfc, 0xf5), // #f7fcf5
                        (0.12500, 0xe5, 0xf5, 0xe0), // #e5f5e0
                        (0.25000, 0xc7, 0xe9, 0xc0), // #c7e9c0
                        (0.37500, 0xa1, 0xd9, 0x9b), // #a1d99b
                        (0.50000, 0x74, 0xc4, 0x76), // #74c476
                        (0.62500, 0x41, 0xab, 0x5d), // #41ab5d
                        (0.75000, 0x23, 0x8b, 0x45), // #238b45
                        (0.87500, 0x00, 0x6d, 0x2c), // #006d2c
                        (1.00000, 0x00, 0x44, 0x1b), // #00441b
                    ],
                    clamped_value,
                )
            }
            ColorMap::Greys => {
                // Greens colormap - white to green
                Self::interpolate_color_stops(
                    &[
                        (0.00000, 0xff, 0xff, 0xff), // #ffffff
                        (0.12500, 0xf0, 0xf0, 0xf0), // #f0f0f0
                        (0.25000, 0xd9, 0xd9, 0xd9), // #d9d9d9
                        (0.37500, 0xbd, 0xbd, 0xbd), // #bdbdbd
                        (0.50000, 0x96, 0x96, 0x96), // #969696
                        (0.62500, 0x73, 0x73, 0x73), // #737373
                        (0.75000, 0x52, 0x52, 0x52), // #525252
                        (0.87500, 0x25, 0x25, 0x25), // #252525
                        (1.00000, 0x00, 0x00, 0x00), // #000000
                    ],
                    clamped_value,
                )
            }
            ColorMap::Oranges => {
                // Greens colormap - white to green
                Self::interpolate_color_stops(
                    &[
                        (0.00000, 0xff, 0xf5, 0xeb), // #fff5eb
                        (0.12500, 0xfe, 0xe6, 0xce), // #fee6ce
                        (0.25000, 0xfd, 0xd0, 0xa2), // #fdd0a2
                        (0.37500, 0xfd, 0xae, 0x6b), // #fdae6b
                        (0.50000, 0xfd, 0x8d, 0x3c), // #fd8d3c
                        (0.62500, 0xf1, 0x69, 0x13), // #f16913
                        (0.75000, 0xd9, 0x48, 0x01), // #d94801
                        (0.87500, 0xa6, 0x36, 0x03), // #a63603
                        (1.00000, 0x7f, 0x27, 0x04), // #7f2704
                    ],
                    clamped_value,
                )
            }
            ColorMap::Purples => {
                // Greens colormap - white to green
                Self::interpolate_color_stops(
                    &[
                        (0.00000, 0xfc, 0xfb, 0xfd), // #fcfbfd
                        (0.12500, 0xef, 0xed, 0xf5), // #efedf5
                        (0.25000, 0xda, 0xda, 0xeb), // #dadaeb
                        (0.37500, 0xbc, 0xbd, 0xdc), // #bcbddc
                        (0.50000, 0x9e, 0x9a, 0xc8), // #9e9ac8
                        (0.62500, 0x80, 0x7d, 0xba), // #807dba
                        (0.75000, 0x6a, 0x51, 0xa3), // #6a51a3
                        (0.87500, 0x54, 0x27, 0x8f), // #54278f
                        (1.00000, 0x3f, 0x00, 0x7d), // #3f007d
                    ],
                    clamped_value,
                )
            }
            ColorMap::Reds => {
                // Reds colormap - white to red
                Self::interpolate_color_stops(
                    &[
                        (0.00000, 0xff, 0xf5, 0xf0), // #fff5f0
                        (0.12500, 0xfe, 0xe0, 0xd2), // #fee0d2
                        (0.25000, 0xfc, 0xbb, 0xa1), // #fcbba1
                        (0.37500, 0xfc, 0x92, 0x72), // #fc9272
                        (0.50000, 0xfb, 0x6a, 0x4a), // #fb6a4a
                        (0.62500, 0xef, 0x3b, 0x2c), // #ef3b2c
                        (0.75000, 0xcb, 0x18, 0x1d), // #cb181d
                        (0.87500, 0xa5, 0x0f, 0x15), // #a50f15
                        (1.00000, 0x67, 0x00, 0x0d), // #67000d
                    ],
                    clamped_value,
                )
            }
            ColorMap::BuGn => {
                // BuGn colormap - blue to green
                Self::interpolate_color_stops(
                    &[
                        (0.00000, 0xf7, 0xfc, 0xfd), // #f7fcfd
                        (0.12500, 0xe5, 0xf5, 0xf9), // #e5f5f9
                        (0.25000, 0xcc, 0xec, 0xe6), // #ccece6
                        (0.37500, 0x99, 0xd8, 0xc9), // #99d8c9
                        (0.50000, 0x66, 0xc2, 0xa4), // #66c2a4
                        (0.62500, 0x41, 0xae, 0x76), // #41ae76
                        (0.75000, 0x23, 0x8b, 0x45), // #238b45
                        (0.87500, 0x00, 0x6d, 0x2c), // #006d2c
                        (1.00000, 0x00, 0x44, 0x1b), // #00441b
                    ],
                    clamped_value,
                )
            }
            ColorMap::BuPu => {
                // BuPu colormap -blue to purple
                Self::interpolate_color_stops(
                    &[
                        (0.00000, 0xf7, 0xfc, 0xfd), // #f7fcfd
                        (0.12500, 0xe0, 0xec, 0xf4), // #e0ecf4
                        (0.25000, 0xbf, 0xd3, 0xe6), // #bfd3e6
                        (0.37500, 0x9e, 0xbc, 0xda), // #9ebcda
                        (0.50000, 0x8c, 0x96, 0xc6), // #8c96c6
                        (0.62500, 0x8c, 0x6b, 0xb1), // #8c6bb1
                        (0.75000, 0x88, 0x41, 0x9d), // #88419d
                        (0.87500, 0x81, 0x0f, 0x7c), // #810f7c
                        (1.00000, 0x4d, 0x00, 0x4b), // #4d004b
                    ],
                    clamped_value,
                )
            }
            ColorMap::GnBu => {
                // GnBu colormap - green to blue
                Self::interpolate_color_stops(
                    &[
                        (0.00000, 0xf7, 0xfc, 0xf0), // #f7fcf0
                        (0.12500, 0xe0, 0xf3, 0xdb), // #e0f3db
                        (0.25000, 0xcc, 0xeb, 0xc5), // #ccebc5
                        (0.37500, 0xa8, 0xdd, 0xb5), // #a8ddb5
                        (0.50000, 0x7b, 0xcc, 0xc4), // #7bccc4
                        (0.62500, 0x4e, 0xb3, 0xd3), // #4eb3d3
                        (0.75000, 0x2b, 0x8c, 0xbe), // #2b8cbe
                        (0.87500, 0x08, 0x68, 0xac), // #0868ac
                        (1.00000, 0x08, 0x40, 0x81), // #084081
                    ],
                    clamped_value,
                )
            }
            ColorMap::OrRd => {
                // OrRd colormap - orange to red
                Self::interpolate_color_stops(
                    &[
                        (0.00000, 0xff, 0xf7, 0xec), // #fff7ec
                        (0.12500, 0xfe, 0xe8, 0xc8), // #fee8c8
                        (0.25000, 0xfd, 0xd4, 0x9e), // #fdd49e
                        (0.37500, 0xfd, 0xbb, 0x84), // #fdbb84
                        (0.50000, 0xfc, 0x8d, 0x59), // #fc8d59
                        (0.62500, 0xef, 0x65, 0x48), // #ef6548
                        (0.75000, 0xd7, 0x30, 0x1f), // #d7301f
                        (0.87500, 0xb3, 0x00, 0x00), // #b30000
                        (1.00000, 0x7f, 0x00, 0x00), // #7f0000
                    ],
                    clamped_value,
                )
            }
            ColorMap::PuBuGn => {
                // PuBuGn colormap - purple to blue to green
                Self::interpolate_color_stops(
                    &[
                        (0.00000, 0xff, 0xf7, 0xfb), // #fff7fb
                        (0.12500, 0xec, 0xe2, 0xf0), // #ece2f0
                        (0.25000, 0xd0, 0xd1, 0xe6), // #d0d1e6
                        (0.37500, 0xa6, 0xbd, 0xdb), // #a6bddb
                        (0.50000, 0x67, 0xa9, 0xcf), // #67a9cf
                        (0.62500, 0x36, 0x90, 0xc0), // #3690c0
                        (0.75000, 0x02, 0x81, 0x8a), // #02818a
                        (0.87500, 0x01, 0x6c, 0x59), // #016c59
                        (1.00000, 0x01, 0x46, 0x36), // #014636
                    ],
                    clamped_value,
                )
            }
            ColorMap::PuBu => {
                // PuBu colormap - purple to blue
                Self::interpolate_color_stops(
                    &[
                        (0.00000, 0xff, 0xf7, 0xfb), // #fff7fb
                        (0.12500, 0xec, 0xe7, 0xf2), // #ece7f2
                        (0.25000, 0xd0, 0xd1, 0xe6), // #d0d1e6
                        (0.37500, 0xa6, 0xbd, 0xdb), // #a6bddb
                        (0.50000, 0x74, 0xa9, 0xcf), // #74a9cf
                        (0.62500, 0x36, 0x90, 0xc0), // #3690c0
                        (0.75000, 0x05, 0x70, 0xb0), // #0570b0
                        (0.87500, 0x04, 0x5a, 0x8d), // #045a8d
                        (1.00000, 0x02, 0x38, 0x58), // #023858
                    ],
                    clamped_value,
                )
            }
            ColorMap::PuRd => {
                // PuRd colormap - purple to red
                Self::interpolate_color_stops(
                    &[
                        (0.00000, 0xf7, 0xf4, 0xf9), // #f7f4f9
                        (0.12500, 0xe7, 0xe1, 0xef), // #e7e1ef
                        (0.25000, 0xd4, 0xb9, 0xda), // #d4b9da
                        (0.37500, 0xc9, 0x94, 0xc7), // #c994c7
                        (0.50000, 0xdf, 0x65, 0xb0), // #df65b0
                        (0.62500, 0xe7, 0x29, 0x8a), // #e7298a
                        (0.75000, 0xce, 0x12, 0x56), // #ce1256
                        (0.87500, 0x98, 0x00, 0x43), // #980043
                        (1.00000, 0x67, 0x00, 0x1f), // #67001f
                    ],
                    clamped_value,
                )
            }
            ColorMap::RdPu => {
                // RdPu colormap - red to purple
                Self::interpolate_color_stops(
                    &[
                        (0.00000, 0xff, 0xf7, 0xf3), // #fff7f3
                        (0.12500, 0xfd, 0xe0, 0xdd), // #fde0dd
                        (0.25000, 0xfc, 0xc5, 0xc0), // #fcc5c0
                        (0.37500, 0xfa, 0x9f, 0xb5), // #fa9fb5
                        (0.50000, 0xf7, 0x68, 0xa1), // #f768a1
                        (0.62500, 0xdd, 0x34, 0x97), // #dd3497
                        (0.75000, 0xae, 0x01, 0x7e), // #ae017e
                        (0.87500, 0x7a, 0x01, 0x77), // #7a0177
                        (1.00000, 0x49, 0x00, 0x6a), // #49006a
                    ],
                    clamped_value,
                )
            }
            ColorMap::YlGnBu => {
                // YlGnBu colormap - yellow to green to blue
                Self::interpolate_color_stops(
                    &[
                        (0.00000, 0xff, 0xff, 0xd9), // #ffffd9
                        (0.12500, 0xed, 0xf8, 0xb1), // #edf8b1
                        (0.25000, 0xc7, 0xe9, 0xb4), // #c7e9b4
                        (0.37500, 0x7f, 0xcd, 0xbb), // #7fcdbb
                        (0.50000, 0x41, 0xb6, 0xc4), // #41b6c4
                        (0.62500, 0x1d, 0x91, 0xc0), // #1d91c0
                        (0.75000, 0x22, 0x5e, 0xa8), // #225ea8
                        (0.87500, 0x25, 0x34, 0x94), // #253494
                        (1.00000, 0x08, 0x1d, 0x58), // #081d58
                    ],
                    clamped_value,
                )
            }
            ColorMap::YlGn => {
                // YlGn colormap - yellow to green
                Self::interpolate_color_stops(
                    &[
                        (0.00000, 0xff, 0xff, 0xe5), // #ffffe5
                        (0.12500, 0xf7, 0xfc, 0xb9), // #f7fcb9
                        (0.25000, 0xd9, 0xf0, 0xa3), // #d9f0a3
                        (0.37500, 0xad, 0xdd, 0x8e), // #addd8e
                        (0.50000, 0x78, 0xc6, 0x79), // #78c679
                        (0.62500, 0x41, 0xab, 0x5d), // #41ab5d
                        (0.75000, 0x23, 0x84, 0x43), // #238443
                        (0.87500, 0x00, 0x68, 0x37), // #006837
                        (1.00000, 0x00, 0x45, 0x29), // #004529
                    ],
                    clamped_value,
                )
            }
            ColorMap::YlOrBr => {
                // YlOrBr colormap - yellow to orange to brown
                Self::interpolate_color_stops(
                    &[
                        (0.00000, 0xff, 0xff, 0xe5), // #ffffe5
                        (0.12500, 0xff, 0xf7, 0xbc), // #fff7bc
                        (0.25000, 0xfe, 0xe3, 0x91), // #fee391
                        (0.37500, 0xfe, 0xc4, 0x4f), // #fec44f
                        (0.50000, 0xfe, 0x99, 0x29), // #fe9929
                        (0.62500, 0xec, 0x70, 0x14), // #ec7014
                        (0.75000, 0xcc, 0x4c, 0x02), // #cc4c02
                        (0.87500, 0x99, 0x34, 0x04), // #993404
                        (1.00000, 0x66, 0x25, 0x06), // #662506
                    ],
                    clamped_value,
                )
            }
            ColorMap::YlOrRd => {
                // YlOrRd colormap - yellow to orange to red
                Self::interpolate_color_stops(
                    &[
                        (0.00000, 0xff, 0xff, 0xcc), // #ffffcc
                        (0.12500, 0xff, 0xed, 0xa0), // #ffeda0
                        (0.25000, 0xfe, 0xd9, 0x76), // #fed976
                        (0.37500, 0xfe, 0xb2, 0x4c), // #feb24c
                        (0.50000, 0xfd, 0x8d, 0x3c), // #fd8d3c
                        (0.62500, 0xfc, 0x4e, 0x2a), // #fc4e2a
                        (0.75000, 0xe3, 0x1a, 0x1c), // #e31a1c
                        (0.87500, 0xbd, 0x00, 0x26), // #bd0026
                        (1.00000, 0x80, 0x00, 0x26), // #800026
                    ],
                    clamped_value,
                )
            }

            ColorMap::Rainbow => {
                // Rainbow colormap - red to violet
                Self::hsv_to_rgb((1.0 - clamped_value) * 300.0, 1.0, 1.0)
            }
            ColorMap::Jet => {
                // Jet colormap - blue to red
                Self::interpolate_color_stops(
                    &[
                        (0.0, 0x00, 0x00, 0xff),  // Blue
                        (0.33, 0x00, 0xff, 0xff), // Cyan
                        (0.66, 0xff, 0xff, 0x00), // Yellow
                        (1.0, 0xff, 0x00, 0x00),  // Red
                    ],
                    clamped_value,
                )
            }
            ColorMap::Hot => {
                // Hot colormap - black to red to yellow
                if clamped_value < 0.33 {
                    let t = clamped_value / 0.33;
                    format!("#{:02x}0000", (t * 255.0) as u8)
                } else if clamped_value < 0.66 {
                    let t = (clamped_value - 0.33) / 0.33;
                    format!("#ff{:02x}00", (t * 255.0) as u8)
                } else {
                    let t = (clamped_value - 0.66) / 0.34;
                    format!("#ffff{:02x}", (t * 255.0) as u8)
                }
            }
            ColorMap::Cool => {
                // Cool colormap - cyan to magenta
                let r = (clamped_value * 255.0) as u8;
                let g = ((1.0 - clamped_value) * 255.0) as u8;
                let b = 255;
                format!("#{:02x}{:02x}{:02x}", r, g, b)
            }
        }
    }

    /// Interpolate between multiple color stops
    fn interpolate_color_stops(stops: &[(f64, u8, u8, u8)], value: f64) -> String {
        // Find the two stops to interpolate between
        for i in 0..stops.len() - 1 {
            let (pos1, r1, g1, b1) = stops[i];
            let (pos2, r2, g2, b2) = stops[i + 1];

            if value >= pos1 && value <= pos2 {
                let t = if pos2 - pos1 > 0.0 {
                    (value - pos1) / (pos2 - pos1)
                } else {
                    0.0
                };

                let r = (r1 as f64 + t * (r2 as f64 - r1 as f64)) as u8;
                let g = (g1 as f64 + t * (g2 as f64 - g1 as f64)) as u8;
                let b = (b1 as f64 + t * (b2 as f64 - b1 as f64)) as u8;

                return format!("#{:02x}{:02x}{:02x}", r, g, b);
            }
        }

        // Fallback to first or last color
        if value <= stops[0].0 {
            let (_, r, g, b) = stops[0];
            format!("#{:02x}{:02x}{:02x}", r, g, b)
        } else {
            let (_, r, g, b) = stops[stops.len() - 1];
            format!("#{:02x}{:02x}{:02x}", r, g, b)
        }
    }

    /// Convert HSV to RGB hexadecimal string
    fn hsv_to_rgb(h: f64, s: f64, v: f64) -> String {
        let c = v * s;
        let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
        let m = v - c;

        let (r, g, b) = if h < 60.0 {
            (c, x, 0.0)
        } else if h < 120.0 {
            (x, c, 0.0)
        } else if h < 180.0 {
            (0.0, c, x)
        } else if h < 240.0 {
            (0.0, x, c)
        } else if h < 300.0 {
            (x, 0.0, c)
        } else {
            (c, 0.0, x)
        };

        let r = ((r + m) * 255.0) as u8;
        let g = ((g + m) * 255.0) as u8;
        let b = ((b + m) * 255.0) as u8;

        format!("#{:02x}{:02x}{:02x}", r, g, b)
    }
}

// Discrete color palettes (palette) - for categorical data
/// Discrete color palettes for categorical data visualization.
///
/// This enum provides various predefined color palettes that are suitable for
/// representing categorical or discrete data. Each palette contains a fixed
/// set of distinct colors that can be used to differentiate between different
/// categories in visualizations.
///
/// These palettes are commonly used for:
/// - Bar charts with different categories
/// - Scatter plots with categorical coloring
/// - Line charts with multiple series
/// - Any visualization where distinct categories need to be visually separated
///
/// Each variant represents a different color scheme with specific characteristics:
/// - **Tab palettes** (Tab10, Tab20): Color schemes based on MATLAB and Tableau defaults
/// - **Set palettes** (Set1, Set2, Set3): Color schemes designed for information visualization
/// - **Pastel palettes** (Pastel1, Pastel2): Soft pastel colors for less emphasis
/// - **Dark palettes** (Dark2): Darker colors for better contrast
/// - **Accent palette** (Accent): Bright, high-contrast colors for emphasis
///
/// # Examples
///
/// ```
/// use charton::visual::color::ColorPalette;
///
/// // Get the first color from the Set1 palette
/// let color = ColorPalette::Set1.get_color(0);
/// assert_eq!(color, "#e41a1c");
///
/// // Get colors with automatic wrapping for indices beyond palette size
/// let color2 = ColorPalette::Set1.get_color(10);
/// ```
#[derive(Clone, Debug)]
pub enum ColorPalette {
    Tab10,
    Tab20,
    Set1,
    Set2,
    Set3,
    Pastel1,
    Pastel2,
    Dark2,
    Accent,
}

impl ColorPalette {
    /// Returns the color values for each palette
    pub(crate) fn colors(&self) -> Vec<&'static str> {
        match self {
            ColorPalette::Tab10 => vec![
                "#1f77b4", "#ff7f0e", "#2ca02c", "#d62728", "#9467bd", "#8c564b", "#e377c2",
                "#7f7f7f", "#bcbd22", "#17becf",
            ],
            ColorPalette::Tab20 => vec![
                "#1f77b4", "#aec7e8", "#ff7f0e", "#ffbb78", "#2ca02c", "#98df8a", "#d62728",
                "#ff9896", "#9467bd", "#c5b0d5", "#8c564b", "#c49c94", "#e377c2", "#f7b6d2",
                "#7f7f7f", "#c7c7c7", "#bcbd22", "#dbdb8d", "#17becf", "#9edae5",
            ],
            ColorPalette::Set1 => vec![
                "#e41a1c", "#377eb8", "#4daf4a", "#984ea3", "#ff7f00", "#ffff33", "#a65628",
                "#f781bf", "#999999",
            ],
            ColorPalette::Set2 => vec![
                "#66c2a5", "#fc8d62", "#8da0cb", "#e78ac3", "#a6d854", "#ffd92f", "#e5c494",
                "#b3b3b3",
            ],
            ColorPalette::Set3 => vec![
                "#8dd3c7", "#ffffb3", "#bebada", "#fb8072", "#80b1d3", "#fdb462", "#b3de69",
                "#fccde5", "#d9d9d9", "#bc80bd", "#ccebc5", "#ffed6f",
            ],
            ColorPalette::Pastel1 => vec![
                "#fbb4ae", "#b3cde3", "#ccebc5", "#decbe4", "#fed9a6", "#ffffcc", "#e5d8bd",
                "#fddaec", "#f2f2f2",
            ],
            ColorPalette::Pastel2 => vec![
                "#b3e2cd", "#fdcdac", "#cbd5e8", "#f4cae4", "#e6f5c9", "#fff2ae", "#f1e2cc",
                "#cccccc",
            ],
            ColorPalette::Dark2 => vec![
                "#1b9e77", "#d95f02", "#7570b3", "#e7298a", "#66a61e", "#e6ab02", "#a6761d",
                "#666666",
            ],
            ColorPalette::Accent => vec![
                "#7fc97f", "#beaed4", "#fdc086", "#ffff99", "#386cb0", "#f0027f", "#bf5b17",
                "#666666",
            ],
        }
    }

    /// Get a specific color from the palette by index (with wrapping)
    pub(crate) fn get_color(&self, index: usize) -> String {
        let colors = self.colors();
        let color = colors[index % colors.len()];
        color.to_string()
    }
}

/// A simple wrapper for a single color value.
///
/// This struct represents a single color value stored as a string. It's useful when you need
/// to work with individual colors rather than color palettes or gradients.
///
/// # Examples
///
/// ```
/// use charton::visual::color::SingleColor;
///
/// let red = SingleColor::new("#ff0000");
/// assert_eq!(red.get_color(), "#ff0000");
/// ```
#[derive(Clone, Debug)]
pub struct SingleColor(String);

impl SingleColor {
    /// Creates a new `SingleColor` instance from a color string.
    ///
    /// # Arguments
    ///
    /// * `color` - A string slice representing the color in CSS format (e.g., "#ff0000" for red).
    ///
    /// # Returns
    ///
    /// A new `SingleColor` instance containing the provided color value.
    ///
    /// # Examples
    ///
    /// ```
    /// use charton::visual::color::SingleColor;
    ///
    /// let color = SingleColor::new("#00ff00"); // Green
    /// ```
    pub fn new(color: &str) -> Self {
        Self(color.to_string())
    }
    pub(crate) fn get_color(&self) -> String {
        self.0.clone()
    }
}
