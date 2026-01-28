use csscolorparser::Color;
use crate::Precision;

// Continuous color mapping schemes (colormaps) for numerical data visualization.
// The color was got from https://hauselin.github.io/colorpalettejs/ and
// https://docs.rs/colorous/latest/colorous/index.html

/// Continuous color mapping schemes (colormaps) for numerical data visualization.
/// Optimized for direct SingleColor (f64) output to support high-performance rendering.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ColorMap {
    // Perceptually Uniform (Best for accurate data representation)
    Viridis,
    Inferno,
    Magma,
    Plasma,
    Cividis,

    // Sequential Single Hue
    Blues,
    Greens,
    Greys,
    Oranges,
    Purples,
    Reds,

    // Sequential Multi-Hue
    BuGn,   // Blue-Green
    BuPu,   // Blue-Purple
    GnBu,   // Green-Blue
    OrRd,   // Orange-Red
    PuBuGn, // Purple-Blue-Green
    PuBu,   // Purple-Blue
    PuRd,   // Purple-Red
    RdPu,   // Red-Purple
    YlGnBu, // Yellow-Green-Blue
    YlGn,   // Yellow-Green
    YlOrBr, // Yellow-Orange-Brown
    YlOrRd, // Yellow-Orange-Red

    // Specialized / Legacy
    Rainbow,
    Jet,
    Hot,
    Cool,
}

