use crate::protocol::ast::*;
use crate::protocol::inputsource::*;
use crate::protocol::lexer::*;
use crate::protocol::library;

// The following indirection is needed due to a bug in the cbindgen tool.
type Unit = ();
type VisitorResult = Result<Unit, ParseError>;

trait Visitor: Sized {
    fn visit_protocol_description(&mut self, h: &mut Heap, pd: RootId) -> VisitorResult {
        recursive_protocol_description(self, h, pd)
    }
    fn visit_pragma(&mut self, _h: &mut Heap, _pragma: PragmaId) -> VisitorResult {
        Ok(())
    }
    fn visit_import(&mut self, _h: &mut Heap, _import: ImportId) -> VisitorResult {
        Ok(())
    }

    fn visit_symbol_definition(&mut self, h: &mut Heap, def: DefinitionId) -> VisitorResult {
        recursive_symbol_definition(self, h, def)
    }
    fn visit_component_definition(&mut self, h: &mut Heap, def: ComponentId) -> VisitorResult {
        recursive_component_definition(self, h, def)
    }
    fn visit_composite_definition(&mut self, h: &mut Heap, def: CompositeId) -> VisitorResult {
        recursive_composite_definition(self, h, def)
    }
    fn visit_primitive_definition(&mut self, h: &mut Heap, def: PrimitiveId) -> VisitorResult {
        recursive_primitive_definition(self, h, def)
    }
    fn visit_function_definition(&mut self, h: &mut Heap, def: FunctionId) -> VisitorResult {
        recursive_function_definition(self, h, def)
    }

    fn visit_variable_declaration(&mut self, h: &mut Heap, decl: VariableId) -> VisitorResult {
        recursive_variable_declaration(self, h, decl)
    }
    fn visit_parameter_declaration(&mut self, _h: &mut Heap, _decl: ParameterId) -> VisitorResult {
        Ok(())
    }
    fn visit_local_declaration(&mut self, _h: &mut Heap, _decl: LocalId) -> VisitorResult {
        Ok(())
    }

    fn visit_statement(&mut self, h: &mut Heap, stmt: StatementId) -> VisitorResult {
        recursive_statement(self, h, stmt)
    }
    fn visit_local_statement(&mut self, h: &mut Heap, stmt: LocalStatementId) -> VisitorResult {
        recursive_local_statement(self, h, stmt)
    }
    fn visit_memory_statement(&mut self, h: &mut Heap, stmt: MemoryStatementId) -> VisitorResult {
        recursive_memory_statement(self, h, stmt)
    }
    fn visit_channel_statement(
        &mut self,
        _h: &mut Heap,
        _stmt: ChannelStatementId,
    ) -> VisitorResult {
        Ok(())
    }
    fn visit_block_statement(&mut self, h: &mut Heap, stmt: BlockStatementId) -> VisitorResult {
        recursive_block_statement(self, h, stmt)
    }
    fn visit_labeled_statement(&mut self, h: &mut Heap, stmt: LabeledStatementId) -> VisitorResult {
        recursive_labeled_statement(self, h, stmt)
    }
    fn visit_skip_statement(&mut self, _h: &mut Heap, _stmt: SkipStatementId) -> VisitorResult {
        Ok(())
    }
    fn visit_if_statement(&mut self, h: &mut Heap, stmt: IfStatementId) -> VisitorResult {
        recursive_if_statement(self, h, stmt)
    }
    fn visit_while_statement(&mut self, h: &mut Heap, stmt: WhileStatementId) -> VisitorResult {
        recursive_while_statement(self, h, stmt)
    }
    fn visit_break_statement(&mut self, _h: &mut Heap, _stmt: BreakStatementId) -> VisitorResult {
        Ok(())
    }
    fn visit_continue_statement(
        &mut self,
        _h: &mut Heap,
        _stmt: ContinueStatementId,
    ) -> VisitorResult {
        Ok(())
    }
    fn visit_synchronous_statement(
        &mut self,
        h: &mut Heap,
        stmt: SynchronousStatementId,
    ) -> VisitorResult {
        recursive_synchronous_statement(self, h, stmt)
    }
    fn visit_return_statement(&mut self, h: &mut Heap, stmt: ReturnStatementId) -> VisitorResult {
        recursive_return_statement(self, h, stmt)
    }
    fn visit_assert_statement(&mut self, h: &mut Heap, stmt: AssertStatementId) -> VisitorResult {
        recursive_assert_statement(self, h, stmt)
    }
    fn visit_goto_statement(&mut self, _h: &mut Heap, _stmt: GotoStatementId) -> VisitorResult {
        Ok(())
    }
    fn visit_new_statement(&mut self, h: &mut Heap, stmt: NewStatementId) -> VisitorResult {
        recursive_new_statement(self, h, stmt)
    }
    fn visit_put_statement(&mut self, h: &mut Heap, stmt: PutStatementId) -> VisitorResult {
        recursive_put_statement(self, h, stmt)
    }
    fn visit_expression_statement(
        &mut self,
        h: &mut Heap,
        stmt: ExpressionStatementId,
    ) -> VisitorResult {
        recursive_expression_statement(self, h, stmt)
    }

    fn visit_expression(&mut self, h: &mut Heap, expr: ExpressionId) -> VisitorResult {
        recursive_expression(self, h, expr)
    }
    fn visit_assignment_expression(
        &mut self,
        h: &mut Heap,
        expr: AssignmentExpressionId,
    ) -> VisitorResult {
        recursive_assignment_expression(self, h, expr)
    }
    fn visit_conditional_expression(
        &mut self,
        h: &mut Heap,
        expr: ConditionalExpressionId,
    ) -> VisitorResult {
        recursive_conditional_expression(self, h, expr)
    }
    fn visit_binary_expression(&mut self, h: &mut Heap, expr: BinaryExpressionId) -> VisitorResult {
        recursive_binary_expression(self, h, expr)
    }
    fn visit_unary_expression(&mut self, h: &mut Heap, expr: UnaryExpressionId) -> VisitorResult {
        recursive_unary_expression(self, h, expr)
    }
    fn visit_indexing_expression(
        &mut self,
        h: &mut Heap,
        expr: IndexingExpressionId,
    ) -> VisitorResult {
        recursive_indexing_expression(self, h, expr)
    }
    fn visit_slicing_expression(
        &mut self,
        h: &mut Heap,
        expr: SlicingExpressionId,
    ) -> VisitorResult {
        recursive_slicing_expression(self, h, expr)
    }
    fn visit_select_expression(&mut self, h: &mut Heap, expr: SelectExpressionId) -> VisitorResult {
        recursive_select_expression(self, h, expr)
    }
    fn visit_array_expression(&mut self, h: &mut Heap, expr: ArrayExpressionId) -> VisitorResult {
        recursive_array_expression(self, h, expr)
    }
    fn visit_call_expression(&mut self, h: &mut Heap, expr: CallExpressionId) -> VisitorResult {
        recursive_call_expression(self, h, expr)
    }
    fn visit_constant_expression(
        &mut self,
        _h: &mut Heap,
        _expr: ConstantExpressionId,
    ) -> VisitorResult {
        Ok(())
    }
    fn visit_variable_expression(
        &mut self,
        _h: &mut Heap,
        _expr: VariableExpressionId,
    ) -> VisitorResult {
        Ok(())
    }
}

// Bubble-up helpers
fn recursive_parameter_as_variable<T: Visitor>(
    this: &mut T,
    h: &mut Heap,
    param: ParameterId,
) -> VisitorResult {
    this.visit_variable_declaration(h, param.upcast())
}

fn recursive_local_as_variable<T: Visitor>(
    this: &mut T,
    h: &mut Heap,
    local: LocalId,
) -> VisitorResult {
    this.visit_variable_declaration(h, local.upcast())
}

fn recursive_call_expression_as_expression<T: Visitor>(
    this: &mut T,
    h: &mut Heap,
    call: CallExpressionId,
) -> VisitorResult {
    this.visit_expression(h, call.upcast())
}

