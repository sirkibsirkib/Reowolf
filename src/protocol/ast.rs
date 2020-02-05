use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Index, IndexMut};

use id_arena::{Arena, Id};

use crate::protocol::inputsource::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RootId(Id<Root>);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PragmaId(Id<Pragma>);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImportId(Id<Import>);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IdentifierId(Id<Identifier>);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceIdentifierId(IdentifierId);

impl SourceIdentifierId {
    pub fn upcast(self) -> IdentifierId {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExternalIdentifierId(IdentifierId);

impl ExternalIdentifierId {
    pub fn upcast(self) -> IdentifierId {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TypeAnnotationId(Id<TypeAnnotation>);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VariableId(Id<Variable>);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ParameterId(VariableId);

impl ParameterId {
    pub fn upcast(self) -> VariableId {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocalId(VariableId);

impl LocalId {
    pub fn upcast(self) -> VariableId {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DefinitionId(Id<Definition>);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ComponentId(DefinitionId);

impl ComponentId {
    pub fn upcast(self) -> DefinitionId {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FunctionId(DefinitionId);

impl FunctionId {
    pub fn upcast(self) -> DefinitionId {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompositeId(ComponentId);

impl CompositeId {
    pub fn upcast(self) -> ComponentId {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PrimitiveId(ComponentId);

impl PrimitiveId {
    pub fn upcast(self) -> ComponentId {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StatementId(Id<Statement>);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlockStatementId(StatementId);

impl BlockStatementId {
    pub fn upcast(self) -> StatementId {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalStatementId(StatementId);

impl LocalStatementId {
    pub fn upcast(self) -> StatementId {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MemoryStatementId(LocalStatementId);

impl MemoryStatementId {
    pub fn upcast(self) -> LocalStatementId {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChannelStatementId(LocalStatementId);

impl ChannelStatementId {
    pub fn upcast(self) -> LocalStatementId {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkipStatementId(StatementId);

impl SkipStatementId {
    pub fn upcast(self) -> StatementId {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LabeledStatementId(StatementId);

impl LabeledStatementId {
    pub fn upcast(self) -> StatementId {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IfStatementId(StatementId);

impl IfStatementId {
    pub fn upcast(self) -> StatementId {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EndIfStatementId(StatementId);

impl EndIfStatementId {
    pub fn upcast(self) -> StatementId {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WhileStatementId(StatementId);

impl WhileStatementId {
    pub fn upcast(self) -> StatementId {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EndWhileStatementId(StatementId);

impl EndWhileStatementId {
    pub fn upcast(self) -> StatementId {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BreakStatementId(StatementId);

impl BreakStatementId {
    pub fn upcast(self) -> StatementId {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContinueStatementId(StatementId);

impl ContinueStatementId {
    pub fn upcast(self) -> StatementId {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SynchronousStatementId(StatementId);

impl SynchronousStatementId {
    pub fn upcast(self) -> StatementId {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EndSynchronousStatementId(StatementId);

impl EndSynchronousStatementId {
    pub fn upcast(self) -> StatementId {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReturnStatementId(StatementId);

impl ReturnStatementId {
    pub fn upcast(self) -> StatementId {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AssertStatementId(StatementId);

impl AssertStatementId {
    pub fn upcast(self) -> StatementId {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GotoStatementId(StatementId);

impl GotoStatementId {
    pub fn upcast(self) -> StatementId {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NewStatementId(StatementId);

impl NewStatementId {
    pub fn upcast(self) -> StatementId {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PutStatementId(StatementId);

impl PutStatementId {
    pub fn upcast(self) -> StatementId {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExpressionStatementId(StatementId);

impl ExpressionStatementId {
    pub fn upcast(self) -> StatementId {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExpressionId(Id<Expression>);

#[derive(Debug, Clone, Copy)]
pub struct AssignmentExpressionId(ExpressionId);

impl AssignmentExpressionId {
    pub fn upcast(self) -> ExpressionId {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConditionalExpressionId(ExpressionId);

impl ConditionalExpressionId {
    pub fn upcast(self) -> ExpressionId {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BinaryExpressionId(ExpressionId);

impl BinaryExpressionId {
    pub fn upcast(self) -> ExpressionId {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UnaryExpressionId(ExpressionId);

impl UnaryExpressionId {
    pub fn upcast(self) -> ExpressionId {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IndexingExpressionId(ExpressionId);

impl IndexingExpressionId {
    pub fn upcast(self) -> ExpressionId {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SlicingExpressionId(ExpressionId);

impl SlicingExpressionId {
    pub fn upcast(self) -> ExpressionId {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SelectExpressionId(ExpressionId);

impl SelectExpressionId {
    pub fn upcast(self) -> ExpressionId {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArrayExpressionId(ExpressionId);

impl ArrayExpressionId {
    pub fn upcast(self) -> ExpressionId {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConstantExpressionId(ExpressionId);

impl ConstantExpressionId {
    pub fn upcast(self) -> ExpressionId {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CallExpressionId(ExpressionId);

impl CallExpressionId {
    pub fn upcast(self) -> ExpressionId {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VariableExpressionId(ExpressionId);

impl VariableExpressionId {
    pub fn upcast(self) -> ExpressionId {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DeclarationId(Id<Declaration>);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DefinedDeclarationId(DeclarationId);

impl DefinedDeclarationId {
    pub fn upcast(self) -> DeclarationId {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImportedDeclarationId(DeclarationId);

impl ImportedDeclarationId {
    pub fn upcast(self) -> DeclarationId {
        self.0
    }
}

pub struct Heap {
    // Phase 0: allocation
    protocol_descriptions: Arena<Root>,
    pragmas: Arena<Pragma>,
    imports: Arena<Import>,
    identifiers: Arena<Identifier>,
    type_annotations: Arena<TypeAnnotation>,
    variables: Arena<Variable>,
    definitions: Arena<Definition>,
    statements: Arena<Statement>,
    expressions: Arena<Expression>,
    declarations: Arena<Declaration>,
}

impl Heap {
    pub fn new() -> Heap {
        Heap {
            protocol_descriptions: Arena::new(),
            pragmas: Arena::new(),
            imports: Arena::new(),
            identifiers: Arena::new(),
            type_annotations: Arena::new(),
            variables: Arena::new(),
            definitions: Arena::new(),
            statements: Arena::new(),
            expressions: Arena::new(),
            declarations: Arena::new(),
        }
    }
    pub fn alloc_source_identifier(
        &mut self,
        f: impl FnOnce(SourceIdentifierId) -> SourceIdentifier,
    ) -> SourceIdentifierId {
        SourceIdentifierId(IdentifierId(
            self.identifiers
                .alloc_with_id(|id| Identifier::Source(f(SourceIdentifierId(IdentifierId(id))))),
        ))
    }
    pub fn alloc_external_identifier(
        &mut self,
        f: impl FnOnce(ExternalIdentifierId) -> ExternalIdentifier,
    ) -> ExternalIdentifierId {
        ExternalIdentifierId(IdentifierId(
            self.identifiers.alloc_with_id(|id| {
                Identifier::External(f(ExternalIdentifierId(IdentifierId(id))))
            }),
        ))
    }
    pub fn alloc_type_annotation(
        &mut self,
        f: impl FnOnce(TypeAnnotationId) -> TypeAnnotation,
    ) -> TypeAnnotationId {
        TypeAnnotationId(self.type_annotations.alloc_with_id(|id| f(TypeAnnotationId(id))))
    }
    pub fn alloc_parameter(&mut self, f: impl FnOnce(ParameterId) -> Parameter) -> ParameterId {
        ParameterId(VariableId(
            self.variables.alloc_with_id(|id| Variable::Parameter(f(ParameterId(VariableId(id))))),
        ))
    }
    pub fn alloc_local(&mut self, f: impl FnOnce(LocalId) -> Local) -> LocalId {
        LocalId(VariableId(
            self.variables.alloc_with_id(|id| Variable::Local(f(LocalId(VariableId(id))))),
        ))
    }
    pub fn alloc_assignment_expression(
        &mut self,
        f: impl FnOnce(AssignmentExpressionId) -> AssignmentExpression,
    ) -> AssignmentExpressionId {
        AssignmentExpressionId(ExpressionId(self.expressions.alloc_with_id(|id| {
            Expression::Assignment(f(AssignmentExpressionId(ExpressionId(id))))
        })))
    }
    pub fn alloc_conditional_expression(
        &mut self,
        f: impl FnOnce(ConditionalExpressionId) -> ConditionalExpression,
    ) -> ConditionalExpressionId {
        ConditionalExpressionId(ExpressionId(self.expressions.alloc_with_id(|id| {
            Expression::Conditional(f(ConditionalExpressionId(ExpressionId(id))))
        })))
    }
    pub fn alloc_binary_expression(
        &mut self,
        f: impl FnOnce(BinaryExpressionId) -> BinaryExpression,
    ) -> BinaryExpressionId {
        BinaryExpressionId(ExpressionId(
            self.expressions
                .alloc_with_id(|id| Expression::Binary(f(BinaryExpressionId(ExpressionId(id))))),
        ))
    }
    pub fn alloc_unary_expression(
        &mut self,
        f: impl FnOnce(UnaryExpressionId) -> UnaryExpression,
    ) -> UnaryExpressionId {
        UnaryExpressionId(ExpressionId(
            self.expressions
                .alloc_with_id(|id| Expression::Unary(f(UnaryExpressionId(ExpressionId(id))))),
        ))
    }
    pub fn alloc_slicing_expression(
        &mut self,
        f: impl FnOnce(SlicingExpressionId) -> SlicingExpression,
    ) -> SlicingExpressionId {
        SlicingExpressionId(ExpressionId(
            self.expressions
                .alloc_with_id(|id| Expression::Slicing(f(SlicingExpressionId(ExpressionId(id))))),
        ))
    }
    pub fn alloc_indexing_expression(
        &mut self,
        f: impl FnOnce(IndexingExpressionId) -> IndexingExpression,
    ) -> IndexingExpressionId {
        IndexingExpressionId(ExpressionId(
            self.expressions.alloc_with_id(|id| {
                Expression::Indexing(f(IndexingExpressionId(ExpressionId(id))))
            }),
        ))
    }
    pub fn alloc_select_expression(
        &mut self,
        f: impl FnOnce(SelectExpressionId) -> SelectExpression,
    ) -> SelectExpressionId {
        SelectExpressionId(ExpressionId(
            self.expressions
                .alloc_with_id(|id| Expression::Select(f(SelectExpressionId(ExpressionId(id))))),
        ))
    }
    pub fn alloc_array_expression(
        &mut self,
        f: impl FnOnce(ArrayExpressionId) -> ArrayExpression,
    ) -> ArrayExpressionId {
        ArrayExpressionId(ExpressionId(
            self.expressions
                .alloc_with_id(|id| Expression::Array(f(ArrayExpressionId(ExpressionId(id))))),
        ))
    }
    pub fn alloc_constant_expression(
        &mut self,
        f: impl FnOnce(ConstantExpressionId) -> ConstantExpression,
    ) -> ConstantExpressionId {
        ConstantExpressionId(ExpressionId(
            self.expressions.alloc_with_id(|id| {
                Expression::Constant(f(ConstantExpressionId(ExpressionId(id))))
            }),
        ))
    }
    pub fn alloc_call_expression(
        &mut self,
        f: impl FnOnce(CallExpressionId) -> CallExpression,
    ) -> CallExpressionId {
        CallExpressionId(ExpressionId(
            self.expressions
                .alloc_with_id(|id| Expression::Call(f(CallExpressionId(ExpressionId(id))))),
        ))
    }
    pub fn alloc_variable_expression(
        &mut self,
        f: impl FnOnce(VariableExpressionId) -> VariableExpression,
    ) -> VariableExpressionId {
        VariableExpressionId(ExpressionId(
            self.expressions.alloc_with_id(|id| {
                Expression::Variable(f(VariableExpressionId(ExpressionId(id))))
            }),
        ))
    }
    pub fn alloc_block_statement(
        &mut self,
        f: impl FnOnce(BlockStatementId) -> BlockStatement,
    ) -> BlockStatementId {
        BlockStatementId(StatementId(
            self.statements
                .alloc_with_id(|id| Statement::Block(f(BlockStatementId(StatementId(id))))),
        ))
    }
    pub fn alloc_memory_statement(
        &mut self,
        f: impl FnOnce(MemoryStatementId) -> MemoryStatement,
    ) -> MemoryStatementId {
        MemoryStatementId(LocalStatementId(StatementId(self.statements.alloc_with_id(|id| {
            Statement::Local(LocalStatement::Memory(f(MemoryStatementId(LocalStatementId(
                StatementId(id),
            )))))
        }))))
    }
    pub fn alloc_channel_statement(
        &mut self,
        f: impl FnOnce(ChannelStatementId) -> ChannelStatement,
    ) -> ChannelStatementId {
        ChannelStatementId(LocalStatementId(StatementId(self.statements.alloc_with_id(|id| {
            Statement::Local(LocalStatement::Channel(f(ChannelStatementId(LocalStatementId(
                StatementId(id),
            )))))
        }))))
    }
    pub fn alloc_skip_statement(
        &mut self,
        f: impl FnOnce(SkipStatementId) -> SkipStatement,
    ) -> SkipStatementId {
        SkipStatementId(StatementId(
            self.statements
                .alloc_with_id(|id| Statement::Skip(f(SkipStatementId(StatementId(id))))),
        ))
    }
    pub fn alloc_if_statement(
        &mut self,
        f: impl FnOnce(IfStatementId) -> IfStatement,
    ) -> IfStatementId {
        IfStatementId(StatementId(
            self.statements.alloc_with_id(|id| Statement::If(f(IfStatementId(StatementId(id))))),
        ))
    }
    pub fn alloc_end_if_statement(
        &mut self,
        f: impl FnOnce(EndIfStatementId) -> EndIfStatement,
    ) -> EndIfStatementId {
        EndIfStatementId(StatementId(
            self.statements
                .alloc_with_id(|id| Statement::EndIf(f(EndIfStatementId(StatementId(id))))),
        ))
    }
    pub fn alloc_while_statement(
        &mut self,
        f: impl FnOnce(WhileStatementId) -> WhileStatement,
    ) -> WhileStatementId {
        WhileStatementId(StatementId(
            self.statements
                .alloc_with_id(|id| Statement::While(f(WhileStatementId(StatementId(id))))),
        ))
    }
    pub fn alloc_end_while_statement(
        &mut self,
        f: impl FnOnce(EndWhileStatementId) -> EndWhileStatement,
    ) -> EndWhileStatementId {
        EndWhileStatementId(StatementId(
            self.statements
                .alloc_with_id(|id| Statement::EndWhile(f(EndWhileStatementId(StatementId(id))))),
        ))
    }
    pub fn alloc_break_statement(
        &mut self,
        f: impl FnOnce(BreakStatementId) -> BreakStatement,
    ) -> BreakStatementId {
        BreakStatementId(StatementId(
            self.statements
                .alloc_with_id(|id| Statement::Break(f(BreakStatementId(StatementId(id))))),
        ))
    }
    pub fn alloc_continue_statement(
        &mut self,
        f: impl FnOnce(ContinueStatementId) -> ContinueStatement,
    ) -> ContinueStatementId {
        ContinueStatementId(StatementId(
            self.statements
                .alloc_with_id(|id| Statement::Continue(f(ContinueStatementId(StatementId(id))))),
        ))
    }
    pub fn alloc_synchronous_statement(
        &mut self,
        f: impl FnOnce(SynchronousStatementId) -> SynchronousStatement,
    ) -> SynchronousStatementId {
        SynchronousStatementId(StatementId(self.statements.alloc_with_id(|id| {
            Statement::Synchronous(f(SynchronousStatementId(StatementId(id))))
        })))
    }
    pub fn alloc_end_synchronous_statement(
        &mut self,
        f: impl FnOnce(EndSynchronousStatementId) -> EndSynchronousStatement,
    ) -> EndSynchronousStatementId {
        EndSynchronousStatementId(StatementId(self.statements.alloc_with_id(|id| {
            Statement::EndSynchronous(f(EndSynchronousStatementId(StatementId(id))))
        })))
    }
    pub fn alloc_return_statement(
        &mut self,
        f: impl FnOnce(ReturnStatementId) -> ReturnStatement,
    ) -> ReturnStatementId {
        ReturnStatementId(StatementId(
            self.statements
                .alloc_with_id(|id| Statement::Return(f(ReturnStatementId(StatementId(id))))),
        ))
    }
    pub fn alloc_assert_statement(
        &mut self,
        f: impl FnOnce(AssertStatementId) -> AssertStatement,
    ) -> AssertStatementId {
        AssertStatementId(StatementId(
            self.statements
                .alloc_with_id(|id| Statement::Assert(f(AssertStatementId(StatementId(id))))),
        ))
    }
    pub fn alloc_goto_statement(
        &mut self,
        f: impl FnOnce(GotoStatementId) -> GotoStatement,
    ) -> GotoStatementId {
        GotoStatementId(StatementId(
            self.statements
                .alloc_with_id(|id| Statement::Goto(f(GotoStatementId(StatementId(id))))),
        ))
    }
    pub fn alloc_new_statement(
        &mut self,
        f: impl FnOnce(NewStatementId) -> NewStatement,
    ) -> NewStatementId {
        NewStatementId(StatementId(
            self.statements.alloc_with_id(|id| Statement::New(f(NewStatementId(StatementId(id))))),
        ))
    }
    pub fn alloc_put_statement(
        &mut self,
        f: impl FnOnce(PutStatementId) -> PutStatement,
    ) -> PutStatementId {
        PutStatementId(StatementId(
            self.statements.alloc_with_id(|id| Statement::Put(f(PutStatementId(StatementId(id))))),
        ))
    }
    pub fn alloc_labeled_statement(
        &mut self,
        f: impl FnOnce(LabeledStatementId) -> LabeledStatement,
    ) -> LabeledStatementId {
        LabeledStatementId(StatementId(
            self.statements
                .alloc_with_id(|id| Statement::Labeled(f(LabeledStatementId(StatementId(id))))),
        ))
    }
    pub fn alloc_expression_statement(
        &mut self,
        f: impl FnOnce(ExpressionStatementId) -> ExpressionStatement,
    ) -> ExpressionStatementId {
        ExpressionStatementId(StatementId(
            self.statements.alloc_with_id(|id| {
                Statement::Expression(f(ExpressionStatementId(StatementId(id))))
            }),
        ))
    }
    pub fn alloc_composite(&mut self, f: impl FnOnce(CompositeId) -> Composite) -> CompositeId {
        CompositeId(ComponentId(DefinitionId(self.definitions.alloc_with_id(|id| {
            Definition::Component(Component::Composite(f(CompositeId(ComponentId(DefinitionId(
                id,
            ))))))
        }))))
    }
    pub fn alloc_primitive(&mut self, f: impl FnOnce(PrimitiveId) -> Primitive) -> PrimitiveId {
        PrimitiveId(ComponentId(DefinitionId(self.definitions.alloc_with_id(|id| {
            Definition::Component(Component::Primitive(f(PrimitiveId(ComponentId(DefinitionId(
                id,
            ))))))
        }))))
    }
    pub fn alloc_function(&mut self, f: impl FnOnce(FunctionId) -> Function) -> FunctionId {
        FunctionId(DefinitionId(
            self.definitions
                .alloc_with_id(|id| Definition::Function(f(FunctionId(DefinitionId(id))))),
        ))
    }
    pub fn alloc_pragma(&mut self, f: impl FnOnce(PragmaId) -> Pragma) -> PragmaId {
        PragmaId(self.pragmas.alloc_with_id(|id| f(PragmaId(id))))
    }
    pub fn alloc_import(&mut self, f: impl FnOnce(ImportId) -> Import) -> ImportId {
        ImportId(self.imports.alloc_with_id(|id| f(ImportId(id))))
    }
    pub fn alloc_protocol_description(&mut self, f: impl FnOnce(RootId) -> Root) -> RootId {
        RootId(self.protocol_descriptions.alloc_with_id(|id| f(RootId(id))))
    }
    pub fn alloc_imported_declaration(
        &mut self,
        f: impl FnOnce(ImportedDeclarationId) -> ImportedDeclaration,
    ) -> ImportedDeclarationId {
        ImportedDeclarationId(DeclarationId(self.declarations.alloc_with_id(|id| {
            Declaration::Imported(f(ImportedDeclarationId(DeclarationId(id))))
        })))
    }
    pub fn alloc_defined_declaration(
        &mut self,
        f: impl FnOnce(DefinedDeclarationId) -> DefinedDeclaration,
    ) -> DefinedDeclarationId {
        DefinedDeclarationId(DeclarationId(
            self.declarations.alloc_with_id(|id| {
                Declaration::Defined(f(DefinedDeclarationId(DeclarationId(id))))
            }),
        ))
    }

    pub fn get_external_identifier(&mut self, ident: &[u8]) -> ExternalIdentifierId {
        for (_, id) in self.identifiers.iter() {
            if id.is_external() && id.ident() == ident {
                return id.as_external().this;
            }
        }
        // Not found
        self.alloc_external_identifier(|this| ExternalIdentifier { this, value: ident.to_vec() })
    }
}

impl Index<RootId> for Heap {
    type Output = Root;
    fn index(&self, index: RootId) -> &Self::Output {
        &self.protocol_descriptions[index.0]
    }
}

impl IndexMut<RootId> for Heap {
    fn index_mut(&mut self, index: RootId) -> &mut Self::Output {
        &mut self.protocol_descriptions[index.0]
    }
}

impl Index<PragmaId> for Heap {
    type Output = Pragma;
    fn index(&self, index: PragmaId) -> &Self::Output {
        &self.pragmas[index.0]
    }
}

impl Index<ImportId> for Heap {
    type Output = Import;
    fn index(&self, index: ImportId) -> &Self::Output {
        &self.imports[index.0]
    }
}

impl Index<IdentifierId> for Heap {
    type Output = Identifier;
    fn index(&self, index: IdentifierId) -> &Self::Output {
        &self.identifiers[index.0]
    }
}

impl Index<SourceIdentifierId> for Heap {
    type Output = SourceIdentifier;
    fn index(&self, index: SourceIdentifierId) -> &Self::Output {
        &self.identifiers[(index.0).0].as_source()
    }
}

impl Index<ExternalIdentifierId> for Heap {
    type Output = ExternalIdentifier;
    fn index(&self, index: ExternalIdentifierId) -> &Self::Output {
        &self.identifiers[(index.0).0].as_external()
    }
}

impl Index<TypeAnnotationId> for Heap {
    type Output = TypeAnnotation;
    fn index(&self, index: TypeAnnotationId) -> &Self::Output {
        &self.type_annotations[index.0]
    }
}

impl Index<VariableId> for Heap {
    type Output = Variable;
    fn index(&self, index: VariableId) -> &Self::Output {
        &self.variables[index.0]
    }
}

impl Index<ParameterId> for Heap {
    type Output = Parameter;
    fn index(&self, index: ParameterId) -> &Self::Output {
        &self.variables[(index.0).0].as_parameter()
    }
}

impl Index<LocalId> for Heap {
    type Output = Local;
    fn index(&self, index: LocalId) -> &Self::Output {
        &self.variables[(index.0).0].as_local()
    }
}

impl Index<DefinitionId> for Heap {
    type Output = Definition;
    fn index(&self, index: DefinitionId) -> &Self::Output {
        &self.definitions[index.0]
    }
}

impl Index<ComponentId> for Heap {
    type Output = Component;
    fn index(&self, index: ComponentId) -> &Self::Output {
        &self.definitions[(index.0).0].as_component()
    }
}

impl Index<FunctionId> for Heap {
    type Output = Function;
    fn index(&self, index: FunctionId) -> &Self::Output {
        &self.definitions[(index.0).0].as_function()
    }
}

impl Index<CompositeId> for Heap {
    type Output = Composite;
    fn index(&self, index: CompositeId) -> &Self::Output {
        &self.definitions[((index.0).0).0].as_composite()
    }
}

impl Index<PrimitiveId> for Heap {
    type Output = Primitive;
    fn index(&self, index: PrimitiveId) -> &Self::Output {
        &self.definitions[((index.0).0).0].as_primitive()
    }
}

impl Index<StatementId> for Heap {
    type Output = Statement;
    fn index(&self, index: StatementId) -> &Self::Output {
        &self.statements[index.0]
    }
}

impl IndexMut<StatementId> for Heap {
    fn index_mut(&mut self, index: StatementId) -> &mut Self::Output {
        &mut self.statements[index.0]
    }
}

impl Index<BlockStatementId> for Heap {
    type Output = BlockStatement;
    fn index(&self, index: BlockStatementId) -> &Self::Output {
        &self.statements[(index.0).0].as_block()
    }
}

impl IndexMut<BlockStatementId> for Heap {
    fn index_mut(&mut self, index: BlockStatementId) -> &mut Self::Output {
        (&mut self.statements[(index.0).0]).as_block_mut()
    }
}

impl Index<LocalStatementId> for Heap {
    type Output = LocalStatement;
    fn index(&self, index: LocalStatementId) -> &Self::Output {
        &self.statements[(index.0).0].as_local()
    }
}

impl Index<MemoryStatementId> for Heap {
    type Output = MemoryStatement;
    fn index(&self, index: MemoryStatementId) -> &Self::Output {
        &self.statements[((index.0).0).0].as_memory()
    }
}

impl Index<ChannelStatementId> for Heap {
    type Output = ChannelStatement;
    fn index(&self, index: ChannelStatementId) -> &Self::Output {
        &self.statements[((index.0).0).0].as_channel()
    }
}

impl Index<SkipStatementId> for Heap {
    type Output = SkipStatement;
    fn index(&self, index: SkipStatementId) -> &Self::Output {
        &self.statements[(index.0).0].as_skip()
    }
}

impl Index<LabeledStatementId> for Heap {
    type Output = LabeledStatement;
    fn index(&self, index: LabeledStatementId) -> &Self::Output {
        &self.statements[(index.0).0].as_labeled()
    }
}

impl IndexMut<LabeledStatementId> for Heap {
    fn index_mut(&mut self, index: LabeledStatementId) -> &mut Self::Output {
        (&mut self.statements[(index.0).0]).as_labeled_mut()
    }
}

impl Index<IfStatementId> for Heap {
    type Output = IfStatement;
    fn index(&self, index: IfStatementId) -> &Self::Output {
        &self.statements[(index.0).0].as_if()
    }
}

impl Index<EndIfStatementId> for Heap {
    type Output = EndIfStatement;
    fn index(&self, index: EndIfStatementId) -> &Self::Output {
        &self.statements[(index.0).0].as_end_if()
    }
}

impl Index<WhileStatementId> for Heap {
    type Output = WhileStatement;
    fn index(&self, index: WhileStatementId) -> &Self::Output {
        &self.statements[(index.0).0].as_while()
    }
}

impl IndexMut<WhileStatementId> for Heap {
    fn index_mut(&mut self, index: WhileStatementId) -> &mut Self::Output {
        (&mut self.statements[(index.0).0]).as_while_mut()
    }
}

impl Index<BreakStatementId> for Heap {
    type Output = BreakStatement;
    fn index(&self, index: BreakStatementId) -> &Self::Output {
        &self.statements[(index.0).0].as_break()
    }
}

impl IndexMut<BreakStatementId> for Heap {
    fn index_mut(&mut self, index: BreakStatementId) -> &mut Self::Output {
        (&mut self.statements[(index.0).0]).as_break_mut()
    }
}

impl Index<ContinueStatementId> for Heap {
    type Output = ContinueStatement;
    fn index(&self, index: ContinueStatementId) -> &Self::Output {
        &self.statements[(index.0).0].as_continue()
    }
}

impl IndexMut<ContinueStatementId> for Heap {
    fn index_mut(&mut self, index: ContinueStatementId) -> &mut Self::Output {
        (&mut self.statements[(index.0).0]).as_continue_mut()
    }
}

impl Index<SynchronousStatementId> for Heap {
    type Output = SynchronousStatement;
    fn index(&self, index: SynchronousStatementId) -> &Self::Output {
        &self.statements[(index.0).0].as_synchronous()
    }
}

impl IndexMut<SynchronousStatementId> for Heap {
    fn index_mut(&mut self, index: SynchronousStatementId) -> &mut Self::Output {
        (&mut self.statements[(index.0).0]).as_synchronous_mut()
    }
}

impl Index<EndSynchronousStatementId> for Heap {
    type Output = EndSynchronousStatement;
    fn index(&self, index: EndSynchronousStatementId) -> &Self::Output {
        &self.statements[(index.0).0].as_end_synchronous()
    }
}

impl Index<ReturnStatementId> for Heap {
    type Output = ReturnStatement;
    fn index(&self, index: ReturnStatementId) -> &Self::Output {
        &self.statements[(index.0).0].as_return()
    }
}

impl Index<AssertStatementId> for Heap {
    type Output = AssertStatement;
    fn index(&self, index: AssertStatementId) -> &Self::Output {
        &self.statements[(index.0).0].as_assert()
    }
}

impl Index<GotoStatementId> for Heap {
    type Output = GotoStatement;
    fn index(&self, index: GotoStatementId) -> &Self::Output {
        &self.statements[(index.0).0].as_goto()
    }
}

impl IndexMut<GotoStatementId> for Heap {
    fn index_mut(&mut self, index: GotoStatementId) -> &mut Self::Output {
        (&mut self.statements[(index.0).0]).as_goto_mut()
    }
}

impl Index<NewStatementId> for Heap {
    type Output = NewStatement;
    fn index(&self, index: NewStatementId) -> &Self::Output {
        &self.statements[(index.0).0].as_new()
    }
}

impl Index<PutStatementId> for Heap {
    type Output = PutStatement;
    fn index(&self, index: PutStatementId) -> &Self::Output {
        &self.statements[(index.0).0].as_put()
    }
}

impl Index<ExpressionStatementId> for Heap {
    type Output = ExpressionStatement;
    fn index(&self, index: ExpressionStatementId) -> &Self::Output {
        &self.statements[(index.0).0].as_expression()
    }
}

impl Index<ExpressionId> for Heap {
    type Output = Expression;
    fn index(&self, index: ExpressionId) -> &Self::Output {
        &self.expressions[index.0]
    }
}

impl Index<AssignmentExpressionId> for Heap {
    type Output = AssignmentExpression;
    fn index(&self, index: AssignmentExpressionId) -> &Self::Output {
        &self.expressions[(index.0).0].as_assignment()
    }
}

impl Index<ConditionalExpressionId> for Heap {
    type Output = ConditionalExpression;
    fn index(&self, index: ConditionalExpressionId) -> &Self::Output {
        &self.expressions[(index.0).0].as_conditional()
    }
}

impl Index<BinaryExpressionId> for Heap {
    type Output = BinaryExpression;
    fn index(&self, index: BinaryExpressionId) -> &Self::Output {
        &self.expressions[(index.0).0].as_binary()
    }
}

impl Index<UnaryExpressionId> for Heap {
    type Output = UnaryExpression;
    fn index(&self, index: UnaryExpressionId) -> &Self::Output {
        &self.expressions[(index.0).0].as_unary()
    }
}

impl Index<IndexingExpressionId> for Heap {
    type Output = IndexingExpression;
    fn index(&self, index: IndexingExpressionId) -> &Self::Output {
        &self.expressions[(index.0).0].as_indexing()
    }
}

impl Index<SlicingExpressionId> for Heap {
    type Output = SlicingExpression;
    fn index(&self, index: SlicingExpressionId) -> &Self::Output {
        &self.expressions[(index.0).0].as_slicing()
    }
}

impl Index<SelectExpressionId> for Heap {
    type Output = SelectExpression;
    fn index(&self, index: SelectExpressionId) -> &Self::Output {
        &self.expressions[(index.0).0].as_select()
    }
}

impl Index<ArrayExpressionId> for Heap {
    type Output = ArrayExpression;
    fn index(&self, index: ArrayExpressionId) -> &Self::Output {
        &self.expressions[(index.0).0].as_array()
    }
}

impl Index<ConstantExpressionId> for Heap {
    type Output = ConstantExpression;
    fn index(&self, index: ConstantExpressionId) -> &Self::Output {
        &self.expressions[(index.0).0].as_constant()
    }
}

impl Index<CallExpressionId> for Heap {
    type Output = CallExpression;
    fn index(&self, index: CallExpressionId) -> &Self::Output {
        &self.expressions[(index.0).0].as_call()
    }
}

impl IndexMut<CallExpressionId> for Heap {
    fn index_mut(&mut self, index: CallExpressionId) -> &mut Self::Output {
        (&mut self.expressions[(index.0).0]).as_call_mut()
    }
}

impl Index<VariableExpressionId> for Heap {
    type Output = VariableExpression;
    fn index(&self, index: VariableExpressionId) -> &Self::Output {
        &self.expressions[(index.0).0].as_variable()
    }
}

impl IndexMut<VariableExpressionId> for Heap {
    fn index_mut(&mut self, index: VariableExpressionId) -> &mut Self::Output {
        (&mut self.expressions[(index.0).0]).as_variable_mut()
    }
}

impl Index<DeclarationId> for Heap {
    type Output = Declaration;
    fn index(&self, index: DeclarationId) -> &Self::Output {
        &self.declarations[index.0]
    }
}

#[derive(Debug, Clone)]
pub struct Root {
    pub this: RootId,
    // Phase 1: parser
    pub position: InputPosition,
    pub pragmas: Vec<PragmaId>,
    pub imports: Vec<ImportId>,
    pub definitions: Vec<DefinitionId>,
    // Pase 2: linker
    pub declarations: Vec<DeclarationId>,
}

impl Root {
    pub fn get_definition(&self, h: &Heap, id: IdentifierId) -> Option<DefinitionId> {
        for &def in self.definitions.iter() {
            if h[h[def].identifier()] == h[id] {
                return Some(def);
            }
        }
        None
    }
    pub fn get_definition_ident(&self, h: &Heap, id: &[u8]) -> Option<DefinitionId> {
        for &def in self.definitions.iter() {
            if h[h[def].identifier()].ident() == id {
                return Some(def);
            }
        }
        None
    }
    pub fn get_declaration(&self, h: &Heap, id: IdentifierId) -> Option<DeclarationId> {
        for &decl in self.declarations.iter() {
            if h[h[decl].identifier()] == h[id] {
                return Some(decl);
            }
        }
        None
    }
}

impl SyntaxElement for Root {
    fn position(&self) -> InputPosition {
        self.position
    }
}

#[derive(Debug, Clone)]
pub struct Pragma {
    pub this: PragmaId,
    // Phase 1: parser
    pub position: InputPosition,
    pub value: Vec<u8>,
}

impl SyntaxElement for Pragma {
    fn position(&self) -> InputPosition {
        self.position
    }
}

#[derive(Debug, Clone)]
pub struct Import {
    pub this: ImportId,
    // Phase 1: parser
    pub position: InputPosition,
    pub value: Vec<u8>,
}

impl SyntaxElement for Import {
    fn position(&self) -> InputPosition {
        self.position
    }
}

#[derive(Debug, Clone)]
pub enum Identifier {
    External(ExternalIdentifier),
    Source(SourceIdentifier),
}

impl Identifier {
    pub fn as_source(&self) -> &SourceIdentifier {
        match self {
            Identifier::Source(result) => result,
            _ => panic!("Unable to cast `Identifier` to `SourceIdentifier`"),
        }
    }
    pub fn is_external(&self) -> bool {
        match self {
            Identifier::External(_) => true,
            _ => false,
        }
    }
    pub fn as_external(&self) -> &ExternalIdentifier {
        match self {
            Identifier::External(result) => result,
            _ => panic!("Unable to cast `Identifier` to `ExternalIdentifier`"),
        }
    }
    fn ident(&self) -> &[u8] {
        match self {
            Identifier::External(eid) => eid.ident(),
            Identifier::Source(sid) => sid.ident(),
        }
    }
}

impl Display for Identifier {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // A source identifier is in ASCII range.
        write!(f, "{}", String::from_utf8_lossy(self.ident()))
    }
}

impl PartialEq<Identifier> for Identifier {
    fn eq(&self, rhs: &Identifier) -> bool {
        self.ident() == rhs.ident()
    }
}

impl PartialEq<SourceIdentifier> for Identifier {
    fn eq(&self, rhs: &SourceIdentifier) -> bool {
        self.ident() == rhs.ident()
    }
}

#[derive(Debug, Clone)]
pub struct ExternalIdentifier {
    pub this: ExternalIdentifierId,
    // Phase 1: parser
    pub value: Vec<u8>,
}

impl ExternalIdentifier {
    fn ident(&self) -> &[u8] {
        &self.value
    }
}

impl Display for ExternalIdentifier {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // A source identifier is in ASCII range.
        write!(f, "{}", String::from_utf8_lossy(&self.value))
    }
}

#[derive(Debug, Clone)]
pub struct SourceIdentifier {
    pub this: SourceIdentifierId,
    // Phase 1: parser
    pub position: InputPosition,
    pub value: Vec<u8>,
}

impl SourceIdentifier {
    fn ident(&self) -> &[u8] {
        &self.value
    }
}

impl SyntaxElement for SourceIdentifier {
    fn position(&self) -> InputPosition {
        self.position
    }
}

impl Display for SourceIdentifier {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // A source identifier is in ASCII range.
        write!(f, "{}", String::from_utf8_lossy(&self.value))
    }
}

impl PartialEq<Identifier> for SourceIdentifier {
    fn eq(&self, rhs: &Identifier) -> bool {
        self.ident() == rhs.ident()
    }
}

impl PartialEq<SourceIdentifier> for SourceIdentifier {
    fn eq(&self, rhs: &SourceIdentifier) -> bool {
        self.ident() == rhs.ident()
    }
}

type TypeData = Vec<u8>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrimitiveType {
    Input,
    Output,
    Message,
    Boolean,
    Byte,
    Short,
    Int,
    Long,
    Symbolic(TypeData),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Type {
    pub primitive: PrimitiveType,
    pub array: bool,
}

#[allow(dead_code)]
impl Type {
    pub const INPUT: Type = Type { primitive: PrimitiveType::Input, array: false };
    pub const OUTPUT: Type = Type { primitive: PrimitiveType::Output, array: false };
    pub const MESSAGE: Type = Type { primitive: PrimitiveType::Message, array: false };
    pub const BOOLEAN: Type = Type { primitive: PrimitiveType::Boolean, array: false };
    pub const BYTE: Type = Type { primitive: PrimitiveType::Byte, array: false };
    pub const SHORT: Type = Type { primitive: PrimitiveType::Short, array: false };
    pub const INT: Type = Type { primitive: PrimitiveType::Int, array: false };
    pub const LONG: Type = Type { primitive: PrimitiveType::Long, array: false };

    pub const INPUT_ARRAY: Type = Type { primitive: PrimitiveType::Input, array: true };
    pub const OUTPUT_ARRAY: Type = Type { primitive: PrimitiveType::Output, array: true };
    pub const MESSAGE_ARRAY: Type = Type { primitive: PrimitiveType::Message, array: true };
    pub const BOOLEAN_ARRAY: Type = Type { primitive: PrimitiveType::Boolean, array: true };
    pub const BYTE_ARRAY: Type = Type { primitive: PrimitiveType::Byte, array: true };
    pub const SHORT_ARRAY: Type = Type { primitive: PrimitiveType::Short, array: true };
    pub const INT_ARRAY: Type = Type { primitive: PrimitiveType::Int, array: true };
    pub const LONG_ARRAY: Type = Type { primitive: PrimitiveType::Long, array: true };
}

impl Display for Type {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.primitive {
            PrimitiveType::Input => {
                write!(f, "in")?;
            }
            PrimitiveType::Output => {
                write!(f, "out")?;
            }
            PrimitiveType::Message => {
                write!(f, "msg")?;
            }
            PrimitiveType::Boolean => {
                write!(f, "boolean")?;
            }
            PrimitiveType::Byte => {
                write!(f, "byte")?;
            }
            PrimitiveType::Short => {
                write!(f, "short")?;
            }
            PrimitiveType::Int => {
                write!(f, "int")?;
            }
            PrimitiveType::Long => {
                write!(f, "long")?;
            }
            PrimitiveType::Symbolic(data) => {
                // Type data is in ASCII range.
                write!(f, "{}", String::from_utf8_lossy(&data))?;
            }
        }
        if self.array {
            write!(f, "[]")
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Clone)]
pub struct TypeAnnotation {
    pub this: TypeAnnotationId,
    // Phase 1: parser
    pub position: InputPosition,
    pub the_type: Type,
}

impl SyntaxElement for TypeAnnotation {
    fn position(&self) -> InputPosition {
        self.position
    }
}

type CharacterData = Vec<u8>;
type IntegerData = Vec<u8>;

#[derive(Debug, Clone)]
pub enum Constant {
    Null, // message
    True,
    False,
    Character(CharacterData),
    Integer(IntegerData),
}

#[derive(Debug, Clone)]
pub enum Method {
    Get,
    Fires,
    Create,
    Symbolic(SourceIdentifierId),
}

#[derive(Debug, Clone)]
pub enum Field {
    Length,
    Symbolic(SourceIdentifierId),
}
impl Field {
    pub fn is_length(&self) -> bool {
        match self {
            Field::Length => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Scope {
    Definition(DefinitionId),
    Block(BlockStatementId),
    Synchronous(SynchronousStatementId),
}

impl Scope {
    pub fn to_block(&self) -> BlockStatementId {
        match &self {
            Scope::Block(id) => *id,
            _ => panic!("Unable to cast `Scope` to `BlockStatement`"),
        }
    }
}

pub trait VariableScope {
    fn parent_scope(&self, h: &Heap) -> Option<Scope>;
    fn get_variable(&self, h: &Heap, id: SourceIdentifierId) -> Option<VariableId>;
}

impl VariableScope for Scope {
    fn parent_scope(&self, h: &Heap) -> Option<Scope> {
        match self {
            Scope::Definition(def) => h[*def].parent_scope(h),
            Scope::Block(stmt) => h[*stmt].parent_scope(h),
            Scope::Synchronous(stmt) => h[*stmt].parent_scope(h),
        }
    }
    fn get_variable(&self, h: &Heap, id: SourceIdentifierId) -> Option<VariableId> {
        match self {
            Scope::Definition(def) => h[*def].get_variable(h, id),
            Scope::Block(stmt) => h[*stmt].get_variable(h, id),
            Scope::Synchronous(stmt) => h[*stmt].get_variable(h, id),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Variable {
    Parameter(Parameter),
    Local(Local),
}

impl Variable {
    pub fn identifier(&self) -> SourceIdentifierId {
        match self {
            Variable::Parameter(var) => var.identifier,
            Variable::Local(var) => var.identifier,
        }
    }
    pub fn is_parameter(&self) -> bool {
        match self {
            Variable::Parameter(_) => true,
            _ => false,
        }
    }
    pub fn as_parameter(&self) -> &Parameter {
        match self {
            Variable::Parameter(result) => result,
            _ => panic!("Unable to cast `Variable` to `Parameter`"),
        }
    }
    pub fn as_local(&self) -> &Local {
        match self {
            Variable::Local(result) => result,
            _ => panic!("Unable to cast `Variable` to `Local`"),
        }
    }
    pub fn the_type<'b>(&self, h: &'b Heap) -> &'b Type {
        match self {
            Variable::Parameter(param) => &h[param.type_annotation].the_type,
            Variable::Local(local) => &h[local.type_annotation].the_type,
        }
    }
}

impl SyntaxElement for Variable {
    fn position(&self) -> InputPosition {
        match self {
            Variable::Parameter(decl) => decl.position(),
            Variable::Local(decl) => decl.position(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Parameter {
    pub this: ParameterId,
    // Phase 1: parser
    pub position: InputPosition,
    pub type_annotation: TypeAnnotationId,
    pub identifier: SourceIdentifierId,
}

impl SyntaxElement for Parameter {
    fn position(&self) -> InputPosition {
        self.position
    }
}

#[derive(Debug, Clone)]
pub struct Local {
    pub this: LocalId,
    // Phase 1: parser
    pub position: InputPosition,
    pub type_annotation: TypeAnnotationId,
    pub identifier: SourceIdentifierId,
}
impl SyntaxElement for Local {
    fn position(&self) -> InputPosition {
        self.position
    }
}

#[derive(Debug, Clone)]
pub enum Definition {
    Component(Component),
    Function(Function),
}

impl Definition {
    pub fn is_component(&self) -> bool {
        match self {
            Definition::Component(_) => true,
            _ => false,
        }
    }
    pub fn as_component(&self) -> &Component {
        match self {
            Definition::Component(result) => result,
            _ => panic!("Unable to cast `Definition` to `Component`"),
        }
    }
    pub fn as_function(&self) -> &Function {
        match self {
            Definition::Function(result) => result,
            _ => panic!("Unable to cast `Definition` to `Function`"),
        }
    }
    pub fn as_composite(&self) -> &Composite {
        self.as_component().as_composite()
    }
    pub fn as_primitive(&self) -> &Primitive {
        self.as_component().as_primitive()
    }
    pub fn identifier(&self) -> SourceIdentifierId {
        match self {
            Definition::Component(com) => com.identifier(),
            Definition::Function(fun) => fun.identifier,
        }
    }
    pub fn parameters(&self) -> &Vec<ParameterId> {
        match self {
            Definition::Component(com) => com.parameters(),
            Definition::Function(fun) => &fun.parameters,
        }
    }
    pub fn body(&self) -> StatementId {
        match self {
            Definition::Component(com) => com.body(),
            Definition::Function(fun) => fun.body,
        }
    }
}

impl SyntaxElement for Definition {
    fn position(&self) -> InputPosition {
        match self {
            Definition::Component(def) => def.position(),
            Definition::Function(def) => def.position(),
        }
    }
}

impl VariableScope for Definition {
    fn parent_scope(&self, _h: &Heap) -> Option<Scope> {
        None
    }
    fn get_variable(&self, h: &Heap, id: SourceIdentifierId) -> Option<VariableId> {
        for &param in self.parameters().iter() {
            if h[h[param].identifier] == h[id] {
                return Some(param.0);
            }
        }
        None
    }
}

#[derive(Debug, Clone)]
pub enum Component {
    Composite(Composite),
    Primitive(Primitive),
}

impl Component {
    pub fn this(&self) -> ComponentId {
        match self {
            Component::Composite(com) => com.this.upcast(),
            Component::Primitive(prim) => prim.this.upcast(),
        }
    }
    pub fn as_composite(&self) -> &Composite {
        match self {
            Component::Composite(result) => result,
            _ => panic!("Unable to cast `Component` to `Composite`"),
        }
    }
    pub fn as_primitive(&self) -> &Primitive {
        match self {
            Component::Primitive(result) => result,
            _ => panic!("Unable to cast `Component` to `Primitive`"),
        }
    }
    fn identifier(&self) -> SourceIdentifierId {
        match self {
            Component::Composite(com) => com.identifier,
            Component::Primitive(prim) => prim.identifier,
        }
    }
    pub fn parameters(&self) -> &Vec<ParameterId> {
        match self {
            Component::Composite(com) => &com.parameters,
            Component::Primitive(prim) => &prim.parameters,
        }
    }
    pub fn body(&self) -> StatementId {
        match self {
            Component::Composite(com) => com.body,
            Component::Primitive(prim) => prim.body,
        }
    }
}

impl SyntaxElement for Component {
    fn position(&self) -> InputPosition {
        match self {
            Component::Composite(def) => def.position(),
            Component::Primitive(def) => def.position(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Composite {
    pub this: CompositeId,
    // Phase 1: parser
    pub position: InputPosition,
    pub identifier: SourceIdentifierId,
    pub parameters: Vec<ParameterId>,
    pub body: StatementId,
}

impl SyntaxElement for Composite {
    fn position(&self) -> InputPosition {
        self.position
    }
}

#[derive(Debug, Clone)]
pub struct Primitive {
    pub this: PrimitiveId,
    // Phase 1: parser
    pub position: InputPosition,
    pub identifier: SourceIdentifierId,
    pub parameters: Vec<ParameterId>,
    pub body: StatementId,
}

impl SyntaxElement for Primitive {
    fn position(&self) -> InputPosition {
        self.position
    }
}

#[derive(Debug, Clone)]
pub struct Function {
    pub this: FunctionId,
    // Phase 1: parser
    pub position: InputPosition,
    pub return_type: TypeAnnotationId,
    pub identifier: SourceIdentifierId,
    pub parameters: Vec<ParameterId>,
    pub body: StatementId,
}

impl SyntaxElement for Function {
    fn position(&self) -> InputPosition {
        self.position
    }
}

#[derive(Debug, Clone)]
pub enum Declaration {
    Defined(DefinedDeclaration),
    Imported(ImportedDeclaration),
}

impl Declaration {
    pub fn signature(&self) -> &Signature {
        match self {
            Declaration::Defined(decl) => &decl.signature,
            Declaration::Imported(decl) => &decl.signature,
        }
    }
    pub fn identifier(&self) -> IdentifierId {
        self.signature().identifier()
    }
    pub fn is_component(&self) -> bool {
        self.signature().is_component()
    }
    pub fn is_function(&self) -> bool {
        self.signature().is_function()
    }
}

#[derive(Debug, Clone)]
pub struct DefinedDeclaration {
    pub this: DefinedDeclarationId,
    // Phase 2: linker
    pub definition: DefinitionId,
    pub signature: Signature,
}

#[derive(Debug, Clone)]
pub struct ImportedDeclaration {
    pub this: ImportedDeclarationId,
    // Phase 2: linker
    pub import: ImportId,
    pub signature: Signature,
}

#[derive(Debug, Clone)]
pub enum Signature {
    Component(ComponentSignature),
    Function(FunctionSignature),
}

impl Signature {
    pub fn from_definition(h: &Heap, def: DefinitionId) -> Signature {
        match &h[def] {
            Definition::Component(com) => Signature::Component(ComponentSignature {
                identifier: com.identifier().0,
                arity: Signature::convert_parameters(h, com.parameters()),
            }),
            Definition::Function(fun) => Signature::Function(FunctionSignature {
                return_type: h[fun.return_type].the_type.clone(),
                identifier: fun.identifier.0,
                arity: Signature::convert_parameters(h, &fun.parameters),
            }),
        }
    }
    fn convert_parameters(h: &Heap, params: &Vec<ParameterId>) -> Vec<Type> {
        let mut result = Vec::new();
        for &param in params.iter() {
            result.push(h[h[param].type_annotation].the_type.clone());
        }
        result
    }
    fn identifier(&self) -> IdentifierId {
        match self {
            Signature::Component(com) => com.identifier,
            Signature::Function(fun) => fun.identifier,
        }
    }
    pub fn is_component(&self) -> bool {
        match self {
            Signature::Component(_) => true,
            Signature::Function(_) => false,
        }
    }
    pub fn is_function(&self) -> bool {
        match self {
            Signature::Component(_) => false,
            Signature::Function(_) => true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ComponentSignature {
    pub identifier: IdentifierId,
    pub arity: Vec<Type>,
}

#[derive(Debug, Clone)]
pub struct FunctionSignature {
    pub return_type: Type,
    pub identifier: IdentifierId,
    pub arity: Vec<Type>,
}

#[derive(Debug, Clone)]
pub enum Statement {
    Block(BlockStatement),
    Local(LocalStatement),
    Skip(SkipStatement),
    Labeled(LabeledStatement),
    If(IfStatement),
    EndIf(EndIfStatement),
    While(WhileStatement),
    EndWhile(EndWhileStatement),
    Break(BreakStatement),
    Continue(ContinueStatement),
    Synchronous(SynchronousStatement),
    EndSynchronous(EndSynchronousStatement),
    Return(ReturnStatement),
    Assert(AssertStatement),
    Goto(GotoStatement),
    New(NewStatement),
    Put(PutStatement),
    Expression(ExpressionStatement),
}

impl Statement {
    pub fn as_block(&self) -> &BlockStatement {
        match self {
            Statement::Block(result) => result,
            _ => panic!("Unable to cast `Statement` to `BlockStatement`"),
        }
    }
    pub fn as_block_mut(&mut self) -> &mut BlockStatement {
        match self {
            Statement::Block(result) => result,
            _ => panic!("Unable to cast `Statement` to `BlockStatement`"),
        }
    }
    pub fn as_local(&self) -> &LocalStatement {
        match self {
            Statement::Local(result) => result,
            _ => panic!("Unable to cast `Statement` to `LocalStatement`"),
        }
    }
    pub fn as_memory(&self) -> &MemoryStatement {
        self.as_local().as_memory()
    }
    pub fn as_channel(&self) -> &ChannelStatement {
        self.as_local().as_channel()
    }
    pub fn as_skip(&self) -> &SkipStatement {
        match self {
            Statement::Skip(result) => result,
            _ => panic!("Unable to cast `Statement` to `SkipStatement`"),
        }
    }
    pub fn as_labeled(&self) -> &LabeledStatement {
        match self {
            Statement::Labeled(result) => result,
            _ => panic!("Unable to cast `Statement` to `LabeledStatement`"),
        }
    }
    pub fn as_labeled_mut(&mut self) -> &mut LabeledStatement {
        match self {
            Statement::Labeled(result) => result,
            _ => panic!("Unable to cast `Statement` to `LabeledStatement`"),
        }
    }
    pub fn as_if(&self) -> &IfStatement {
        match self {
            Statement::If(result) => result,
            _ => panic!("Unable to cast `Statement` to `IfStatement`"),
        }
    }
    pub fn as_end_if(&self) -> &EndIfStatement {
        match self {
            Statement::EndIf(result) => result,
            _ => panic!("Unable to cast `Statement` to `EndIfStatement`"),
        }
    }
    pub fn is_while(&self) -> bool {
        match self {
            Statement::While(_) => true,
            _ => false,
        }
    }
    pub fn as_while(&self) -> &WhileStatement {
        match self {
            Statement::While(result) => result,
            _ => panic!("Unable to cast `Statement` to `WhileStatement`"),
        }
    }
    pub fn as_while_mut(&mut self) -> &mut WhileStatement {
        match self {
            Statement::While(result) => result,
            _ => panic!("Unable to cast `Statement` to `WhileStatement`"),
        }
    }
    pub fn as_end_while(&self) -> &EndWhileStatement {
        match self {
            Statement::EndWhile(result) => result,
            _ => panic!("Unable to cast `Statement` to `EndWhileStatement`"),
        }
    }
    pub fn as_break(&self) -> &BreakStatement {
        match self {
            Statement::Break(result) => result,
            _ => panic!("Unable to cast `Statement` to `BreakStatement`"),
        }
    }
    pub fn as_break_mut(&mut self) -> &mut BreakStatement {
        match self {
            Statement::Break(result) => result,
            _ => panic!("Unable to cast `Statement` to `BreakStatement`"),
        }
    }
    pub fn as_continue(&self) -> &ContinueStatement {
        match self {
            Statement::Continue(result) => result,
            _ => panic!("Unable to cast `Statement` to `ContinueStatement`"),
        }
    }
    pub fn as_continue_mut(&mut self) -> &mut ContinueStatement {
        match self {
            Statement::Continue(result) => result,
            _ => panic!("Unable to cast `Statement` to `ContinueStatement`"),
        }
    }
    pub fn as_synchronous(&self) -> &SynchronousStatement {
        match self {
            Statement::Synchronous(result) => result,
            _ => panic!("Unable to cast `Statement` to `SynchronousStatement`"),
        }
    }
    pub fn as_synchronous_mut(&mut self) -> &mut SynchronousStatement {
        match self {
            Statement::Synchronous(result) => result,
            _ => panic!("Unable to cast `Statement` to `SynchronousStatement`"),
        }
    }
    pub fn as_end_synchronous(&self) -> &EndSynchronousStatement {
        match self {
            Statement::EndSynchronous(result) => result,
            _ => panic!("Unable to cast `Statement` to `EndSynchronousStatement`"),
        }
    }
    pub fn as_return(&self) -> &ReturnStatement {
        match self {
            Statement::Return(result) => result,
            _ => panic!("Unable to cast `Statement` to `ReturnStatement`"),
        }
    }
    pub fn as_assert(&self) -> &AssertStatement {
        match self {
            Statement::Assert(result) => result,
            _ => panic!("Unable to cast `Statement` to `AssertStatement`"),
        }
    }
    pub fn as_goto(&self) -> &GotoStatement {
        match self {
            Statement::Goto(result) => result,
            _ => panic!("Unable to cast `Statement` to `GotoStatement`"),
        }
    }
    pub fn as_goto_mut(&mut self) -> &mut GotoStatement {
        match self {
            Statement::Goto(result) => result,
            _ => panic!("Unable to cast `Statement` to `GotoStatement`"),
        }
    }
    pub fn as_new(&self) -> &NewStatement {
        match self {
            Statement::New(result) => result,
            _ => panic!("Unable to cast `Statement` to `NewStatement`"),
        }
    }
    pub fn as_put(&self) -> &PutStatement {
        match self {
            Statement::Put(result) => result,
            _ => panic!("Unable to cast `Statement` to `PutStatement`"),
        }
    }
    pub fn as_expression(&self) -> &ExpressionStatement {
        match self {
            Statement::Expression(result) => result,
            _ => panic!("Unable to cast `Statement` to `ExpressionStatement`"),
        }
    }
    pub fn link_next(&mut self, next: StatementId) {
        match self {
            Statement::Block(stmt) => panic!(),
            Statement::Local(stmt) => match stmt {
                LocalStatement::Channel(stmt) => stmt.next = Some(next),
                LocalStatement::Memory(stmt) => stmt.next = Some(next),
            },
            Statement::Skip(stmt) => stmt.next = Some(next),
            Statement::Labeled(stmt) => panic!(),
            Statement::If(stmt) => panic!(),
            Statement::EndIf(stmt) => stmt.next = Some(next),
            Statement::While(stmt) => panic!(), // although while has a next field, it is linked manually
            Statement::EndWhile(stmt) => stmt.next = Some(next),
            Statement::Break(stmt) => panic!(),
            Statement::Continue(stmt) => panic!(),
            Statement::Synchronous(stmt) => panic!(),
            Statement::EndSynchronous(stmt) => stmt.next = Some(next),
            Statement::Return(stmt) => panic!(),
            Statement::Assert(stmt) => stmt.next = Some(next),
            Statement::Goto(stmt) => panic!(),
            Statement::New(stmt) => stmt.next = Some(next),
            Statement::Put(stmt) => stmt.next = Some(next),
            Statement::Expression(stmt) => stmt.next = Some(next),
        }
    }
}

impl SyntaxElement for Statement {
    fn position(&self) -> InputPosition {
        match self {
            Statement::Block(stmt) => stmt.position(),
            Statement::Local(stmt) => stmt.position(),
            Statement::Skip(stmt) => stmt.position(),
            Statement::Labeled(stmt) => stmt.position(),
            Statement::If(stmt) => stmt.position(),
            Statement::EndIf(stmt) => stmt.position(),
            Statement::While(stmt) => stmt.position(),
            Statement::EndWhile(stmt) => stmt.position(),
            Statement::Break(stmt) => stmt.position(),
            Statement::Continue(stmt) => stmt.position(),
            Statement::Synchronous(stmt) => stmt.position(),
            Statement::EndSynchronous(stmt) => stmt.position(),
            Statement::Return(stmt) => stmt.position(),
            Statement::Assert(stmt) => stmt.position(),
            Statement::Goto(stmt) => stmt.position(),
            Statement::New(stmt) => stmt.position(),
            Statement::Put(stmt) => stmt.position(),
            Statement::Expression(stmt) => stmt.position(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BlockStatement {
    pub this: BlockStatementId,
    // Phase 1: parser
    pub position: InputPosition,
    pub statements: Vec<StatementId>,
    // Phase 2: linker
    pub parent_scope: Option<Scope>,
    pub locals: Vec<LocalId>,
    pub labels: Vec<LabeledStatementId>,
}

impl BlockStatement {
    pub fn parent_block(&self, h: &Heap) -> Option<BlockStatementId> {
        let parent = self.parent_scope.unwrap();
        match parent {
            Scope::Definition(_) => {
                // If the parent scope is a definition, then there is no
                // parent block.
                None
            }
            Scope::Synchronous(parent) => {
                // It is always the case that when this function is called,
                // the parent of a synchronous statement is a block statement:
                // nested synchronous statements are flagged illegal,
                // and that happens before resolving variables that
                // creates the parent_scope references in the first place.
                Some(h[parent].parent_scope(h).unwrap().to_block())
            }
            Scope::Block(parent) => {
                // A variable scope is either a definition, sync, or block.
                Some(parent)
            }
        }
    }
    pub fn first(&self) -> StatementId {
        *self.statements.first().unwrap()
    }
}

impl SyntaxElement for BlockStatement {
    fn position(&self) -> InputPosition {
        self.position
    }
}

impl VariableScope for BlockStatement {
    fn parent_scope(&self, _h: &Heap) -> Option<Scope> {
        self.parent_scope
    }
    fn get_variable(&self, h: &Heap, id: SourceIdentifierId) -> Option<VariableId> {
        for &local in self.locals.iter() {
            if h[h[local].identifier] == h[id] {
                return Some(local.0);
            }
        }
        None
    }
}

#[derive(Debug, Clone)]
pub enum LocalStatement {
    Memory(MemoryStatement),
    Channel(ChannelStatement),
}

impl LocalStatement {
    pub fn this(&self) -> LocalStatementId {
        match self {
            LocalStatement::Memory(stmt) => stmt.this.upcast(),
            LocalStatement::Channel(stmt) => stmt.this.upcast(),
        }
    }
    pub fn as_memory(&self) -> &MemoryStatement {
        match self {
            LocalStatement::Memory(result) => result,
            _ => panic!("Unable to cast `LocalStatement` to `MemoryStatement`"),
        }
    }
    pub fn as_channel(&self) -> &ChannelStatement {
        match self {
            LocalStatement::Channel(result) => result,
            _ => panic!("Unable to cast `LocalStatement` to `ChannelStatement`"),
        }
    }
    pub fn next(&self) -> Option<StatementId> {
        match self {
            LocalStatement::Memory(stmt) => stmt.next,
            LocalStatement::Channel(stmt) => stmt.next,
        }
    }
}

impl SyntaxElement for LocalStatement {
    fn position(&self) -> InputPosition {
        match self {
            LocalStatement::Memory(stmt) => stmt.position(),
            LocalStatement::Channel(stmt) => stmt.position(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MemoryStatement {
    pub this: MemoryStatementId,
    // Phase 1: parser
    pub position: InputPosition,
    pub variable: LocalId,
    pub initial: ExpressionId,
    // Phase 2: linker
    pub next: Option<StatementId>,
}

impl SyntaxElement for MemoryStatement {
    fn position(&self) -> InputPosition {
        self.position
    }
}

#[derive(Debug, Clone)]
pub struct ChannelStatement {
    pub this: ChannelStatementId,
    // Phase 1: parser
    pub position: InputPosition,
    pub from: LocalId, // output
    pub to: LocalId,   // input
    // Phase 2: linker
    pub next: Option<StatementId>,
}

impl SyntaxElement for ChannelStatement {
    fn position(&self) -> InputPosition {
        self.position
    }
}

#[derive(Debug, Clone)]
pub struct SkipStatement {
    pub this: SkipStatementId,
    // Phase 1: parser
    pub position: InputPosition,
    // Phase 2: linker
    pub next: Option<StatementId>,
}

impl SyntaxElement for SkipStatement {
    fn position(&self) -> InputPosition {
        self.position
    }
}

#[derive(Debug, Clone)]
pub struct LabeledStatement {
    pub this: LabeledStatementId,
    // Phase 1: parser
    pub position: InputPosition,
    pub label: SourceIdentifierId,
    pub body: StatementId,
    // Phase 2: linker
    pub in_sync: Option<SynchronousStatementId>,
}

impl SyntaxElement for LabeledStatement {
    fn position(&self) -> InputPosition {
        self.position
    }
}

#[derive(Debug, Clone)]
pub struct IfStatement {
    pub this: IfStatementId,
    // Phase 1: parser
    pub position: InputPosition,
    pub test: ExpressionId,
    pub true_body: StatementId,
    pub false_body: StatementId,
}

impl SyntaxElement for IfStatement {
    fn position(&self) -> InputPosition {
        self.position
    }
}

#[derive(Debug, Clone)]
pub struct EndIfStatement {
    pub this: EndIfStatementId,
    // Phase 2: linker
    pub position: InputPosition, // of corresponding if statement
    pub next: Option<StatementId>,
}

impl SyntaxElement for EndIfStatement {
    fn position(&self) -> InputPosition {
        self.position
    }
}

#[derive(Debug, Clone)]
pub struct WhileStatement {
    pub this: WhileStatementId,
    // Phase 1: parser
    pub position: InputPosition,
    pub test: ExpressionId,
    pub body: StatementId,
    // Phase 2: linker
    pub next: Option<EndWhileStatementId>,
    pub in_sync: Option<SynchronousStatementId>,
}

impl SyntaxElement for WhileStatement {
    fn position(&self) -> InputPosition {
        self.position
    }
}

#[derive(Debug, Clone)]
pub struct EndWhileStatement {
    pub this: EndWhileStatementId,
    // Phase 2: linker
    pub position: InputPosition, // of corresponding while
    pub next: Option<StatementId>,
}

impl SyntaxElement for EndWhileStatement {
    fn position(&self) -> InputPosition {
        self.position
    }
}

#[derive(Debug, Clone)]
pub struct BreakStatement {
    pub this: BreakStatementId,
    // Phase 1: parser
    pub position: InputPosition,
    pub label: Option<SourceIdentifierId>,
    // Phase 2: linker
    pub target: Option<EndWhileStatementId>,
}

impl SyntaxElement for BreakStatement {
    fn position(&self) -> InputPosition {
        self.position
    }
}

#[derive(Debug, Clone)]
pub struct ContinueStatement {
    pub this: ContinueStatementId,
    // Phase 1: parser
    pub position: InputPosition,
    pub label: Option<SourceIdentifierId>,
    // Phase 2: linker
    pub target: Option<WhileStatementId>,
}

impl SyntaxElement for ContinueStatement {
    fn position(&self) -> InputPosition {
        self.position
    }
}

#[derive(Debug, Clone)]
pub struct SynchronousStatement {
    pub this: SynchronousStatementId,
    // Phase 1: parser
    pub position: InputPosition,
    pub parameters: Vec<ParameterId>,
    pub body: StatementId,
    // Phase 2: linker
    pub parent_scope: Option<Scope>,
}

impl SyntaxElement for SynchronousStatement {
    fn position(&self) -> InputPosition {
        self.position
    }
}

impl VariableScope for SynchronousStatement {
    fn parent_scope(&self, _h: &Heap) -> Option<Scope> {
        self.parent_scope
    }
    fn get_variable(&self, h: &Heap, id: SourceIdentifierId) -> Option<VariableId> {
        for &param in self.parameters.iter() {
            if h[h[param].identifier] == h[id] {
                return Some(param.0);
            }
        }
        None
    }
}

#[derive(Debug, Clone)]
pub struct EndSynchronousStatement {
    pub this: EndSynchronousStatementId,
    // Phase 2: linker
    pub position: InputPosition, // of corresponding sync statement
    pub next: Option<StatementId>,
}

impl SyntaxElement for EndSynchronousStatement {
    fn position(&self) -> InputPosition {
        self.position
    }
}

#[derive(Debug, Clone)]
pub struct ReturnStatement {
    pub this: ReturnStatementId,
    // Phase 1: parser
    pub position: InputPosition,
    pub expression: ExpressionId,
}

impl SyntaxElement for ReturnStatement {
    fn position(&self) -> InputPosition {
        self.position
    }
}

#[derive(Debug, Clone)]
pub struct AssertStatement {
    pub this: AssertStatementId,
    // Phase 1: parser
    pub position: InputPosition,
    pub expression: ExpressionId,
    // Phase 2: linker
    pub next: Option<StatementId>,
}

impl SyntaxElement for AssertStatement {
    fn position(&self) -> InputPosition {
        self.position
    }
}

#[derive(Debug, Clone)]
pub struct GotoStatement {
    pub this: GotoStatementId,
    // Phase 1: parser
    pub position: InputPosition,
    pub label: SourceIdentifierId,
    // Phase 2: linker
    pub target: Option<LabeledStatementId>,
}

impl SyntaxElement for GotoStatement {
    fn position(&self) -> InputPosition {
        self.position
    }
}

#[derive(Debug, Clone)]
pub struct NewStatement {
    pub this: NewStatementId,
    // Phase 1: parser
    pub position: InputPosition,
    pub expression: CallExpressionId,
    // Phase 2: linker
    pub next: Option<StatementId>,
}

impl SyntaxElement for NewStatement {
    fn position(&self) -> InputPosition {
        self.position
    }
}

#[derive(Debug, Clone)]
pub struct PutStatement {
    pub this: PutStatementId,
    // Phase 1: parser
    pub position: InputPosition,
    pub port: ExpressionId,
    pub message: ExpressionId,
    // Phase 2: linker
    pub next: Option<StatementId>,
}

impl SyntaxElement for PutStatement {
    fn position(&self) -> InputPosition {
        self.position
    }
}

#[derive(Debug, Clone)]
pub struct ExpressionStatement {
    pub this: ExpressionStatementId,
    // Phase 1: parser
    pub position: InputPosition,
    pub expression: ExpressionId,
    // Phase 2: linker
    pub next: Option<StatementId>,
}

impl SyntaxElement for ExpressionStatement {
    fn position(&self) -> InputPosition {
        self.position
    }
}

#[derive(Debug, Clone)]
pub enum Expression {
    Assignment(AssignmentExpression),
    Conditional(ConditionalExpression),
    Binary(BinaryExpression),
    Unary(UnaryExpression),
    Indexing(IndexingExpression),
    Slicing(SlicingExpression),
    Select(SelectExpression),
    Array(ArrayExpression),
    Constant(ConstantExpression),
    Call(CallExpression),
    Variable(VariableExpression),
}

impl Expression {
    pub fn as_assignment(&self) -> &AssignmentExpression {
        match self {
            Expression::Assignment(result) => result,
            _ => panic!("Unable to cast `Expression` to `AssignmentExpression`"),
        }
    }
    pub fn as_conditional(&self) -> &ConditionalExpression {
        match self {
            Expression::Conditional(result) => result,
            _ => panic!("Unable to cast `Expression` to `ConditionalExpression`"),
        }
    }
    pub fn as_binary(&self) -> &BinaryExpression {
        match self {
            Expression::Binary(result) => result,
            _ => panic!("Unable to cast `Expression` to `BinaryExpression`"),
        }
    }
    pub fn as_unary(&self) -> &UnaryExpression {
        match self {
            Expression::Unary(result) => result,
            _ => panic!("Unable to cast `Expression` to `UnaryExpression`"),
        }
    }
    pub fn as_indexing(&self) -> &IndexingExpression {
        match self {
            Expression::Indexing(result) => result,
            _ => panic!("Unable to cast `Expression` to `IndexingExpression`"),
        }
    }
    pub fn as_slicing(&self) -> &SlicingExpression {
        match self {
            Expression::Slicing(result) => result,
            _ => panic!("Unable to cast `Expression` to `SlicingExpression`"),
        }
    }
    pub fn as_select(&self) -> &SelectExpression {
        match self {
            Expression::Select(result) => result,
            _ => panic!("Unable to cast `Expression` to `SelectExpression`"),
        }
    }
    pub fn as_array(&self) -> &ArrayExpression {
        match self {
            Expression::Array(result) => result,
            _ => panic!("Unable to cast `Expression` to `ArrayExpression`"),
        }
    }
    pub fn as_constant(&self) -> &ConstantExpression {
        match self {
            Expression::Constant(result) => result,
            _ => panic!("Unable to cast `Expression` to `ConstantExpression`"),
        }
    }
    pub fn as_call(&self) -> &CallExpression {
        match self {
            Expression::Call(result) => result,
            _ => panic!("Unable to cast `Expression` to `CallExpression`"),
        }
    }
    pub fn as_call_mut(&mut self) -> &mut CallExpression {
        match self {
            Expression::Call(result) => result,
            _ => panic!("Unable to cast `Expression` to `CallExpression`"),
        }
    }
    pub fn as_variable(&self) -> &VariableExpression {
        match self {
            Expression::Variable(result) => result,
            _ => panic!("Unable to cast `Expression` to `VariableExpression`"),
        }
    }
    pub fn as_variable_mut(&mut self) -> &mut VariableExpression {
        match self {
            Expression::Variable(result) => result,
            _ => panic!("Unable to cast `Expression` to `VariableExpression`"),
        }
    }
}

impl SyntaxElement for Expression {
    fn position(&self) -> InputPosition {
        match self {
            Expression::Assignment(expr) => expr.position(),
            Expression::Conditional(expr) => expr.position(),
            Expression::Binary(expr) => expr.position(),
            Expression::Unary(expr) => expr.position(),
            Expression::Indexing(expr) => expr.position(),
            Expression::Slicing(expr) => expr.position(),
            Expression::Select(expr) => expr.position(),
            Expression::Array(expr) => expr.position(),
            Expression::Constant(expr) => expr.position(),
            Expression::Call(expr) => expr.position(),
            Expression::Variable(expr) => expr.position(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum AssignmentOperator {
    Set,
    Multiplied,
    Divided,
    Remained,
    Added,
    Subtracted,
    ShiftedLeft,
    ShiftedRight,
    BitwiseAnded,
    BitwiseXored,
    BitwiseOred,
}

#[derive(Debug, Clone)]
pub struct AssignmentExpression {
    pub this: AssignmentExpressionId,
    // Phase 1: parser
    pub position: InputPosition,
    pub left: ExpressionId,
    pub operation: AssignmentOperator,
    pub right: ExpressionId,
}

impl SyntaxElement for AssignmentExpression {
    fn position(&self) -> InputPosition {
        self.position
    }
}

#[derive(Debug, Clone)]
pub struct ConditionalExpression {
    pub this: ConditionalExpressionId,
    // Phase 1: parser
    pub position: InputPosition,
    pub test: ExpressionId,
    pub true_expression: ExpressionId,
    pub false_expression: ExpressionId,
}

impl SyntaxElement for ConditionalExpression {
    fn position(&self) -> InputPosition {
        self.position
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinaryOperator {
    Concatenate,
    LogicalOr,
    LogicalAnd,
    BitwiseOr,
    BitwiseXor,
    BitwiseAnd,
    Equality,
    Inequality,
    LessThan,
    GreaterThan,
    LessThanEqual,
    GreaterThanEqual,
    ShiftLeft,
    ShiftRight,
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
}

#[derive(Debug, Clone)]
pub struct BinaryExpression {
    pub this: BinaryExpressionId,
    // Phase 1: parser
    pub position: InputPosition,
    pub left: ExpressionId,
    pub operation: BinaryOperator,
    pub right: ExpressionId,
}

impl SyntaxElement for BinaryExpression {
    fn position(&self) -> InputPosition {
        self.position
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnaryOperation {
    Positive,
    Negative,
    BitwiseNot,
    LogicalNot,
    PreIncrement,
    PreDecrement,
    PostIncrement,
    PostDecrement,
}

#[derive(Debug, Clone)]
pub struct UnaryExpression {
    pub this: UnaryExpressionId,
    // Phase 1: parser
    pub position: InputPosition,
    pub operation: UnaryOperation,
    pub expression: ExpressionId,
}

impl SyntaxElement for UnaryExpression {
    fn position(&self) -> InputPosition {
        self.position
    }
}

#[derive(Debug, Clone)]
pub struct IndexingExpression {
    pub this: IndexingExpressionId,
    // Phase 1: parser
    pub position: InputPosition,
    pub subject: ExpressionId,
    pub index: ExpressionId,
}

impl SyntaxElement for IndexingExpression {
    fn position(&self) -> InputPosition {
        self.position
    }
}

#[derive(Debug, Clone)]
pub struct SlicingExpression {
    pub this: SlicingExpressionId,
    // Phase 1: parser
    pub position: InputPosition,
    pub subject: ExpressionId,
    pub from_index: ExpressionId,
    pub to_index: ExpressionId,
}

impl SyntaxElement for SlicingExpression {
    fn position(&self) -> InputPosition {
        self.position
    }
}

#[derive(Debug, Clone)]
pub struct SelectExpression {
    pub this: SelectExpressionId,
    // Phase 1: parser
    pub position: InputPosition,
    pub subject: ExpressionId,
    pub field: Field,
}

impl SyntaxElement for SelectExpression {
    fn position(&self) -> InputPosition {
        self.position
    }
}

#[derive(Debug, Clone)]
pub struct ArrayExpression {
    pub this: ArrayExpressionId,
    // Phase 1: parser
    pub position: InputPosition,
    pub elements: Vec<ExpressionId>,
}

impl SyntaxElement for ArrayExpression {
    fn position(&self) -> InputPosition {
        self.position
    }
}

#[derive(Debug, Clone)]
pub struct CallExpression {
    pub this: CallExpressionId,
    // Phase 1: parser
    pub position: InputPosition,
    pub method: Method,
    pub arguments: Vec<ExpressionId>,
    // Phase 2: linker
    pub declaration: Option<DeclarationId>,
}

impl SyntaxElement for CallExpression {
    fn position(&self) -> InputPosition {
        self.position
    }
}

#[derive(Debug, Clone)]
pub struct ConstantExpression {
    pub this: ConstantExpressionId,
    // Phase 1: parser
    pub position: InputPosition,
    pub value: Constant,
}

impl SyntaxElement for ConstantExpression {
    fn position(&self) -> InputPosition {
        self.position
    }
}

#[derive(Debug, Clone)]
pub struct VariableExpression {
    pub this: VariableExpressionId,
    // Phase 1: parser
    pub position: InputPosition,
    pub identifier: SourceIdentifierId,
    // Phase 2: linker
    pub declaration: Option<VariableId>,
}

impl SyntaxElement for VariableExpression {
    fn position(&self) -> InputPosition {
        self.position
    }
}
