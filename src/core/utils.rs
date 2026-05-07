use ahash::AHashMap;

/// Estimates text width using character categorization.
pub(crate) fn estimate_text_width(text: &str, font_size: f64) -> f64 {
    let mut narrow_chars = 0;
    let mut uppercase_chars = 0;
    let mut other_chars = 0;

    for c in text.chars() {
        if matches!(
            c,
            '.' | ',' | ':' | ';' | '!' | 'i' | 'j' | 'l' | '-' | '|' | '1' | 't' | 'f' | 'r'
        ) {
            narrow_chars += 1;
        } else if c.is_ascii_uppercase() {
            uppercase_chars += 1;
        } else {
            other_chars += 1;
        }
    }

    (narrow_chars as f64 * 0.3 + uppercase_chars as f64 * 0.65 + other_chars as f64 * 0.55)
        * font_size
}

// =============================== Font Database Utilities =====================================
#[cfg(feature = "pdf")]
use std::sync::{Arc, OnceLock};

/// Global cache for the font database to avoid expensive system scans on every render.
/// We use Arc to allow thread-safe sharing across multiple chart generation tasks.
#[cfg(feature = "pdf")]
static GLOBAL_FONT_DB: OnceLock<Arc<svg2pdf::usvg::fontdb::Database>> = OnceLock::new();

/// Retrieves a shared instance of the font database.
///
/// The first call triggers a full system font scan and loads the embedded fallback fonts.
/// Subsequent calls return a cloned reference to the cached database, reducing the
/// font initialization overhead to nearly 0ms.
#[cfg(feature = "pdf")]
pub(crate) fn get_font_db() -> Arc<svg2pdf::usvg::fontdb::Database> {
    GLOBAL_FONT_DB
        .get_or_init(|| {
            // Initialize the database using the usvg version re-exported by svg2pdf.
            let mut fontdb = svg2pdf::usvg::fontdb::Database::new();

            // 1. Scan the operating system's font directories.
            // This is an expensive I/O operation, which is why we cache the result globally.
            fontdb.load_system_fonts();

            // 2. Load the built-in "emergency" font.
            // This ensures consistent text rendering even in restricted environments
            // like minimal Docker containers or CI runners where system fonts may be missing.
            let default_font_data = include_bytes!("../../assets/fonts/Inter-Regular.ttf");
            fontdb.load_font_data(default_font_data.to_vec());

            // 3. Define the default fallback family for 'sans-serif' requests.
            // When an SVG specifies "sans-serif" but no system mapping exists,
            // the renderer will use the "Inter" font we just loaded.
            fontdb.set_sans_serif_family("Inter");

            Arc::new(fontdb)
        })
        .clone()
}

// =============================== Raster Font Utilities (PNG) =====================================
#[cfg(feature = "png")]
use ab_glyph::FontArc;
#[cfg(feature = "png")]
use std::sync::{OnceLock, RwLock};

/// Global cache for raster fonts.
/// Maps lowercase font family names to FontArc instances.
#[cfg(feature = "png")]
static RASTER_FONT_REGISTRY: OnceLock<RwLock<AHashMap<String, FontArc>>> = OnceLock::new();

/// Global cache for the system font database.
/// This allows us to search for system fonts by name without rescanning the OS directories every time.
#[cfg(feature = "png")]
static SYSTEM_FONT_DB: OnceLock<fontdb::Database> = OnceLock::new();

/// Retrieves or initializes the global system font database.
/// This performs an expensive I/O operation (scanning OS font dirs) only once.
#[cfg(feature = "png")]
fn get_system_font_db() -> &'static fontdb::Database {
    SYSTEM_FONT_DB.get_or_init(|| {
        let mut db = fontdb::Database::new();
        // Load all available system fonts
        db.load_system_fonts();
        db
    })
}

/// Initializes the font registry with the default embedded font (Inter).
#[cfg(feature = "png")]
fn get_raster_registry() -> &'static RwLock<AHashMap<String, FontArc>> {
    RASTER_FONT_REGISTRY.get_or_init(|| {
        let mut map = AHashMap::new();

        // Load default fallback font (Inter) from embedded assets
        let default_font_data = include_bytes!("../../assets/fonts/Inter-Regular.ttf");
        if let Ok(font) = FontArc::try_from_slice(default_font_data) {
            // Register as "inter" for explicit requests
            map.insert("inter".to_string(), font.clone());
            // Register as "sans-serif" for generic fallbacks
            map.insert("sans-serif".to_string(), font);
        } else {
            eprintln!("Warning: Failed to load default Inter font for raster rendering.");
        }

        RwLock::new(map)
    })
}

