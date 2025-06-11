#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum SyntaxMode {
    /// Text and markup, as in the top level.
    Markup,
    /// Math atoms, operators, etc., as in equations.
    Math,
    /// Keywords, literals and operators, as after hashes.
    Code,
}