// Recursive procedures
fn recursive_protocol_description<T: Visitor>(
    this: &mut T,
    h: &mut Heap,
    pd: RootId,
) -> VisitorResult {
    for &pragma in h[pd].pragmas.clone().iter() {
        this.visit_pragma(h, pragma)?;
    }
    for &import in h[pd].imports.clone().iter() {
        this.visit_import(h, import)?;
    }
    for &def in h[pd].definitions.clone().iter() {
        this.visit_symbol_definition(h, def)?;
    }
    Ok(())
}

fn recursive_symbol_definition<T: Visitor>(
    this: &mut T,
    h: &mut Heap,
    def: DefinitionId,
) -> VisitorResult {
    // We clone the definition in case it is modified
    match h[def].clone() {
        Definition::Component(cdef) => this.visit_component_definition(h, cdef.this()),
        Definition::Function(fdef) => this.visit_function_definition(h, fdef.this),
    }
}

fn recursive_component_definition<T: Visitor>(
    this: &mut T,
    h: &mut Heap,
    def: ComponentId,
) -> VisitorResult {
    match h[def].clone() {
        Component::Composite(cdef) => this.visit_composite_definition(h, cdef.this),
        Component::Primitive(pdef) => this.visit_primitive_definition(h, pdef.this),
    }
}

fn recursive_composite_definition<T: Visitor>(
    this: &mut T,
    h: &mut Heap,
    def: CompositeId,
) -> VisitorResult {
    for &param in h[def].parameters.clone().iter() {
        recursive_parameter_as_variable(this, h, param)?;
    }
    this.visit_statement(h, h[def].body)
}

fn recursive_primitive_definition<T: Visitor>(
    this: &mut T,
    h: &mut Heap,
    def: PrimitiveId,
) -> VisitorResult {
    for &param in h[def].parameters.clone().iter() {
        recursive_parameter_as_variable(this, h, param)?;
    }
    this.visit_statement(h, h[def].body)
}

fn recursive_function_definition<T: Visitor>(
    this: &mut T,
    h: &mut Heap,
    def: FunctionId,
) -> VisitorResult {
    for &param in h[def].parameters.clone().iter() {
        recursive_parameter_as_variable(this, h, param)?;
    }
    this.visit_statement(h, h[def].body)
}

fn recursive_variable_declaration<T: Visitor>(
    this: &mut T,
    h: &mut Heap,
    decl: VariableId,
) -> VisitorResult {
    match h[decl].clone() {
        Variable::Parameter(decl) => this.visit_parameter_declaration(h, decl.this),
        Variable::Local(decl) => this.visit_local_declaration(h, decl.this),
    }
}

fn recursive_statement<T: Visitor>(this: &mut T, h: &mut Heap, stmt: StatementId) -> VisitorResult {
    match h[stmt].clone() {
        Statement::Block(stmt) => this.visit_block_statement(h, stmt.this),
        Statement::Local(stmt) => this.visit_local_statement(h, stmt.this()),
        Statement::Skip(stmt) => this.visit_skip_statement(h, stmt.this),
        Statement::Labeled(stmt) => this.visit_labeled_statement(h, stmt.this),
        Statement::If(stmt) => this.visit_if_statement(h, stmt.this),
        Statement::EndIf(stmt) => unreachable!(), // pseudo-statement
        Statement::While(stmt) => this.visit_while_statement(h, stmt.this),
        Statement::EndWhile(stmt) => unreachable!(), // pseudo-statement
        Statement::Break(stmt) => this.visit_break_statement(h, stmt.this),
        Statement::Continue(stmt) => this.visit_continue_statement(h, stmt.this),
        Statement::Synchronous(stmt) => this.visit_synchronous_statement(h, stmt.this),
        Statement::EndSynchronous(stmt) => unreachable!(), // pseudo-statement
        Statement::Return(stmt) => this.visit_return_statement(h, stmt.this),
        Statement::Assert(stmt) => this.visit_assert_statement(h, stmt.this),
        Statement::Goto(stmt) => this.visit_goto_statement(h, stmt.this),
        Statement::New(stmt) => this.visit_new_statement(h, stmt.this),
        Statement::Put(stmt) => this.visit_put_statement(h, stmt.this),
        Statement::Expression(stmt) => this.visit_expression_statement(h, stmt.this),
    }
}

fn recursive_block_statement<T: Visitor>(
    this: &mut T,
    h: &mut Heap,
    block: BlockStatementId,
) -> VisitorResult {
    for &local in h[block].locals.clone().iter() {
        recursive_local_as_variable(this, h, local)?;
    }
    for &stmt in h[block].statements.clone().iter() {
        this.visit_statement(h, stmt)?;
    }
    Ok(())
}

fn recursive_local_statement<T: Visitor>(
    this: &mut T,
    h: &mut Heap,
    stmt: LocalStatementId,
) -> VisitorResult {
    match h[stmt].clone() {
        LocalStatement::Channel(stmt) => this.visit_channel_statement(h, stmt.this),
        LocalStatement::Memory(stmt) => this.visit_memory_statement(h, stmt.this),
    }
}

fn recursive_memory_statement<T: Visitor>(
    this: &mut T,
    h: &mut Heap,
    stmt: MemoryStatementId,
) -> VisitorResult {
    this.visit_expression(h, h[stmt].initial)
}

fn recursive_labeled_statement<T: Visitor>(
    this: &mut T,
    h: &mut Heap,
    stmt: LabeledStatementId,
) -> VisitorResult {
    this.visit_statement(h, h[stmt].body)
}

fn recursive_if_statement<T: Visitor>(
    this: &mut T,
    h: &mut Heap,
    stmt: IfStatementId,
) -> VisitorResult {
    this.visit_expression(h, h[stmt].test)?;
    this.visit_statement(h, h[stmt].true_body)?;
    this.visit_statement(h, h[stmt].false_body)
}

fn recursive_while_statement<T: Visitor>(
    this: &mut T,
    h: &mut Heap,
    stmt: WhileStatementId,
) -> VisitorResult {
    this.visit_expression(h, h[stmt].test)?;
    this.visit_statement(h, h[stmt].body)
}

fn recursive_synchronous_statement<T: Visitor>(
    this: &mut T,
    h: &mut Heap,
    stmt: SynchronousStatementId,
) -> VisitorResult {
    for &param in h[stmt].parameters.clone().iter() {
        recursive_parameter_as_variable(this, h, param)?;
    }
    this.visit_statement(h, h[stmt].body)
}

fn recursive_return_statement<T: Visitor>(
    this: &mut T,
    h: &mut Heap,
    stmt: ReturnStatementId,
) -> VisitorResult {
    this.visit_expression(h, h[stmt].expression)
}

fn recursive_assert_statement<T: Visitor>(
    this: &mut T,
    h: &mut Heap,
    stmt: AssertStatementId,
) -> VisitorResult {
    this.visit_expression(h, h[stmt].expression)
}

fn recursive_new_statement<T: Visitor>(
    this: &mut T,
    h: &mut Heap,
    stmt: NewStatementId,
) -> VisitorResult {
    recursive_call_expression_as_expression(this, h, h[stmt].expression)
}

fn recursive_put_statement<T: Visitor>(
    this: &mut T,
    h: &mut Heap,
    stmt: PutStatementId,
) -> VisitorResult {
    this.visit_expression(h, h[stmt].port)?;
    this.visit_expression(h, h[stmt].message)
}

fn recursive_expression_statement<T: Visitor>(
    this: &mut T,
    h: &mut Heap,
    stmt: ExpressionStatementId,
) -> VisitorResult {
    this.visit_expression(h, h[stmt].expression)
}

fn recursive_expression<T: Visitor>(
    this: &mut T,
    h: &mut Heap,
    expr: ExpressionId,
) -> VisitorResult {
    match h[expr].clone() {
        Expression::Assignment(expr) => this.visit_assignment_expression(h, expr.this),
        Expression::Conditional(expr) => this.visit_conditional_expression(h, expr.this),
        Expression::Binary(expr) => this.visit_binary_expression(h, expr.this),
        Expression::Unary(expr) => this.visit_unary_expression(h, expr.this),
        Expression::Indexing(expr) => this.visit_indexing_expression(h, expr.this),
        Expression::Slicing(expr) => this.visit_slicing_expression(h, expr.this),
        Expression::Select(expr) => this.visit_select_expression(h, expr.this),
        Expression::Array(expr) => this.visit_array_expression(h, expr.this),
        Expression::Constant(expr) => this.visit_constant_expression(h, expr.this),
        Expression::Call(expr) => this.visit_call_expression(h, expr.this),
        Expression::Variable(expr) => this.visit_variable_expression(h, expr.this),
    }
}