/// Retrieves a raster font by family name.
///
/// Logic:
/// 1. Check if the font is already loaded in our local cache.
/// 2. If not, search the System Font Database for the family name.
/// 3. If found in system, load it into memory, cache it, and return it.
/// 4. If not found in system, fallback to "sans-serif" (Inter).
///
/// # Arguments
/// * `family` - The font family name (e.g., "Arial", "Times New Roman", "Inter"). Case-insensitive.
#[cfg(feature = "png")]
pub(crate) fn get_raster_font(family: &str) -> FontArc {
    let registry = get_raster_registry();

    // 1. Fast path: Check if already loaded in our local cache
    {
        let map = registry.read().expect("Failed to read font registry");
        if let Some(font) = map.get(&family.to_lowercase()) {
            return font.clone();
        }
    }

    // 2. Slow path: Search system fonts and load if found
    // We need a write lock to insert the newly loaded font into the cache
    let mut map = registry.write().expect("Failed to write to font registry");

    // Double-check after acquiring write lock (another thread might have loaded it)
    if let Some(font) = map.get(&family.to_lowercase()) {
        return font.clone();
    }

    // Search in the global system font database
    let sys_db = get_system_font_db();
    let families = [fontdb::Family::Name(family)];
    let query = fontdb::Query {
        families: &families,
        weight: fontdb::Weight::NORMAL,
        stretch: fontdb::Stretch::Normal,
        style: fontdb::Style::Normal,
    };

    if let Some(id) = sys_db.query(&query) {
        if let Some(face_info) = sys_db.face(id) {
            // Check if the font source is a file and read it
            if let fontdb::Source::File(ref path) = face_info.source {
                if let Ok(font_data) = std::fs::read(path) {
                    // Parse the font data into an AbGlyph FontArc
                    if let Ok(font) = FontArc::try_from_vec(font_data) {
                        // Cache it for future requests
                        map.insert(family.to_lowercase(), font.clone());
                        return font;
                    }
                }
            }
        }
    }

    // 3. Fallback: If system font not found or failed to load, use sans-serif (Inter)
    if let Some(font) = map.get("sans-serif") {
        return font.clone();
    }

    // Ultimate fallback: Panic if no fonts are available (should not happen if Inter loads correctly)
    panic!(
        "No fonts available. Default 'Inter' font failed to load and '{}' not found in system.",
        family
    );
}

/// Registers a new font for raster rendering from raw data.
///
/// This allows users to add custom fonts that are not present in the system
/// or embedded assets.
///
/// # Arguments
/// * `name` - The font family name (e.g., "MyCustomFont").
/// * `data` - The raw TTF/OTF font data.
///
/// # Example
/// ```ignore
/// use charton::core::utils::register_raster_font;
/// let font_data = std::fs::read("path/to/font.ttf")?;
/// register_raster_font("MyFont", font_data)?;
/// ```
#[cfg(feature = "png")]
pub fn register_raster_font(name: &str, data: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
    // try_from_vec takes ownership of the Vec, ensuring the data lives as long as the FontArc
    let font = FontArc::try_from_vec(data)?;

    let registry = get_raster_registry();
    let mut map = registry.write().expect("Failed to write to font registry");
    map.insert(name.to_lowercase(), font);
    Ok(())
}

//=============================== Parallelization Utilities =====================================
#[cfg(feature = "parallel")]
use rayon::prelude::*;

// --- Trait Definitions ---

pub trait IntoParallelizable {
    type Item;
    #[cfg(feature = "parallel")]
    type Iter: ParallelIterator<Item = Self::Item>;
    #[cfg(not(feature = "parallel"))]
    type Iter: Iterator<Item = Self::Item>;

    fn maybe_into_par_iter(self) -> Self::Iter;
}

pub trait Parallelizable {
    type Item;
    #[cfg(feature = "parallel")]
    type Iter: ParallelIterator<Item = Self::Item>;
    #[cfg(not(feature = "parallel"))]
    type Iter: Iterator<Item = Self::Item>;

    fn maybe_par_iter(self) -> Self::Iter;
}

// --- Implementation for Shared References (Read-only) ---