impl ColorMap {
    /// Returns a SingleColor based on a normalized value between 0.0 and 1.0.
    /// Following standard convention, the alpha channel is set to 1.0 (opaque).
    pub(crate) fn get_color(&self, value: f64) -> SingleColor {
        let t = value.clamp(0.0, 1.0) as f64;

        match self {
            ColorMap::Viridis => Self::interpolate_stops(&[
                (0.000, 0.267, 0.004, 0.329), // #440154
                (0.067, 0.282, 0.102, 0.424), // #481a6c
                (0.133, 0.278, 0.184, 0.490), // #472f7d
                (0.200, 0.255, 0.267, 0.529), // #414487
                (0.267, 0.224, 0.337, 0.549), // #39568c
                (0.333, 0.192, 0.408, 0.557), // #31688e
                (0.400, 0.165, 0.471, 0.557), // #2a788e
                (0.467, 0.137, 0.533, 0.557), // #23888e
                (0.533, 0.122, 0.596, 0.545), // #1f988b
                (0.600, 0.133, 0.659, 0.518), // #22a884
                (0.667, 0.208, 0.718, 0.475), // #35b779
                (0.733, 0.329, 0.773, 0.408), // #54c568
                (0.800, 0.478, 0.820, 0.318), // #7ad151
                (0.867, 0.647, 0.859, 0.212), // #a5db36
                (0.933, 0.824, 0.886, 0.106), // #d2e21b
                (1.000, 0.992, 0.906, 0.145), // #fde725
            ], t),
            ColorMap::Inferno => Self::interpolate_stops(&[
                (0.000, 0.000, 0.000, 0.016), // #000004
                (0.067, 0.047, 0.031, 0.149), // #0c0826
                (0.133, 0.141, 0.047, 0.310), // #240c4f
                (0.200, 0.259, 0.039, 0.408), // #420a68
                (0.267, 0.365, 0.071, 0.431), // #5d126e
                (0.333, 0.471, 0.110, 0.427), // #781c6d
                (0.400, 0.576, 0.149, 0.404), // #932667
                (0.467, 0.682, 0.188, 0.361), // #ae305c
                (0.533, 0.780, 0.243, 0.298), // #c73e4c
                (0.600, 0.867, 0.318, 0.227), // #dd513a
                (0.667, 0.929, 0.412, 0.145), // #ed6925
                (0.733, 0.973, 0.522, 0.059), // #f8850f
                (0.800, 0.988, 0.647, 0.039), // #fca50a
                (0.867, 0.980, 0.776, 0.176), // #fac62d
                (0.933, 0.949, 0.902, 0.380), // #f2e661
                (1.000, 0.988, 1.000, 0.643), // #fcffa4
            ], t),
            ColorMap::Magma => Self::interpolate_stops(&[
                (0.000, 0.000, 0.000, 0.016), // #000004
                (0.067, 0.043, 0.035, 0.141), // #0b0924
                (0.133, 0.125, 0.067, 0.294), // #20114b
                (0.200, 0.231, 0.059, 0.439), // #3b0f70
                (0.267, 0.341, 0.082, 0.494), // #57157e
                (0.333, 0.447, 0.122, 0.506), // #721f81
                (0.400, 0.549, 0.161, 0.506), // #8c2981
                (0.467, 0.659, 0.196, 0.490), // #a8327d
                (0.533, 0.769, 0.235, 0.459), // #c43c75
                (0.600, 0.871, 0.286, 0.408), // #de4968
                (0.667, 0.945, 0.376, 0.365), // #f1605d
                (0.733, 0.980, 0.498, 0.369), // #fa7f5e
                (0.800, 0.996, 0.624, 0.427), // #fe9f6d
                (0.867, 0.996, 0.749, 0.518), // #febf84
                (0.933, 0.992, 0.871, 0.627), // #fddea0
                (1.000, 0.988, 0.992, 0.749), // #fcfdbf
            ], t),
            ColorMap::Plasma => Self::interpolate_stops(&[
                (0.000, 0.051, 0.031, 0.529), // #0d0887
                (0.067, 0.200, 0.020, 0.592), // #330597
                (0.133, 0.314, 0.008, 0.635), // #5002a2
                (0.200, 0.416, 0.000, 0.659), // #6a00a8
                (0.267, 0.518, 0.020, 0.655), // #8405a7
                (0.333, 0.612, 0.090, 0.620), // #9c179e
                (0.400, 0.694, 0.165, 0.565), // #b12a90
                (0.467, 0.765, 0.239, 0.502), // #c33d80
                (0.533, 0.827, 0.318, 0.443), // #d35171
                (0.600, 0.882, 0.392, 0.384), // #e16462
                (0.667, 0.929, 0.475, 0.325), // #ed7953
                (0.733, 0.965, 0.561, 0.267), // #f68f44
                (0.800, 0.988, 0.651, 0.212), // #fca636
                (0.867, 0.996, 0.753, 0.161), // #fec029
                (0.933, 0.976, 0.863, 0.141), // #f9dc24
                (1.000, 0.941, 0.976, 0.129), // #f0f921
            ], t),
            ColorMap::Cividis => Self::interpolate_stops(&[
                (0.000, 0.000, 0.125, 0.318), // #002051
                (0.067, 0.008, 0.173, 0.396), // #022c65
                (0.133, 0.078, 0.220, 0.427), // #14386d
                (0.200, 0.169, 0.267, 0.431), // #2b446e
                (0.267, 0.259, 0.314, 0.431), // #42506e
                (0.333, 0.341, 0.361, 0.431), // #575c6e
                (0.400, 0.412, 0.412, 0.439), // #696970
                (0.467, 0.471, 0.459, 0.451), // #787573
                (0.533, 0.525, 0.510, 0.463), // #868276
                (0.600, 0.580, 0.561, 0.471), // #948f78
                (0.667, 0.643, 0.616, 0.471), // #a49d78
                (0.733, 0.714, 0.671, 0.443), // #b6ab73
                (0.800, 0.792, 0.729, 0.416), // #caba6a
                (0.867, 0.878, 0.788, 0.365), // #e0c95d
                (0.933, 0.949, 0.851, 0.314), // #f2d950
                (1.000, 0.992, 0.918, 0.271), // #fdea45
            ], t),

            // --- Sequential ---
            ColorMap::Blues => Self::interpolate_stops(&[
                (0.000, 0.969, 0.984, 1.000), // #f7fbff
                (0.125, 0.871, 0.922, 0.969), // #deebf7
                (0.250, 0.776, 0.859, 0.937), // #c6dbef
                (0.375, 0.620, 0.792, 0.882), // #9ecae1
                (0.500, 0.420, 0.682, 0.839), // #6baed6
                (0.625, 0.259, 0.573, 0.776), // #4292c6
                (0.750, 0.129, 0.443, 0.710), // #2171b5
                (0.875, 0.031, 0.318, 0.612), // #08519c
                (1.000, 0.031, 0.188, 0.420), // #08306b
            ], t),
            ColorMap::Greens => Self::interpolate_stops(&[
                (0.000, 0.969, 0.988, 0.961), // #f7fcf5
                (0.125, 0.898, 0.961, 0.878), // #e5f5e0
                (0.250, 0.780, 0.914, 0.753), // #c7e9c0
                (0.375, 0.631, 0.851, 0.608), // #a1d99b
                (0.500, 0.455, 0.769, 0.463), // #74c476
                (0.625, 0.255, 0.671, 0.365), // #41ab5d
                (0.750, 0.137, 0.545, 0.271), // #238b45
                (0.875, 0.000, 0.427, 0.173), // #006d2c
                (1.000, 0.000, 0.267, 0.106), // #00441b
            ], t),
            ColorMap::Greys => Self::interpolate_stops(&[
                (0.000, 1.000, 1.000, 1.000), // #ffffff
                (0.125, 0.941, 0.941, 0.941), // #f0f0f0
                (0.250, 0.851, 0.851, 0.851), // #d9d9d9
                (0.375, 0.741, 0.741, 0.741), // #bdbdbd
                (0.500, 0.588, 0.588, 0.588), // #969696
                (0.625, 0.451, 0.451, 0.451), // #737373
                (0.750, 0.322, 0.322, 0.322), // #525252
                (0.875, 0.145, 0.145, 0.145), // #252525
                (1.000, 0.000, 0.000, 0.000), // #000000
            ], t),
            ColorMap::Oranges => Self::interpolate_stops(&[
                (0.000, 1.000, 0.961, 0.922), // #fff5eb
                (0.125, 0.996, 0.902, 0.808), // #fee6ce
                (0.250, 0.992, 0.816, 0.635), // #fdd0a2
                (0.375, 0.992, 0.682, 0.420), // #fdae6b
                (0.500, 0.992, 0.553, 0.235), // #fd8d3c
                (0.625, 0.945, 0.412, 0.075), // #f16913
                (0.750, 0.851, 0.282, 0.004), // #d94801
                (0.875, 0.651, 0.212, 0.012), // #a63603
                (1.000, 0.498, 0.153, 0.016), // #7f2704
            ], t),
            ColorMap::Purples => Self::interpolate_stops(&[
                (0.000, 0.988, 0.984, 0.992), // #fcfbfd
                (0.125, 0.937, 0.929, 0.961), // #efedf5
                (0.250, 0.855, 0.855, 0.922), // #dadaeb
                (0.375, 0.737, 0.741, 0.863), // #bcbddc
                (0.500, 0.620, 0.604, 0.784), // #9e9ac8
                (0.625, 0.502, 0.490, 0.729), // #807dba
                (0.750, 0.416, 0.318, 0.639), // #6a51a3
                (0.875, 0.329, 0.153, 0.561), // #54278f
                (1.000, 0.247, 0.000, 0.490), // #3f007d
            ], t),
            ColorMap::Reds => Self::interpolate_stops(&[
                (0.000, 1.000, 0.961, 0.941), // #fff5f0
                (0.125, 0.996, 0.878, 0.824), // #fee0d2
                (0.250, 0.988, 0.733, 0.631), // #fcbba1
                (0.375, 0.988, 0.573, 0.447), // #fc9272
                (0.500, 0.984, 0.416, 0.290), // #fb6a4a
                (0.625, 0.937, 0.231, 0.173), // #ef3b2c
                (0.750, 0.796, 0.094, 0.114), // #cb181d
                (0.875, 0.647, 0.059, 0.082), // #a50f15
                (1.000, 0.404, 0.000, 0.051), // #67000d
            ], t),

            // --- Multi-hue Sequential ---
            ColorMap::BuGn => Self::interpolate_stops(&[
                (0.000, 0.969, 0.988, 0.992), // #f7fcfd
                (0.125, 0.898, 0.961, 0.976), // #e5f5f9
                (0.250, 0.800, 0.925, 0.902), // #ccece6
                (0.375, 0.600, 0.847, 0.788), // #99d8c9
                (0.500, 0.400, 0.761, 0.643), // #66c2a4
                (0.625, 0.255, 0.682, 0.463), // #41ae76
                (0.750, 0.137, 0.545, 0.271), // #238b45
                (0.875, 0.000, 0.427, 0.173), // #006d2c
                (1.000, 0.000, 0.267, 0.106), // #00441b
            ], t),
            ColorMap::BuPu => Self::interpolate_stops(&[
                (0.000, 0.969, 0.988, 0.992), // #f7fcfd
                (0.125, 0.878, 0.925, 0.957), // #e0ecf4
                (0.250, 0.749, 0.827, 0.902), // #bfd3e6
                (0.375, 0.620, 0.737, 0.855), // #9ebcda
                (0.500, 0.549, 0.588, 0.776), // #8c96c6
                (0.625, 0.549, 0.420, 0.694), // #8c6bb1
                (0.750, 0.533, 0.255, 0.616), // #88419d
                (0.875, 0.506, 0.059, 0.486), // #810f7c
                (1.000, 0.302, 0.000, 0.294), // #4d004b
            ], t),
            ColorMap::GnBu => Self::interpolate_stops(&[
                (0.000, 0.969, 0.988, 0.941), // #f7fcf0
                (0.125, 0.878, 0.953, 0.859), // #e0f3db
                (0.250, 0.800, 0.922, 0.773), // #ccebc5
                (0.375, 0.659, 0.867, 0.710), // #a8ddb5
                (0.500, 0.482, 0.800, 0.769), // #7bccc4
                (0.625, 0.306, 0.702, 0.827), // #4eb3d3
                (0.750, 0.169, 0.549, 0.745), // #2b8cbe
                (0.875, 0.031, 0.408, 0.675), // #0868ac
                (1.000, 0.031, 0.251, 0.506), // #084081
            ], t),
            ColorMap::OrRd => Self::interpolate_stops(&[
                (0.000, 1.000, 0.969, 0.925), // #fff7ec
                (0.125, 0.996, 0.910, 0.784), // #fee8c8
                (0.250, 0.992, 0.831, 0.620), // #fdd49e
                (0.375, 0.992, 0.733, 0.518), // #fdbb84
                (0.500, 0.988, 0.553, 0.349), // #fc8d59
                (0.625, 0.937, 0.396, 0.282), // #ef6548
                (0.750, 0.843, 0.188, 0.122), // #d7301f
                (0.875, 0.702, 0.000, 0.000), // #b30000
                (1.000, 0.498, 0.000, 0.000), // #7f0000
            ], t),
            ColorMap::PuBuGn => Self::interpolate_stops(&[
                (0.000, 1.000, 0.969, 0.984), // #fff7fb
                (0.125, 0.925, 0.886, 0.941), // #ece2f0
                (0.250, 0.816, 0.820, 0.902), // #d0d1e6
                (0.375, 0.651, 0.741, 0.859), // #a6bddb
                (0.500, 0.404, 0.663, 0.812), // #67a9cf
                (0.625, 0.212, 0.565, 0.753), // #3690c0
                (0.750, 0.008, 0.506, 0.541), // #02818a
                (0.875, 0.004, 0.424, 0.349), // #016c59
                (1.000, 0.004, 0.275, 0.212), // #014636
            ], t),
            ColorMap::PuBu => Self::interpolate_stops(&[
                (0.000, 1.000, 0.969, 0.984), // #fff7fb
                (0.125, 0.925, 0.906, 0.949), // #ece7f2
                (0.250, 0.816, 0.820, 0.902), // #d0d1e6
                (0.375, 0.651, 0.741, 0.859), // #a6bddb
                (0.500, 0.455, 0.663, 0.812), // #74a9cf
                (0.625, 0.212, 0.565, 0.753), // #3690c0
                (0.750, 0.020, 0.439, 0.690), // #0570b0
                (0.875, 0.016, 0.353, 0.553), // #045a8d
                (1.000, 0.008, 0.220, 0.345), // #023858
            ], t),
            ColorMap::PuRd => Self::interpolate_stops(&[
                (0.000, 0.969, 0.957, 0.976), // #f7f4f9
                (0.125, 0.906, 0.882, 0.937), // #e7e1ef
                (0.250, 0.831, 0.725, 0.855), // #d4b9da
                (0.375, 0.788, 0.580, 0.780), // #c994c7
                (0.500, 0.875, 0.396, 0.690), // #df65b0
                (0.625, 0.906, 0.161, 0.541), // #e7298a
                (0.750, 0.808, 0.071, 0.337), // #ce1256
                (0.875, 0.596, 0.000, 0.263), // #980043
                (1.000, 0.404, 0.000, 0.122), // #67001f
            ], t),
            ColorMap::RdPu => Self::interpolate_stops(&[
                (0.000, 1.000, 0.969, 0.953), // #fff7f3
                (0.125, 0.992, 0.878, 0.867), // #fde0dd
                (0.250, 0.988, 0.773, 0.753), // #fcc5c0
                (0.375, 0.980, 0.624, 0.710), // #fa9fb5
                (0.500, 0.969, 0.408, 0.631), // #f768a1
                (0.625, 0.867, 0.204, 0.592), // #dd3497
                (0.750, 0.682, 0.004, 0.494), // #ae017e
                (0.875, 0.478, 0.004, 0.467), // #7a0177
                (1.000, 0.286, 0.000, 0.416), // #49006a
            ], t),
            ColorMap::YlGnBu => Self::interpolate_stops(&[
                (0.000, 1.000, 1.000, 0.851), // #ffffd9
                (0.125, 0.929, 0.973, 0.694), // #edf8b1
                (0.250, 0.780, 0.914, 0.706), // #c7e9b4
                (0.375, 0.498, 0.804, 0.733), // #7fcdbb
                (0.500, 0.255, 0.714, 0.769), // #41b6c4
                (0.625, 0.114, 0.569, 0.753), // #1d91c0
                (0.750, 0.133, 0.369, 0.659), // #225ea8
                (0.875, 0.145, 0.204, 0.580), // #253494
                (1.000, 0.031, 0.114, 0.345), // #081d58
            ], t),
            ColorMap::YlGn => Self::interpolate_stops(&[
                (0.000, 1.000, 1.000, 0.898), // #ffffe5
                (0.125, 0.969, 0.988, 0.725), // #f7fcb9
                (0.250, 0.851, 0.941, 0.639), // #d9f0a3
                (0.375, 0.678, 0.867, 0.557), // #addd8e
                (0.500, 0.471, 0.776, 0.475), // #78c679
                (0.625, 0.255, 0.671, 0.365), // #41ab5d
                (0.750, 0.137, 0.518, 0.263), // #238443
                (0.875, 0.000, 0.408, 0.216), // #006837
                (1.000, 0.000, 0.271, 0.161), // #004529
            ], t),
            ColorMap::YlOrBr => Self::interpolate_stops(&[
                (0.000, 1.000, 1.000, 0.898), // #ffffe5
                (0.125, 1.000, 0.969, 0.737), // #fff7bc
                (0.250, 0.996, 0.890, 0.569), // #fee391
                (0.375, 0.996, 0.769, 0.310), // #fec44f
                (0.500, 0.996, 0.600, 0.161), // #fe9929
                (0.625, 0.925, 0.439, 0.078), // #ec7014
                (0.750, 0.800, 0.298, 0.008), // #cc4c02
                (0.875, 0.600, 0.204, 0.016), // #993404
                (1.000, 0.400, 0.145, 0.024), // #662506
            ], t),
            ColorMap::YlOrRd => Self::interpolate_stops(&[
                (0.000, 1.000, 1.000, 0.800), // #ffffcc
                (0.125, 1.000, 0.929, 0.627), // #ffeda0
                (0.250, 0.996, 0.851, 0.463), // #fed976
                (0.375, 0.996, 0.698, 0.298), // #feb24c
                (0.500, 0.992, 0.553, 0.235), // #fd8d3c
                (0.625, 0.988, 0.306, 0.165), // #fc4e2a
                (0.750, 0.890, 0.102, 0.110), // #e31a1c
                (0.875, 0.741, 0.000, 0.149), // #bd0026
                (1.000, 0.502, 0.000, 0.149), // #800026
            ], t),

            // --- Specialized ---
            ColorMap::Rainbow => Self::hsv_to_rgb((1.0 - t) * 300.0, 1.0, 1.0),
            ColorMap::Jet => Self::interpolate_stops(&[(0.00, 0.0, 0.0, 1.0), (0.33, 0.0, 1.0, 1.0), (0.66, 1.0, 1.0, 0.0), (1.00, 1.0, 0.0, 0.0)], t),
            ColorMap::Hot => {
                if t < 0.33 { SingleColor::from_rgba(t / 0.33, 0.0, 0.0, 1.0) }
                else if t < 0.66 { SingleColor::from_rgba(1.0, (t - 0.33) / 0.33, 0.0, 1.0) }
                else { SingleColor::from_rgba(1.0, 1.0, (t - 0.66) / 0.34, 1.0) }
            },
            ColorMap::Cool => SingleColor::from_rgba(t, 1.0 - t, 1.0, 1.0),
        }
    }