fn recursive_assignment_expression<T: Visitor>(
    this: &mut T,
    h: &mut Heap,
    expr: AssignmentExpressionId,
) -> VisitorResult {
    this.visit_expression(h, h[expr].left)?;
    this.visit_expression(h, h[expr].right)
}

fn recursive_conditional_expression<T: Visitor>(
    this: &mut T,
    h: &mut Heap,
    expr: ConditionalExpressionId,
) -> VisitorResult {
    this.visit_expression(h, h[expr].test)?;
    this.visit_expression(h, h[expr].true_expression)?;
    this.visit_expression(h, h[expr].false_expression)
}

fn recursive_binary_expression<T: Visitor>(
    this: &mut T,
    h: &mut Heap,
    expr: BinaryExpressionId,
) -> VisitorResult {
    this.visit_expression(h, h[expr].left)?;
    this.visit_expression(h, h[expr].right)
}

fn recursive_unary_expression<T: Visitor>(
    this: &mut T,
    h: &mut Heap,
    expr: UnaryExpressionId,
) -> VisitorResult {
    this.visit_expression(h, h[expr].expression)
}

fn recursive_indexing_expression<T: Visitor>(
    this: &mut T,
    h: &mut Heap,
    expr: IndexingExpressionId,
) -> VisitorResult {
    this.visit_expression(h, h[expr].subject)?;
    this.visit_expression(h, h[expr].index)
}

fn recursive_slicing_expression<T: Visitor>(
    this: &mut T,
    h: &mut Heap,
    expr: SlicingExpressionId,
) -> VisitorResult {
    this.visit_expression(h, h[expr].subject)?;
    this.visit_expression(h, h[expr].from_index)?;
    this.visit_expression(h, h[expr].to_index)
}

fn recursive_select_expression<T: Visitor>(
    this: &mut T,
    h: &mut Heap,
    expr: SelectExpressionId,
) -> VisitorResult {
    this.visit_expression(h, h[expr].subject)
}

fn recursive_array_expression<T: Visitor>(
    this: &mut T,
    h: &mut Heap,
    expr: ArrayExpressionId,
) -> VisitorResult {
    for &expr in h[expr].elements.clone().iter() {
        this.visit_expression(h, expr)?;
    }
    Ok(())
}

fn recursive_call_expression<T: Visitor>(
    this: &mut T,
    h: &mut Heap,
    expr: CallExpressionId,
) -> VisitorResult {
    for &expr in h[expr].arguments.clone().iter() {
        this.visit_expression(h, expr)?;
    }
    Ok(())
}

// ====================
// Grammar Rules
// ====================

struct NestedSynchronousStatements {
    illegal: bool,
}

impl NestedSynchronousStatements {
    fn new() -> Self {
        NestedSynchronousStatements { illegal: false }
    }
}

impl Visitor for NestedSynchronousStatements {
    fn visit_composite_definition(&mut self, h: &mut Heap, def: CompositeId) -> VisitorResult {
        assert!(!self.illegal);
        self.illegal = true;
        recursive_composite_definition(self, h, def)?;
        self.illegal = false;
        Ok(())
    }
    fn visit_function_definition(&mut self, h: &mut Heap, def: FunctionId) -> VisitorResult {
        assert!(!self.illegal);
        self.illegal = true;
        recursive_function_definition(self, h, def)?;
        self.illegal = false;
        Ok(())
    }
    fn visit_synchronous_statement(
        &mut self,
        h: &mut Heap,
        stmt: SynchronousStatementId,
    ) -> VisitorResult {
        if self.illegal {
            return Err(ParseError::new(
                h[stmt].position(),
                "Illegal nested synchronous statement",
            ));
        }
        self.illegal = true;
        recursive_synchronous_statement(self, h, stmt)?;
        self.illegal = false;
        Ok(())
    }
    fn visit_expression(&mut self, _h: &mut Heap, _expr: ExpressionId) -> VisitorResult {
        Ok(())
    }
}

struct ChannelStatementOccurrences {
    illegal: bool,
}

impl ChannelStatementOccurrences {
    fn new() -> Self {
        ChannelStatementOccurrences { illegal: false }
    }
}

impl Visitor for ChannelStatementOccurrences {
    fn visit_primitive_definition(&mut self, h: &mut Heap, def: PrimitiveId) -> VisitorResult {
        assert!(!self.illegal);
        self.illegal = true;
        recursive_primitive_definition(self, h, def)?;
        self.illegal = false;
        Ok(())
    }
    fn visit_function_definition(&mut self, h: &mut Heap, def: FunctionId) -> VisitorResult {
        assert!(!self.illegal);
        self.illegal = true;
        recursive_function_definition(self, h, def)?;
        self.illegal = false;
        Ok(())
    }
    fn visit_channel_statement(&mut self, h: &mut Heap, stmt: ChannelStatementId) -> VisitorResult {
        if self.illegal {
            return Err(ParseError::new(h[stmt].position(), "Illegal channel delcaration"));
        }
        Ok(())
    }
    fn visit_expression(&mut self, _h: &mut Heap, _expr: ExpressionId) -> VisitorResult {
        Ok(())
    }
}

struct FunctionStatementReturns {}

impl FunctionStatementReturns {
    fn new() -> Self {
        FunctionStatementReturns {}
    }
    fn function_error(&self, position: InputPosition) -> VisitorResult {
        Err(ParseError::new(position, "Function definition must return"))
    }
}

impl Visitor for FunctionStatementReturns {
    fn visit_component_definition(&mut self, _h: &mut Heap, _def: ComponentId) -> VisitorResult {
        Ok(())
    }
    fn visit_variable_declaration(&mut self, _h: &mut Heap, _decl: VariableId) -> VisitorResult {
        Ok(())
    }
    fn visit_block_statement(&mut self, h: &mut Heap, block: BlockStatementId) -> VisitorResult {
        let len = h[block].statements.len();
        assert!(len > 0);
        self.visit_statement(h, h[block].statements[len - 1])
    }
    fn visit_skip_statement(&mut self, h: &mut Heap, stmt: SkipStatementId) -> VisitorResult {
        self.function_error(h[stmt].position)
    }
    fn visit_break_statement(&mut self, h: &mut Heap, stmt: BreakStatementId) -> VisitorResult {
        self.function_error(h[stmt].position)
    }
    fn visit_continue_statement(
        &mut self,
        h: &mut Heap,
        stmt: ContinueStatementId,
    ) -> VisitorResult {
        self.function_error(h[stmt].position)
    }
    fn visit_assert_statement(&mut self, h: &mut Heap, stmt: AssertStatementId) -> VisitorResult {
        self.function_error(h[stmt].position)
    }
    fn visit_new_statement(&mut self, h: &mut Heap, stmt: NewStatementId) -> VisitorResult {
        self.function_error(h[stmt].position)
    }
    fn visit_expression_statement(
        &mut self,
        h: &mut Heap,
        stmt: ExpressionStatementId,
    ) -> VisitorResult {
        self.function_error(h[stmt].position)
    }
    fn visit_expression(&mut self, _h: &mut Heap, _expr: ExpressionId) -> VisitorResult {
        Ok(())
    }
}

struct ComponentStatementReturnNew {
    illegal_new: bool,
    illegal_return: bool,
}

impl ComponentStatementReturnNew {
    fn new() -> Self {
        ComponentStatementReturnNew { illegal_new: false, illegal_return: false }
    }
}

