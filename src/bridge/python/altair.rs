use crate::bridge::base::{
    Altair, ExternalRendererExecutor, InputData, Plot, SerializedData, Visualization,
};
use crate::error::ChartonError;
use base64::Engine;
use polars::prelude::*;
use regex::Regex;
use std::io::{Cursor, Seek, SeekFrom, Write};
use std::marker::PhantomData;
use std::process::{Command, Stdio};

impl Plot<Altair> {
    /// Generates and returns the Vega-Lite JSON representation of the chart.
    ///
    /// This method executes the Python plotting code with JSON output format and
    /// returns the resulting Vega-Lite specification as a JSON string. The JSON
    /// can be used directly in web applications or Jupyter notebooks that support
    /// Vega-Lite visualizations.
    ///
    /// # Returns
    /// A Result containing either:
    /// - Ok(String) with the Vega-Lite JSON specification of the chart
    /// - Err(ChartonError) if there was an error during execution
    ///
    /// # Example
    /// ```
    /// let json_spec = plot.to_json()?;
    /// // Use the JSON specification in a web application or save to file
    /// ```
    pub fn to_json(&self) -> Result<String, ChartonError> {
        let full_plotting_code = self.generate_full_plotting_code("json")?;
        let json_content = self.execute_plotting_code(&full_plotting_code)?;
        Ok(json_content)
    }

    fn to_svg(&self) -> Result<String, ChartonError> {
        let full_plotting_code = self.generate_full_plotting_code("svg")?;
        let svg_content = self.execute_plotting_code(&full_plotting_code)?;
        Ok(svg_content)
    }
}

impl ExternalRendererExecutor for Plot<Altair> {
    fn generate_full_plotting_code(&self, output_format: &str) -> Result<String, ChartonError> {
        let ipc_to_df = r#"
import json
import sys
import base64
import polars as pl
from io import BytesIO

data = json.loads(sys.stdin.read())
ipc_data = base64.b64decode(data["value"])
__charton_temp_df_name_fm_n9jh3 = pl.read_ipc(BytesIO(ipc_data))
"#;

        let output = match output_format {
            "svg" => {
                r#"
import vl_convert as vlc

__charton_temp_svg_fm_n9jh3 = vlc.vegalite_to_svg(chart.to_json())
print(__charton_temp_svg_fm_n9jh3)
"#
            }
            "json" => {
                r#"
print(chart.to_json())
"#
            }
            _ => {
                return Err(ChartonError::Unimplemented(format!(
                    "Output format '{}' is not supported",
                    output_format
                )));
            }
        };

        let full_plotting_code = format!("{}{}{}", ipc_to_df, self.raw_plotting_code, output);
        // Use regular expressions to replace the dataframe name with the actual dataframe name
        let re =
            Regex::new(r"__charton_temp_df_name_fm_n9jh3 = pl.read_ipc\(BytesIO\(ipc_data\)\)")
                .map_err(|_| ChartonError::Render("Failed to create regex".to_string()))?;
        let full_plotting_code = re.replace_all(
            &full_plotting_code,
            format!("{} = pl.read_ipc(BytesIO(ipc_data))", self.data.name),
        );

        Ok(full_plotting_code.to_string())
    }

    fn execute_plotting_code(&self, code: &str) -> Result<String, ChartonError> {
        let mut child = Command::new(&self.exe_path)
            .arg("-c")
            .arg(code)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| ChartonError::Io(e))?;

        if let Some(mut stdin) = child.stdin.take() {
            let json_data = serde_json::to_string(&self.data)
                .map_err(|_| ChartonError::Data("Failed to serialize data".to_string()))?;
            stdin
                .write_all(json_data.as_bytes())
                .map_err(|e| ChartonError::Io(e))?;
        }

        let output = child.wait_with_output().map_err(|e| ChartonError::Io(e))?;