impl<'a, T: Sync + Send + 'a> Parallelizable for &'a Vec<T> {
    type Item = &'a T;

    #[cfg(feature = "parallel")]
    type Iter = rayon::slice::Iter<'a, T>;
    #[cfg(not(feature = "parallel"))]
    type Iter = std::slice::Iter<'a, T>;

    fn maybe_par_iter(self) -> Self::Iter {
        #[cfg(feature = "parallel")]
        {
            self.par_iter()
        }
        #[cfg(not(feature = "parallel"))]
        {
            self.iter()
        }
    }
}

impl<'a, T: Sync + Send + 'a> Parallelizable for &'a [T] {
    type Item = &'a T;

    #[cfg(feature = "parallel")]
    type Iter = rayon::slice::Iter<'a, T>;
    #[cfg(not(feature = "parallel"))]
    type Iter = std::slice::Iter<'a, T>;

    fn maybe_par_iter(self) -> Self::Iter {
        #[cfg(feature = "parallel")]
        {
            self.par_iter()
        }
        #[cfg(not(feature = "parallel"))]
        {
            self.iter()
        }
    }
}

// --- Implementation for Mutable References (Read-Write) ---

impl<'a, T: Sync + Send + 'a> Parallelizable for &'a mut Vec<T> {
    type Item = &'a mut T;

    #[cfg(feature = "parallel")]
    type Iter = rayon::slice::IterMut<'a, T>;
    #[cfg(not(feature = "parallel"))]
    type Iter = std::slice::IterMut<'a, T>;

    fn maybe_par_iter(self) -> Self::Iter {
        #[cfg(feature = "parallel")]
        {
            self.par_iter_mut()
        }
        #[cfg(not(feature = "parallel"))]
        {
            self.iter_mut()
        }
    }
}

// --- Implementation for AHashMap (Shared) ---

impl<'a, K: Sync + Send + 'a, V: Sync + Send + 'a> Parallelizable for &'a AHashMap<K, V> {
    type Item = (&'a K, &'a V);

    #[cfg(feature = "parallel")]
    type Iter = rayon::collections::hash_map::Iter<'a, K, V>;
    #[cfg(not(feature = "parallel"))]
    type Iter = std::collections::hash_map::Iter<'a, K, V>;

    fn maybe_par_iter(self) -> Self::Iter {
        #[cfg(feature = "parallel")]
        {
            self.par_iter()
        }
        #[cfg(not(feature = "parallel"))]
        {
            self.iter()
        }
    }
}

// --- Implementation for AHashMap (Mutable) ---

impl<'a, K: Sync + Send + 'a, V: Sync + Send + 'a> Parallelizable for &'a mut AHashMap<K, V> {
    type Item = (&'a K, &'a mut V);

    #[cfg(feature = "parallel")]
    type Iter = rayon::collections::hash_map::IterMut<'a, K, V>;
    #[cfg(not(feature = "parallel"))]
    type Iter = std::collections::hash_map::IterMut<'a, K, V>;

    fn maybe_par_iter(self) -> Self::Iter {
        #[cfg(feature = "parallel")]
        {
            self.par_iter_mut()
        }
        #[cfg(not(feature = "parallel"))]
        {
            self.iter_mut()
        }
    }
}

// --- Implementation for Owned Types (Consuming) ---

impl<T: Send + Sync> IntoParallelizable for Vec<T> {
    type Item = T;
    #[cfg(feature = "parallel")]
    type Iter = rayon::vec::IntoIter<T>;
    #[cfg(not(feature = "parallel"))]
    type Iter = std::vec::IntoIter<T>;

    fn maybe_into_par_iter(self) -> Self::Iter {
        #[cfg(feature = "parallel")]
        {
            self.into_par_iter()
        }
        #[cfg(not(feature = "parallel"))]
        {
            self.into_iter()
        }
    }
}

impl IntoParallelizable for std::ops::Range<usize> {
    type Item = usize;
    #[cfg(feature = "parallel")]
    type Iter = rayon::range::Iter<usize>;
    #[cfg(not(feature = "parallel"))]
    type Iter = std::ops::Range<usize>;

    fn maybe_into_par_iter(self) -> Self::Iter {
        #[cfg(feature = "parallel")]
        {
            self.into_par_iter()
        }
        #[cfg(not(feature = "parallel"))]
        {
            self.into_iter()
        }
    }
}
