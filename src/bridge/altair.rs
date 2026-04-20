use crate::bridge::{Altair, Plot};
use crate::error::ChartonError;
use std::fs;
use std::path::Path;

impl Plot<Altair> {
    /// Generates and returns the Vega-Lite JSON representation.
    pub fn to_json(&self) -> Result<String, ChartonError> {
        let session = self.get_session()?;

        // 1. Inject data (In-memory)
        session.feed_dataset(&self.data.name, &self.data)?;

        // 2. Prepare code to extract JSON
        let full_code = format!(
            "import altair as alt\n{}\n__json_out = chart.to_json()",
            self.raw_plotting_code
        );
        session.run_code(&full_code)?;

        // 3. Extract from Python heap
        pyo3::Python::with_gil(|py| {
            let globals = session.globals.bind(py);
            let json_val: String = globals
                .get_item("__json_out")?
                .ok_or_else(|| {
                    ChartonError::Internal(
                        "Variable 'chart' not found. Ensure your code defines 'chart = ...'".into(),
                    )
                })?
                .extract()?;
            Ok(json_val)
        })
    }

    /// Generates SVG using Python's vl_convert (matching your previous logic)
    pub fn to_svg(&self) -> Result<String, ChartonError> {
        let session = self.get_session()?;
        session.feed_dataset(&self.data.name, &self.data)?;

        // We use vl_convert in Python side to ensure high-quality SVG generation
        let full_code = format!(
            "import altair as alt\nimport vl_convert as vlc\n{}\n__svg_out = vlc.vegalite_to_svg(chart.to_json())",
            self.raw_plotting_code
        );
        session.run_code(&full_code)?;

        pyo3::Python::with_gil(|py| {
            let globals = session.globals.bind(py);
            let svg_val: String = globals
                .get_item("__svg_out")?
                .ok_or_else(|| {
                    ChartonError::Internal("Failed to generate SVG via vl_convert".into())
                })?
                .extract()?;
            Ok(svg_val)
        })
    }

    /// Displays the plot. Supports Jupyter (EVCXR) environment.
    pub fn show(&self) -> Result<(), ChartonError> {
        // Check if we are in EVCXR (Jupyter Rust)
        if std::env::var("EVCXR_IS_RUNTIME").is_ok() {
            let json_spec = self.to_json()?;
            println!("EVCXR_BEGIN_CONTENT application/vnd.vegalite.v5+json");
            println!("{}", json_spec);
            println!("EVCXR_END_CONTENT");
        } else {
            // Standard environment: Use Altair's native show() which opens a browser
            let session = self.get_session()?;
            session.feed_dataset(&self.data.name, &self.data)?;
            let full_code = format!(
                "import altair as alt\n{}\nchart.show()",
                self.raw_plotting_code
            );
            session.run_code(&full_code)?;
        }
        Ok(())
    }

    /// Saves the plot to a file, inferring format from extension.
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), ChartonError> {
        let path_obj = path.as_ref();

        // Ensure parent directory exists (Matching your previous logic)
        if let Some(parent) = path_obj.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).map_err(ChartonError::Io)?;
            }
        }

        let ext = path_obj
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase());

        match ext.as_deref() {
            Some("json") => {
                let content = self.to_json()?;
                fs::write(path_obj, content).map_err(ChartonError::Io)?;
            }
            Some("svg") => {
                let content = self.to_svg()?;
                fs::write(path_obj, content).map_err(ChartonError::Io)?;
            }
            Some("png") => {
                #[cfg(feature = "png")]
                {
                    self.save_png_via_resvg(path_obj)?;
                }
                #[cfg(not(feature = "png"))]
                {
                    return Err(ChartonError::Internal(
                        "PNG support is disabled. Enable the 'png' feature in Cargo.toml".into(),
                    ));
                }
            }
            Some("html") => {
                let session = self.get_session()?;
                session.feed_dataset(&self.data.name, &self.data)?;
                let save_path = path_obj.to_string_lossy();
                let full_code = format!(
                    "import altair as alt\n{}\nchart.save(r'{}')",
                    self.raw_plotting_code, save_path
                );
                session.run_code(&full_code)?;
            }
            _ => {
                return Err(ChartonError::Internal(format!(
                    "Unsupported format: {:?}",
                    ext
                )));
            }
        }
        Ok(())
    }

    /// Internal helper for PNG rendering (ported from your previous logic)
    #[cfg(feature = "png")]
    fn save_png_via_resvg(&self, path: &Path) -> Result<(), ChartonError> {
        let svg_content = self.to_svg()?;

        let mut opts = resvg::usvg::Options::default();
        let mut fontdb = (*opts.fontdb).clone();
        fontdb.load_system_fonts();
        opts.fontdb = std::sync::Arc::new(fontdb);

        let tree = resvg::usvg::Tree::from_str(&svg_content, &opts)
            .map_err(|e| ChartonError::Internal(format!("SVG parsing error: {:?}", e)))?;

        let pixmap_size = tree.size();
        let scale = 2.0;
        let mut pixmap = resvg::tiny_skia::Pixmap::new(
            (pixmap_size.width() * scale) as u32,
            (pixmap_size.height() * scale) as u32,
        )
        .ok_or_else(|| ChartonError::Internal("Failed to create pixmap".into()))?;

        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::from_scale(scale, scale),
            &mut pixmap.as_mut(),
        );
        pixmap
            .save_png(path)
            .map_err(|e| ChartonError::Internal(format!("PNG saving error: {:?}", e)))?;

        Ok(())
    }
}