    /// Linearly interpolates between RGB stops. Alpha remains 1.0 by convention.
    fn interpolate_stops(stops: &[(f64, f64, f64, f64)], t: f64) -> SingleColor {
        let first = stops[0];
        let last = stops[stops.len() - 1];

        if t <= first.0 { return SingleColor::from_rgba(first.1, first.2, first.3, 1.0); }
        if t >= last.0 { return SingleColor::from_rgba(last.1, last.2, last.3, 1.0); }

        for i in 0..stops.len() - 1 {
            let (p1, r1, g1, b1) = stops[i];
            let (p2, r2, g2, b2) = stops[i + 1];
            if t >= p1 && t <= p2 {
                let f = (t - p1) / (p2 - p1);
                return SingleColor::from_rgba(r1 + f * (r2 - r1), g1 + f * (g2 - g1), b1 + f * (b2 - b1), 1.0);
            }
        }
        SingleColor::from_rgba(first.1, first.2, first.3, 1.0)
    }

    fn hsv_to_rgb(h: f64, s: f64, v: f64) -> SingleColor {
        // Ensure hue is within [0.0, 360.0) range using Euclidean remainder
        let h = h.rem_euclid(360.0);
        
        // Chroma: the intensity of the color
        let c = v * s;
        
        // x: intermediate value for the second largest component
        let h_prime = h / 60.0;
        let x = c * (1.0 - (h_prime % 2.0 - 1.0).abs());
        
        // m: matching value to add to each component to match lightness (value)
        let m = v - c;
        
        // Determine R', G', B' based on which 60-degree sector of the color wheel we are in
        let (r_prime, g_prime, b_prime) = if h_prime < 1.0 { (c, x, 0.0) }
            else if h_prime < 2.0 { (x, c, 0.0) }
            else if h_prime < 3.0 { (0.0, c, x) }
            else if h_prime < 4.0 { (0.0, x, c) }
            else if h_prime < 5.0 { (x, 0.0, c) }
            else { (c, 0.0, x) };

        // Add m to each component and return as SingleColor (Alpha set to 1.0)
        SingleColor::from_rgba(r_prime + m, g_prime + m, b_prime + m, 1.0)
    }
}