impl Visitor for ComponentStatementReturnNew {
    fn visit_component_definition(&mut self, h: &mut Heap, def: ComponentId) -> VisitorResult {
        assert!(!(self.illegal_new || self.illegal_return));
        self.illegal_return = true;
        recursive_component_definition(self, h, def)?;
        self.illegal_return = false;
        Ok(())
    }
    fn visit_primitive_definition(&mut self, h: &mut Heap, def: PrimitiveId) -> VisitorResult {
        assert!(!self.illegal_new);
        self.illegal_new = true;
        recursive_primitive_definition(self, h, def)?;
        self.illegal_new = false;
        Ok(())
    }
    fn visit_function_definition(&mut self, h: &mut Heap, def: FunctionId) -> VisitorResult {
        assert!(!(self.illegal_new || self.illegal_return));
        self.illegal_new = true;
        recursive_function_definition(self, h, def)?;
        self.illegal_new = false;
        Ok(())
    }
    fn visit_variable_declaration(&mut self, _h: &mut Heap, _decl: VariableId) -> VisitorResult {
        Ok(())
    }
    fn visit_return_statement(&mut self, h: &mut Heap, stmt: ReturnStatementId) -> VisitorResult {
        if self.illegal_return {
            Err(ParseError::new(h[stmt].position, "Component definition must not return"))
        } else {
            recursive_return_statement(self, h, stmt)
        }
    }
    fn visit_new_statement(&mut self, h: &mut Heap, stmt: NewStatementId) -> VisitorResult {
        if self.illegal_new {
            Err(ParseError::new(
                h[stmt].position,
                "Symbol definition contains illegal new statement",
            ))
        } else {
            recursive_new_statement(self, h, stmt)
        }
    }
    fn visit_expression(&mut self, _h: &mut Heap, _expr: ExpressionId) -> VisitorResult {
        Ok(())
    }
}

struct CheckBuiltinOccurrences {
    legal: bool,
}

impl CheckBuiltinOccurrences {
    fn new() -> Self {
        CheckBuiltinOccurrences { legal: false }
    }
}

impl Visitor for CheckBuiltinOccurrences {
    fn visit_synchronous_statement(
        &mut self,
        h: &mut Heap,
        stmt: SynchronousStatementId,
    ) -> VisitorResult {
        assert!(!self.legal);
        self.legal = true;
        recursive_synchronous_statement(self, h, stmt)?;
        self.legal = false;
        Ok(())
    }
    fn visit_call_expression(&mut self, h: &mut Heap, expr: CallExpressionId) -> VisitorResult {
        match h[expr].method {
            Method::Get | Method::Fires => {
                if !self.legal {
                    return Err(ParseError::new(h[expr].position, "Illegal built-in occurrence"));
                }
            }
            _ => {}
        }
        recursive_call_expression(self, h, expr)
    }
}

struct BuildSymbolDeclarations {
    declarations: Vec<DeclarationId>,
}

impl BuildSymbolDeclarations {
    fn new() -> Self {
        BuildSymbolDeclarations { declarations: Vec::new() }
    }
    fn checked_add(&mut self, h: &mut Heap, decl: DeclarationId) -> VisitorResult {
        for &old in self.declarations.iter() {
            let id = h[decl].identifier();
            if h[id] == h[h[old].identifier()] {
                return match h[decl].clone() {
                    Declaration::Defined(defined) => Err(ParseError::new(
                        h[defined.definition].position(),
                        format!("Defined symbol clash: {}", h[id]),
                    )),
                    Declaration::Imported(imported) => Err(ParseError::new(
                        h[imported.import].position(),
                        format!("Imported symbol clash: {}", h[id]),
                    )),
                };
            }
        }
        self.declarations.push(decl);
        Ok(())
    }
}

impl Visitor for BuildSymbolDeclarations {
    fn visit_protocol_description(&mut self, h: &mut Heap, pd: RootId) -> VisitorResult {
        recursive_protocol_description(self, h, pd)?;
        // Move all collected declarations to the protocol description
        h[pd].declarations.append(&mut self.declarations);
        Ok(())
    }
    fn visit_import(&mut self, h: &mut Heap, import: ImportId) -> VisitorResult {
        let vec = library::get_declarations(h, import)?;
        // Destructively iterate over the vector
        for decl in vec {
            self.checked_add(h, decl)?;
        }
        Ok(())
    }
    fn visit_symbol_definition(&mut self, h: &mut Heap, definition: DefinitionId) -> VisitorResult {
        let signature = Signature::from_definition(h, definition);
        let decl = h
            .alloc_defined_declaration(|this| DefinedDeclaration { this, definition, signature })
            .upcast();
        self.checked_add(h, decl)?;
        Ok(())
    }
}

struct LinkCallExpressions {
    pd: Option<RootId>,
    composite: bool,
    new_statement: bool,
}

impl LinkCallExpressions {
    fn new() -> Self {
        LinkCallExpressions { pd: None, composite: false, new_statement: false }
    }
    fn get_declaration(
        &self,
        h: &Heap,
        id: SourceIdentifierId,
    ) -> Result<DeclarationId, ParseError> {
        match h[self.pd.unwrap()].get_declaration(h, id.upcast()) {
            Some(id) => Ok(id),
            None => Err(ParseError::new(h[id].position, "Unresolved method")),
        }
    }
}

impl Visitor for LinkCallExpressions {
    fn visit_protocol_description(&mut self, h: &mut Heap, pd: RootId) -> VisitorResult {
        self.pd = Some(pd);
        recursive_protocol_description(self, h, pd)?;
        self.pd = None;
        Ok(())
    }
    fn visit_composite_definition(&mut self, h: &mut Heap, def: CompositeId) -> VisitorResult {
        assert!(!self.composite);
        self.composite = true;
        recursive_composite_definition(self, h, def)?;
        self.composite = false;
        Ok(())
    }
    fn visit_new_statement(&mut self, h: &mut Heap, stmt: NewStatementId) -> VisitorResult {
        assert!(self.composite);
        assert!(!self.new_statement);
        self.new_statement = true;
        recursive_new_statement(self, h, stmt)?;
        self.new_statement = false;
        Ok(())
    }
    fn visit_call_expression(&mut self, h: &mut Heap, expr: CallExpressionId) -> VisitorResult {
        if let Method::Symbolic(id) = h[expr].method {
            let decl = self.get_declaration(h, id)?;
            if self.new_statement && h[decl].is_function() {
                return Err(ParseError::new(h[id].position, "Illegal call expression"));
            }
            if !self.new_statement && h[decl].is_component() {
                return Err(ParseError::new(h[id].position, "Illegal call expression"));
            }
            // Set the corresponding declaration of the call
            h[expr].declaration = Some(decl);
        }
        // A new statement's call expression may have as arguments function calls
        let old = self.new_statement;
        self.new_statement = false;
        recursive_call_expression(self, h, expr)?;
        self.new_statement = old;
        Ok(())
    }
}

struct BuildScope {
    scope: Option<Scope>,
}

impl BuildScope {
    fn new() -> Self {
        BuildScope { scope: None }
    }
}

impl Visitor for BuildScope {
    fn visit_symbol_definition(&mut self, h: &mut Heap, def: DefinitionId) -> VisitorResult {
        assert!(self.scope.is_none());
        self.scope = Some(Scope::Definition(def));
        recursive_symbol_definition(self, h, def)?;
        self.scope = None;
        Ok(())
    }
    fn visit_block_statement(&mut self, h: &mut Heap, stmt: BlockStatementId) -> VisitorResult {
        assert!(!self.scope.is_none());
        let old = self.scope;
        // First store the current scope
        h[stmt].parent_scope = self.scope;
        // Then move scope down to current block
        self.scope = Some(Scope::Block(stmt));
        recursive_block_statement(self, h, stmt)?;
        // Move scope back up
        self.scope = old;
        Ok(())
    }
    fn visit_synchronous_statement(
        &mut self,
        h: &mut Heap,
        stmt: SynchronousStatementId,
    ) -> VisitorResult {
        assert!(!self.scope.is_none());
        let old = self.scope;
        // First store the current scope
        h[stmt].parent_scope = self.scope;
        // Then move scope down to current sync
        self.scope = Some(Scope::Synchronous(stmt));
        recursive_synchronous_statement(self, h, stmt)?;
        // Move scope back up
        self.scope = old;
        Ok(())
    }
    fn visit_expression(&mut self, h: &mut Heap, expr: ExpressionId) -> VisitorResult {
        Ok(())
    }
}

