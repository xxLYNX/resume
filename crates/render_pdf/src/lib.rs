use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result};

pub enum PdfEngineKind {
    Tectonic,
    Pdflatex,
}

pub struct PdfEngine {
    kind: PdfEngineKind,
}

impl PdfEngine {
    pub fn detect() -> Option<Self> {
        if which("tectonic") {
            return Some(Self {
                kind: PdfEngineKind::Tectonic,
            });
        }
        if which("pdflatex") {
            return Some(Self {
                kind: PdfEngineKind::Pdflatex,
            });
        }
        None
    }

    pub fn compile(&self, tex_filename: &str, output_dir: &Path) -> Result<()> {
        match self.kind {
            PdfEngineKind::Tectonic => {
                let status = Command::new("tectonic")
                    .arg("--synctex=0")
                    .arg(tex_filename)
                    .current_dir(output_dir)
                    .stdout(Stdio::null())
                    .stderr(Stdio::piped())
                    .status()
                    .context("spawn tectonic")?;
                if !status.success() {
                    anyhow::bail!("tectonic failed with status {status}");
                }
            }
            PdfEngineKind::Pdflatex => {
                for _ in 0..2 {
                    let status = Command::new("pdflatex")
                        .args(["-interaction=nonstopmode", tex_filename])
                        .current_dir(output_dir)
                        .stdout(Stdio::null())
                        .stderr(Stdio::piped())
                        .status()
                        .context("spawn pdflatex")?;
                    if !status.success() {
                        anyhow::bail!("pdflatex failed with status {status}");
                    }
                }
            }
        }
        Ok(())
    }

    pub fn name(&self) -> &'static str {
        match self.kind {
            PdfEngineKind::Tectonic => "tectonic",
            PdfEngineKind::Pdflatex => "pdflatex",
        }
    }
}

fn which(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn write_tex_and_compile(tex: &str, output_dir: &Path, filename: &str) -> Result<PathBuf> {
    std::fs::create_dir_all(output_dir)?;
    let tex_path = output_dir.join(filename);
    std::fs::write(&tex_path, tex).with_context(|| format!("write {}", tex_path.display()))?;

    let engine = PdfEngine::detect().context(
        "no PDF engine found (install tectonic or pdflatex)",
    )?;
    engine.compile(filename, output_dir)?;

    let pdf_name = filename.replace(".tex", ".pdf");
    Ok(output_dir.join(pdf_name))
}
