#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
//! A Rust library to generate publication-quality figures.
//!
//! This crate is a PGFPlots code generator, and provides utilities to create,
//! customize, and compile high-quality plots. The `inclusive` feature allows
//! users to fully process figures without relying on any externally installed
//! software.
//!
//! The library's API is designed to feel natural for LaTeX and PGFPlots users,
//! but no previous experience is required to start generating
//! publication-quality plots in Rust.
//!
//! # Quick Start
//!
//! To get you started quickly, the easiest way to generate a plot is to use a
//! [`Plot2D`]. Plotting a quadratic function is as simple as:
//!
//! ```no_run
//! use pgfplots::axis::plot::Plot2D;
//!
//! let mut plot = Plot2D::new();
//! plot.coordinates = (-100..100)
//!     .into_iter()
//!     .map(|i| (f64::from(i), f64::from(i*i)).into())
//!     .collect();
//!
//! # #[cfg(feature = "inclusive")]
//! plot.show();
//! ```
//!
//! It is possible to show multiple plots in the same axis environment by
//! creating an [`Axis`] and adding plots to it. An [`Axis`] and its individual
//! [`Plot2D`]s are customized by [`AxisKey`]s and [`PlotKey`]s respectively.

// Only imported for documentation. If you notice that this is no longer the
// case, please change it.
#[allow(unused_imports)]
use crate::axis::{
    plot::{Plot2D, PlotKey},
    AxisKey,
};

use crate::axis::Axis;
use std::fmt;
use std::io::Write;
use thiserror::Error;

const OUT_NAME: &str = "figure";

/// Axis environment inside a [`Picture`].
pub mod axis;

/// The error type returned when showing a figure fails.
#[derive(Debug, Error)]
pub enum ShowPdfError {
    /// Compilation of LaTeX source failed using Tectonic.
    #[cfg(feature = "inclusive")]
    #[error("failed to compile LaTeX source: {0}")]
    Tectonic(#[from] tectonic::Error),
    /// Encountered some kind of Io error.
    #[error("io task failed: {0}")]
    IoError(#[from] std::io::Error),
    /// Failed to open file.
    #[error("failed to open file: {0}")]
    Open(#[from] opener::OpenError),
}

pub enum Compiler {
    #[cfg(feature = "inclusive")]
    Tectonic,
    Installed(Engine),
}

#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum Engine {
    PdfLatex,
}

impl fmt::Display for Engine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PdfLatex => write!(f, "pdflatex"),
        }
    }
}

/// Ti*k*Z options passed to the [`Picture`] environment.
///
/// The most commonly used key-value pairs are variants of the [`PictureKey`]
/// enum. The [`PictureKey::Custom`] variant is provided to add unimplemented
/// keys and will be written verbatim in the options of the [`Picture`]
/// environment.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum PictureKey {
    /// Custom key-value pairs that have not been implemented. These will be
    /// appended verbatim to the options of the [`Picture`].
    Custom(String),
}

impl fmt::Display for PictureKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PictureKey::Custom(key) => write!(f, "{key}"),
        }
    }
}

/// Picture environment.
///
/// Creating a [`Picture`] is equivalent to the Ti*k*Z graphics environment:
///
/// ```text
/// \begin{tikzpicture}[PictureKeys]
///     % axis environments
/// \end{tikzpicture}
/// ```
///
/// You will rarely interact with a [`Picture`]. It is only useful to generate
/// complex layouts with multiple axis environments.
#[derive(Clone, Debug, Default)]
pub struct Picture {
    keys: Vec<PictureKey>,
    pub axes: Vec<Axis>,
}

impl fmt::Display for Picture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\\begin{{tikzpicture}}")?;
        // If there are keys, print one per line. It makes it easier for a
        // human later to find keys if they are divided by lines.
        if !self.keys.is_empty() {
            writeln!(f, "[")?;
            for key in self.keys.iter() {
                writeln!(f, "\t{key},")?;
            }
            write!(f, "]")?;
        }
        writeln!(f)?;

        for axis in self.axes.iter() {
            writeln!(f, "{axis}")?;
        }

        write!(f, "\\end{{tikzpicture}}")?;

        Ok(())
    }
}