struct ResolveVariables {
    scope: Option<Scope>,
}

impl ResolveVariables {
    fn new() -> Self {
        ResolveVariables { scope: None }
    }
    fn get_variable(&self, h: &Heap, id: SourceIdentifierId) -> Result<VariableId, ParseError> {
        if let Some(var) = self.find_variable(h, id) {
            Ok(var)
        } else {
            Err(ParseError::new(h[id].position, "Unresolved variable"))
        }
    }
    fn find_variable(&self, h: &Heap, id: SourceIdentifierId) -> Option<VariableId> {
        ResolveVariables::find_variable_impl(h, self.scope, id)
    }
    fn find_variable_impl(
        h: &Heap,
        scope: Option<Scope>,
        id: SourceIdentifierId,
    ) -> Option<VariableId> {
        if let Some(scope) = scope {
            // The order in which we check for variables is important:
            // otherwise, two variables with the same name are shadowed.
            if let Some(var) = ResolveVariables::find_variable_impl(h, scope.parent_scope(h), id) {
                Some(var)
            } else {
                scope.get_variable(h, id)
            }
        } else {
            None
        }
    }
}

impl Visitor for ResolveVariables {
    fn visit_symbol_definition(&mut self, h: &mut Heap, def: DefinitionId) -> VisitorResult {
        assert!(self.scope.is_none());
        self.scope = Some(Scope::Definition(def));
        recursive_symbol_definition(self, h, def)?;
        self.scope = None;
        Ok(())
    }
    fn visit_variable_declaration(&mut self, h: &mut Heap, decl: VariableId) -> VisitorResult {
        // This is only called for parameters of definitions and synchronous statements,
        // since the local variables of block statements are still empty
        // the moment it is traversed. After resolving variables, this
        // function is also called for every local variable declaration.

        // We want to make sure that the resolved variable is the variable declared itself;
        // otherwise, there is some variable defined in the parent scope. This check
        // imposes that the order in which find_variable looks is significant!
        let id = h[decl].identifier();
        let check_same = self.find_variable(h, id);
        if let Some(check_same) = check_same {
            if check_same != decl {
                return Err(ParseError::new(h[id].position, "Declared variable clash"));
            }
        }
        recursive_variable_declaration(self, h, decl)
    }
    fn visit_memory_statement(&mut self, h: &mut Heap, stmt: MemoryStatementId) -> VisitorResult {
        assert!(!self.scope.is_none());
        let var = h[stmt].variable;
        let id = h[var].identifier;
        // First check whether variable with same identifier is in scope
        let check_duplicate = self.find_variable(h, id);
        if !check_duplicate.is_none() {
            return Err(ParseError::new(h[id].position, "Declared variable clash"));
        }
        // Then check the expression's variables (this should not refer to own variable)
        recursive_memory_statement(self, h, stmt)?;
        // Finally, we may add the variable to the scope, which is guaranteed to be a block
        {
            let mut block = &mut h[self.scope.unwrap().to_block()];
            block.locals.push(var);
        }
        Ok(())
    }
    fn visit_channel_statement(&mut self, h: &mut Heap, stmt: ChannelStatementId) -> VisitorResult {
        assert!(!self.scope.is_none());
        // First handle the from variable
        {
            let var = h[stmt].from;
            let id = h[var].identifier;
            let check_duplicate = self.find_variable(h, id);
            if !check_duplicate.is_none() {
                return Err(ParseError::new(h[id].position, "Declared variable clash"));
            }
            let mut block = &mut h[self.scope.unwrap().to_block()];
            block.locals.push(var);
        }
        // Then handle the to variable (which may not be the same as the from)
        {
            let var = h[stmt].to;
            let id = h[var].identifier;
            let check_duplicate = self.find_variable(h, id);
            if !check_duplicate.is_none() {
                return Err(ParseError::new(h[id].position, "Declared variable clash"));
            }
            let mut block = &mut h[self.scope.unwrap().to_block()];
            block.locals.push(var);
        }
        Ok(())
    }
    fn visit_block_statement(&mut self, h: &mut Heap, stmt: BlockStatementId) -> VisitorResult {
        assert!(!self.scope.is_none());
        let old = self.scope;
        self.scope = Some(Scope::Block(stmt));
        recursive_block_statement(self, h, stmt)?;
        self.scope = old;
        Ok(())
    }
    fn visit_synchronous_statement(
        &mut self,
        h: &mut Heap,
        stmt: SynchronousStatementId,
    ) -> VisitorResult {
        assert!(!self.scope.is_none());
        let old = self.scope;
        self.scope = Some(Scope::Synchronous(stmt));
        recursive_synchronous_statement(self, h, stmt)?;
        self.scope = old;
        Ok(())
    }
    fn visit_variable_expression(
        &mut self,
        h: &mut Heap,
        expr: VariableExpressionId,
    ) -> VisitorResult {
        let var = self.get_variable(h, h[expr].identifier)?;
        h[expr].declaration = Some(var);
        Ok(())
    }
}

struct UniqueStatementId(StatementId);

struct LinkStatements {
    prev: Option<UniqueStatementId>,
}

impl LinkStatements {
    fn new() -> Self {
        LinkStatements { prev: None }
    }
}

impl Visitor for LinkStatements {
    fn visit_symbol_definition(&mut self, h: &mut Heap, def: DefinitionId) -> VisitorResult {
        assert!(self.prev.is_none());
        recursive_symbol_definition(self, h, def)?;
        // Clear out last statement
        self.prev = None;
        Ok(())
    }
    fn visit_statement(&mut self, h: &mut Heap, stmt: StatementId) -> VisitorResult {
        if let Some(UniqueStatementId(prev)) = self.prev.take() {
            h[prev].link_next(stmt);
        }
        recursive_statement(self, h, stmt)
    }
    fn visit_local_statement(&mut self, _h: &mut Heap, stmt: LocalStatementId) -> VisitorResult {
        self.prev = Some(UniqueStatementId(stmt.upcast()));
        Ok(())
    }
    fn visit_labeled_statement(&mut self, h: &mut Heap, stmt: LabeledStatementId) -> VisitorResult {
        recursive_labeled_statement(self, h, stmt)
    }
    fn visit_skip_statement(&mut self, _h: &mut Heap, stmt: SkipStatementId) -> VisitorResult {
        self.prev = Some(UniqueStatementId(stmt.upcast()));
        Ok(())
    }
    fn visit_if_statement(&mut self, h: &mut Heap, stmt: IfStatementId) -> VisitorResult {
        // We allocate a pseudo-statement, which combines both branches into one next statement
        let position = h[stmt].position;
        let pseudo =
            h.alloc_end_if_statement(|this| EndIfStatement { this, position, next: None }).upcast();
        assert!(self.prev.is_none());
        self.visit_statement(h, h[stmt].true_body)?;
        if let Some(UniqueStatementId(prev)) = self.prev.take() {
            h[prev].link_next(pseudo);
        }
        assert!(self.prev.is_none());
        self.visit_statement(h, h[stmt].false_body)?;
        if let Some(UniqueStatementId(prev)) = self.prev.take() {
            h[prev].link_next(pseudo);
        }
        // Use the pseudo-statement as the statement where to update the next pointer
        self.prev = Some(UniqueStatementId(pseudo));
        Ok(())
    }
    fn visit_while_statement(&mut self, h: &mut Heap, stmt: WhileStatementId) -> VisitorResult {
        // We allocate a pseudo-statement, to which the break statement finds its target
        let position = h[stmt].position;
        let pseudo =
            h.alloc_end_while_statement(|this| EndWhileStatement { this, position, next: None });
        // Update the while's next statement to point to the pseudo-statement
        h[stmt].next = Some(pseudo);
        assert!(self.prev.is_none());
        self.visit_statement(h, h[stmt].body)?;
        // The body's next statement loops back to the while statement itself
        // Note: continue statements also loop back to the while statement itself
        if let Some(UniqueStatementId(prev)) = std::mem::replace(&mut self.prev, None) {
            h[prev].link_next(stmt.upcast());
        }
        // Use the while statement as the statement where the next pointer is updated
        self.prev = Some(UniqueStatementId(pseudo.upcast()));
        Ok(())
    }
    fn visit_break_statement(&mut self, _h: &mut Heap, _stmt: BreakStatementId) -> VisitorResult {
        Ok(())
    }
    fn visit_continue_statement(
        &mut self,
        _h: &mut Heap,
        _stmt: ContinueStatementId,
    ) -> VisitorResult {
        Ok(())
    }
    fn visit_synchronous_statement(
        &mut self,
        h: &mut Heap,
        stmt: SynchronousStatementId,
    ) -> VisitorResult {
        // Allocate a pseudo-statement, that is added for helping the evaluator to issue a command
        // that marks the end of the synchronous block. Every evaluation has to pause at this
        // point, only to resume later when the thread is selected as unique thread to continue.
        let position = h[stmt].position;
        let pseudo = h
            .alloc_end_synchronous_statement(|this| EndSynchronousStatement {
                this,
                position,
                next: None,
            })
            .upcast();
        assert!(self.prev.is_none());
        self.visit_statement(h, h[stmt].body)?;
        // The body's next statement points to the pseudo element
        if let Some(UniqueStatementId(prev)) = std::mem::replace(&mut self.prev, None) {
            h[prev].link_next(pseudo);
        }
        // Use the pseudo-statement as the statement where the next pointer is updated
        self.prev = Some(UniqueStatementId(pseudo));
        Ok(())
    }
    fn visit_return_statement(&mut self, h: &mut Heap, stmt: ReturnStatementId) -> VisitorResult {
        Ok(())
    }
    fn visit_assert_statement(&mut self, h: &mut Heap, stmt: AssertStatementId) -> VisitorResult {
        self.prev = Some(UniqueStatementId(stmt.upcast()));
        Ok(())
    }
    fn visit_goto_statement(&mut self, _h: &mut Heap, _stmt: GotoStatementId) -> VisitorResult {
        Ok(())
    }
    fn visit_new_statement(&mut self, h: &mut Heap, stmt: NewStatementId) -> VisitorResult {
        self.prev = Some(UniqueStatementId(stmt.upcast()));
        Ok(())
    }
    fn visit_put_statement(&mut self, h: &mut Heap, stmt: PutStatementId) -> VisitorResult {
        self.prev = Some(UniqueStatementId(stmt.upcast()));
        Ok(())
    }
    fn visit_expression_statement(
        &mut self,
        h: &mut Heap,
        stmt: ExpressionStatementId,
    ) -> VisitorResult {
        self.prev = Some(UniqueStatementId(stmt.upcast()));
        Ok(())
    }
    fn visit_expression(&mut self, h: &mut Heap, expr: ExpressionId) -> VisitorResult {
        Ok(())
    }
}

