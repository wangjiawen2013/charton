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

//=============================== Parallelization Utilities =====================================
use ahash::AHashMap;
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
