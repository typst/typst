//! Diagram rendering support for Mermaid and PlantUML.

// Removed external command dependencies for CI safety

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
/// directly in your Typst documents. Currently returns placeholder SVGs.
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
/// Note: This is a placeholder implementation that returns a simple SVG.
/// In a real implementation, you would need to integrate with a Mermaid
/// rendering library or use external tools.
fn render_mermaid(source: &str) -> StrResult<Bytes> {
    // For now, return a simple placeholder SVG
    let svg = format!(
        r#"<svg width="400" height="200" xmlns="http://www.w3.org/2000/svg">
  <rect width="400" height="200" fill="#f0f0f0" stroke="#ccc" stroke-width="2"/>
  <text x="200" y="100" text-anchor="middle" font-family="Arial, sans-serif" font-size="16" fill="#666">
    Mermaid Diagram
  </text>
  <text x="200" y="120" text-anchor="middle" font-family="Arial, sans-serif" font-size="12" fill="#999">
    Source: {}
  </text>
</svg>"#,
        source.chars().take(50).collect::<String>()
    );
    
    Ok(Bytes::from(svg.as_bytes()))
}

/// Render a PlantUML diagram to SVG.
/// Note: This is a placeholder implementation that returns a simple SVG.
/// In a real implementation, you would need to integrate with a PlantUML
/// rendering library or use external tools.
fn render_plantuml(source: &str) -> StrResult<Bytes> {
    // For now, return a simple placeholder SVG
    let svg = format!(
        r#"<svg width="400" height="200" xmlns="http://www.w3.org/2000/svg">
  <rect width="400" height="200" fill="#f0f0f0" stroke="#ccc" stroke-width="2"/>
  <text x="200" y="100" text-anchor="middle" font-family="Arial, sans-serif" font-size="16" fill="#666">
    PlantUML Diagram
  </text>
  <text x="200" y="120" text-anchor="middle" font-family="Arial, sans-serif" font-size="12" fill="#999">
    Source: {}
  </text>
</svg>"#,
        source.chars().take(50).collect::<String>()
    );
    
    Ok(Bytes::from(svg.as_bytes()))
}