struct BuildLabels {
    block: Option<BlockStatementId>,
    sync_enclosure: Option<SynchronousStatementId>,
}

impl BuildLabels {
    fn new() -> Self {
        BuildLabels { block: None, sync_enclosure: None }
    }
}

impl Visitor for BuildLabels {
    fn visit_block_statement(&mut self, h: &mut Heap, stmt: BlockStatementId) -> VisitorResult {
        assert_eq!(self.block, h[stmt].parent_block(h));
        let old = self.block;
        self.block = Some(stmt);
        recursive_block_statement(self, h, stmt)?;
        self.block = old;
        Ok(())
    }
    fn visit_labeled_statement(&mut self, h: &mut Heap, stmt: LabeledStatementId) -> VisitorResult {
        assert!(!self.block.is_none());
        // Store label in current block (on the fly)
        h[self.block.unwrap()].labels.push(stmt);
        // Update synchronous scope of label
        h[stmt].in_sync = self.sync_enclosure;
        recursive_labeled_statement(self, h, stmt)
    }
    fn visit_while_statement(&mut self, h: &mut Heap, stmt: WhileStatementId) -> VisitorResult {
        h[stmt].in_sync = self.sync_enclosure;
        recursive_while_statement(self, h, stmt)
    }
    fn visit_synchronous_statement(
        &mut self,
        h: &mut Heap,
        stmt: SynchronousStatementId,
    ) -> VisitorResult {
        assert!(self.sync_enclosure.is_none());
        self.sync_enclosure = Some(stmt);
        recursive_synchronous_statement(self, h, stmt)?;
        self.sync_enclosure = None;
        Ok(())
    }
    fn visit_expression(&mut self, h: &mut Heap, expr: ExpressionId) -> VisitorResult {
        Ok(())
    }
}

struct ResolveLabels {
    block: Option<BlockStatementId>,
    while_enclosure: Option<WhileStatementId>,
    sync_enclosure: Option<SynchronousStatementId>,
}

impl ResolveLabels {
    fn new() -> Self {
        ResolveLabels { block: None, while_enclosure: None, sync_enclosure: None }
    }
    fn check_duplicate_impl(
        h: &Heap,
        block: Option<BlockStatementId>,
        stmt: LabeledStatementId,
    ) -> VisitorResult {
        if let Some(block) = block {
            // Checking the parent first is important. Otherwise, labels
            // overshadow previously defined labels: and this is illegal!
            ResolveLabels::check_duplicate_impl(h, h[block].parent_block(h), stmt)?;
            // For the current block, check for a duplicate.
            for &other_stmt in h[block].labels.iter() {
                if other_stmt == stmt {
                    continue;
                } else {
                    if h[h[other_stmt].label] == h[h[stmt].label] {
                        return Err(ParseError::new(h[stmt].position, "Duplicate label"));
                    }
                }
            }
        }
        Ok(())
    }
    fn check_duplicate(&self, h: &Heap, stmt: LabeledStatementId) -> VisitorResult {
        ResolveLabels::check_duplicate_impl(h, self.block, stmt)
    }
    fn get_target(
        &self,
        h: &Heap,
        id: SourceIdentifierId,
    ) -> Result<LabeledStatementId, ParseError> {
        if let Some(stmt) = ResolveLabels::find_target(h, self.block, id) {
            Ok(stmt)
        } else {
            Err(ParseError::new(h[id].position, "Unresolved label"))
        }
    }
    fn find_target(
        h: &Heap,
        block: Option<BlockStatementId>,
        id: SourceIdentifierId,
    ) -> Option<LabeledStatementId> {
        if let Some(block) = block {
            // It does not matter in what order we find the labels.
            // If there are duplicates: that is checked elsewhere.
            for &stmt in h[block].labels.iter() {
                if h[h[stmt].label] == h[id] {
                    return Some(stmt);
                }
            }
            if let Some(stmt) = ResolveLabels::find_target(h, h[block].parent_block(h), id) {
                return Some(stmt);
            }
        }
        None
    }
}