/// Discrete color palettes for categorical data visualization.
/// Optimized to return `SingleColor` with pre-calculated f64 RGBA values.
#[derive(Clone, Copy, Debug, PartialEq)]
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
    /// Returns a specific color from the palette by index (with automatic wrapping).
    /// Bypasses hex parsing by using pre-calculated RGBA components.
    pub(crate) fn get_color(&self, index: usize) -> SingleColor {
        let colors = self.rgba_colors();
        let (r, g, b) = colors[index % colors.len()];
        SingleColor::from_rgba(r, g, b, 1.0)
    }

    /// Internal storage of palette colors as normalized (r, g, b) f64 tuples.
    /// This avoids hex string parsing at runtime.
    fn rgba_colors(&self) -> &'static [(f64, f64, f64)] {
        match self {
            ColorPalette::Tab10 => &[
                (0.122, 0.467, 0.706), (1.000, 0.498, 0.055), (0.173, 0.627, 0.173), (0.839, 0.153, 0.157),
                (0.580, 0.404, 0.741), (0.549, 0.337, 0.294), (0.890, 0.467, 0.761), (0.498, 0.498, 0.498),
                (0.737, 0.741, 0.133), (0.090, 0.745, 0.812),
            ],
            ColorPalette::Tab20 => &[
                (0.122, 0.467, 0.706), (0.682, 0.780, 0.910), (1.000, 0.498, 0.055), (1.000, 0.733, 0.471),
                (0.173, 0.627, 0.173), (0.596, 0.875, 0.541), (0.839, 0.153, 0.157), (1.000, 0.596, 0.588),
                (0.580, 0.404, 0.741), (0.773, 0.690, 0.835), (0.549, 0.337, 0.294), (0.769, 0.612, 0.580),
                (0.890, 0.467, 0.761), (0.969, 0.714, 0.824), (0.498, 0.498, 0.498), (0.780, 0.780, 0.780),
                (0.737, 0.741, 0.133), (0.859, 0.859, 0.553), (0.090, 0.745, 0.812), (0.620, 0.855, 0.898),
            ],
            ColorPalette::Set1 => &[
                (0.894, 0.102, 0.110), (0.216, 0.494, 0.722), (0.302, 0.686, 0.290), (0.596, 0.306, 0.639),
                (1.000, 0.498, 0.000), (1.000, 1.000, 0.200), (0.651, 0.337, 0.157), (0.969, 0.506, 0.749),
                (0.600, 0.600, 0.600),
            ],
            ColorPalette::Set2 => &[
                (0.400, 0.761, 0.647), (0.988, 0.553, 0.384), (0.553, 0.627, 0.796), (0.906, 0.541, 0.765),
                (0.651, 0.847, 0.329), (1.000, 0.851, 0.184), (0.898, 0.769, 0.580), (0.702, 0.702, 0.702),
            ],
            ColorPalette::Set3 => &[
                (0.553, 0.827, 0.780), (1.000, 1.000, 0.702), (0.745, 0.729, 0.855), (0.984, 0.502, 0.447),
                (0.502, 0.694, 0.827), (0.992, 0.706, 0.384), (0.702, 0.871, 0.412), (0.988, 0.804, 0.898),
                (0.851, 0.851, 0.851), (0.737, 0.502, 0.741), (0.800, 0.922, 0.773), (1.000, 0.929, 0.435),
            ],
            ColorPalette::Pastel1 => &[
                (0.984, 0.706, 0.682), (0.702, 0.804, 0.890), (0.800, 0.922, 0.773), (0.871, 0.796, 0.894),
                (0.996, 0.851, 0.651), (1.000, 1.000, 0.800), (0.898, 0.847, 0.741), (0.992, 0.855, 0.925),
                (0.949, 0.949, 0.949),
            ],
            ColorPalette::Pastel2 => &[
                (0.702, 0.886, 0.804), (0.992, 0.804, 0.675), (0.796, 0.835, 0.910), (0.957, 0.792, 0.894),
                (0.902, 0.961, 0.788), (1.000, 0.949, 0.682), (0.945, 0.886, 0.800), (0.800, 0.800, 0.800),
            ],
            ColorPalette::Dark2 => &[
                (0.106, 0.620, 0.467), (0.851, 0.373, 0.008), (0.459, 0.439, 0.702), (0.906, 0.161, 0.541),
                (0.400, 0.651, 0.118), (0.902, 0.671, 0.008), (0.651, 0.463, 0.114), (0.400, 0.400, 0.400),
            ],
            ColorPalette::Accent => &[
                (0.498, 0.788, 0.498), (0.745, 0.682, 0.831), (0.992, 0.753, 0.525), (1.000, 1.000, 0.600),
                (0.220, 0.424, 0.690), (0.941, 0.008, 0.498), (0.749, 0.357, 0.090), (0.400, 0.400, 0.400),
            ],
        }
    }
}

