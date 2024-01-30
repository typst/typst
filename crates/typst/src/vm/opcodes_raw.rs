// THIS IS PURPOSELY NOT ADDED IN A MODULE, BECAUSE IT IS USED SOMEWHERE ELSE.
// THE GOAL IS TO KEEP ALL OPCODES IN ONE PLACE, SO THAT THEY CAN BE EASILY
// REFERENCED.

opcodes! {
    // -----------------------------------------------------------------------------
    // --------------------------------- OPERATORS ---------------------------------
    // -----------------------------------------------------------------------------

    /// No operation.
    Nop: noop = 0x00,

    /// Adds two values together.
    Add: add -> Writable => {
        /// The left-hand side of the addition.
        lhs: Readable,
        /// The right-hand side of the addition.
        rhs: Readable,
    } = 0x01,

    /// Subtracts two values.
    Sub: sub -> Writable => {
        /// The left-hand side of the subtraction.
        lhs: Readable,
        /// The right-hand side of the subtraction.
        rhs: Readable,
    } = 0x02,

    /// Multiplies two values.
    Mul: mul -> Writable => {
        /// The left-hand side of the multiplication.
        lhs: Readable,
        /// The right-hand side of the multiplication.
        rhs: Readable,
    } = 0x03,

    /// Divides two values.
    Div: div -> Writable => {
        /// The left-hand side of the division.
        lhs: Readable,
        /// The right-hand side of the division.
        rhs: Readable,
    } = 0x04,

    /// Negates a value.
    Neg: neg -> Writable => {
        /// The value to negate.
        value: Readable,
    } = 0x05,

    /// Positivizes a value.
    Pos: pos -> Writable => {
        /// The value to negate.
        value: Readable,
    } = 0x06,

    /// Logical not.
    Not: not -> Writable => {
        /// The value to negate.
        value: Readable,
    } = 0x07,

    /// Greater than.
    Gt: gt -> Writable => {
        /// The left-hand side of the comparison.
        lhs: Readable,
        /// The right-hand side of the comparison.
        rhs: Readable,
    } = 0x08,

    /// Greater than or equal to.
    Geq: geq -> Writable => {
        /// The left-hand side of the comparison.
        lhs: Readable,
        /// The right-hand side of the comparison.
        rhs: Readable,
    } = 0x09,

    /// Less than.
    Lt: lt -> Writable => {
        /// The left-hand side of the comparison.
        lhs: Readable,
        /// The right-hand side of the comparison.
        rhs: Readable,
    } = 0x0A,

    /// Less than or equal to.
    Leq: leq -> Writable => {
        /// The left-hand side of the comparison.
        lhs: Readable,
        /// The right-hand side of the comparison.
        rhs: Readable,
    } = 0x0B,

    /// Equal to.
    Eq: eq -> Writable => {
        /// The left-hand side of the comparison.
        lhs: Readable,
        /// The right-hand side of the comparison.
        rhs: Readable,
    } = 0x0C,

    /// Not equal to.
    Neq: neq -> Writable => {
        /// The left-hand side of the comparison.
        lhs: Readable,
        /// The right-hand side of the comparison.
        rhs: Readable,
    } = 0x0D,

    /// Whether the left-hand side is in the right-hand side.
    In: in_ -> Writable => {
        /// The left-hand side of the comparison.
        lhs: Readable,
        /// The right-hand side of the comparison.
        rhs: Readable,
    } = 0x0E,

    /// Whether the left-hand side is not in the right-hand side.
    NotIn: not_in -> Writable => {
        /// The left-hand side of the comparison.
        lhs: Readable,
        /// The right-hand side of the comparison.
        rhs: Readable,
    } = 0x0F,

    /// Logical and.
    And: and -> Writable => {
        /// The left-hand side of the comparison.
        lhs: Readable,
        /// The right-hand side of the comparison.
        rhs: Readable,
    } = 0x10,

    /// Logical or.
    Or: or -> Writable => {
        /// The left-hand side of the comparison.
        lhs: Readable,
        /// The right-hand side of the comparison.
        rhs: Readable,
    } = 0x11,

    /// Copies a value.
    Copy: copy -> Writable => {
        /// The value to copy.
        value: Readable,
    } = 0x12,

    /// Creates a new [`Value::None`].
    None: none -> Writable => { } = 0x13,

    /// Creates a new [`Value::Auto`].
    Auto: auto -> Writable => { } = 0x14,

    // -----------------------------------------------------------------------------
    // ---------------------------------- ASSIGN -----------------------------------
    // -----------------------------------------------------------------------------

    /// Assign to a value.
    Assign: assign -> AccessId => {
        /// The value to assign.
        value: Readable,
    } = 0x20,

    /// Assign and add to a value.
    AddAssign: add_assign -> AccessId => {
        /// The value to assign.
        value: Readable,
    } = 0x21,

    /// Assign and subtract from a value.
    SubAssign: sub_assign -> AccessId => {
        /// The value to assign.
        value: Readable,
    } = 0x22,

    /// Assign and multiply a value.
    MulAssign: mul_assign -> AccessId => {
        /// The value to assign.
        value: Readable,
    } = 0x23,

    /// Assign and divide a value.
    DivAssign: div_assign -> AccessId => {
        /// The value to assign.
        value: Readable,
    } = 0x24,

    /// Destructures a value into a pattern.
    Destructure: destructure -> PatternId => {
        /// The value to destructure.
        value: Readable,
    } = 0x25,

    // -----------------------------------------------------------------------------
    // ---------------------------------- STYLING ----------------------------------
    // -----------------------------------------------------------------------------

    /// Creates a new set rule.
    Set: set -> Writable => {
        /// The target to set the rule on.
        target: Readable,
        /// The arguments to supply to the set rule.
        args: Readable,
    } = 0xA0,

    /// Creates a new show rule.
    Show: show -> Writable => {
        /// The selector for the value to show.
        selector: OptionalReadable,
        /// The transform to apply.
        transform: Readable,
    } = 0xA1,

    /// Style the remaining joined items with the given style.
    Styled: styled => {
        /// The style to apply.
        style: Readable,
    } = 0xA2,

    // -----------------------------------------------------------------------------
    // ----------------------------- FUNCTIONS & FLOW ------------------------------
    // -----------------------------------------------------------------------------

    /// Instantiates a closure.
    ///
    /// This involves:
    /// - Capturing all necessary values.
    /// - Capturing the default values of named arguments.
    ///
    /// And produces a [`Func`] which can then be called.
    Instantiate: instantiate -> Writable => {
        /// The closure to instantiate.
        closure: ClosureId,
    } = 0xB0,

    /// Calls a function.
    Call: call -> Writable => {
        /// The closure to call.
        closure: AccessId,
        /// The arguments to call the closure with.
        args: Readable,
        /// The flags:
        /// - Bit 0: Whether the call is done in a math context.
        /// - Bit 1: Whether the call contains a trailing comma.
        flags: u8,
    } = 0xB1,

    /// Accesses a field.
    Field: field -> Writable => {
        /// The value to access.
        access: AccessId,
    } = 0xB2,

    /// Enter a new iterator scope with optional joining.
    Iter: iter -> OptionalWritable => {
        /// The length of the scope to enter.
        len: u32,
        /// The value to iterate over.
        iterable: Readable,
        /// Whether the scope is a loop.
        ///
        /// - Bit 1: Whether joining is enabled.
        /// - Bit 2: Whether joining results in a content.
        flags: u8,
    } = 0xB3,

    /// Queries the next value of an iterator.
    /// Returns from the iterator scope if the iterator is exhausted.
    Next: next -> Writable => { } = 0xB4,

    /// Continues a loop.
    Continue: continue_ => {} = 0xB5,

    /// Breaks out of a loop.
    Break: break_ => {} = 0xB6,

    /// Returns a value from a function.
    Return: return_ => {
        /// The value to return.
        value: OptionalReadable,
    } = 0xB7,

    // -----------------------------------------------------------------------------
    // ---------------------------------- VALUES------------------------------------
    // -----------------------------------------------------------------------------

    /// Allocates a new array.
    Array: array -> Writable => {
        /// The capacity of the array.
        capacity: u32,
    } = 0xC0,

    /// Push a value to an array.
    Push: push -> Writable => {
        /// The value to push.
        value: Readable,
    } = 0xC1,

    /// Allocates a new dictionary.
    Dict: dict -> Writable => {
        /// The capacity of the dictionary.
        capacity: u32,
    } = 0xC2,

    /// Insert a value into a dictionary.
    Insert: insert -> Writable => {
        /// The key to insert.
        key: Readable,
        /// The value to insert.
        value: Readable,
    } = 0xC3,

    /// Allocates a new argument set.
    Args: args -> Writable => {
        /// The capacity of the argument set.
        capacity: u32,
    } = 0xC4,

    /// Pushes a value into an argument set.
    PushArg: push_arg -> Writable => {
        /// The value to insert.
        value: Readable,
    } = 0xC5,

    /// Inserts a named value into an argument set.
    InsertArg: insert_arg -> Writable => {
        /// The key to insert.
        key: Readable,
        /// The value to insert.
        value: Readable,
    } = 0xC6,

    /// Spreads this value into either:
    /// - An array.
    /// - A dictionary.
    /// - An argument set.
    Spread: spread -> Writable => {
        /// The value to spread.
        value: Readable,
    } = 0xC7,

    // -----------------------------------------------------------------------------
    // ----------------------------- CONDITIONAL JUMPS -----------------------------
    // -----------------------------------------------------------------------------

    /// Enter a new scope with optional joining.
    Enter: enter -> OptionalWritable => {
        /// The length of the scope to enter.
        len: u32,
        /// The scope to load.
        scope: ScopeId,
        /// Whether the scope is a loop.
        ///
        /// - Bit 0: Whether this is a loop.
        /// - Bit 1: Whether joining.
        /// - Bit 2: Whether joining results in a content.
        flags: u8,
    } = 0xD0,

    /// Jump to a new instruction.
    Jump: jump => {
        /// The instruction to jump to.
        instruction: Pointer,
    } = 0xD1,

    /// Jump to a new instruction if the condition is true.
    JumpIf: jump_if => {
        /// The condition to check.
        condition: Readable,
        /// The instruction to jump to.
        instruction: Pointer,
    } = 0xD2,

    JumpIfNot: jump_if_not => {
        /// The condition to check.
        condition: Readable,
        /// The instruction to jump to.
        instruction: Pointer,
    } = 0xD3,

    /// Select one of two values based on a condition.
    Select: select -> Writable => {
        /// The condition to check.
        condition: Readable,
        /// The value to select if the condition is true.
        true_: Readable,
        /// The value to select if the condition is
        false_: Readable,
    } = 0xD4,

    // -----------------------------------------------------------------------------
    // ----------------------------------- MATH ------------------------------------
    // -----------------------------------------------------------------------------

    /// Creates a new [`LrElem`].
    Delimited: delimited -> Writable => {
        /// The left delimiter.
        left: Readable,
        /// The body.
        body: Readable,
        /// The right delimiter.
        right: Readable,
    } = 0xE0,

    /// Builds an [`AttachElem`].
    Attach: attach -> Writable => {
        /// The base value.
        base: Readable,
        /// The top supplement.
        top: OptionalReadable,
        /// The bottom supplement.
        bottom: OptionalReadable,
    } = 0xE1,

    /// Builds a fraction.
    Frac: frac -> Writable => {
        /// The numerator.
        numerator: Readable,
        /// The denominator.
        denominator: Readable,
    } = 0xE2,

    /// Builds a root.
    Root: root -> Writable => {
        /// The degree.
        degree: OptionalReadable,
        /// The radicand.
        radicand: Readable,
    } = 0xE3,

    // -----------------------------------------------------------------------------
    // ---------------------------------- CONTENT ----------------------------------
    // -----------------------------------------------------------------------------

    /// Creates a new [`RefElem`].
    Ref: ref_ -> Writable => {
        /// The label of the reference.
        label: LabelId,

        /// The supplement (if any).
        supplement: OptionalReadable,
    } = 0xF0,

    /// Makes a value strong.
    Strong: strong -> Writable => {
        /// The value to make strong.
        value: Readable,
    } = 0xF1,

    /// Makes a value emphasized.
    Emph: emph -> Writable => {
        /// The value to emphasize.
        value: Readable,
    } = 0xF2,

    /// Makes a value into a heading.
    Heading: heading -> Writable => {
        /// The value to make into a heading.
        value: Readable,
        /// The level of the heading.
        level: u32,
    } = 0xF3,

    /// Makes a list item.
    ListItem: list_item -> Writable => {
        /// The value to make into a list item.
        value: Readable,
    } = 0xF4,

    /// Makes an enum item.
    EnumItem: enum_item -> Writable => {
        /// The value to make into an enum item.
        value: Readable,
        /// The optional number of the enum item.
        number: Option<NonZeroU32>,
    } = 0xF5,

    /// Markes a term.
    TermItem: term_item -> Writable => {
        /// The term to make into a term.
        term: Readable,
        /// The description of the term.
        description: Readable,
    } = 0xF6,

    /// Makes an equation.
    Equation: equation -> Writable => {
        /// The value to make into an equation.
        value: Readable,
    } = 0xF7,
}