impl Picture {
    /// Create a new, empty picture environment.
    ///
    /// # Examples
    ///
    /// ```
    /// use pgfplots::Picture;
    ///
    /// let mut picture = Picture::new();
    /// ```
    pub fn new() -> Self {
        Default::default()
    }
    /// Add a key to control the appearance of the picture. This will overwrite
    /// any previous mutually exclusive key.
    ///
    /// # Examples
    ///
    /// ```
    /// use pgfplots::{Picture, PictureKey};
    ///
    /// let mut picture = Picture::new();
    /// picture.add_key(PictureKey::Custom(String::from("baseline")));
    /// ```
    pub fn add_key(&mut self, key: PictureKey) {
        match key {
            PictureKey::Custom(_) => (),
            // If/whenever another variant is added, handle it the same way as
            // Axis::add_key and Plot2D::add_key
        }
        self.keys.push(key);
    }
    /// Return a [`String`] with valid LaTeX code that generates a standalone
    /// PDF with the picture environment.
    ///
    /// # Note
    ///
    /// Passing this string directly to e.g. `pdflatex` will fail to generate a
    /// PDF document. It is usually necessary to [`str::replace`] all the
    /// occurrences of `\n` and `\t` with white space before sending this string
    /// as an argument to a LaTeX compiler.
    ///
    /// # Examples
    ///
    /// ```
    /// use pgfplots::Picture;
    ///
    /// let mut picture = Picture::new();
    /// assert_eq!(
    /// r#"\documentclass{standalone}
    /// \usepackage{pgfplots}
    /// \begin{document}
    /// \begin{tikzpicture}
    /// \end{tikzpicture}
    /// \end{document}"#,
    /// picture.standalone_string());
    /// ```
    pub fn standalone_string(&self) -> String {
        format!("\\documentclass{{standalone}}\n\\usepackage{{pgfplots}}\n\\begin{{document}}\n{}\n\\end{{document}}", self)
    }
    /// Show the picture as a standalone PDF. This will create a file in the
    /// location returned by [`std::env::temp_dir()`] and open it with the
    /// default PDF viewer in your system.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use pgfplots::Picture;
    ///
    /// let mut picture = Picture::new();
    /// picture.show();
    /// ```
    #[cfg(feature = "inclusive")]
    pub fn show(&self) -> Result<(), ShowPdfError> {
        let pdf_data = tectonic::latex_to_pdf(self.standalone_string())?;
        let mut path = temp_output_dir()?;
        path.push(OUT_NAME);
        path.set_extension("pdf");

        let mut file = std::fs::File::create(&path)?;
        file.write_all(&pdf_data)?;

        opener::open(&path)?;

        Ok(())
    }

    pub fn show_with(&self, builder: &Compiler) -> Result<(), ShowPdfError> {
        match builder {
            #[cfg(feature = "inclusive")]
            Compiler::Tectonic => self.show(),
            Compiler::Installed(engine) => {
                // generate output dir in /tmp (on linux)
                let out_dir = temp_output_dir()?;
                // generate the .tex source file
                let mut source_file = out_dir.clone();
                source_file.push(OUT_NAME);
                source_file.set_extension("tex");
                // write the code to the source file (otherwise args can get too large)
                let mut file = std::fs::File::create(&source_file)?;
                file.write_all(self.standalone_string().as_bytes())?;
                // compile the figure with the pre-installed latex compiler
                compile_figure_with(
                    &engine.to_string(),
                    source_file.file_name().unwrap(),
                    &out_dir,
                )?;
                // open the resulting .pdf
                let mut out_file = out_dir;
                out_file.push(OUT_NAME);
                out_file.set_extension("pdf");
                opener::open(out_file)?;
                Ok(())
            }
        }
    }
}

fn temp_output_dir() -> std::io::Result<std::path::PathBuf> {
    let mut path = std::env::temp_dir();
    path.push("output");
    if path.exists() {
        std::fs::remove_dir_all(&path)?;
    }
    std::fs::create_dir(&path)?;
    Ok(path)
}

fn compile_figure_with(
    engine: &str,
    source: &std::ffi::OsStr,
    out_dir: &std::path::Path,
) -> Result<(), ShowPdfError> {
    std::process::Command::new(engine)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .arg("-interaction=batchmode")
        .arg("-halt-on-error")
        .arg("-jobname=figure")
        .arg(source)
        .current_dir(out_dir)
        .status()?;
    Ok(())
}

#[cfg(test)]
mod tests;