/// A lightweight, copyable color representation using normalized RGBA values.
/// 
/// By storing only numerical values, this struct is optimized for:
/// 1. **Memory Efficiency**: It implements `Copy`, allowing it to be passed 
///    by value without heap allocations.
/// 2. **GPU Performance**: The `[Precision; 4]` array maps directly to 
///    GPU vertex buffers (f32) without runtime conversion.
/// 3. **Backend Agnostic**: SVG strings are generated on-the-fly only when 
///    needed, ensuring no redundant memory is used for thousands of points.
/// 
/// This version integrates `csscolorparser` to allow creation from CSS strings
/// (like "#ff0000" or "rgba(255,0,0,0.5)") while maintaining a stack-allocated 
/// internal structure for performance.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SingleColor {
    /// Pre-parsed RGBA normalized values [0.0 - 1.0].
    rgba: [Precision; 4],
}

impl SingleColor {
    /// Creates a new `SingleColor` by parsing a CSS color string.
    /// 
    /// If parsing fails, it defaults to opaque black.
    /// Supports: Hex, RGB, RGBA, HSL, and named colors.
    pub fn new(color_str: &str) -> Self {
        let color_lc = color_str.to_lowercase();
        
        // Handle the "none" state which is common in SVG but not always 
        // in standard CSS parsers.
        if color_lc == "none" || color_lc == "transparent" {
            return Self::none();
        }

        // Parse using csscolorparser
        let parsed = color_str.parse::<Color>().unwrap_or_else(|_| {
            // Fallback to opaque black on error
            Color::new(0.0, 0.0, 0.0, 1.0)
        });

        Self {
            rgba: [
                parsed.r as Precision,
                parsed.g as Precision,
                parsed.b as Precision,
                parsed.a as Precision,
            ],
        }
    }