        if !output.status.success() {
            return Err(ChartonError::Render(format!(
                "Python script execution failed with status: {:?}",
                output.status
            )));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

impl Visualization for Plot<Altair> {
    fn build(data: InputData) -> Result<Self, ChartonError> {
        // Convert Polars DataFrame to Base64 encoded Arrow IPC format string
        // Create an in-memory buffer
        let mut buf = Cursor::new(Vec::new());
        // Create IPC writer and write data
        IpcWriter::new(&mut buf).finish(&mut data.df.clone())?;
        // Reset cursor position for reading
        buf.seek(SeekFrom::Start(0))?;
        // Get raw byte data
        let ipc_data = buf.into_inner();
        // Encode binary data using base64
        let base64_ipc = base64::engine::general_purpose::STANDARD.encode(ipc_data);

        let data = SerializedData::new(&data.name, base64_ipc);

        Ok(Plot {
            data,
            exe_path: String::new(),
            raw_plotting_code: String::new(),
            _renderer: PhantomData,
        })
    }

    fn with_exe_path<P: AsRef<std::path::Path>>(
        mut self,
        exe_path: P,
    ) -> Result<Self, ChartonError> {
        let path = exe_path.as_ref();

        // Check if the path exists
        if !path.exists() {
            return Err(ChartonError::ExecutablePath(format!(
                "Python executable not found at path: {}",
                path.display()
            )));
        }

        // Check if the path is a file (not a directory)
        if !path.is_file() {
            return Err(ChartonError::ExecutablePath(format!(
                "Provided path is not a file: {}",
                path.display()
            )));
        }

        // On Unix systems, we can also check if the file is executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let metadata = path.metadata().map_err(|e| ChartonError::Io(e))?;

            if metadata.mode() & 0o111 == 0 {
                return Err(ChartonError::ExecutablePath(format!(
                    "Python executable is not executable: {}",
                    path.display()
                )));
            }
        }

        // Convert path to string for process execution
        let exe_path_str = path.to_str().ok_or_else(|| {
            ChartonError::ExecutablePath(
                "Python executable path contains invalid characters".to_string(),
            )
        })?;

        // Verify that this is actually a Python interpreter by checking its version
        let output = std::process::Command::new(exe_path_str)
            .arg("--version")
            .output()
            .map_err(|e| ChartonError::Io(e))?;

        if !output.status.success() {
            return Err(ChartonError::ExecutablePath(format!(
                "File at {} is not a valid Python interpreter",
                path.display()
            )));
        }

        let version_output = String::from_utf8_lossy(&output.stdout);
        let version_stderr = String::from_utf8_lossy(&output.stderr);

        // Python version output is typically in format "Python X.Y.Z"
        // It can be in either stdout or stderr depending on the Python version
        if !(version_output.starts_with("Python ") || version_stderr.starts_with("Python ")) {
            return Err(ChartonError::ExecutablePath(format!(
                "File at {} is not a Python interpreter",
                path.display()
            )));
        }

        self.exe_path = exe_path_str.to_string();
        Ok(self)
    }

    fn with_plotting_code(mut self, code: &str) -> Self {
        self.raw_plotting_code = code.to_string();

        self
    }

    fn show(&self) -> Result<(), ChartonError> {
        let vega_lite_json_string = self.to_json()?;

        // Check if we're in EVCXR Jupyter environment
        if std::env::var("EVCXR_IS_RUNTIME").is_ok() {
            // MIME type must be application/vnd.vegalite.v5+json
            println!("EVCXR_BEGIN_CONTENT application/vnd.vegalite.v5+json");
            println!("{}", &vega_lite_json_string);
            println!("EVCXR_END_CONTENT");
        }

        Ok(())
    }

    fn save<P: AsRef<std::path::Path>>(&self, path: P) -> Result<(), ChartonError> {
        let path_obj = path.as_ref();
        let ext = path_obj
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase());

        match ext.as_deref() {
            Some("svg") => {
                let svg_content = self.to_svg()?;
                std::fs::write(path_obj, svg_content).map_err(|e| ChartonError::Io(e))?;
            }
            Some("png") => {
                let svg_content = self.to_svg()?;

                // Load system fonts
                let mut opts = resvg::usvg::Options::default();
                let mut fontdb = (*opts.fontdb).clone();
                fontdb.load_system_fonts();
                opts.fontdb = std::sync::Arc::new(fontdb);

                // Parse svg string
                let tree = resvg::usvg::Tree::from_str(&svg_content, &opts)
                    .map_err(|e| ChartonError::Render(format!("SVG parsing error: {:?}", e)))?;

                // Scale the image size to higher resolution
                let pixmap_size = tree.size();
                let scale = 2.0;
                let width = (pixmap_size.width() * scale) as u32;
                let height = (pixmap_size.height() * scale) as u32;

                // Create pixmap
                let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)
                    .ok_or(ChartonError::Render("Failed to create pixmap".into()))?;

                // Render and save
                let transform = resvg::tiny_skia::Transform::from_scale(scale, scale);
                resvg::render(&tree, transform, &mut pixmap.as_mut());
                pixmap
                    .save_png(path_obj)
                    .map_err(|e| ChartonError::Render(format!("PNG saving error: {:?}", e)))?;
            }
            Some("json") => {
                let json_content = self.to_json()?;
                std::fs::write(path_obj, json_content).map_err(|e| ChartonError::Io(e))?;
            }
            Some(format) => {
                return Err(ChartonError::Unimplemented(format!(
                    "Output format '{}' is not supported",
                    format
                )));
            }
            None => {
                return Err(ChartonError::Unimplemented(
                    "Output format could not be determined from file extension".to_string(),
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data;
    #[test]
    #[ignore = "Requires Python environment with altair"]
    fn build_works() -> Result<(), ChartonError> {
        let df1 = df![
            "a" => [1, 2],
            "b" => [4, 5]
        ]?;
        let altair = Plot::<Altair>::build(data!(&df1)?)?;

        let expected = "QVJST1cxAAD/////qAAAAAQAAADy////\
            FAAAAAQAAQAAAAoACwAIAAoABAD4////DAAAAAgACAAAAAQAAgAAADQAAAAEAAA\
            AwP///yAAAAAQAAAACAAAAAECAAAAAAAAuP///yAAAAABAAAAAQAAAGIAAADs////\
            OAAAACAAAAAYAAAAAQIAABAAEgAEABAAEQAIAAAADAAAAAAA9P///yAAAAABAAAAC\
            AAJAAQACAABAAAAYQAAAP////+wAAAABAAAAOz///+AAAAAAAAAABQAAAAEAAMADAA\
            TABAAEgAMAAQA6v///wIAAAAAAAAAXAAAABAAAAAAAAoAFAAEAAwAEAAEAAAAAAAAA\
            AAAAAAAAAAAAAAAAAAAAAAAAAAACAAAAAAAAABAAAAAAAAAAAAAAAAAAAAAQAAAAAAA\
            AAAIAAAAAAAAAAAAAAACAAAAAgAAAAAAAAAAAAAAAAAAAAIAAAAAAAAAAAAAAAAAAAA\
            BAAAAAgAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\
            AAAAAAAAAAAAAAAAAABAAAAAUAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\
            AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAP////8AAAAABAAAAOz///9AAAAAOAAAABQ\
            AAAAEAAAADAASABAABAAIAAwAAQAAALgAAAAAAAAAuAAAAAAAAACAAAAAAAAAAAAAAAAA\
            AAAA+P///wwAAAAIAAgAAAAEAAIAAAA0AAAABAAAAMD///8gAAAAEAAAAAgAAAABAgAAAA\
            AAALj///8gAAAAAQAAAAEAAABiAAAA7P///zgAAAAgAAAAGAAAAAECAAAQABIABAAQABEA\
            CAAAAAwAAAAAAPT///8gAAAAAQAAAAgACQAEAAgAAQAAAGEA0gAAAEFSUk9XMQ==";
        assert_eq!(altair.data.value, expected);
        Ok(())
    }

    #[test]
    #[ignore = "Requires Python environment with altair"]
    fn with_exe_path_works() -> Result<(), ChartonError> {
        let df1 = df![
            "a" => [1, 2],
            "b" => [4, 5]
        ]?;
        let exe_path = r"D:\Programs\miniconda3\envs\cellpy\python.exe";
        let altair = Plot::<Altair>::build(data!(&df1)?)?.with_exe_path(exe_path)?;

        assert_eq!(&altair.exe_path, exe_path);
        Ok(())
    }

    #[test]
    #[ignore = "Requires Python environment with altair"]
    fn generate_full_plotting_code_works() -> Result<(), ChartonError> {
        let df1 = df![
            "a" => [1, 2],
            "b" => [4, 5]
        ]?;
        // Python code as string
        let raw_plotting_code = r#"
import altair as alt

chart = alt.Chart(df1).mark_point().encode(
    x='Price',
    y='Discount',
    color='Model',
).properties(width=200, height=200)
"#;

        let expected = r#"
import json
import sys
import base64
import polars as pl
from io import BytesIO

data = json.loads(sys.stdin.read())
ipc_data = base64.b64decode(data["value"])
df1 = pl.read_ipc(BytesIO(ipc_data))

import altair as alt

chart = alt.Chart(df1).mark_point().encode(
    x='Price',
    y='Discount',
    color='Model',
).properties(width=200, height=200)

import vl_convert as vlc

__charton_temp_svg_fm_n9jh3 = vlc.vegalite_to_svg(chart.to_json())
print(__charton_temp_svg_fm_n9jh3)
"#;

        let altair = Plot::<Altair>::build(data!(&df1)?)?.with_plotting_code(raw_plotting_code);
        let full_plotting_code = altair.generate_full_plotting_code("svg")?;
        assert_eq!(&full_plotting_code, expected);
        Ok(())
    }

    #[test]
    #[ignore = "Requires Python environment with altair"]
    fn show_works() -> Result<(), ChartonError> {
        let exe_path = r"D:\Programs\miniconda3\envs\cellpy\python.exe";
        let df1 = df![
            "Model" => ["S1", "M1", "R2", "P8", "M4", "T5", "V1"],
            "Price" => [2430, 3550, 5700, 8750, 2315, 3560, 980],
            "Discount" => [Some(0.65), Some(0.73), Some(0.82), None, Some(0.51), None, Some(0.26)],
        ]?;

        let raw_plotting_code = r#"
import altair as alt

chart = alt.Chart(df1).mark_point().encode(
    x='Price',
    y='Discount',
    color='Model',
).properties(width=200, height=200)
"#;

        let result = Plot::<Altair>::build(data!(&df1)?)?
            .with_exe_path(exe_path)?
            .with_plotting_code(raw_plotting_code)
            .show()?;

        assert_eq!(result, ());
        Ok(())
    }
}