impl Visitor for ResolveLabels {
    fn visit_block_statement(&mut self, h: &mut Heap, stmt: BlockStatementId) -> VisitorResult {
        assert_eq!(self.block, h[stmt].parent_block(h));
        let old = self.block;
        self.block = Some(stmt);
        recursive_block_statement(self, h, stmt)?;
        self.block = old;
        Ok(())
    }
    fn visit_labeled_statement(&mut self, h: &mut Heap, stmt: LabeledStatementId) -> VisitorResult {
        assert!(!self.block.is_none());
        self.check_duplicate(h, stmt)?;
        recursive_labeled_statement(self, h, stmt)
    }
    fn visit_while_statement(&mut self, h: &mut Heap, stmt: WhileStatementId) -> VisitorResult {
        let old = self.while_enclosure;
        self.while_enclosure = Some(stmt);
        recursive_while_statement(self, h, stmt)?;
        self.while_enclosure = old;
        Ok(())
    }
    fn visit_break_statement(&mut self, h: &mut Heap, stmt: BreakStatementId) -> VisitorResult {
        let the_while;
        if let Some(label) = h[stmt].label {
            let target = self.get_target(h, label)?;
            let target = &h[h[target].body];
            if !target.is_while() {
                return Err(ParseError::new(
                    h[stmt].position,
                    "Illegal break: target not a while statement",
                ));
            }
            the_while = target.as_while();
        // TODO: check if break is nested under while
        } else {
            if self.while_enclosure.is_none() {
                return Err(ParseError::new(
                    h[stmt].position,
                    "Illegal break: no surrounding while statement",
                ));
            }
            the_while = &h[self.while_enclosure.unwrap()];
            // break is always nested under while, by recursive vistor
        }
        if the_while.in_sync != self.sync_enclosure {
            return Err(ParseError::new(
                h[stmt].position,
                "Illegal break: synchronous statement escape",
            ));
        }
        h[stmt].target = the_while.next;
        Ok(())
    }
    fn visit_continue_statement(
        &mut self,
        h: &mut Heap,
        stmt: ContinueStatementId,
    ) -> VisitorResult {
        let the_while;
        if let Some(label) = h[stmt].label {
            let target = self.get_target(h, label)?;
            let target = &h[h[target].body];
            if !target.is_while() {
                return Err(ParseError::new(
                    h[stmt].position,
                    "Illegal continue: target not a while statement",
                ));
            }
            the_while = target.as_while();
        // TODO: check if continue is nested under while
        } else {
            if self.while_enclosure.is_none() {
                return Err(ParseError::new(
                    h[stmt].position,
                    "Illegal continue: no surrounding while statement",
                ));
            }
            the_while = &h[self.while_enclosure.unwrap()];
            // continue is always nested under while, by recursive vistor
        }
        if the_while.in_sync != self.sync_enclosure {
            return Err(ParseError::new(
                h[stmt].position,
                "Illegal continue: synchronous statement escape",
            ));
        }
        h[stmt].target = Some(the_while.this);
        Ok(())
    }
    fn visit_synchronous_statement(
        &mut self,
        h: &mut Heap,
        stmt: SynchronousStatementId,
    ) -> VisitorResult {
        assert!(self.sync_enclosure.is_none());
        self.sync_enclosure = Some(stmt);
        recursive_synchronous_statement(self, h, stmt)?;
        self.sync_enclosure = None;
        Ok(())
    }
    fn visit_goto_statement(&mut self, h: &mut Heap, stmt: GotoStatementId) -> VisitorResult {
        let target = self.get_target(h, h[stmt].label)?;
        if h[target].in_sync != self.sync_enclosure {
            return Err(ParseError::new(
                h[stmt].position,
                "Illegal goto: synchronous statement escape",
            ));
        }
        h[stmt].target = Some(target);
        Ok(())
    }
    fn visit_expression(&mut self, h: &mut Heap, expr: ExpressionId) -> VisitorResult {
        Ok(())
    }
}

struct AssignableExpressions {
    assignable: bool,
}

impl AssignableExpressions {
    fn new() -> Self {
        AssignableExpressions { assignable: false }
    }
    fn error(&self, position: InputPosition) -> VisitorResult {
        Err(ParseError::new(position, "Unassignable expression"))
    }
}

impl Visitor for AssignableExpressions {
    fn visit_assignment_expression(
        &mut self,
        h: &mut Heap,
        expr: AssignmentExpressionId,
    ) -> VisitorResult {
        if self.assignable {
            self.error(h[expr].position)
        } else {
            self.assignable = true;
            self.visit_expression(h, h[expr].left)?;
            self.assignable = false;
            self.visit_expression(h, h[expr].right)
        }
    }
    fn visit_conditional_expression(
        &mut self,
        h: &mut Heap,
        expr: ConditionalExpressionId,
    ) -> VisitorResult {
        if self.assignable {
            self.error(h[expr].position)
        } else {
            recursive_conditional_expression(self, h, expr)
        }
    }
    fn visit_binary_expression(&mut self, h: &mut Heap, expr: BinaryExpressionId) -> VisitorResult {
        if self.assignable {
            self.error(h[expr].position)
        } else {
            recursive_binary_expression(self, h, expr)
        }
    }
    fn visit_unary_expression(&mut self, h: &mut Heap, expr: UnaryExpressionId) -> VisitorResult {
        if self.assignable {
            self.error(h[expr].position)
        } else {
            match h[expr].operation {
                UnaryOperation::PostDecrement
                | UnaryOperation::PreDecrement
                | UnaryOperation::PostIncrement
                | UnaryOperation::PreIncrement => {
                    self.assignable = true;
                    recursive_unary_expression(self, h, expr)?;
                    self.assignable = false;
                    Ok(())
                }
                _ => recursive_unary_expression(self, h, expr),
            }
        }
    }
    fn visit_indexing_expression(
        &mut self,
        h: &mut Heap,
        expr: IndexingExpressionId,
    ) -> VisitorResult {
        let old = self.assignable;
        self.assignable = false;
        recursive_indexing_expression(self, h, expr)?;
        self.assignable = old;
        Ok(())
    }
    fn visit_slicing_expression(
        &mut self,
        h: &mut Heap,
        expr: SlicingExpressionId,
    ) -> VisitorResult {
        let old = self.assignable;
        self.assignable = false;
        recursive_slicing_expression(self, h, expr)?;
        self.assignable = old;
        Ok(())
    }
    fn visit_select_expression(&mut self, h: &mut Heap, expr: SelectExpressionId) -> VisitorResult {
        if h[expr].field.is_length() && self.assignable {
            return self.error(h[expr].position);
        }
        let old = self.assignable;
        self.assignable = false;
        recursive_select_expression(self, h, expr)?;
        self.assignable = old;
        Ok(())
    }
    fn visit_array_expression(&mut self, h: &mut Heap, expr: ArrayExpressionId) -> VisitorResult {
        if self.assignable {
            self.error(h[expr].position)
        } else {
            recursive_array_expression(self, h, expr)
        }
    }
    fn visit_call_expression(&mut self, h: &mut Heap, expr: CallExpressionId) -> VisitorResult {
        if self.assignable {
            self.error(h[expr].position)
        } else {
            recursive_call_expression(self, h, expr)
        }
    }
    fn visit_constant_expression(
        &mut self,
        h: &mut Heap,
        expr: ConstantExpressionId,
    ) -> VisitorResult {
        if self.assignable {
            self.error(h[expr].position)
        } else {
            Ok(())
        }
    }
    fn visit_variable_expression(
        &mut self,
        h: &mut Heap,
        expr: VariableExpressionId,
    ) -> VisitorResult {
        Ok(())
    }
}

struct IndexableExpressions {
    indexable: bool,
}

impl IndexableExpressions {
    fn new() -> Self {
        IndexableExpressions { indexable: false }
    }
    fn error(&self, position: InputPosition) -> VisitorResult {
        Err(ParseError::new(position, "Unindexable expression"))
    }
}

impl Visitor for IndexableExpressions {
    fn visit_assignment_expression(
        &mut self,
        h: &mut Heap,
        expr: AssignmentExpressionId,
    ) -> VisitorResult {
        if self.indexable {
            self.error(h[expr].position)
        } else {
            recursive_assignment_expression(self, h, expr)
        }
    }
    fn visit_conditional_expression(
        &mut self,
        h: &mut Heap,
        expr: ConditionalExpressionId,
    ) -> VisitorResult {
        let old = self.indexable;
        self.indexable = false;
        self.visit_expression(h, h[expr].test)?;
        self.indexable = old;
        self.visit_expression(h, h[expr].true_expression)?;
        self.visit_expression(h, h[expr].false_expression)
    }
    fn visit_binary_expression(&mut self, h: &mut Heap, expr: BinaryExpressionId) -> VisitorResult {
        if self.indexable && h[expr].operation != BinaryOperator::Concatenate {
            self.error(h[expr].position)
        } else {
            recursive_binary_expression(self, h, expr)
        }
    }
    fn visit_unary_expression(&mut self, h: &mut Heap, expr: UnaryExpressionId) -> VisitorResult {
        if self.indexable {
            self.error(h[expr].position)
        } else {
            recursive_unary_expression(self, h, expr)
        }
    }
    fn visit_indexing_expression(
        &mut self,
        h: &mut Heap,
        expr: IndexingExpressionId,
    ) -> VisitorResult {
        if self.indexable {
            self.error(h[expr].position)
        } else {
            self.indexable = true;
            self.visit_expression(h, h[expr].subject)?;
            self.indexable = false;
            self.visit_expression(h, h[expr].index)
        }
    }
    fn visit_slicing_expression(
        &mut self,
        h: &mut Heap,
        expr: SlicingExpressionId,
    ) -> VisitorResult {
        let old = self.indexable;
        self.indexable = true;
        self.visit_expression(h, h[expr].subject)?;
        self.indexable = false;
        self.visit_expression(h, h[expr].from_index)?;
        self.visit_expression(h, h[expr].to_index)?;
        self.indexable = old;
        Ok(())
    }
    fn visit_select_expression(&mut self, h: &mut Heap, expr: SelectExpressionId) -> VisitorResult {
        let old = self.indexable;
        self.indexable = false;
        recursive_select_expression(self, h, expr)?;
        self.indexable = old;
        Ok(())
    }
    fn visit_array_expression(&mut self, h: &mut Heap, expr: ArrayExpressionId) -> VisitorResult {
        let old = self.indexable;
        self.indexable = false;
        recursive_array_expression(self, h, expr)?;
        self.indexable = old;
        Ok(())
    }
    fn visit_call_expression(&mut self, h: &mut Heap, expr: CallExpressionId) -> VisitorResult {
        let old = self.indexable;
        self.indexable = false;
        recursive_call_expression(self, h, expr)?;
        self.indexable = old;
        Ok(())
    }
    fn visit_constant_expression(
        &mut self,
        h: &mut Heap,
        expr: ConstantExpressionId,
    ) -> VisitorResult {
        if self.indexable {
            self.error(h[expr].position)
        } else {
            Ok(())
        }
    }
}