    pub fn none() -> Self {
        Self { rgba: [0.0, 0.0, 0.0, 0.0] }
    }

    pub fn from_rgba(r: f64, g: f64, b: f64, a: f64) -> Self {
        Self {
            rgba: [
                r.clamp(0.0, 1.0) as Precision,
                g.clamp(0.0, 1.0) as Precision,
                b.clamp(0.0, 1.0) as Precision,
                a.clamp(0.0, 1.0) as Precision,
            ],
        }
    }

    pub fn rgba(&self) -> [Precision; 4] {
        self.rgba
    }

    pub fn is_none(&self) -> bool {
        self.rgba[3] <= 0.0
    }

    /// Generates a CSS-compatible string. 
    /// Note: This allocates a new String. Use sparingly in tight loops.
    pub fn to_css_string(&self) -> String {
        if self.is_none() {
            "none".to_string()
        } else {
            let c = self.rgba;
            format!(
                "rgba({},{},{},{:.3})",
                (c[0] * 255.0).round() as u8,
                (c[1] * 255.0).round() as u8,
                (c[2] * 255.0).round() as u8,
                c[3]
            )
        }
    }
}

// --- Fluent API Support ---

impl From<&str> for SingleColor {
    /// Allows: let c: SingleColor = "#ff0000".into();
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for SingleColor {
    /// Allows: let c: SingleColor = String::from("red").into();
    fn from(s: String) -> Self {
        Self::new(&s)
    }
}

impl From<[f64; 4]> for SingleColor {
    fn from(c: [f64; 4]) -> Self {
        Self::from_rgba(c[0], c[1], c[2], c[3])
    }
}