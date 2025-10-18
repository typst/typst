//! Diagram rendering support for Mermaid and PlantUML.

use std::process::Command;
use std::io::Write;
use tempfile::NamedTempFile;

use crate::diag::{At, SourceResult, StrResult, bail};
use crate::engine::Engine;
use crate::foundations::{
    Bytes, cast, elem, Content, NativeElement, Packed, Smart, StyleChain
};
use crate::layout::{Length, Rel, Sizing};
use crate::loading::{DataSource, Derived, Loaded, LoadSource};
use crate::visualize::{ImageElem, VectorFormat};
use typst_syntax::{Span, Spanned};

/// Renders a diagram from Mermaid or PlantUML syntax.
///
/// This element allows you to create diagrams using Mermaid or PlantUML syntax
/// directly in your Typst documents.
///
/// # Example
/// ```example
/// #diagram(
///   kind: "mermaid",
///   "graph TD; A-->B"
/// )
/// ```
///
/// # PlantUML Example
/// ```example
/// #diagram(
///   kind: "plantuml",
///   "@startuml\nAlice -> Bob\n@enduml"
/// )
/// ```
#[elem(title = "Diagram")]
pub struct DiagramElem {
    /// The diagram syntax to use.
    ///
    /// Can be either `"mermaid"` or `"plantuml"`.
    #[required]
    pub kind: DiagramKind,

    /// The diagram source code.
    #[required]
    pub source: String,

    /// The width of the diagram.
    pub width: Smart<Rel<Length>>,

    /// The height of the diagram.
    #[default(Sizing::Auto)]
    pub height: Sizing,

    /// Alternative text for the diagram.
    pub alt: Option<String>,
}

impl NativeElement for DiagramElem {
    fn construct(
        _: &mut Engine,
        args: &mut crate::foundations::Args,
    ) -> SourceResult<Content> {
        let kind = args.expect("kind")?;
        let source = args.expect("source")?;
        let width = args.named("width")?;
        let height = args.named("height")?;
        let alt = args.named("alt")?;
        
        let mut elem = Self::new(kind, source);
        if let Some(width) = width {
            elem.push_width(width);
        }
        if let Some(height) = height {
            elem.push_height(height);
        }
        if let Some(alt) = alt {
            elem.push_alt(alt);
        }
        
        Ok(elem.pack())
    }
}

/// The kind of diagram to render.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum DiagramKind {
    /// Mermaid diagram.
    Mermaid,
    /// PlantUML diagram.
    PlantUML,
}

cast! {
    DiagramKind,
    self => match self {
        Self::Mermaid => "mermaid",
        Self::PlantUML => "plantuml",
    }.into_value(),
    v: String => match v.as_str() {
        "mermaid" => Self::Mermaid,
        "plantuml" => Self::PlantUML,
        _ => bail!("expected 'mermaid' or 'plantuml', found '{}'", v),
    },
}

impl Packed<DiagramElem> {
    /// Render the diagram to an SVG image element.
    pub fn render(
        &self,
        engine: &mut Engine,
        styles: StyleChain,
    ) -> SourceResult<Content> {
        let span = self.span();
        let kind = self.kind(styles);
        let source = self.source(styles);
        
        let svg_data = match kind {
            DiagramKind::Mermaid => render_mermaid(&source).at(span)?,
            DiagramKind::PlantUML => render_plantuml(&source).at(span)?,
        };
        
        // Create a Loaded wrapper for the diagram data
        let loaded = Loaded::new(
            Spanned::new(LoadSource::Bytes, span),
            svg_data.clone(),
        );
        
        // Create an image element from the SVG data
        let source_data = Derived::new(DataSource::Bytes(svg_data), loaded);
        let mut image = ImageElem::new(source_data);
        
        // Copy properties from diagram to image
        image.push_format(Smart::Custom(VectorFormat::Svg.into()));
        image.push_width(self.width(styles));
        image.push_height(self.height(styles));
        
        if let Some(alt) = self.alt(styles) {
            image.push_alt(Some(alt.clone().into()));
        }
        
        Ok(image.pack().spanned(span))
    }
}

/// Render a Mermaid diagram to SVG.
fn render_mermaid(source: &str) -> StrResult<Bytes> {
    // Check if mmdc is available first
    if Command::new("mmdc").arg("--version").output().is_err() {
        bail!(
            "mermaid-cli (mmdc) not found. Please install it with: \
            npm install -g @mermaid-js/mermaid-cli"
        );
    }

    // Create a temporary file for the Mermaid source
    let mut input_file = NamedTempFile::new()
        .map_err(|e| format!("failed to create temporary file: {}", e))?;
    
    input_file.write_all(source.as_bytes())
        .map_err(|e| format!("failed to write diagram source: {}", e))?;
    
    let input_path = input_file.path();
    let output_path = input_path.with_extension("svg");

    // Try to use mmdc (mermaid-cli)
    let output = Command::new("mmdc")
        .arg("-i")
        .arg(input_path)
        .arg("-o")
        .arg(&output_path)
        .arg("--outputFormat")
        .arg("svg")
        .output();

    match output {
        Ok(result) if result.status.success() => {
            let svg_data = std::fs::read(&output_path)
                .map_err(|e| format!("failed to read rendered diagram: {}", e))?;
            
            // Clean up
            let _ = std::fs::remove_file(&output_path);
            
            Ok(Bytes::from(svg_data))
        }
        Ok(result) => {
            let stderr = String::from_utf8_lossy(&result.stderr);
            bail!("mermaid rendering failed: {}", stderr)
        }
        Err(_) => {
            bail!(
                "mermaid-cli (mmdc) not found. Please install it with: \
                npm install -g @mermaid-js/mermaid-cli"
            )
        }
    }
}

/// Render a PlantUML diagram to SVG.
fn render_plantuml(source: &str) -> StrResult<Bytes> {
    // Check if plantuml is available first
    if Command::new("plantuml").arg("-version").output().is_err() {
        bail!(
            "plantuml not found. Please install it from: \
            https://plantuml.com/download"
        );
    }

    // Create a temporary file for the PlantUML source
    let mut input_file = NamedTempFile::new()
        .map_err(|e| format!("failed to create temporary file: {}", e))?;
    
    input_file.write_all(source.as_bytes())
        .map_err(|e| format!("failed to write diagram source: {}", e))?;
    
    let input_path = input_file.path();

    // Try to use plantuml
    let output = Command::new("plantuml")
        .arg("-tsvg")
        .arg(input_path)
        .output();

    match output {
        Ok(result) if result.status.success() => {
            let output_path = input_path.with_extension("svg");
            let svg_data = std::fs::read(&output_path)
                .map_err(|e| format!("failed to read rendered diagram: {}", e))?;
            
            // Clean up
            let _ = std::fs::remove_file(&output_path);
            
            Ok(Bytes::from(svg_data))
        }
        Ok(result) => {
            let stderr = String::from_utf8_lossy(&result.stderr);
            bail!("plantuml rendering failed: {}", stderr)
        }
        Err(_) => {
            bail!(
                "plantuml not found. Please install it from: \
                https://plantuml.com/download"
            )
        }
    }
}
