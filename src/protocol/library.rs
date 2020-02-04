use crate::protocol::ast::*;
use crate::protocol::inputsource::*;

pub fn get_declarations(h: &mut Heap, i: ImportId) -> Result<Vec<DeclarationId>, ParseError> {
    if h[i].value == b"std.reo" {
        let mut vec = Vec::new();
        vec.push(cd(h, i, b"sync", &[Type::INPUT, Type::OUTPUT]));
        vec.push(cd(h, i, b"syncdrain", &[Type::INPUT, Type::INPUT]));
        vec.push(cd(h, i, b"syncspout", &[Type::OUTPUT, Type::OUTPUT]));
        vec.push(cd(h, i, b"asyncdrain", &[Type::INPUT, Type::INPUT]));
        vec.push(cd(h, i, b"asyncspout", &[Type::OUTPUT, Type::OUTPUT]));
        vec.push(cd(h, i, b"merger", &[Type::INPUT_ARRAY, Type::OUTPUT]));
        vec.push(cd(h, i, b"router", &[Type::INPUT, Type::OUTPUT_ARRAY]));
        vec.push(cd(h, i, b"consensus", &[Type::INPUT_ARRAY, Type::OUTPUT]));
        vec.push(cd(h, i, b"replicator", &[Type::INPUT, Type::OUTPUT_ARRAY]));
        vec.push(cd(h, i, b"alternator", &[Type::INPUT_ARRAY, Type::OUTPUT]));
        vec.push(cd(h, i, b"roundrobin", &[Type::INPUT, Type::OUTPUT_ARRAY]));
        vec.push(cd(h, i, b"node", &[Type::INPUT_ARRAY, Type::OUTPUT_ARRAY]));
        vec.push(cd(h, i, b"fifo", &[Type::INPUT, Type::OUTPUT]));
        vec.push(cd(h, i, b"xfifo", &[Type::INPUT, Type::OUTPUT, Type::MESSAGE]));
        vec.push(cd(h, i, b"nfifo", &[Type::INPUT, Type::OUTPUT, Type::INT]));
        vec.push(cd(h, i, b"ufifo", &[Type::INPUT, Type::OUTPUT]));
        Ok(vec)
    } else if h[i].value == b"std.buf" {
        let mut vec = Vec::new();
        vec.push(fd(h, i, b"writeByte", Type::BYTE, &[Type::MESSAGE, Type::INT, Type::BYTE]));
        vec.push(fd(h, i, b"writeShort", Type::SHORT, &[Type::MESSAGE, Type::INT, Type::SHORT]));
        vec.push(fd(h, i, b"writeInt", Type::INT, &[Type::MESSAGE, Type::INT, Type::INT]));
        vec.push(fd(h, i, b"writeLong", Type::LONG, &[Type::MESSAGE, Type::INT, Type::LONG]));
        vec.push(fd(h, i, b"readByte", Type::BYTE, &[Type::MESSAGE, Type::INT]));
        vec.push(fd(h, i, b"readShort", Type::SHORT, &[Type::MESSAGE, Type::INT]));
        vec.push(fd(h, i, b"readInt", Type::INT, &[Type::MESSAGE, Type::INT]));
        vec.push(fd(h, i, b"readLong", Type::LONG, &[Type::MESSAGE, Type::INT]));
        Ok(vec)
    } else {
        Err(ParseError::new(h[i].position, "Unknown import"))
    }
}

fn cd(h: &mut Heap, import: ImportId, ident: &[u8], sig: &[Type]) -> DeclarationId {
    let identifier = h.get_external_identifier(ident).upcast();
    h.alloc_imported_declaration(|this| ImportedDeclaration {
        this,
        import,
        signature: Signature::Component(ComponentSignature { identifier, arity: sig.to_vec() }),
    })
    .upcast()
}

fn fd(h: &mut Heap, import: ImportId, ident: &[u8], ret: Type, sig: &[Type]) -> DeclarationId {
    let identifier = h.get_external_identifier(ident).upcast();
    h.alloc_imported_declaration(|this| ImportedDeclaration {
        this,
        import,
        signature: Signature::Function(FunctionSignature {
            return_type: ret,
            identifier,
            arity: sig.to_vec(),
        }),
    })
    .upcast()
}