struct SelectableExpressions {
    selectable: bool,
}

impl SelectableExpressions {
    fn new() -> Self {
        SelectableExpressions { selectable: false }
    }
    fn error(&self, position: InputPosition) -> VisitorResult {
        Err(ParseError::new(position, "Unselectable expression"))
    }
}

impl Visitor for SelectableExpressions {
    fn visit_assignment_expression(
        &mut self,
        h: &mut Heap,
        expr: AssignmentExpressionId,
    ) -> VisitorResult {
        // left-hand side of assignment can be skipped
        let old = self.selectable;
        self.selectable = false;
        self.visit_expression(h, h[expr].right)?;
        self.selectable = old;
        Ok(())
    }
    fn visit_conditional_expression(
        &mut self,
        h: &mut Heap,
        expr: ConditionalExpressionId,
    ) -> VisitorResult {
        let old = self.selectable;
        self.selectable = false;
        self.visit_expression(h, h[expr].test)?;
        self.selectable = old;
        self.visit_expression(h, h[expr].true_expression)?;
        self.visit_expression(h, h[expr].false_expression)
    }
    fn visit_binary_expression(&mut self, h: &mut Heap, expr: BinaryExpressionId) -> VisitorResult {
        if self.selectable && h[expr].operation != BinaryOperator::Concatenate {
            self.error(h[expr].position)
        } else {
            recursive_binary_expression(self, h, expr)
        }
    }
    fn visit_unary_expression(&mut self, h: &mut Heap, expr: UnaryExpressionId) -> VisitorResult {
        if self.selectable {
            self.error(h[expr].position)
        } else {
            recursive_unary_expression(self, h, expr)
        }
    }
    fn visit_indexing_expression(
        &mut self,
        h: &mut Heap,
        expr: IndexingExpressionId,
    ) -> VisitorResult {
        let old = self.selectable;
        self.selectable = false;
        recursive_indexing_expression(self, h, expr)?;
        self.selectable = old;
        Ok(())
    }
    fn visit_slicing_expression(
        &mut self,
        h: &mut Heap,
        expr: SlicingExpressionId,
    ) -> VisitorResult {
        let old = self.selectable;
        self.selectable = false;
        recursive_slicing_expression(self, h, expr)?;
        self.selectable = old;
        Ok(())
    }
    fn visit_select_expression(&mut self, h: &mut Heap, expr: SelectExpressionId) -> VisitorResult {
        let old = self.selectable;
        self.selectable = false;
        recursive_select_expression(self, h, expr)?;
        self.selectable = old;
        Ok(())
    }
    fn visit_array_expression(&mut self, h: &mut Heap, expr: ArrayExpressionId) -> VisitorResult {
        let old = self.selectable;
        self.selectable = false;
        recursive_array_expression(self, h, expr)?;
        self.selectable = old;
        Ok(())
    }
    fn visit_call_expression(&mut self, h: &mut Heap, expr: CallExpressionId) -> VisitorResult {
        let old = self.selectable;
        self.selectable = false;
        recursive_call_expression(self, h, expr)?;
        self.selectable = old;
        Ok(())
    }
    fn visit_constant_expression(
        &mut self,
        h: &mut Heap,
        expr: ConstantExpressionId,
    ) -> VisitorResult {
        if self.selectable {
            self.error(h[expr].position)
        } else {
            Ok(())
        }
    }
}

pub struct Parser<'a> {
    source: &'a mut InputSource,
}

impl<'a> Parser<'a> {
    pub fn new(source: &'a mut InputSource) -> Self {
        Parser { source }
    }
    pub fn parse(&mut self, h: &mut Heap) -> Result<RootId, ParseError> {
        let mut lex = Lexer::new(self.source);
        let pd = lex.consume_protocol_description(h)?;
        NestedSynchronousStatements::new().visit_protocol_description(h, pd)?;
        ChannelStatementOccurrences::new().visit_protocol_description(h, pd)?;
        FunctionStatementReturns::new().visit_protocol_description(h, pd)?;
        ComponentStatementReturnNew::new().visit_protocol_description(h, pd)?;
        CheckBuiltinOccurrences::new().visit_protocol_description(h, pd)?;
        BuildSymbolDeclarations::new().visit_protocol_description(h, pd)?;
        LinkCallExpressions::new().visit_protocol_description(h, pd)?;
        BuildScope::new().visit_protocol_description(h, pd)?;
        ResolveVariables::new().visit_protocol_description(h, pd)?;
        LinkStatements::new().visit_protocol_description(h, pd)?;
        BuildLabels::new().visit_protocol_description(h, pd)?;
        ResolveLabels::new().visit_protocol_description(h, pd)?;
        AssignableExpressions::new().visit_protocol_description(h, pd)?;
        IndexableExpressions::new().visit_protocol_description(h, pd)?;
        SelectableExpressions::new().visit_protocol_description(h, pd)?;
        Ok(pd)
    }
}

#[cfg(test)]
mod tests {
    extern crate test_generator;

    use std::fs::File;
    use std::io::Read;
    use std::path::Path;

    use test_generator::test_resources;

    use super::*;

    #[test_resources("testdata/parser/positive/*.pdl")]
    fn batch1(resource: &str) {
        let path = Path::new(resource);
        let mut heap = Heap::new();
        let mut source = InputSource::from_file(&path).unwrap();
        let mut parser = Parser::new(&mut source);
        match parser.parse(&mut heap) {
            Ok(_) => {}
            Err(err) => {
                println!("{}", err.display(&source));
                println!("{:?}", err);
                assert!(false);
            }
        }
    }

    #[test_resources("testdata/parser/negative/*.pdl")]
    fn batch2(resource: &str) {
        let path = Path::new(resource);
        let expect = path.with_extension("txt");
        let mut heap = Heap::new();
        let mut source = InputSource::from_file(&path).unwrap();
        let mut parser = Parser::new(&mut source);
        match parser.parse(&mut heap) {
            Ok(pd) => {
                println!("{:?}", heap[pd]);
                println!("Expected parse error:");

                let mut cev: Vec<u8> = Vec::new();
                let mut f = File::open(expect).unwrap();
                f.read_to_end(&mut cev).unwrap();
                println!("{}", String::from_utf8_lossy(&cev));
                assert!(false);
            }
            Err(err) => {
                println!("{:?}", err);

                let mut vec: Vec<u8> = Vec::new();
                err.write(&source, &mut vec).unwrap();
                println!("{}", String::from_utf8_lossy(&vec));

                let mut cev: Vec<u8> = Vec::new();
                let mut f = File::open(expect).unwrap();
                f.read_to_end(&mut cev).unwrap();
                println!("{}", String::from_utf8_lossy(&cev));

                assert_eq!(vec, cev);
            }
        }
    }
}
